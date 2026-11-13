//! HUD поверх 3D-сцены: счёт, жизни, волна и подсказки.
//!
//! `Renderer2D` работает в screen space, поэтому интерфейс не зависит от
//! камеры. Сначала рисуется виньетка и вспышка урона, затем панели и текст.

use frglib::gpu::{FontId, Renderer2D};
use frglib::{Color, Rect, Vec2};

use crate::game::{Game, COINS};

pub fn draw(
    ui: &mut Renderer2D,
    font: FontId,
    game: &Game,
    size: (u32, u32),
) -> Result<(), frglib::gpu::Error> {
    let (width, height) = (size.0 as f32, size.1 as f32);

    if game.flash > 0.0 {
        ui.fill_rect(
            Rect::new(0.0, 0.0, width, height),
            Color::rgba(190, 28, 38, (game.flash * 95.0) as u8),
        );
    }

    draw_stats(ui, font, game)?;
    draw_wave_panel(ui, font, game, width)?;

    if game.over {
        draw_game_over(ui, font, game, width, height)?;
    } else {
        draw_banner(ui, font, game, width, height)?;
        draw_hint(ui, font, width, height)?;
    }

    Ok(())
}

fn draw_stats(ui: &mut Renderer2D, font: FontId, game: &Game) -> Result<(), frglib::gpu::Error> {
    let panel = Rect::new(20.0, 20.0, 312.0, 112.0);
    ui.fill_rect(panel, Color::rgba(7, 11, 22, 218));
    ui.draw_rect(panel, 1.0, Color::rgba(88, 138, 210, 150));

    draw_text(
        ui,
        font,
        "СЧЁТ",
        36.0,
        30.0,
        13.0,
        Color::rgb(255, 255, 255),
    )?;
    draw_text_thin(
        ui,
        font,
        &format!("{}", game.score),
        36.0,
        47.0,
        34.0,
        Color::WHITE,
    )?;

    draw_text(
        ui,
        font,
        "ЖИЗНИ",
        192.0,
        30.0,
        13.0,
        Color::rgb(255, 255, 255),
    )?;

    for i in 0..3 {
        let x = 198.0 + i as f32 * 32.0;
        let filled = (i as u32) < game.lives;
        let color = if filled {
            Color::rgb(255, 88, 88)
        } else {
            Color::rgba(68, 78, 100, 120)
        };

        ui.fill_circle(Vec2::new(x, 60.0), 9.0, color);

        if filled {
            ui.fill_circle(Vec2::new(x, 60.0), 3.5, Color::rgba(255, 220, 220, 190));
        }
    }

    draw_text(
        ui,
        font,
        &format!("ВОЛНА {}", game.wave),
        36.0,
        94.0,
        16.0,
        Color::rgb(255, 200, 100),
    )?;

    Ok(())
}

fn draw_wave_panel(
    ui: &mut Renderer2D,
    font: FontId,
    game: &Game,
    width: f32,
) -> Result<(), frglib::gpu::Error> {
    let panel = Rect::new(width - 250.0, 20.0, 230.0, 78.0);
    ui.fill_rect(panel, Color::rgba(7, 11, 22, 218));
    ui.draw_rect(panel, 1.0, Color::rgba(88, 138, 210, 150));

    draw_text(
        ui,
        font,
        "МОНЕТЫ",
        width - 230.0,
        30.0,
        13.0,
        Color::rgb(255, 255, 255),
    )?;
    draw_text(
        ui,
        font,
        &format!("{} / {}", game.coins.len(), COINS),
        width - 230.0,
        48.0,
        24.0,
        Color::rgb(255, 214, 116),
    )?;

    Ok(())
}

fn draw_banner(
    ui: &mut Renderer2D,
    font: FontId,
    game: &Game,
    width: f32,
    height: f32,
) -> Result<(), frglib::gpu::Error> {
    if game.banner <= 0.0 {
        return Ok(());
    }

    // Плавно появляется и так же плавно исчезает.
    let fade_in = ((2.4 - game.banner) / 0.3).clamp(0.0, 1.0);
    let fade_out = (game.banner / 0.6).clamp(0.0, 1.0);
    let alpha = (fade_in * fade_out * 255.0) as u8;

    draw_text_centered_thin(
        ui,
        font,
        &format!("ВОЛНА {}", game.wave),
        width / 2.0,
        height * 0.30,
        44.0,
        Color::rgba(255, 255, 255, alpha),
    )?;
    draw_text_centered(
        ui,
        font,
        "собери все монеты",
        width / 2.0,
        height * 0.30 + 38.0,
        17.0,
        Color::rgba(255, 255, 255, alpha),
    )?;

    Ok(())
}

