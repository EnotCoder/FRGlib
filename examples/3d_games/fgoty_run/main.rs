//! 3D-игра на `wgpu`: Феррис собирает золотые монеты и убегает от ботов.
//!
//! Запуск: `cargo run --example 3d_games-fgoty_run --features wgpu`
//!
//! WASD или стрелки двигают краба по арене, R перезапускает игру, Escape
//! закрывает окно. Каждая собранная монета даёт очки; когда монеты
//! кончаются, начинается следующая волна с более быстрыми ботами. Три
//! столкновения — и игра окончена.
//!
//! Модели лежат в `examples/3d_games/assets/fgoty_run`: `player.glb` — краб,
//! `bot.glb` — враг, `coin.glb` — монета, `map.glb` — трава. Сцена и интерфейс
//! делят один кадр: `Renderer3D` рисует мир, `Renderer2D` — HUD поверх него.

mod assets;
mod game;
mod hud;
mod render;

use frglib::gpu::{Renderer2D, Renderer3D, WgpuWindow};
use frglib::{Color, Event, FrameClock, Keycode};

use assets::Assets;
use game::{seed, Game, Keys};
use render::CameraRig;

/// Шрифт HUD из примеров интерфейса.
const FONT: &[u8] = include_bytes!("../assets/font.otf");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = WgpuWindow::builder("FRGLib 3D — Феррис против ботов")
        .size(1100, 720)
        .background(Color::rgb(8, 12, 26))
        .build()?;

    let info = window.gpu().adapter_info();
    println!("{} ({:?} {})", info.name, info.backend, info.driver);

    let mut scene = Renderer3D::new(window.gpu());
    let mut ui = Renderer2D::new(window.gpu());
    let font = ui.load_font(FONT.to_vec())?;

    let assets = Assets::load(&scene, window.gpu())?;
    scene.set_skybox(assets.skybox.clone());

    let mut game = Game::new(seed());
    let mut keys = Keys::default();
    let mut camera = CameraRig::new();
    let mut clock = FrameClock::new(60.0);

    loop {
        let events = window.poll()?;

        for event in &events {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::KeyDown {
                    key: Keycode::Escape,
                    ..
                } => return Ok(()),
                Event::KeyDown { key, .. } => {
                    if matches!(key, Keycode::R) || (game.over && matches!(key, Keycode::Space)) {
                        game = Game::new(seed());
                        keys = Keys::default();
                        camera = CameraRig::new();
                    } else {
                        keys.press(*key, true);
                    }
                }
                Event::KeyUp { key, .. } => keys.press(*key, false),
                // Иначе после alt-tab краб продолжит ехать сам по себе.
                Event::FocusLost => keys = Keys::default(),
                _ => {}
            }
        }

        // Кадр мог задержаться: длинный шаг физики проскочил бы столкновение.
        let dt = clock.dt().min(0.05);
        game.update(dt, &keys);

        let Some(mut frame) = window.begin_frame()? else {
            clock.tick();
            continue;
        };

        let view = camera.update(&game, dt);
        scene.begin(&view);
        render::draw_scene(&game, &mut scene, &assets, &view);
        scene.finish(&mut frame);

        ui.begin_overlay(&frame);
        hud::draw(&mut ui, font, &game, frame.size)?;
        ui.finish(&mut frame);

        window.present(frame)?;
        clock.tick();
    }
}
