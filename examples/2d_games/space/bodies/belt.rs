use frglib::Color;
use std::f32::consts::TAU;

/// Один небольшой объект в поясе.
pub struct BeltParticle {
    pub orbit_radius: f32,
    pub size: f32,
    pub speed: f32,
    pub start_angle: f32,
    pub color: Color,
}

/// Пояс астероидов или пояс Койпера.
pub struct Belt {
    pub particles: Vec<BeltParticle>,
}

/// Простой линейный генератор случайных чисел:
/// пояс должен выглядеть одинаково при каждом запуске.
struct Random {
    seed: u32,
}

impl Random {
    fn new(seed: u32) -> Self {
        Self { seed }
    }

    /// Следующее число в диапазоне `0.0..1.0`.
    fn next(&mut self) -> f32 {
        self.seed = self
            .seed
            .wrapping_mul(1664525)
            .wrapping_add(1013904223);

        (self.seed >> 8) as f32 / 16_777_216.0
    }

    fn range(&mut self, min: f32, max: f32) -> f32 {
        min + self.next() * (max - min)
    }
}

/// Пояс астероидов -- между Марсом и Юпитером.
pub fn get_asteroid_belt() -> Belt {
    let mut random = Random::new(0xA57E_2019);
    let mut particles = Vec::with_capacity(180);

    for _ in 0..180 {
        let orbit_radius = random.range(178.0, 205.0);

        // Чем дальше от Солнца, тем медленнее движение по орбите.
        let speed = 0.60 * (190.0 / orbit_radius).sqrt();

        let brightness = random.range(0.65, 1.0);
        let color = Color::rgb(
            (135.0 * brightness) as u8,
            (125.0 * brightness) as u8,
            (115.0 * brightness) as u8,
        );

        particles.push(BeltParticle {
            orbit_radius,
            size: random.range(1.0, 2.6),
            speed,
            start_angle: random.range(0.0, TAU),
            color,
        });
    }

    Belt { particles }
}

/// Пояс Койпера -- за Нептуном.
pub fn get_kuiper_belt() -> Belt {
    let mut random = Random::new(0xC01D_2E4D);
    let mut particles = Vec::with_capacity(320);

    for _ in 0..320 {
        let orbit_radius = random.range(418.0, 442.0);

        let speed = 0.16 * (425.0 / orbit_radius).sqrt();

        let brightness = random.range(0.65, 1.0);
        let color = Color::rgb(
            (135.0 * brightness) as u8,
            (125.0 * brightness) as u8,
            (115.0 * brightness) as u8,
        );

        particles.push(BeltParticle {
            orbit_radius,
            size: random.range(1.0, 2.4),
            speed,
            start_angle: random.range(0.0, TAU),
            color,
        });
    }

    Belt { particles }
}
