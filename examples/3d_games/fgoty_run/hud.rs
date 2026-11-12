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

    draw_vignette(ui, width, height);

    if game.flash > 0.0 {
        ui.fill_rect(
            Rect::new(0.0, 0.0, width, height),
            Color::rgba(190, 28, 38, (game.flash * 95.0) as u8),
        );
    }

    draw_stats(ui, font, game)?;
    draw_wave_panel(ui, font, game, width)?;
    draw_minimap(ui, game, width, height);

    if game.over {
        draw_game_over(ui, font, game, width, height)?;
    } else {
        draw_banner(ui, font, game, width, height)?;
        draw_hint(ui, font, width, height)?;
    }

    Ok(())
}

/// Маленькая карта в углу: Феррис (голубой), враги (красные), монеты (золотые).
fn draw_minimap(ui: &mut Renderer2D, game: &Game, width: f32, height: f32) {
    use crate::game::ARENA;

    let size = 130.0;
    let margin = 25.0;
    let rect = Rect::new(width - size - margin, height - size - margin, size, size);

    ui.fill_rect(rect, Color::rgba(10, 15, 28, 200));
    ui.draw_rect(rect, 1.0, Color::rgba(88, 138, 210, 120));

    // Масштабируем так, чтобы вся арена + небольшой запас влезли в квадрат.
    let scale = size / (ARENA * 2.15);
    let center = Vec2::new(rect.x + size / 2.0, rect.y + size / 2.0);

    // Сначала монеты (самые маленькие точки)
    for coin in &game.coins {
        ui.fill_circle(
            center + Vec2::new(coin.pos.x, coin.pos.z) * scale,
            1.6,
            Color::rgb(255, 200, 100),
        );
    }

    // Затем враги
    for enemy in &game.enemies {
        ui.fill_circle(
            center + Vec2::new(enemy.pos.x, enemy.pos.z) * scale,
            2.8,
            Color::rgb(255, 70, 70),
        );
    }

    // Игрок поверх всех
    ui.fill_circle(
        center + Vec2::new(game.player.x, game.player.z) * scale,
        3.5,
        Color::rgb(100, 210, 255),
    );
}

/// Тёмные края кадра: сцена выглядит глубже, а HUD читается лучше.
fn draw_vignette(ui: &mut Renderer2D, width: f32, height: f32) {
    ui.fill_rect(
        Rect::new(0.0, 0.0, width, height),
        Color::rgba(4, 8, 20, 28),
    );

    const STEPS: usize = 14;
    const BAND: f32 = 14.0;

    for i in 0..STEPS {
        let t = i as f32 / STEPS as f32;
        let alpha = ((1.0 - t) * 46.0) as u8;
        let inset = i as f32 * BAND;
        let color = Color::rgba(2, 4, 12, alpha);
        let span = width - inset * 2.0;
        let tall = height - inset * 2.0;

        ui.fill_rect(Rect::new(inset, inset, span, BAND), color);
        ui.fill_rect(Rect::new(inset, height - inset - BAND, span, BAND), color);
        ui.fill_rect(Rect::new(inset, inset, BAND, tall), color);
        ui.fill_rect(Rect::new(width - inset - BAND, inset, BAND, tall), color);
    }
}

fn draw_stats(ui: &mut Renderer2D, font: FontId, game: &Game) -> Result<(), frglib::gpu::Error> {
    let panel = Rect::new(20.0, 20.0, 312.0, 112.0);
    ui.fill_rect(panel, Color::rgba(7, 11, 22, 218));
    ui.draw_rect(panel, 1.0, Color::rgba(88, 138, 210, 150));

    ui.draw_text(
        font,
        "СЧЁТ",
        36.0,
        30.0,
        13.0,
        Color::rgb(126, 158, 208),
    )?;
    ui.draw_text(
        font,
        &format!("{}", game.score),
        36.0,
        47.0,
        34.0,
        Color::WHITE,
    )?;

    ui.draw_text(
        font,
        "ЖИЗНИ",
        192.0,
        30.0,
        13.0,
        Color::rgb(126, 158, 208),
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

    ui.draw_text(
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

    ui.draw_text(
        font,
        "МОНЕТЫ",
        width - 230.0,
        30.0,
        13.0,
        Color::rgb(126, 158, 208),
    )?;
    ui.draw_text(
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

    ui.draw_text_centered(
        font,
        &format!("ВОЛНА {}", game.wave),
        width / 2.0,
        height * 0.30,
        44.0,
        Color::rgba(255, 220, 150, alpha),
    )?;
    ui.draw_text_centered(
        font,
        "собери все монеты",
        width / 2.0,
        height * 0.30 + 38.0,
        17.0,
        Color::rgba(180, 205, 235, alpha),
    )?;

    Ok(())
}

fn draw_hint(
    ui: &mut Renderer2D,
    font: FontId,
    width: f32,
    height: f32,
) -> Result<(), frglib::gpu::Error> {
    ui.draw_text_centered(
        font,
        "WASD / стрелки — движение · собирай монеты · R — заново",
        width / 2.0,
        height - 30.0,
        15.0,
        Color::rgba(190, 205, 225, 205),
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

    ui.draw_text_centered(
        font,
        "ИГРА ОКОНЧЕНА",
        width / 2.0,
        height / 2.0 - 76.0,
        46.0,
        Color::WHITE,
    )?;
    ui.draw_text_centered(
        font,
        &format!("Счёт: {}", game.score),
        width / 2.0,
        height / 2.0 - 10.0,
        28.0,
        Color::rgb(255, 206, 112),
    )?;
    ui.draw_text_centered(
        font,
        &format!("Волна: {}", game.wave),
        width / 2.0,
        height / 2.0 + 32.0,
        18.0,
        Color::rgb(150, 180, 220),
    )?;
    ui.draw_text_centered(
        font,
        "R или пробел — заново",
        width / 2.0,
        height / 2.0 + 82.0,
        18.0,
        Color::rgba(190, 205, 225, (205.0 * pulse) as u8),
    )?;

    Ok(())
}
