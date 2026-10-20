//! Небольшая 2D-RPG на тайловой карте: фрогти ходит по лесу, камера едет за ним.
//!
//! Стрелки или WASD -- движение.
//!
//! Тайлмап режется из одного PNG на куски 16x16, и каждый кусок становится
//! своей текстурой. Дальше рисуются только те тайлы, что попали в кадр, так
//! что карта может быть заметно больше окна.
//!
//! Запуск: `cargo run --features cached-textures --example 2d_games-fgoty`

mod map;
mod player;

use std::path::PathBuf;

use frglib::{Camera, Color, Event, FrameClock, Image, Keycode, Object, TileMap, Vec2, Window};

use map::{build_map, slice_tiles, TILE};
use player::{Facing, SPEED};

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
    let mut facing = Facing::Down;
    let mut step = 0u32;

    let mut player = Object::from_xywh(TILE * 2.5, TILE * 2.5, TILE, TILE);
    let mut camera = Camera::new(player.position);

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

        // Пробуем шагнуть и проверяем точку, в которую приземлились: в воду и за
        // край карты не ходим, упираемся в границу вплотную.
        //
        // Проверяется только тот угол, в сторону которого идём: у нуля в Rust
        // знак равен единице, так что через `signum` проверка уехала бы на
        // пол-объекта в сторону, куда движения нет.
        let half = player.size * 0.5;
        let lead = Vec2::new(
            match direction.x {
                x if x > 0.0 => half.x,
                x if x < 0.0 => -half.x,
                _ => 0.0,
            },
            match direction.y {
                y if y > 0.0 => half.y,
                y if y < 0.0 => -half.y,
                _ => 0.0,
            },
        );

        let target = player.position + direction * SPEED * dt;
        if direction == Vec2::ZERO
            || map
                .get_at(target + lead)
                .is_some_and(|tile| tile.walkable())
        {
            player.position = target;
        }
        player.velocity = direction * SPEED;

        if let Some(next) = Facing::of(direction) {
            facing = next;
        }

        // Покачивание при ходьбе: кадров анимации нет, так что просто качаем.
        let moving = direction != Vec2::ZERO;
        if moving {
            step += 1;
        }
        let bob = if moving && (step / 7) % 2 == 0 {
            3.0
        } else {
            0.0
        };

        // Камера едет за игроком и не отпускает карту.
        camera.position += (player.position - camera.position) * (8.0 * dt).min(1.0);
        let view = window.size();
        map.clamp_camera(&mut camera, view);

        let hero = player.with_flip(false, false);
        let sprite = frog_tex[facing.index()];

        window.frame_with(|d| {
            d.set_camera(camera);

            // Рисуются только тайлы в кадре; `texture_of` отдаёт текстуру из
            // нарезки атласа по номеру тайла.
            map.draw(d, camera, |tile| tile.texture().map(|i| tile_tex[i]))?;

            let hero = Object::from_xywh(
                hero.position.x,
                hero.position.y - bob,
                hero.size.x,
                hero.size.y,
            );
            d.fill_object_texture(&hero, sprite)?;

            Ok(())
        })?;

        clock.tick();
    }
}
