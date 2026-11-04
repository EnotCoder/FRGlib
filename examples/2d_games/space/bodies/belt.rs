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
    let mut particles = Vec::with_capacity(250);

    for _ in 0..250 {
        let orbit_radius = random.range(300.0, 400.0);

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
            size: random.range(2.0, 3.6),
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
    let mut particles = Vec::with_capacity(600);

    for _ in 0..600 {
        let orbit_radius = random.range(900.0, 1050.0);

        let speed = 0.16 * (425.0 / orbit_radius).sqrt();

        let brightness = random.range(0.65, 1.0);
        let color = Color::rgb(
            (135.0 * brightness) as u8,
            (125.0 * brightness) as u8,
            (115.0 * brightness) as u8,
        );

        particles.push(BeltParticle {
            orbit_radius,
            size: random.range(2.5, 3.5),
            speed,
            start_angle: random.range(0.0, TAU),
            color,
        });
    }

    Belt { particles }
}

/// Облоко Орта -- за Поясом Койпера.
pub fn get_orte_belt() -> Belt {
    let mut random = Random::new(0xC01D_2E4D);
    let mut particles = Vec::with_capacity(3000);

    for _ in 0..3000 {
        let orbit_radius = random.range(1200.0, 1900.0);

        let speed = 0.05 * (425.0 / orbit_radius).sqrt();

        let brightness = random.range(0.65, 1.0);
        let color = Color::rgb(
            (100.0 * brightness) as u8,
            (120.0 * brightness) as u8,
            (150.0 * brightness) as u8,
        );

        particles.push(BeltParticle {
            orbit_radius,
            size: random.range(2.4, 3.4),
            speed,
            start_angle: random.range(0.0, TAU),
            color,
        });
    }

    Belt { particles }
}
