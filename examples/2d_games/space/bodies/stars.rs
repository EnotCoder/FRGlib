use frglib::{Color, Vec2};

pub struct Star {
    pub position: Vec2,
    pub size: f32,
    pub color: Color,
}

pub fn get_stars() -> Vec<Star>{
    // Создаём звёзды один раз.
    let mut stars = Vec::new();
    let mut seed: u32 = 123456789;

    let mut next_random = || {
        seed = seed
            .wrapping_mul(1664525)
            .wrapping_add(1013904223);

        (seed >> 8) as f32 / 16_777_216.0
    };

    for _ in 0..250 {
        let x = next_random();
        let y = next_random();

        let size = 1.0 + next_random() * 1.5;
        let brightness = (90.0 + next_random() * 150.0) as u8;

        stars.push(Star {
            position: Vec2::new(x, y),
            size,
            color: Color::rgb(brightness, brightness, brightness),
        });
    }

    return stars;
}