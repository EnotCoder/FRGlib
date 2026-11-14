use frglib::{Camera, Color, Draw, Vec2};

use crate::bodies::{Star, STAR_PALETTE};

use super::poly::fill_quads;

/// Рисует звёзды в экранных координатах.
///
/// У каждой звезды свой слой параллакса: при движении камеры дальние звёзды
/// почти стоят, ближние заметно смещаются. Мерцание -- синус со своей фазой.
/// Тысячи квадов собираются в 16 батчей по цвету и яркости.
pub fn draw_stars(
    d: &mut Draw,
    stars: &[Star],
    window_size: Vec2,
    camera: Camera,
    elapsed: f32,
) -> Result<(), frglib::Error> {
    // 4 цвета x 4 уровня прозрачности.
    let mut buckets: [Vec<(Vec2, f32)>; 16] = std::array::from_fn(|_| Vec::new());

    for star in stars {
        let twinkle = elapsed * star.twinkle_speed + star.phase;
        let alpha = (star.base_alpha * (0.70 + 0.30 * twinkle.sin())).clamp(0.0, 255.0);
        let alpha_level = ((alpha / 64.0) as usize).min(3);

        let palette = star.palette as usize % STAR_PALETTE.len();
        let index = palette * 4 + alpha_level;

        let x = (star.position.x * window_size.x - camera.position.x * star.parallax)
            .rem_euclid(window_size.x);
        let y = (star.position.y * window_size.y - camera.position.y * star.parallax)
            .rem_euclid(window_size.y);

        buckets[index].push((Vec2::new(x, y), star.size));
    }

    for (palette, color) in STAR_PALETTE.iter().enumerate() {
        for level in 0..4 {
            let bucket = &buckets[palette * 4 + level];
            if bucket.is_empty() {
                continue;
            }

            let alpha = match level {
                0 => 64,
                1 => 128,
                2 => 192,
                _ => 255,
            };
            let tinted = Color::rgba(color.r, color.g, color.b, alpha);

            fill_quads(d, bucket, tinted)?;
        }
    }

    Ok(())
}
