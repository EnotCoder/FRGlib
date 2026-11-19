//! Интерфейс поверх сцены: кнопка перегенерации, seed и подсказки.
//!
//! Всё рисуется `Renderer2D` в экранных координатах, поэтому панели не едут
//! вместе с камерой. Кнопка берётся из библиотеки: она сама отслеживает наведение
//! и клик по отпусканию и умеет рисовать себя на GPU-рендерере.

use frglib::gpu::{FontId, Renderer2D};
use frglib::gpu::glam::Vec3;
use frglib::{Button, Color, Rect, Vec2};

/// Рисует весь HUD. `size` — размер кадра в физических пикселях.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    ui: &mut Renderer2D,
    font: FontId,
    button: &Button,
    seed: u64,
    regenerations: u32,
    camera: Vec3,
    speed: f32,
    size: (u32, u32),
) -> Result<(), frglib::gpu::Error> {
    let width = size.0 as f32;
    let height = size.1 as f32;

    panel(ui, Rect::new(20.0, 20.0, 310.0, 116.0));
    text(ui, font, "ЛАНДШАФТ ПЕРЛИНА", 38.0, 32.0, 18.0, Color::rgb(255, 216, 142))?;
    text(
        ui,
        font,
        &format!("seed: {seed}"),
        38.0,
        62.0,
        22.0,
        Color::WHITE,
    )?;
    text(
        ui,
        font,
        &format!("перегенераций: {regenerations}"),
        38.0,
        96.0,
        15.0,
        Color::rgb(168, 190, 220),
    )?;

    // Кнопка из frglib: три состояния и клик по отпусканию левой кнопки.
    button.draw_gpu(ui, font, "Перегенерировать", 19.0)?;

    panel(ui, Rect::new(width - 266.0, 20.0, 246.0, 166.0));
    text(ui, font, "УПРАВЛЕНИЕ", width - 246.0, 32.0, 16.0, Color::rgb(255, 216, 142))?;
    let lines = [
        "WASD — полёт по карте",
        "Space / C — вверх / вниз",
        "Shift — ускорение",
        "мышь или стрелки — обзор",
        "колесо — скорость",
        "R — новый ландшафт",
    ];
    for (i, line) in lines.iter().enumerate() {
        text(
            ui,
            font,
            line,
            width - 246.0,
            60.0 + i as f32 * 20.0,
            14.0,
            Color::rgb(214, 226, 242),
        )?;
    }

    panel(ui, Rect::new(20.0, height - 92.0, 280.0, 72.0));
    text(
        ui,
        font,
        &format!(
            "x {:>6.1}   y {:>6.1}   z {:>6.1}",
            camera.x, camera.y, camera.z
        ),
        38.0,
        height - 78.0,
        16.0,
        Color::WHITE,
    )?;
    text(
        ui,
        font,
        &format!("скорость: {speed:.0}"),
        38.0,
        height - 52.0,
        14.0,
        Color::rgb(168, 190, 220),
    )?;

    // Небольшой прицел в центре помогает понять, куда смотрит камера.
    let center = Vec2::new(width * 0.5, height * 0.5);
    ui.fill_circle(center, 3.0, Color::rgba(255, 255, 255, 165));
    ui.fill_circle(center, 1.0, Color::rgba(20, 30, 45, 200));

    Ok(())
}

fn panel(ui: &mut Renderer2D, rect: Rect) {
    ui.fill_rect(rect, Color::rgba(8, 14, 26, 205));
    ui.draw_rect(rect, 1.0, Color::rgba(96, 140, 190, 140));
}

fn text(
    ui: &mut Renderer2D,
    font: FontId,
    value: &str,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
) -> Result<(), frglib::gpu::Error> {
    // Тонкая тень читается и на светлом небе, и на тёмной воде.
    ui.draw_text(font, value, x + 1.0, y + 1.0, size, Color::rgba(0, 0, 0, 170))?;
    ui.draw_text(font, value, x, y, size, color)
}
