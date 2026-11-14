use frglib::Color;
use std::f32::consts::TAU;

use super::Random;

/// Один небольшой объект в поясе.
pub struct BeltParticle {
    pub orbit_radius: f32,
    pub size: f32,
    pub speed: f32,
    pub start_angle: f32,
    /// Оттенок частицы, 0..[`SHADES`]. Частицы одного оттенка рисуются
    /// одним вызовом, поэтому пояс стоит несколько вызовов, а не тысячи.
    pub shade: u8,
}

/// Пояс астероидов, пояс Койпера или облако Оорта.
pub struct Belt {
    pub particles: Vec<BeltParticle>,
    pub base_color: Color,
}

/// Сколько оттенков у частиц пояса.
pub const SHADES: u8 = 4;

/// Множитель яркости для каждого оттенка: от тусклого к яркому.
pub const SHADE_BRIGHTNESS: [f32; SHADES as usize] = [0.55, 0.70, 0.85, 1.0];

fn make_belt(
    seed: u32,
    count: usize,
    radius: (f32, f32),
    speed_base: f32,
    base_color: Color,
) -> Belt {
    let mut random = Random::new(seed);
    let mut particles = Vec::with_capacity(count);

    for _ in 0..count {
        let orbit_radius = random.range(radius.0, radius.1);

        // Чем дальше от Солнца, тем медленнее движение по орбите.
        let speed = speed_base * (190.0 / orbit_radius).sqrt();

        particles.push(BeltParticle {
            orbit_radius,
            size: random.range(2.0, 3.6),
            speed,
            start_angle: random.range(0.0, TAU),
            shade: (random.next() * SHADES as f32) as u8,
        });
    }

    Belt {
        particles,
        base_color,
    }
}

/// Пояс астероидов -- между Марсом и Юпитером.
pub fn get_asteroid_belt() -> Belt {
    make_belt(
        0xA57E_2019,
        240,
        (300.0, 400.0),
        0.60,
        Color::rgb(135, 125, 115),
    )
}

/// Пояс Койпера -- за Нептуном.
pub fn get_kuiper_belt() -> Belt {
    make_belt(
        0xC01D_2E4D,
        520,
        (900.0, 1050.0),
        0.16,
        Color::rgb(135, 125, 115),
    )
}

/// Облако Оорта -- за Поясом Койпера.
pub fn get_orte_belt() -> Belt {
    make_belt(
        0x0A17_0BEE,
        2200,
        (1200.0, 1900.0),
        0.05,
        Color::rgb(100, 120, 150),
    )
}
