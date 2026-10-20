//! Grids of tiles: parsing, world coordinates, and what is on screen.
//!
//! A tile map is a rectangle of cells the same size, drawn from a sprite
//! sheet. This module owns the three things every such map needs and none of
//! the meaning: where the cells are, which of them are visible, and how one
//! is drawn.
//!
//! What a cell *means* stays with the caller. The type is [`TileMap<T>`] with
//! no bound on `T` at all -- not [`Copy`], not a trait -- because whether a
//! cell is water, a spike or a floor tile is game logic, and this crate has
//! never claimed to know any. The same reasoning as for
//! [`Button`](crate::Button), which takes its caption as a picture rather than
//! as a string.
//!
//! ```no_run
//! # fn main() -> Result<(), frglib::MapError> {
//! use frglib::TileMap;
//!
//! #[derive(Debug, PartialEq)]
//! enum Cell { Floor, Water }
//!
//! let map = TileMap::from_ascii(
//!     &["....", "..~~", "...."],
//!     32.0,
//!     |c| Some(match c {
//!         '~' => Cell::Water,
//!         _ => Cell::Floor,
//!     }),
//! )?;
//!
//! assert_eq!(map.cols(), 4);
//! assert_eq!(map.rows(), 3);
//! # Ok(())
//! # }
//! ```

use crate::math::{Rect, Vec2};

/// A rectangular grid of cells of one size.
///
/// Row-major: the cell at `(col, row)` is `cells[row * cols + col]`. Cells are
/// held flat rather than as rows of rows, so a map is one allocation and a
/// row of it is one slice.
#[derive(Debug, Clone, PartialEq)]
pub struct TileMap<T> {
    cells: Vec<T>,
    cols: usize,
    rows: usize,
    /// Side of one cell, in world units.
    cell: f32,
    /// World position of the top-left corner of cell `(0, 0)`.
    origin: Vec2,
}

/// What a malformed map failed on.
///
/// Separate from [`Error`](crate::Error) on purpose: `Error` describes the
/// window and the renderer, and adding a variant to it would break every
/// `match` on that enum that does not have a `_` arm -- which is how every
/// past addition to it was a breaking change.
#[derive(Debug, Clone, PartialEq)]
pub enum MapError {
    /// No rows, or rows that are all empty: a map with nothing in it has no
    /// size to draw or look up against.
    NoRows,
    /// Row `row` has `got` cells where row `expected` had `expected`.
    RaggedRow {
        row: usize,
        expected: usize,
        got: usize,
    },
    /// Row `row`, column `col` holds a character the parser rejected.
    UnknownChar { row: usize, col: usize, ch: char },
    /// A cell size of zero or less would put every cell in the same place and
    /// make `cell_at` divide by zero.
    BadCell { cell: f32 },
}

impl std::fmt::Display for MapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MapError::NoRows => write!(f, "a map needs at least one non-empty row"),
            MapError::RaggedRow {
                row,
                expected,
                got,
            } => write!(f, "row {row} has {got} cells, expected {expected}"),
            MapError::UnknownChar { row, col, ch } => {
                write!(f, "unrecognised {ch:?} at row {row}, column {col}")
            }
            MapError::BadCell { cell } => write!(f, "cell size {cell} is not positive"),
        }
    }
}

impl std::error::Error for MapError {}

impl<T> TileMap<T> {
    /// Builds a map from rows of text, one cell per character.
    ///
    /// `parse` decides what each character means; returning [`None`] rejects
    /// it and fails the whole map with [`MapError::UnknownChar`]. That is
    /// deliberate: a typo in a map should stop the program, not silently
    /// become an empty cell that the player then falls through.
    ///
    /// Every row has to be the same length. Trailing spaces in an editor would
    /// otherwise shift rows against each other, so [`MapError::RaggedRow`] is
    /// reported instead of padding.
    ///
    /// No rows, or only empty ones, is [`MapError::NoRows`].
    pub fn from_ascii(
        rows: &[&str],
        cell: f32,
        mut parse: impl FnMut(char) -> Option<T>,
    ) -> Result<Self, MapError> {
        check_cell(cell)?;

        let mut cells = Vec::new();
        let mut width: Option<usize> = None;

        for (row, line) in rows.iter().enumerate() {
            let got = line.chars().count();

            match width {
                None => width = Some(got),
                Some(expected) if expected != got => {
                    return Err(MapError::RaggedRow {
                        row,
                        expected,
                        got,
                    })
                }
                _ => {}
            }

            for (col, ch) in line.chars().enumerate() {
                match parse(ch) {
                    Some(tile) => cells.push(tile),
                    None => return Err(MapError::UnknownChar { row, col, ch }),
                }
            }
        }

        let cols = width.unwrap_or(0);
        if cols == 0 || cells.is_empty() {
            return Err(MapError::NoRows);
        }

        Ok(Self::from_parts(cells, cols, rows.len(), cell))
    }

