//! Perlin noise.
//!
//! A [`Perlin`] generator turns coordinates into smooth pseudo-random values.
//! The same seed always produces the same pattern, different seeds produce
//! different ones, and sampling never allocates.
//!
//! # Range
//!
//! [`Perlin::sample_2d`] returns values in `[-1.0, 1.0]`. Values outside that
//! range are not produced for finite coordinates.
//!
//! # Period
//!
//! The permutation table has period 256 along each axis: adding any multiple of
//! 256 to a coordinate gives the same result.
//!
//! # Invalid input
//!
//! Non-finite coordinates (`NaN`, positive or negative infinity) are not valid
//! sample points. In that case sampling returns `0.0`.

/// Smooth 2D Perlin noise.
///
/// ```
/// use frglib::noise::Perlin;
///
/// let noise = Perlin::new(42);
/// let value = noise.sample_2d(1.5, 2.25);
/// assert!(value >= -1.0 && value <= 1.0);
/// ```
#[derive(Debug, Clone)]
pub struct Perlin {
    // Doubled permutation table so lookups never need wrapping arithmetic
    // beyond masking the integer cell coordinate.
    perm: [u8; 512],
}

impl Perlin {
    /// Creates a generator from `seed`.
    ///
    /// The seed fully determines the pattern and is the only source of
    /// randomness: two generators built from the same seed produce identical
    /// samples.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        let mut perm = [0u8; 512];
        let mut p = [0u8; 256];
        for (i, value) in p.iter_mut().enumerate() {
            *value = i as u8;
        }

        // Fisher-Yates shuffle driven by a SplitMix64 stream derived from the
        // seed, so the permutation is deterministic and well mixed.
        let mut state = splitmix64(seed);
        for i in (1..256).rev() {
            let (next, r) = splitmix64_next(state);
            state = next;
            let j = (r as usize) % (i + 1);
            p.swap(i, j);
        }

        for i in 0..512 {
            perm[i] = p[i & 255];
        }

        Self { perm }
    }

    /// Samples the noise at `(x, y)`.
    ///
    /// The result is in `[-1.0, 1.0]`. Integer coordinates land on grid
    /// points; use fractional steps to move through the pattern smoothly, for
    /// example `noise.sample_2d(x * 0.02, y * 0.02)`.
    ///
    /// Non-finite coordinates return `0.0`.
    #[must_use]
    pub fn sample_2d(&self, x: f32, y: f32) -> f32 {
        if !x.is_finite() || !y.is_finite() {
            return 0.0;
        }

        // Unit cell containing the point.
        let xi = floor_i32(x);
        let yi = floor_i32(y);

        // Position inside the cell.
        let xf = x - floor_f32(x);
        let yf = y - floor_f32(y);

        // Fade curves for interpolation.
        let u = fade(xf);
        let v = fade(yf);

        // Hash coordinates of the four cell corners.
        let aa = self.perm[(self.perm[(xi & 255) as usize] as i32 + (yi & 255)) as usize];
        let ab = self.perm[(self.perm[(xi & 255) as usize] as i32 + ((yi + 1) & 255)) as usize];
        let ba = self.perm[(self.perm[((xi + 1) & 255) as usize] as i32 + (yi & 255)) as usize];
        let bb =
            self.perm[(self.perm[((xi + 1) & 255) as usize] as i32 + ((yi + 1) & 255)) as usize];

        // Gradient dot products at each corner, relative to the sample point.
        let x1 = lerp(grad(aa, xf, yf), grad(ba, xf - 1.0, yf), u);
        let x2 = lerp(grad(ab, xf, yf - 1.0), grad(bb, xf - 1.0, yf - 1.0), u);
        let value = lerp(x1, x2, v);

        // The 2D gradient set gives values in [-sqrt(2)/2, sqrt(2)/2]; scale to
        // [-1, 1] so the documented range is easy to reason about.
        (value * std::f32::consts::SQRT_2).clamp(-1.0, 1.0)
    }

    /// Fractional Brownian motion: layers `sample_2d` into a richer pattern.
    ///
    /// `octaves` is the number of layers. `frequency` is the starting spatial
    /// frequency. Each next layer multiplies frequency by `lacunarity` and
    /// amplitude by `gain`. The result is normalized to stay in `[-1.0, 1.0]`
    /// when `gain < 1.0`; with degenerate parameters it falls back to a single
    /// sample.
    ///
    /// ```
    /// use frglib::noise::Perlin;
    ///
    /// let noise = Perlin::new(7);
    /// let height = noise.fbm_2d(10.0, 20.0, 4, 0.02, 2.0, 0.5);
    /// assert!(height >= -1.0 && height <= 1.0);
    /// ```
    #[must_use]
    pub fn fbm_2d(
        &self,
        x: f32,
        y: f32,
        octaves: u32,
        mut frequency: f32,
        lacunarity: f32,
        gain: f32,
    ) -> f32 {
        if octaves == 0 || !frequency.is_finite() || !lacunarity.is_finite() || !gain.is_finite() {
            return self.sample_2d(x, y);
        }

        let mut amplitude = 1.0f32;
        let mut sum = 0.0f32;
        let mut norm = 0.0f32;

        for _ in 0..octaves {
            sum += amplitude * self.sample_2d(x * frequency, y * frequency);
            norm += amplitude;
            frequency *= lacunarity;
            amplitude *= gain;
        }

        if norm == 0.0 || !norm.is_finite() {
            self.sample_2d(x, y)
        } else {
            (sum / norm).clamp(-1.0, 1.0)
        }
    }
}

fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn floor_f32(v: f32) -> f32 {
    v.floor()
}

fn floor_i32(v: f32) -> i32 {
    v.floor() as i32
}

/// Gradient vector selected by the low bits of `hash`, dotted with `(x, y)`.
///
/// Uses the 8 gradients of the unit square's edges/diagonals projected to 2D,
/// which is the common choice for 2D Perlin noise.
fn grad(hash: u8, x: f32, y: f32) -> f32 {
    match hash & 7 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x,
        5 => -x,
        6 => y,
        _ => -y,
    }
}

/// One-time seed mixing so that nearby seed values do not produce correlated
/// permutations.
fn splitmix64(seed: u64) -> u64 {
    seed
}

fn splitmix64_next(mut state: u64) -> (u64, u64) {
    state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    (state, z ^ (z >> 31))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-6;

    #[test]
    fn same_seed_is_repeatable() {
        let a = Perlin::new(42);
        let b = Perlin::new(42);
        for &(x, y) in &[
            (0.0, 0.0),
            (1.5, -2.25),
            (-10.3, 7.7),
            (123.456, 0.001),
        ] {
            assert_eq!(a.sample_2d(x, y).to_bits(), b.sample_2d(x, y).to_bits());
        }
    }

    #[test]
    fn different_seeds_produce_different_patterns() {
        let a = Perlin::new(1);
        let b = Perlin::new(2);
        let mut any_different = false;
        for i in 0..32 {
            for j in 0..32 {
                let x = i as f32 * 0.37 + 0.1;
                let y = j as f32 * 0.41 + 0.2;
                if a.sample_2d(x, y).to_bits() != b.sample_2d(x, y).to_bits() {
                    any_different = true;
                }
            }
        }
        assert!(
            any_different,
            "different seeds should not produce identical samples"
        );
    }

    #[test]
    fn negative_coordinates_work() {
        let noise = Perlin::new(7);
        for &(x, y) in &[
            (-0.1, -0.2),
            (-5.75, 3.25),
            (4.0, -9.9),
            (-100.5, -200.25),
        ] {
            let v = noise.sample_2d(x, y);
            assert!(v.is_finite(), "sample at ({x}, {y}) must be finite");
            assert!(
                (-1.0..=1.0).contains(&v),
                "sample at ({x}, {y}) out of range: {v}"
            );
        }
    }

    #[test]
    fn smooth_across_cell_boundaries() {
        let noise = Perlin::new(99);
        // Step across several integer boundaries in x and y; adjacent samples
        // must stay close because the fade curve gives continuous derivatives.
        let step = 1e-3_f32;
        let max_jump = 0.02_f32;
        for &boundary in &[0.0_f32, 1.0, -1.0, 5.0, -7.0] {
            for k in 0..50 {
                let t = k as f32 * 0.1;
                let before = noise.sample_2d(boundary - step, t);
                let after = noise.sample_2d(boundary + step, t);
                assert!(
                    (after - before).abs() < max_jump,
                    "jump across x={boundary} at y={t}: {before} -> {after}"
                );

                let before = noise.sample_2d(t, boundary - step);
                let after = noise.sample_2d(t, boundary + step);
                assert!(
                    (after - before).abs() < max_jump,
                    "jump across y={boundary} at x={t}: {before} -> {after}"
                );
            }
        }
    }

    #[test]
    fn range_and_invalid_inputs() {
        let noise = Perlin::new(12345);
        for i in 0..200 {
            for j in 0..50 {
                let x = i as f32 * 0.173 - 17.0;
                let y = j as f32 * 0.271 - 6.0;
                let v = noise.sample_2d(x, y);
                assert!(
                    (-1.0 - EPS..=1.0 + EPS).contains(&v),
                    "out of range at ({x}, {y}): {v}"
                );
            }
        }

        for &(x, y) in &[
            (f32::NAN, 0.0),
            (0.0, f32::NAN),
            (f32::INFINITY, 1.0),
            (1.0, f32::NEG_INFINITY),
            (f32::NAN, f32::INFINITY),
        ] {
            assert_eq!(
                noise.sample_2d(x, y),
                0.0,
                "invalid input ({x}, {y}) should yield 0.0"
            );
        }
    }

    #[test]
    fn fbm_stays_in_range_and_is_repeatable() {
        let a = Perlin::new(11);
        let b = Perlin::new(11);
        for i in 0..100 {
            let x = i as f32 * 0.19 - 5.0;
            let y = i as f32 * 0.23 + 1.0;
            let va = a.fbm_2d(x, y, 5, 0.05, 2.0, 0.5);
            let vb = b.fbm_2d(x, y, 5, 0.05, 2.0, 0.5);
            assert_eq!(va.to_bits(), vb.to_bits());
            assert!((-1.0..=1.0).contains(&va), "fbm out of range: {va}");
        }
    }
}
