//! Небольшая 2D-RPG на тайловой карте: фрогти ходит по лесу, камера едет за ним.
//!
//! Стрелки или WASD -- движение.
//!
//! Тайлмап режется из одного PNG на куски 16x16, и каждый кусок становится
//! своей текстурой. Дальше рисуются только те тайлы, что попали в кадр, так
//! что карта может быть заметно больше окна.
//!
//! Запуск: `cargo run --features cached-textures --example 2d_games-fgoty`

mod draw;
mod game;
mod map;
mod player;

use std::path::PathBuf;

use frglib::{Camera, Color, Event, FrameClock, Image, Keycode, TileMap, Vec2, Window};

use draw::draw_scene;
use game::World;
use map::{build_map, slice_tiles, TILE};
use player::Facing;

/// Спрайты лежат рядом с примером; путь строим от корня крейта, а не от
/// текущего каталога, чтобы `cargo run --example` работал из любого места.
fn asset(kind: &str, name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/2d_games/assets")
        .join(kind)
        .join(name)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sheet = Image::from_file(asset("", "tilemap_forest.png"))?;
    let tiles = slice_tiles(&sheet)?;

    let faces = [Facing::Down, Facing::Left, Facing::Right, Facing::Up];
    let frogs: Vec<Image> = faces
        .iter()
        .map(|f| Image::from_file(asset("rpg_fgoty", f.file())))
        .collect::<Result<_, _>>()?;

    let mut window = Window::builder("frglib fgoty")
        .size(800, 600)
        .background(Color::rgb(14, 17, 21))
        // Тайлы 16x16 рисуются в 32x32, ровно вдвое, и без этого флага SDL
        // размазывает их при увеличении: пиксель-арт должен оставаться
        // пиксель-артом.
        .pixel_art(true)
        .build()?;

    // Всё, что рисуется каждый кадр, заливаем на GPU один раз при старте.
    // Минуту на это ушла загрузка по одной текстуре на тайл на кадр.
    let tile_tex: Vec<_> = tiles
        .iter()
        .map(|t| window.upload(t))
        .collect::<Result<_, frglib::Error>>()?;
    let frog_tex: Vec<_> = frogs
        .iter()
        .map(|f| window.upload(f))
        .collect::<Result<_, frglib::Error>>()?;

    println!("тайлов в атласе: {}", tiles.len());

    // Сетка, координаты и отсечение по кадру -- за `TileMap`; пример даёт ему
    // только то, что про этот лес знает он сам: строки из `build_map`.
    let map = TileMap::from_rows(build_map(), TILE)?;

    let mut pressed: Vec<Keycode> = Vec::new();
    let mut clock = FrameClock::new(60.0);

    let mut world = World::new();
    let mut camera = Camera::new(world.player.position);

    loop {
        let dt = clock.dt();

        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::KeyDown { key, .. } => {
                    if !pressed.contains(&key) {
                        pressed.push(key);
                    }
                }
                Event::KeyUp { key, .. } => pressed.retain(|&held| held != key),
                _ => {}
            }
        }

        let held = |key| pressed.contains(&key);
        let mut direction = Vec2::ZERO;
        for (keys, step) in [
            ([Keycode::Left, Keycode::A], Vec2::new(-1.0, 0.0)),
            ([Keycode::Right, Keycode::D], Vec2::new(1.0, 0.0)),
            ([Keycode::Up, Keycode::W], Vec2::new(0.0, -1.0)),
            ([Keycode::Down, Keycode::S], Vec2::new(0.0, 1.0)),
        ] {
            if keys.iter().any(|&key| held(key)) {
                direction += step;
            }
        }

        let direction = direction.normalized();
        let bob = world.update(&map, direction, dt);

        // Камера едет за игроком и не отпускает карту.
        camera.position +=
            (world.player.position - camera.position) * (8.0 * dt).min(1.0);
        let view = window.size();
        map.clamp_camera(&mut camera, view);

        window.frame_with(|d| {
            draw_scene(
                d,
                &map,
                &tile_tex,
                &frog_tex,
                camera,
                &world.player,
                world.facing,
                bob,
            )
        })?;

        clock.tick();
    }
}