    /// Builds a map from rows that have already been turned into tiles.
    ///
    /// For maps made by code rather than by hand: a generator fills the rows
    /// and this validates them. Same rules as [`from_ascii`](Self::from_ascii)
    /// -- non-empty, and all rows the same length.
    pub fn from_rows(rows: Vec<Vec<T>>, cell: f32) -> Result<Self, MapError> {
        check_cell(cell)?;

        let height = rows.len();
        let cols = rows.first().map_or(0, Vec::len);
        if cols == 0 || height == 0 {
            return Err(MapError::NoRows);
        }

        let mut cells = Vec::with_capacity(cols * height);
        for (row, line) in rows.into_iter().enumerate() {
            let got = line.len();
            if got != cols {
                return Err(MapError::RaggedRow {
                    row,
                    expected: cols,
                    got,
                });
            }
            cells.extend(line);
        }

        Ok(Self::from_parts(cells, cols, height, cell))
    }

    /// Flat storage, with the shape already checked by the constructor.
    fn from_parts(cells: Vec<T>, cols: usize, rows: usize, cell: f32) -> Self {
        TileMap {
            cells,
            cols,
            rows,
            cell,
            origin: Vec2::ZERO,
        }
    }

    /// Moves the top-left corner of cell `(0, 0)` to `(x, y)` in the world.
    ///
    /// A map drawn from the origin sits at the world's own origin; a level
    /// whose ground is at `y = 400` needs the map moved up to meet it, and
    /// that is what this is for.
    pub fn origin(mut self, x: f32, y: f32) -> Self {
        self.origin = Vec2::new(x, y);
        self
    }

    /// Cells across and down.
    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Side of one cell, in world units.
    pub fn cell_size(&self) -> f32 {
        self.cell
    }

    /// How far the map reaches along x, in world units.
    pub fn width(&self) -> f32 {
        self.cols as f32 * self.cell
    }

    /// How far the map reaches along y, in world units.
    pub fn height(&self) -> f32 {
        self.rows as f32 * self.cell
    }

    /// Whether `(col, row)` is a cell this map actually has.
    pub fn contains(&self, col: usize, row: usize) -> bool {
        col < self.cols && row < self.rows
    }

    /// The cell at `(col, row)`, or `None` off the edges.
    pub fn get(&self, col: usize, row: usize) -> Option<&T> {
        if !self.contains(col, row) {
            return None;
        }
        self.cells.get(row * self.cols + col)
    }

    /// The cell containing the world point `at`, or `None` off the map.
    ///
    /// Flooring, not rounding: a point on the line between two cells belongs
    /// to the cell it entered, so walking right across a border flips the
    /// answer exactly at the border.
    pub fn cell_at(&self, at: Vec2) -> Option<(usize, usize)> {
        let local = at - self.origin;
        let col = (local.x / self.cell).floor();
        let row = (local.y / self.cell).floor();

        // NaN and infinities would survive the `as` cast, so they are dropped
        // here rather than turned into a plausible-looking cell.
        if !col.is_finite() || !row.is_finite() || col < 0.0 || row < 0.0 {
            return None;
        }

        let (col, row) = (col as usize, row as usize);
        self.contains(col, row).then_some((col, row))
    }

    /// The cell containing the world point `at`.
    pub fn get_at(&self, at: Vec2) -> Option<&T> {
        let (col, row) = self.cell_at(at)?;
        self.get(col, row)
    }

