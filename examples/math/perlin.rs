//! Шум Перлина: чёрно-белая карта с изменяемой частотой и seed.
//!
//! Карта пересчитывается при изменении параметров. Координаты берутся с
//! дробным шагом (`x * frequency`), как в плане: плотность рисунка задаёт
//! частота, а сам рисунок — seed.
//!
//! Управление:
//! - Left/Right — уменьшить/увеличить частоту
//! - Up/Down — предыдущий/следующий seed
//! - F — переключить fBm (несколько октав)
//! - R — сброс параметров
//! - Escape — выход
//!
//! Запуск: `cargo run --example math-perlin`

use frglib::{noise::Perlin, Color, Event, Keycode, Object, Window};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
// Размер ячейки карты в пикселях: меньше — детальнее, но больше прямоугольников.
const CELL: u32 = 4;

fn main() -> Result<(), frglib::Error> {
    let mut seed: u64 = 42;
    let mut frequency: f32 = 0.02;
    let mut use_fbm = false;
    let mut noise = Perlin::new(seed);

    let mut window = Window::builder("FRGLib Perlin noise")
        .size(WIDTH, HEIGHT)
        .background(Color::BLACK)
        .build()?;

    // Карта значений шума в диапазоне [-1.0, 1.0]; пересчитывается по требованию.
    let cols = (WIDTH / CELL) as usize;
    let rows = (HEIGHT / CELL) as usize;
    let mut map = vec![0.0f32; cols * rows];
    regenerate(&noise, &mut map, cols, rows, frequency, use_fbm);

    loop {
        let mut changed = false;
        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::KeyDown {
                    key: Keycode::Escape,
                    ..
                } => return Ok(()),
                Event::KeyDown {
                    key: Keycode::Left,
                    ..
                } => {
                    frequency = (frequency / 1.1).max(0.001);
                    changed = true;
                }
                Event::KeyDown {
                    key: Keycode::Right,
                    ..
                } => {
                    frequency = (frequency * 1.1).min(1.0);
                    changed = true;
                }
                Event::KeyDown {
                    key: Keycode::Up, ..
                } => {
                    seed = seed.wrapping_add(1);
                    noise = Perlin::new(seed);
                    changed = true;
                }
                Event::KeyDown {
                    key: Keycode::Down,
                    ..
                } => {
                    seed = seed.wrapping_sub(1);
                    noise = Perlin::new(seed);
                    changed = true;
                }
                Event::KeyDown {
                    key: Keycode::F, ..
                } => {
                    use_fbm = !use_fbm;
                    changed = true;
                }
                Event::KeyDown {
                    key: Keycode::R, ..
                } => {
                    seed = 42;
                    frequency = 0.02;
                    use_fbm = false;
                    noise = Perlin::new(seed);
                    changed = true;
                }
                _ => {}
            }
        }

        if changed {
            regenerate(&noise, &mut map, cols, rows, frequency, use_fbm);
            println!(
                "seed={seed} frequency={frequency:.4} mode={}",
                if use_fbm { "fbm" } else { "noise" }
            );
        }

        window.frame_with(|d| {
            let cell = CELL as f32;
            for y in 0..rows {
                for x in 0..cols {
                    let v = map[y * cols + x];
                    // [-1, 1] -> [0, 255] для чёрно-белой карты.
                    let c = ((v * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0) as u8;
                    let color = Color::rgb(c, c, c);
                    let cx = x as f32 * cell + cell * 0.5;
                    let cy = y as f32 * cell + cell * 0.5;
                    d.fill_object(&Object::from_xywh(cx, cy, cell, cell), color)?;
                }
            }
            Ok(())
        })?;
    }
}

fn regenerate(
    noise: &Perlin,
    map: &mut [f32],
    cols: usize,
    rows: usize,
    frequency: f32,
    use_fbm: bool,
) {
    for y in 0..rows {
        for x in 0..cols {
            let sx = x as f32;
            let sy = y as f32;
            let value = if use_fbm {
                // fBm даёт более "рельефный" рисунок из нескольких октав.
                noise.fbm_2d(sx, sy, 5, frequency, 2.0, 0.5)
            } else {
                // Дробный шаг координат: частота управляет масштабом шума.
                noise.sample_2d(sx * frequency, sy * frequency)
            };
            // Пример преобразования как в плане: height = noise * 20.0 (здесь
            // оставляем нормализованное значение для карты яркости).
            let _height = value * 20.0;
            map[y * cols + x] = value;
        }
    }
}
