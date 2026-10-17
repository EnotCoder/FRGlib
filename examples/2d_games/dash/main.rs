//! Frogy Dash: куб бежит вправо сам, игрок только прыгает.
//!
//! Пробел, стрелка вверх или W -- прыжок. Сила всегда одна: нажал или зажал,
//! одинаково. Шип на пути -- смерть, пробел начинает уровень заново.
//!
//! Как в настоящем Geometry Dash, мир едет влево, а куб держится на одном
//! месте экрана: игрок по горизонтали не управляет совсем, вся задача --
//! угадать момент прыжка.
//!
//! Запуск: `cargo run --features cached-textures --example 2d_games-frogy-dash`

mod level;
mod player;

use std::path::PathBuf;

use frglib::{Camera, Color, Event, FrameClock, Image, Keycode, Object, Vec2, Window};

use level::{hit, parse, top_of, Tile, BODY, COLS, GROUND, GROUND_SCREEN, LEVEL_TOP, ROWS, TILE};
use player::{Player, State, GRAVITY, JUMP, LAND_TOLERANCE, SCREEN_X, SPEED, SPIKE_INSET};

fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/2d_games/assets/dash")
        .join(name)
}

/// Сторона спрайта в пикселях файла.
const SRC: u32 = 16;

/// Высота пола в файле.
const FLOOR_H: u32 = 48;

/// Где в файле кончается каменная шапка и начинается грунт.
const CAP: u32 = 18;

/// Высота каменной шапки на экране.
const CAP_H: f32 = 36.0;

/// Высота одного куска грунта на экране.
const DIRT_H: f32 = 60.0;

/// Небо.
const SKY_TOP: Color = Color::rgb(58, 152, 233);
const SKY_LOW: Color = Color::rgb(191, 233, 252);
const SKY_BANDS: usize = 8;