    /// The box cell `(col, row)` covers, in world coordinates.
    ///
    /// Off the map still gives a box: callers asking for a rectangle to draw
    /// or to test against are usually past the point where `None` helps, and
    /// [`contains`](Self::contains) is there for the question of membership.
    pub fn cell_rect(&self, col: usize, row: usize) -> Rect {
        Rect::new(
            self.origin.x + col as f32 * self.cell,
            self.origin.y + row as f32 * self.cell,
            self.cell,
            self.cell,
        )
    }

    /// Centre of cell `(col, row)`, in world coordinates.
    pub fn cell_center(&self, col: usize, row: usize) -> Vec2 {
        let rect = self.cell_rect(col, row);
        Vec2::new(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0)
    }

    /// Cells the camera can see, row by row and left to right.
    ///
    /// Everything outside this is off screen, and a map is usually far larger
    /// than the window -- drawing it cell by cell without this is what makes a
    /// big map slow. The range runs one cell past the visible edge on every
    /// side, so a tile half in the frame still gets drawn rather than popping
    /// in as the camera crosses a boundary.
    ///
    /// Needs the camera it is being viewed through: the same map is visible
    /// or not depending on where the camera points and how far it is zoomed.
    pub fn visible(
        &self,
        camera: crate::Camera,
        view: (u32, u32),
    ) -> impl Iterator<Item = (usize, usize, &T)> + '_ {
        let (col0, col1, row0, row1) = self.visible_bounds(camera, view);

        (row0..row1).flat_map(move |row| {
            (col0..col1).filter_map(move |col| self.get(col, row).map(|tile| (col, row, tile)))
        })
    }

    /// The cell range [`visible`](Self::visible) walks, as `start..end` pairs,
    /// already clipped to the map.
    fn visible_bounds(&self, camera: crate::Camera, view: (u32, u32)) -> (usize, usize, usize, usize) {
        // Same guard as `Camera::scale`: a zero or negative zoom would divide
        // by nothing and report the whole map as visible.
        let zoom = if camera.zoom > f32::EPSILON {
            camera.zoom
        } else {
            1.0
        };

        let half_w = view.0 as f32 / 2.0 / zoom;
        let half_h = view.1 as f32 / 2.0 / zoom;

        let (col0, col1) = self.strip(
            camera.position.x - half_w,
            camera.position.x + half_w,
            self.origin.x,
            self.cols,
        );
        let (row0, row1) = self.strip(
            camera.position.y - half_h,
            camera.position.y + half_h,
            self.origin.y,
            self.rows,
        );

        (col0, col1, row0, row1)
    }

    /// Which cells the world span `from..to` covers along one axis, padded by
    /// one cell on each side and clipped to `len`.
    fn strip(&self, from: f32, to: f32, origin: f32, len: usize) -> (usize, usize) {
        let start = ((from - origin) / self.cell).floor() as i64 - 1;
        let end = ((to - origin) / self.cell).ceil() as i64 + 1;

        // Clamping also handles the map being entirely off one side: the two
        // ends land on the same edge and the range comes out empty.
        let len = len as i64;
        let start = start.clamp(0, len) as usize;
        let end = end.clamp(0, len) as usize;

        (start, end.max(start))
    }

    /// Keeps the camera looking at the map, so there is no empty space past
    /// its edges.
    ///
    /// A map smaller than the window on an axis is simply centred; otherwise
    /// the camera stops at the edge. Same rule as seeing the whole world
    /// behind a window: past the map there is nothing, so the view does not go
    /// there.
    pub fn clamp_camera(&self, camera: &mut crate::Camera, view: (u32, u32)) {
        let zoom = if camera.zoom > f32::EPSILON {
            camera.zoom
        } else {
            1.0
        };

        let half_w = view.0 as f32 / 2.0 / zoom;
        let half_h = view.1 as f32 / 2.0 / zoom;

        let (map_w, map_h) = (self.width(), self.height());
        let (left, top) = (self.origin.x, self.origin.y);

        camera.position.x = if map_w <= half_w * 2.0 {
            left + map_w / 2.0
        } else {
            camera.position.x.clamp(left + half_w, left + map_w - half_w)
        };

        camera.position.y = if map_h <= half_h * 2.0 {
            top + map_h / 2.0
        } else {
            camera.position.y.clamp(top + half_h, top + map_h - half_h)
        };
    }

    /// Draws the visible cells.
    ///
    /// `texture_of` maps a cell to the texture to draw it with; [`None`] skips
    /// the cell, which is how an empty cell stays empty without the map having
    /// to know what "empty" means.
    ///
    /// The camera is the one the frame is being drawn through -- pass the same
    /// one given to [`Draw::set_camera`](crate::Draw::set_camera) -- and it
    /// decides which cells are visible.
    ///
    /// Behind `cached-textures` because the alternative is uploading a texture
    /// per cell per frame, which is what made the first version of this
    /// take a minute to start.
    #[cfg(feature = "cached-textures")]
    pub fn draw(
        &self,
        d: &mut crate::Draw<'_>,
        camera: crate::Camera,
        texture_of: impl Fn(&T) -> Option<crate::graphics::Texture>,
    ) -> Result<(), crate::window::Error> {
        for (col, row, tile) in self.visible(camera, d.size()) {
            let Some(texture) = texture_of(tile) else {
                continue;
            };

            let centre = self.cell_center(col, row);
            d.fill_object_texture(
                &crate::Object::from_xywh(centre.x, centre.y, self.cell, self.cell),
                texture,
            )?;
        }

        Ok(())
    }
}

