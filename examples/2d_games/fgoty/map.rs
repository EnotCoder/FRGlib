//! Карты атласа: из чего она состоит, как режется и чем заполняется.
//!
//! Сама сетка, координаты и отсечение по кадру -- это [`frglib::TileMap`],
//! общее для всех карт. Здесь остаётся только то, что знает про этот лес:
//! какие бывают тайлы, что из них проходимо и как карта собирается.

use frglib::Image;

/// Сторона одного тайла в атласе, в пикселях.
pub const SRC_TILE: u32 = 16;

/// Тот же размер, но целым -- для арифметики с индексами атласа.
pub const SRC_TILE_US: usize = SRC_TILE as usize;

/// Сколько тайлов в атласе по горизонтали и вертикали.
pub const ATLAS: (usize, usize) = (3, 3);

/// Сторона тайла на экране, в единицах мира.
pub const TILE: f32 = 32.0;

/// Размер карты в тайлах.
pub const MAP: (usize, usize) = (48, 32);

/// Номера тайлов в нарезке атласа, считая по строкам.
pub const TILE_GRASS: usize = 0;
pub const TILE_TUFT: usize = 1;
pub const TILE_LOG: usize = 2;
pub const TILE_PATH: usize = 3;
pub const TILE_WATER: usize = 4;
pub const TILE_RUST: usize = 5;

/// Тайлы карты.
///
/// Смысл живёт здесь: библиотеке [`TileMap`] всё равно, что такое трава, --
/// она хранит `T` и не спрашивает, зачем.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tile {
    Grass,
    Tuft,
    Log,
    Path,
    Water,
    Rust,
}

impl Tile {
    /// Номер в нарезке атласа, либо `None` для тайла, который не рисуется.
    pub fn texture(self) -> Option<usize> {
        match self {
            Tile::Grass => Some(TILE_GRASS),
            Tile::Tuft => Some(TILE_TUFT),
            Tile::Log => Some(TILE_LOG),
            Tile::Path => Some(TILE_PATH),
            Tile::Water => Some(TILE_WATER),
            Tile::Rust => Some(TILE_RUST),
        }
    }

    /// Ветка лежит на земле, а пруд глубокий -- по пруду не ходим.
    pub fn walkable(self) -> bool {
        !matches!(self, Tile::Water | Tile::Rust)
    }
}

/// Режет атлас на отдельные картинки: в атласе несколько тайлов рядом, а
/// рисовать их надо по одному.
///
/// `Draw` умеет копировать текстуру целиком, а не её кусок, поэтому нарезка
/// делается заранее и каждый кусок заливается на GPU своей текстурой.
pub fn slice_tiles(sheet: &Image) -> Result<Vec<Image>, frglib::Error> {
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

/// Собирает карту по простым правилам, чтобы пример не тащил за собой формат
/// файлов: трава, дорога через лес и пруд в углу.
///
/// Возвращаются готовые строки -- дальше их принимает
/// [`TileMap::from_rows`](frglib::TileMap::from_rows).
pub fn build_map() -> Vec<Vec<Tile>> {
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

    map[8][15] = Tile::Rust;

    // Упавшие ветки: пара штук тут, пара там.
    for (y, x) in [(3, 14), (8, 27), (13, 6), (17, 37), (23, 18)] {
        map[y][x] = Tile::Log;
    }

    map
}
