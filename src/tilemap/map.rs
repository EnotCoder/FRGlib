//! The storage, the constructors, and the cell lookups.

use super::error::MapError;
use crate::math::{Rect, Vec2};

/// A rectangular grid of cells of one size.
///
/// Row-major: the cell at `(col, row)` is `cells[row * cols + col]`. Cells are
/// held flat rather than as rows of rows, so a map is one allocation and a
/// row of it is one slice.
// The fields are `pub(super)` rather than private because `view.rs` is a
// sibling of this file, not a child: a private field would be invisible
// there. The module tree still cannot hand them out outside `tilemap`, so
// the public surface is the same as it ever was.
#[derive(Debug, Clone, PartialEq)]
pub struct TileMap<T> {
    pub(super) cells: Vec<T>,
    pub(super) cols: usize,
    pub(super) rows: usize,
    /// Side of one cell, in world units.
    pub(super) cell: f32,
    /// World position of the top-left corner of cell `(0, 0)`.
    pub(super) origin: Vec2,
}

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