fn draw_hint(
    ui: &mut Renderer2D,
    font: FontId,
    width: f32,
    height: f32,
) -> Result<(), frglib::gpu::Error> {
    draw_text_centered(
        ui,
        font,
        "WASD / стрелки — движение · собирай монеты · R — заново",
        width / 2.0,
        height - 30.0,
        15.0,
        Color::rgba(255, 255, 255, 205),
    )?;

    Ok(())
}

fn draw_game_over(
    ui: &mut Renderer2D,
    font: FontId,
    game: &Game,
    width: f32,
    height: f32,
) -> Result<(), frglib::gpu::Error> {
    ui.fill_rect(
        Rect::new(0.0, 0.0, width, height),
        Color::rgba(3, 5, 12, 192),
    );

    let pulse = 0.65 + 0.35 * (game.time * 3.0).sin();

    draw_text_centered(
        ui,
        font,
        "ИГРА ОКОНЧЕНА",
        width / 2.0,
        height / 2.0 - 76.0,
        46.0,
        Color::WHITE,
    )?;
    draw_text_centered_thin(
        ui,
        font,
        &format!("Счёт: {}", game.score),
        width / 2.0,
        height / 2.0 - 10.0,
        28.0,
        Color::rgb(255, 206, 112),
    )?;
    draw_text_centered(
        ui,
        font,
        &format!("Волна: {}", game.wave),
        width / 2.0,
        height / 2.0 + 32.0,
        18.0,
        Color::rgb(150, 180, 220),
    )?;
    draw_text_centered(
        ui,
        font,
        "R или пробел — заново",
        width / 2.0,
        height / 2.0 + 82.0,
        18.0,
        Color::rgba(190, 205, 225, (205.0 * pulse) as u8),
    )?;

    Ok(())
}

/// Во сколько раз тоньше обводка у избранных надписей.
const THIN_OUTLINE: f32 = 0.5;

/// Рисует текст с чёрной обводкой, выровненный по левому краю.
fn draw_text(
    ui: &mut Renderer2D,
    font: FontId,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
) -> Result<(), frglib::gpu::Error> {
    draw_outline(ui, font, text, x, y, size, color.a, false, 1.0)?;
    ui.draw_text(font, text, x, y, size, color)
}

/// Рисует текст с чёрной обводкой, центрированный по обоим осям.
fn draw_text_centered(
    ui: &mut Renderer2D,
    font: FontId,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
) -> Result<(), frglib::gpu::Error> {
    draw_outline(ui, font, text, x, y, size, color.a, true, 1.0)?;
    ui.draw_text_centered(font, text, x, y, size, color)
}

/// Как [`draw_text`], но с более тонкой обводкой.
fn draw_text_thin(
    ui: &mut Renderer2D,
    font: FontId,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
) -> Result<(), frglib::gpu::Error> {
    draw_outline(ui, font, text, x, y, size, color.a, false, THIN_OUTLINE)?;
    ui.draw_text(font, text, x, y, size, color)
}

/// Как [`draw_text_centered`], но с более тонкой обводкой.
fn draw_text_centered_thin(
    ui: &mut Renderer2D,
    font: FontId,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
) -> Result<(), frglib::gpu::Error> {
    draw_outline(ui, font, text, x, y, size, color.a, true, THIN_OUTLINE)?;
    ui.draw_text_centered(font, text, x, y, size, color)
}

/// Подкладывает под текст чёрные копии со смещением по восьми направлениям.
///
/// Прозрачность копий повторяет прозрачность самого текста, поэтому
/// исчезающие надписи теряют и обводку.
#[allow(clippy::too_many_arguments)]
fn draw_outline(
    ui: &mut Renderer2D,
    font: FontId,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    alpha: u8,
    centered: bool,
    width_scale: f32,
) -> Result<(), frglib::gpu::Error> {
    let outline = Color::rgba(0, 0, 0, alpha);
    // Обводка толще у крупных надписей, но и мелкий текст не тонет в фоне.
    let offset = ((size * 0.1).clamp(1.5, 4.5) * width_scale).max(1.0);
    let offsets = [
        (-offset, 0.0),
        (offset, 0.0),
        (0.0, -offset),
        (0.0, offset),
        (-offset, -offset),
        (offset, -offset),
        (-offset, offset),
        (offset, offset),
    ];

    for (dx, dy) in offsets {
        if centered {
            ui.draw_text_centered(font, text, x + dx, y + dy, size, outline)?;
        } else {
            ui.draw_text(font, text, x + dx, y + dy, size, outline)?;
        }
    }

    Ok(())
}