/// Смешивает два цвета: `0` — `a`, `1` — `b`.
fn mix(a: Color, b: Color, t: f32) -> Color {
    let ch = |x: u8, y: u8| {
        (x as f32 + (y as f32 - x as f32) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color::rgba(ch(a.r, b.r), ch(a.g, b.g), ch(a.b, b.b), 255)
}

fn main() -> Result<(), frglib::Error> {
    let cube = Image::from_file(asset("frogy_cube.png"))?;
    let cube_dead = Image::from_file(asset("frogy_cube_dead.png"))?;
    let spike = Image::from_file(asset("kill_block.png"))?;
    let floor = Image::from_file(asset("floor.png"))?;

    // Пол из картинки 16x48: каменная шапка сверху, грунт под ней.
    //
    // Шапка в файле выше, чем кажется: серая строка 17 ещё серая, грунт
    // начинается с 18-й. Режем по границе цвета, иначе сверху каждого
    // повтора земли идёт каменная полоска.
    let floor_top = floor.crop(0, 0, SRC, CAP)?;
    let floor_body = floor.crop(0, CAP, SRC, FLOOR_H - CAP)?;

    let mut window = Window::builder("frogy dash")
        .size(960, 540)
        .background(SKY_TOP)
        .pixel_art(true)
        .build()?;

    let cube_tex = window.upload(&cube)?;
    let cube_dead_tex = window.upload(&cube_dead)?;
    let spike_tex = window.upload(&spike)?;
    let top_tex = window.upload(&floor_top)?;
    let body_tex = window.upload(&floor_body)?;

    let grid = parse();

    let mut pressed: Vec<Keycode> = Vec::new();
    let mut clock = FrameClock::new(60.0);
    let mut player = Player::new();
    let mut camera = Camera::new(Vec2::ZERO);

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
        let jump = [Keycode::Space, Keycode::Up, Keycode::W, Keycode::Z]
            .iter()
            .any(|&key| held(key));
        let restart = [Keycode::Space, Keycode::R].iter().any(|&key| held(key));

        match player.state {
            State::Run => {
                if jump && player.grounded {
                    player.vy = -JUMP;
                    player.grounded = false;
                }

                player.vy += GRAVITY * dt;
                player.x += SPEED * dt;

                let previous_bottom = player.y + BODY / 2.0;
                player.y += player.vy * dt;

                // Крутим куб в полёте: у настоящего dash куб вертится.
                if !player.grounded {
                    player.spin += 540.0 * dt;
                }

                let (col0, col1) = player.columns();
                let half = BODY / 2.0;

                for col in col0.max(0)..=col1.min(COLS as isize - 1) {
                    let row_lo = ((player.y - half - LEVEL_TOP) / TILE).floor() as isize;
                    let row_hi = ((player.y + half - LEVEL_TOP) / TILE).floor() as isize;

                    for row in row_lo.max(0)..=row_hi.min(ROWS as isize - 1) {
                        let col = col as usize;
                        let row = row as usize;

                        match grid[row][col] {
                            Tile::Spike => {
                                let center = Vec2::new(
                                    col as f32 * TILE + TILE / 2.0,
                                    top_of(row) + TILE / 2.0,
                                );
                                let reach = TILE / 2.0 - SPIKE_INSET;
                                let reach = Vec2::new(reach, reach);
                                if hit(player.center(), Vec2::new(half, half), center, reach) {
                                    player.state = State::Dead;
                                    println!("шип. Столбец {col}, пробел — заново.");
                                }
                            }
                            Tile::Platform => {
                                let top = top_of(row);
                                let bottom = top + TILE;
                                let left = col as f32 * TILE;
                                let right = left + TILE;

                                let sideways = player.x + half > left && player.x - half < right;

                                if !sideways {
                                    continue;
                                }

                                let lands = previous_bottom <= top + LAND_TOLERANCE
                                    && player.y + half >= top
                                    && player.vy > 0.0;

                                if lands {
                                    player.y = top - half;
                                    player.vy = 0.0;
                                    player.grounded = true;
                                    player.spin = 0.0;
                                } else if player.y - half < bottom {
                                    // Врезались в бок платформы.
                                    player.state = State::Dead;
                                    println!("бок платформы. Столбец {col}, пробел — заново.");
                                }
                            }
                            Tile::Empty => {}
                        }
                    }
                }

                // Земля: одна бесконечная плита под уровнем.
                if player.y + half >= GROUND {
                    player.y = GROUND - half;
                    player.vy = 0.0;
                    player.grounded = true;
                    player.spin = 0.0;
                }

                if player.state == State::Run && player.x >= COLS as f32 * TILE {
                    player.state = State::Won;
                    println!("уровень пройден: {COLS} столбцов.");
                }
            }
            State::Dead | State::Won => {
                if restart {
                    player = Player::new();
                }
            }
        }

        // Мир едет влево, потому что игрок едет вправо: камера держит его на
        // одной и той же точке экрана.
        camera.position.x = player.x - SCREEN_X;

        // Земля стоит на трёх четвертях высоты окна, а не на фиксированной
        // мировой координате. Окно тут WM раздувает до 925x991 вместо
        // запрошенных 960x540, и с жёстким GROUND низ уезжал за кадр.
        let view = window.size();
        camera.position.y = GROUND - view.1 as f32 * (GROUND_SCREEN - 0.5);

        // Видимый кусок мира: камера стоит в центре окна, поэтому в кадр попадает
        // ровно пол-окна в каждую сторону. Рисовать надо в этих мировых
        // координатах -- `Draw` сам спроецирует их на экран.
        let left = camera.position.x - view.0 as f32 / 2.0;
        let right = camera.position.x + view.0 as f32 / 2.0;
        let top = camera.position.y - view.1 as f32 / 2.0;
        let bottom = camera.position.y + view.1 as f32 / 2.0;

        // Видимые столбцы в мире -- со знаком, потому что камера спокойно выезжает
        // и за левый край уровня. Земля отсюда не кончается никогда, а вот
        // шипы и платформы есть только внутри уровня, и их отдельно обрезаем.
        let vis0 = (left / TILE).floor() as isize;
        let vis1 = (right / TILE).ceil() as isize;

        let col0 = vis0.max(0) as usize;
        let col1 = (vis1.min(COLS as isize)).max(0) as usize;
        let row0 = ((top - LEVEL_TOP) / TILE).floor().max(0.0) as usize;
        let row1 = (((bottom - LEVEL_TOP) / TILE).ceil().max(0.0)).min(ROWS as f32) as usize;

        camera.zoom = 1.3;

        window.frame_with(|d| {
            d.set_camera(camera);

            // Небо полосами: дешёвый градиент из одного цвета не выйдет.
            let band = view.1 as f32 / SKY_BANDS as f32;
            for i in 0..SKY_BANDS {
                let t = i as f32 / (SKY_BANDS - 1) as f32;
                d.fill_object(
                    &Object::from_xywh(
                        camera.position.x,
                        top + band * (i as f32 + 0.5),
                        view.0 as f32 + TILE,
                        band + 1.0,
                    ),
                    mix(SKY_TOP, SKY_LOW, t),
                )?;
            }

            // Земля: каменная шапка по верхнему краю, грунт вниз до низа окна.
            for col in vis0..vis1 {
                let x = col as f32 * TILE + TILE / 2.0;
                d.fill_object_texture(
                    &Object::from_xywh(x, GROUND + CAP_H / 2.0, TILE, CAP_H),
                    top_tex,
                )?;
            }

            let mut y = GROUND + CAP_H;
            while y < bottom + DIRT_H {
                for col in vis0..vis1 {
                    let x = col as f32 * TILE + TILE / 2.0;
                    d.fill_object_texture(
                        &Object::from_xywh(x, y + DIRT_H / 2.0, TILE, DIRT_H),
                        body_tex,
                    )?;
                }
                y += DIRT_H;
            }

            for (row, cells) in grid.iter().enumerate().take(row1).skip(row0) {
                let y = top_of(row) + TILE / 2.0;

                for (col, tile) in cells.iter().enumerate().take(col1).skip(col0) {
                    let x = col as f32 * TILE + TILE / 2.0;
                    let cell = Object::from_xywh(x, y, TILE, TILE);

                    match tile {
                        Tile::Spike => d.fill_object_texture(&cell, spike_tex)?,
                        // Платформа — тот же камень, что и верх земли, но
                        // земля под ней, так что платформа читается как выступ.
                        Tile::Platform => d.fill_object_texture(&cell, top_tex)?,
                        Tile::Empty => {}
                    }
                }
            }

            // Куб. После смерти красим, чтобы было видно без текста: в
            // библиотеке нет шрифтов.
            let mut body = Object::from_xywh(player.x, player.y, BODY, BODY);
            body.rotation = player.spin;

            if player.state == State::Dead {
                d.fill_object_texture(&body, cube_dead_tex)?;
            } else {
                d.fill_object_texture(&body, cube_tex)?;
            }

            Ok(())
        })?;

        clock.tick();
    }
}