/// A cell size has to be positive, or every cell lands on the same point and
/// `cell_at` divides by zero.
fn check_cell(cell: f32) -> Result<(), MapError> {
    if cell.is_finite() && cell > 0.0 {
        Ok(())
    } else {
        Err(MapError::BadCell { cell })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Camera;

    /// What a test map holds. `None` in the text is not a cell: this is the
    /// parsed form.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Cell {
        Floor,
        Water,
        Wall,
    }

    /// Parses the characters a test map uses.
    fn parse(ch: char) -> Option<Cell> {
        Some(match ch {
            '.' => Cell::Floor,
            '~' => Cell::Water,
            '#' => Cell::Wall,
            _ => return None,
        })
    }

    /// Four rows of eight cells, all floors except the ones that are not.
    fn map() -> TileMap<Cell> {
        TileMap::from_ascii(
            &[
                "........",
                "..~~~~..",
                "..#...#.",
                "........",
            ],
            32.0,
            parse,
        )
        .expect("map should parse")
    }

    #[test]
    fn a_parsed_map_has_the_shape_of_its_text() {
        let map = map();

        assert_eq!(map.cols(), 8);
        assert_eq!(map.rows(), 4);
        assert_eq!(map.cell_size(), 32.0);
        assert_eq!(map.width(), 8.0 * 32.0);
        assert_eq!(map.height(), 4.0 * 32.0);
    }

    #[test]
    fn cells_land_where_their_text_says() {
        let map = map();

        assert_eq!(map.get(0, 0), Some(&Cell::Floor));
        assert_eq!(map.get(2, 1), Some(&Cell::Water));
        assert_eq!(map.get(2, 2), Some(&Cell::Wall));
        // Off every edge.
        assert_eq!(map.get(8, 0), None);
        assert_eq!(map.get(0, 4), None);
        assert_eq!(map.get(9, 9), None);
    }

    #[test]
    fn a_world_point_finds_the_cell_it_is_in() {
        let map = map();

        // Cell (0, 0) covers 0..32 on both axes.
        assert_eq!(map.cell_at(Vec2::new(0.0, 0.0)), Some((0, 0)));
        assert_eq!(map.cell_at(Vec2::new(31.9, 31.9)), Some((0, 0)));

        // Just over the border belongs to the next cell, not the previous one.
        assert_eq!(map.cell_at(Vec2::new(32.0, 0.0)), Some((1, 0)));
        assert_eq!(map.cell_at(Vec2::new(64.0, 64.0)), Some((2, 2)));

        // Off the map, in every direction.
        assert_eq!(map.cell_at(Vec2::new(-1.0, 5.0)), None);
        assert_eq!(map.cell_at(Vec2::new(5.0, -1.0)), None);
        assert_eq!(map.cell_at(Vec2::new(256.0, 0.0)), None);
        assert_eq!(map.cell_at(Vec2::new(0.0, 128.0)), None);
    }

    #[test]
    fn a_broken_world_point_finds_nothing() {
        let map = map();

        // NaN and infinity must not be cast into a plausible cell index.
        assert_eq!(map.cell_at(Vec2::new(f32::NAN, 0.0)), None);
        assert_eq!(map.cell_at(Vec2::new(0.0, f32::INFINITY)), None);
        assert_eq!(map.cell_at(Vec2::new(f32::NEG_INFINITY, 5.0)), None);
    }

    #[test]
    fn get_at_combines_the_two_lookups() {
        let map = map();

        assert_eq!(map.get_at(Vec2::new(70.0, 70.0)), Some(&Cell::Wall));
        assert_eq!(map.get_at(Vec2::new(70.0, 70.0)), map.get(2, 2));
        assert_eq!(map.get_at(Vec2::new(-100.0, -100.0)), None);
    }

    #[test]
    fn an_origin_moves_every_cell_with_the_map() {
        let plain = map();
        let moved = map().origin(100.0, 200.0);

        // The same cell, asked for in world coordinates offset by the origin.
        assert_eq!(
            moved.cell_at(Vec2::new(100.0, 200.0)),
            Some((0, 0))
        );
        assert_eq!(
            moved.cell_at(Vec2::new(170.0, 270.0)),
            Some((2, 2))
        );

        // The origin is where the map starts, so cell rects move too.
        assert_eq!(plain.cell_rect(1, 1), Rect::new(32.0, 32.0, 32.0, 32.0));
        assert_eq!(
            moved.cell_rect(1, 1),
            Rect::new(132.0, 232.0, 32.0, 32.0)
        );
    }

    #[test]
    fn cell_rects_tile_up_without_gaps_or_overlaps() {
        let map = map();

        let first = map.cell_rect(0, 0);
        let second = map.cell_rect(1, 0);

        assert_eq!(first.x + first.w, second.x);
        assert_eq!(first.y, second.y);
        assert_eq!(first.w, second.w);

        let centre = map.cell_center(0, 0);
        assert_eq!(centre, Vec2::new(16.0, 16.0));
    }

    #[test]
    fn only_the_cells_on_screen_are_visible() {
        let map = map();

        // Centred on cell (1, 1), showing 400x400 world units.
        let camera = Camera::new(Vec2::new(48.0, 48.0));
        let visible: Vec<_> = map
            .visible(camera, (400, 400))
            .map(|(col, row, _)| (col, row))
            .collect();

        // The whole 8x4 map fits inside a 400-wide view, so all of it shows.
        assert_eq!(visible.len(), map.cols() * map.rows());

        // Far away, nothing shows.
        let off = Camera::new(Vec2::new(5000.0, 5000.0));
        assert_eq!(map.visible(off, (400, 400)).count(), 0);
    }

    #[test]
    fn a_narrow_view_only_reaches_some_of_the_columns() {
        let map = map();

        // A 100-wide window centred on x=64 covers 14..114, plus the cell of
        // padding on each side: columns 0..4, so four of the eight.
        let camera = Camera::new(Vec2::new(64.0, 64.0));
        let cols: std::collections::HashSet<_> = map
            .visible(camera, (100, 100))
            .map(|(col, _, _)| col)
            .collect();

        assert!(cols.len() < map.cols(), "should not show every column");
        assert!(cols.contains(&0), "the padding cell at the left shows");
    }

    #[test]
    fn zooming_out_reaches_further_across_the_map() {
        let map = map();

        let near = Camera::new(Vec2::new(64.0, 64.0));
        let far = Camera::new(Vec2::new(64.0, 64.0)).with_zoom(0.25);

        assert!(map.visible(far, (400, 400)).count() >= map.visible(near, (400, 400)).count());
    }

    #[test]
    fn a_map_smaller_than_the_window_is_centred() {
        let mut camera = Camera::new(Vec2::new(9999.0, 9999.0));
        let map = map(); // 256x128 world units.

        map.clamp_camera(&mut camera, (800, 600));

        assert_eq!(camera.position, Vec2::new(128.0, 64.0));
    }

    #[test]
    fn a_larger_map_stops_the_camera_at_its_edges() {
        let mut camera = Camera::new(Vec2::new(0.0, 0.0));
        let map = TileMap::from_rows(vec![vec![Cell::Floor; 40]; 40], 32.0).unwrap();

        map.clamp_camera(&mut camera, (400, 400));

        // Half the view is 200, the map is 1280 wide: the camera may not go
        // below 200 or above 1080, and it was asked to sit at 0.
        assert_eq!(camera.position.x, 200.0);
        assert_eq!(camera.position.y, 200.0);

        // And from the other side, it stops at the far edge.
        camera.position = Vec2::new(99999.0, 99999.0);
        map.clamp_camera(&mut camera, (400, 400));
        assert_eq!(camera.position, Vec2::new(1080.0, 1080.0));
    }

    #[test]
    fn an_origin_shifts_where_the_camera_may_look() {
        let mut camera = Camera::new(Vec2::ZERO);
        let map = TileMap::from_rows(vec![vec![Cell::Floor; 40]; 40], 32.0)
            .unwrap()
            .origin(500.0, 0.0);

        map.clamp_camera(&mut camera, (400, 400));

        // Left edge is now x=500, so the camera cannot sit at 200.
        assert_eq!(camera.position.x, 700.0);
    }

    #[test]
    fn rows_are_flattened_row_major() {
        let rows = vec![
            vec![Cell::Floor, Cell::Wall],
            vec![Cell::Water, Cell::Floor],
        ];
        let map = TileMap::from_rows(rows, 16.0).unwrap();

        assert_eq!(map.cols(), 2);
        assert_eq!(map.rows(), 2);
        assert_eq!(map.get(1, 0), Some(&Cell::Wall));
        assert_eq!(map.get(0, 1), Some(&Cell::Water));
        assert_eq!(map.get(1, 1), Some(&Cell::Floor));
    }

    // --- failures ---

    #[test]
    fn no_rows_is_an_error() {
        assert_eq!(
            TileMap::<Cell>::from_ascii(&[], 32.0, parse),
            Err(MapError::NoRows)
        );
        assert_eq!(
            TileMap::<Cell>::from_ascii(&["", ""], 32.0, parse),
            Err(MapError::NoRows)
        );
        assert_eq!(TileMap::<Cell>::from_rows(vec![], 32.0), Err(MapError::NoRows));
    }

    #[test]
    fn rows_of_different_lengths_are_rejected() {
        let got = TileMap::from_ascii(&["....", "...", "...."], 32.0, parse);

        assert_eq!(
            got,
            Err(MapError::RaggedRow {
                row: 1,
                expected: 4,
                got: 3
            })
        );
    }

    #[test]
    fn rows_of_different_lengths_from_code_are_rejected_too() {
        let got = TileMap::from_rows(
            vec![
                vec![Cell::Floor, Cell::Floor],
                vec![Cell::Floor],
            ],
            32.0,
        );

        assert_eq!(
            got,
            Err(MapError::RaggedRow {
                row: 1,
                expected: 2,
                got: 1
            })
        );
    }

    #[test]
    fn a_character_the_parser_rejects_stops_the_map() {
        let got = TileMap::from_ascii(&["....", "..X."], 32.0, parse);

        assert_eq!(
            got,
            Err(MapError::UnknownChar {
                row: 1,
                col: 2,
                ch: 'X'
            })
        );
    }

    #[test]
    fn a_cell_size_that_is_not_positive_is_rejected() {
        // Compared without `assert_eq` because `NaN == NaN` is false, and
        // `MapError` derives `PartialEq` on a payload that can hold one.
        for cell in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            let got = TileMap::<Cell>::from_ascii(&[".."], cell, parse);

            assert!(
                matches!(got, Err(MapError::BadCell { cell: got }) if got.is_nan() || !got.is_finite() || got <= 0.0),
                "cell size {cell} should be rejected, got {got:?}"
            );
        }
    }

    #[test]
    fn errors_read_as_sentences() {
        assert!(!MapError::NoRows.to_string().is_empty());
        assert!(MapError::RaggedRow {
            row: 2,
            expected: 5,
            got: 3
        }
        .to_string()
        .contains("row 2"));
        assert!(MapError::UnknownChar {
            row: 1,
            col: 0,
            ch: 'x'
        }
        .to_string()
        .contains('x'));
    }
}
