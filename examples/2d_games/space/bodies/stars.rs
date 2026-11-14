use frglib::{Color, Vec2};
use std::f32::consts::TAU;

use super::Random;

/// Цвета звёзд: белые, голубоватые, тёплые и холодные.
pub const STAR_PALETTE: [Color; 4] = [
    Color::rgb(255, 255, 255),
    Color::rgb(195, 220, 255),
    Color::rgb(255, 236, 205),
    Color::rgb(160, 185, 255),
];

pub struct Star {
    /// Положение в долях окна, `0.0..1.0` по каждой оси.
    pub position: Vec2,
    pub size: f32,
    pub palette: u8,
    pub phase: f32,
    pub twinkle_speed: f32,
    pub base_alpha: f32,
    /// Насколько сильно звезда смещается при движении камеры.
    pub parallax: f32,
}

pub fn get_stars() -> Vec<Star> {
    let mut random = Random::new(0x5EED_1234);
    let mut stars = Vec::new();

    // Три слоя глубины: дальние мелкие и тусклые, ближние крупнее и ярче.
    for (count, parallax, (min_size, max_size), base_alpha) in [
        (170, 0.025, (0.8, 1.3), 160.0),
        (90, 0.060, (1.2, 1.9), 205.0),
        (36, 0.120, (1.8, 2.8), 255.0),
    ] {
        for _ in 0..count {
            stars.push(Star {
                position: Vec2::new(random.next(), random.next()),
                size: random.range(min_size, max_size),
                palette: (random.next() * STAR_PALETTE.len() as f32) as u8,
                phase: random.range(0.0, TAU),
                twinkle_speed: random.range(0.6, 2.2),
                base_alpha,
                parallax,
            });
        }
    }

    stars
}
