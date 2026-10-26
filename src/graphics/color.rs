//! Colour, kept separate from SDL's own type.

use crate::math::{Ease, Lerp};

/// An 8-bit-per-channel RGBA colour.
///
/// Deliberately not `sdl3::pixels::Color`: exposing that would tie this crate's
/// public API to a particular version of `sdl3`, so every dependency bump would
/// be a breaking change for whoever depends on us.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Color = Color::rgba(0, 0, 0, 255);
    pub const WHITE: Color = Color::rgba(255, 255, 255, 255);
    pub const FON: Color = Color::rgba(20, 25, 30, 255);

    pub const RED: Color = Color::rgba(200, 0, 0, 255);
    pub const GREEN: Color = Color::rgba(0, 200, 0, 255);
    pub const BLUE: Color = Color::rgba(0, 0, 200, 255);

    /// Opaque colour, `a = 255`.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color { r, g, b, a }
    }

    pub const fn to_array(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl From<(u8, u8, u8)> for Color {
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Color::rgb(r, g, b)
    }
}

impl From<(u8, u8, u8, u8)> for Color {
    fn from((r, g, b, a): (u8, u8, u8, u8)) -> Self {
        Color::rgba(r, g, b, a)
    }
}

impl From<[u8; 4]> for Color {
    fn from([r, g, b, a]: [u8; 4]) -> Self {
        Color::rgba(r, g, b, a)
    }
}

impl Lerp for Color {
    /// Every channel moves at the same rate, then lands back on a whole
    /// number: `u8` has nowhere to keep the fraction.
    ///
    /// Unlike the float types this stops at `0` and `255` rather than
    /// extrapolating, because `u8` cannot hold a value outside its own range.
    fn lerp(self, other: Color, t: f32, ease: Ease) -> Color {
        let t = ease.apply(t);
        let mix = |from: u8, to: u8| {
            let value = f32::from(from) + (f32::from(to) - f32::from(from)) * t;
            value.round().clamp(0.0, 255.0) as u8
        };

        Color::rgba(
            mix(self.r, other.r),
            mix(self.g, other.g),
            mix(self.b, other.b),
            mix(self.a, other.a),
        )
    }
}

impl From<Color> for sdl3::pixels::Color {
    fn from(color: Color) -> Self {
        sdl3::pixels::Color {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three named colours are the one thing in this file a reader reaches
    /// for by reflex, and they were all the same green at one point -- shipping
    /// `Color::RED` that renders green. Worth pinning down.
    #[test]
    fn named_colours_point_where_they_say() {
        assert_eq!(Color::RED, Color::rgb(200, 0, 0));
        assert_eq!(Color::GREEN, Color::rgb(0, 200, 0));
        assert_eq!(Color::BLUE, Color::rgb(0, 0, 200));
    }

    #[test]
    fn black_and_white_are_the_extremes() {
        assert_eq!(Color::BLACK, Color::rgb(0, 0, 0));
        assert_eq!(Color::WHITE, Color::rgb(255, 255, 255));
    }

    #[test]
    fn the_named_colours_are_all_opaque() {
        for colour in [
            Color::RED,
            Color::GREEN,
            Color::BLUE,
            Color::BLACK,
            Color::WHITE,
        ] {
            assert_eq!(colour.a, 255, "{colour:?} should be opaque");
        }
    }

    #[test]
    fn the_named_colours_are_all_different() {
        let all = [
            Color::RED,
            Color::GREEN,
            Color::BLUE,
            Color::BLACK,
            Color::WHITE,
        ];

        for (i, one) in all.iter().enumerate() {
            for other in &all[i + 1..] {
                assert_ne!(one, other, "{one:?} and {other:?} are the same");
            }
        }
    }

    #[test]
    fn rgb_is_opaque_and_rgba_is_not() {
        assert_eq!(Color::rgb(1, 2, 3).a, 255);
        assert_eq!(Color::rgba(1, 2, 3, 4).a, 4);
    }

    #[test]
    fn channels_survive_a_round_trip_through_an_array() {
        let colour = Color::rgba(10, 20, 30, 40);
        assert_eq!(colour.to_array(), [10, 20, 30, 40]);
        assert_eq!(Color::from(colour.to_array()), colour);
    }

    #[test]
    fn tuples_and_triples_convert() {
        assert_eq!(Color::from((1, 2, 3)), Color::rgb(1, 2, 3));
        assert_eq!(Color::from((1, 2, 3, 4)), Color::rgba(1, 2, 3, 4));
        // The three-channel form cannot carry alpha, so it comes out opaque.
        assert_eq!(Color::from((1, 2, 3)).a, 255);
    }

    #[test]
    fn converting_to_the_sdl_type_keeps_every_channel() {
        let sdl: sdl3::pixels::Color = Color::rgba(10, 20, 30, 40).into();
        let (r, g, b, a) = sdl.rgba();
        assert_eq!((r, g, b, a), (10, 20, 30, 40));
    }

    #[test]
    fn colours_lerp_channel_by_channel() {
        // 127.5 lands on 128: u8 keeps whole numbers only.
        assert_eq!(
            Color::BLACK.lerp(Color::WHITE, 0.5, Ease::Linear),
            Color::rgb(128, 128, 128)
        );

        let from = Color::rgba(10, 20, 30, 40);
        let to = Color::rgba(30, 40, 50, 60);
        assert_eq!(
            from.lerp(Color::rgba(20, 30, 40, 50), 0.0, Ease::Linear),
            from
        );
        assert_eq!(from.lerp(to, 1.0, Ease::Linear), to);
    }

    #[test]
    fn the_free_function_lerps_colours_too() {
        use crate::math::lerp;

        assert_eq!(
            lerp(Color::BLACK, Color::WHITE, 1.0, Ease::Linear),
            Color::WHITE
        );
    }

    /// A weight past `1.0` would ask for a channel beyond `255`, or below
    /// `0`: `u8` has nowhere to put it, so the channel stops at the edge.
    #[test]
    fn a_channel_stops_at_the_edge_of_its_range() {
        assert_eq!(
            Color::BLACK.lerp(Color::WHITE, 2.0, Ease::Linear),
            Color::WHITE
        );
        assert_eq!(
            Color::WHITE.lerp(Color::BLACK, 2.0, Ease::Linear),
            Color::BLACK
        );
    }
}
