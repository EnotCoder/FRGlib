//! Небольшая 2D-RPG на тайловой карте: фрогти ходит по лесу, камера едет за ним.
//!
//! Стрелки или WASD -- движение.
//!
//! Тайлмап режется из одного PNG на куски 16x16, и каждый кусок становится
//! своей текстурой. Дальше рисуются только те тайлы, что попали в кадр, так
//! что карта может быть заметно больше окна.
//!
//! Запуск: `cargo run --features cached-textures --example 2d_games-frogy`

use std::path::PathBuf;

use frglib::{Camera, Color, Event, FrameClock, Image, Keycode, Object, Vec2, Window};

/// Спрайты лежат рядом с примером; путь строим от корня крейта, а не от
/// текущего каталога, чтобы `cargo run --example` работал из любого места.
fn asset(kind: &str, name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/2d_games/assets")
        .join(kind)
        .join(name)
}

/// Сторона одного тайла в атласе, в пикселях.
const SRC_TILE: u32 = 16;

/// Тот же размер, но целым -- для арифметики с индексами атласа.
const SRC_TILE_US: usize = SRC_TILE as usize;

/// Сколько тайлов в атласе по горизонтали и вертикали.
const ATLAS: (usize, usize) = (3, 3);

/// Сторона тайла на экране, в единицах мира. В атласе тайл 16x16, а на экране
/// 32: иначе фрогти рядом с игроком выглядит как мошка.
const TILE: f32 = 32.0;

/// Размер карты в тайлах.
const MAP: (usize, usize) = (48, 32);

/// Скорость ходьбы, единиц мира в секунду.
const SPEED: f32 = 190.0;

/// Номера тайлов в нарезке атласа, считая по строкам.
const TILE_GRASS: usize = 0;
const TILE_TUFT: usize = 1;
const TILE_LOG: usize = 2;
const TILE_PATH: usize = 3;
const TILE_WATER: usize = 4;

/// Каким кадром смотрит фрогти.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Facing {
    Down,
    Left,
    Right,
    Up,
}

impl Facing {
    /// По направлению движения. Без движения остаётся то, что было.
    fn of(direction: Vec2) -> Option<Self> {
        if direction.y.abs() > direction.x.abs() {
            Some(if direction.y < 0.0 {
                Facing::Up
            } else {
                Facing::Down
            })
        } else if direction.x < 0.0 {
            Some(Facing::Left)
        } else if direction.x > 0.0 {
            Some(Facing::Right)
        } else {
            None
        }
    }

    fn index(self) -> usize {
        match self {
            Facing::Down => 0,
            Facing::Left => 1,
            Facing::Right => 2,
            Facing::Up => 3,
        }
    }

    fn file(self) -> &'static str {
        match self {
            Facing::Down => "frogy_down.png",
            Facing::Left => "frogy_left.png",
            Facing::Right => "frogy_right.png",
            Facing::Up => "frogy_up.png",
        }
    }
}

/// Режет атлас на отдельные картинки: в атласе несколько тайлов рядом, а
/// рисовать их надо по одному.
fn slice_tiles(sheet: &Image) -> Result<Vec<Image>, frglib::Error> {
    let (cols, rows) = ATLAS;
    let mut tiles = Vec::with_capacity(cols * rows);

    for ty in 0..rows {
        for tx in 0..cols {
            // Срез берёт копию пикселей, поэтому атлас можно сразу забыть.
            tiles.push(sheet.crop(
                (tx * SRC_TILE_US) as u32,
                (ty * SRC_TILE_US) as u32,
                SRC_TILE,
                SRC_TILE,
            )?);
        }
    }

    Ok(tiles)
}

/// Тайлы карты.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Tile {
    Grass,
    Tuft,
    Log,
    Path,
    Water,
}

impl Tile {
    /// Индекс тайла в атласе, либо `None`, если тайла там нет и он рисуется
    /// заливкой.
    ///
    /// В атласе воды нет вообще: нижние два ряда и правый крайний тайл
    /// прозрачные, а в центре лежит зелёный зверёк на пустом фоне -- это
    /// спрайт, а не тайл. Пруд поэтому заливается цветом.
    fn texture(self) -> Option<usize> {
        match self {
            Tile::Grass => Some(TILE_GRASS),
            Tile::Tuft => Some(TILE_TUFT),
            Tile::Log => Some(TILE_LOG),
            Tile::Path => Some(TILE_PATH),
            Tile::Water => Some(TILE_WATER),
        }
    }

    /// Ветка лежит на земле, а пруд глубокий -- по пруду не ходим.
    fn walkable(self) -> bool {
        !matches!(self, Tile::Water)
    }
}

