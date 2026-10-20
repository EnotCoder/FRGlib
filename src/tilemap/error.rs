//! What a malformed map failed on.

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
            } => {
                write!(f, "row {row} has {got} cells, expected {expected}")
            }
            MapError::UnknownChar { row, col, ch } => {
                write!(f, "unrecognised {ch:?} at row {row}, column {col}")
            }
            MapError::BadCell { cell } => write!(f, "cell size {cell} is not positive"),
        }
    }
}

impl std::error::Error for MapError {}