/// Собирает карту по простым правилам, чтобы пример не тащил за собой формат
/// файлов: трава, дорога через лес и пруд в углу.
fn build_map() -> Vec<Vec<Tile>> {
    let (w, h) = MAP;
    let mut map = vec![vec![Tile::Grass; w]; h];

    // Дорога: вертикальная через центр и ответвление вбок.
    let road_x = w / 2;
    for row in &mut map {
        row[road_x..=road_x + 1].fill(Tile::Path);
    }

    // Пруд в левом нижнем углу.
    for row in map.iter_mut().take(h - 3).skip(h - 9) {
        row[4..12].fill(Tile::Water);
    }

    // Кустики: ими карта перестаёт быть ровным ковром.
    for (y, x) in [
        (2, 8),
        (4, 19),
        (6, 31),
        (9, 4),
        (11, 22),
        (15, 12),
        (20, 33),
    ] {
        map[y][x] = Tile::Tuft;
    }

    // Упавшие ветки: пара штук тут, пара там.
    for (y, x) in [(3, 14), (8, 27), (13, 6), (17, 37), (23, 18)] {
        map[y][x] = Tile::Log;
    }

    map
}

/// Тайл под точкой мира, либо `None`, если точка за краем карты.
fn tile_at(map: &[Vec<Tile>], world: Vec2) -> Option<Tile> {
    let col = (world.x / TILE).floor();
    let row = (world.y / TILE).floor();

    if col < 0.0 || row < 0.0 {
        return None;
    }
    map.get(row as usize)?.get(col as usize).copied()
}

/// Можно ли сюда встать: за край карты и в воду -- нельзя.
fn walkable(map: &[Vec<Tile>], world: Vec2) -> bool {
    tile_at(map, world).is_some_and(Tile::walkable)
}

/// Держит камеру на карте, чтобы за её краями не было пустоты. Если карта уже
/// меньше окна, камера просто центрируется.
fn clamp_camera(camera: &mut Camera, view: (u32, u32)) {
    let (w, h) = MAP;
    let (map_w, map_h) = (w as f32 * TILE, h as f32 * TILE);

    let half_w = view.0 as f32 / 2.0 / camera.zoom;
    let half_h = view.1 as f32 / 2.0 / camera.zoom;

    camera.position.x = if map_w <= half_w * 2.0 {
        map_w / 2.0
    } else {
        camera.position.x.clamp(half_w, map_w - half_w)
    };
    camera.position.y = if map_h <= half_h * 2.0 {
        map_h / 2.0
    } else {
        camera.position.y.clamp(half_h, map_h - half_h)
    };
}

fn main() -> Result<(), frglib::Error> {
    let sheet = Image::from_file(asset("", "tilemap_forest.png"))?;
    let tiles = slice_tiles(&sheet)?;

    let faces = [Facing::Down, Facing::Left, Facing::Right, Facing::Up];
    let frogs: Vec<Image> = faces
        .iter()
        .map(|f| Image::from_file(asset("rpg_frogy", f.file())))
        .collect::<Result<_, _>>()?;

    let mut window = Window::builder("frglib frogy")
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

    let map = build_map();

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
        if direction == Vec2::ZERO || walkable(&map, target + lead) {
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
        clamp_camera(&mut camera, window.size());

        let view = window.size();
        let (fw, fh) = (view.0 as f32, view.1 as f32);
        let half_w = fw / 2.0 / camera.zoom;
        let half_h = fh / 2.0 / camera.zoom;

        // Видимая полоса тайлов. Всё за её пределами и не рисуется.
        let col0 = ((camera.position.x - half_w) / TILE).floor() as isize - 1;
        let col1 = ((camera.position.x + half_w) / TILE).ceil() as isize + 1;
        let row0 = ((camera.position.y - half_h) / TILE).floor() as isize - 1;
        let row1 = ((camera.position.y + half_h) / TILE).ceil() as isize + 1;

        let (map_w, map_h) = MAP;
        let visible: Vec<(usize, usize)> = (row0.max(0) as usize
            ..row1.min(map_h as isize) as usize)
            .flat_map(|row| {
                (col0.max(0) as usize..col1.min(map_w as isize) as usize).map(move |col| (col, row))
            })
            .collect();

        let hero = player.with_flip(false, false);
        let sprite = frog_tex[facing.index()];

        window.frame_with(|d| {
            d.set_camera(camera);

            for (col, row) in &visible {
                let tile = map[*row][*col];
                let cell = Object::from_xywh(
                    *col as f32 * TILE + TILE / 2.0,
                    *row as f32 * TILE + TILE / 2.0,
                    TILE,
                    TILE,
                );

                let index: usize = tile.texture().expect("REASON");
                d.fill_object_texture(&cell, tile_tex[index])?;
            }

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
