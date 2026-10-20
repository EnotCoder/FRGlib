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
//!
//! The files:
//!
//! - [`error`] -- what a malformed map failed on;
//! - [`map`] -- the storage, the constructors, and the cell lookups;
//! - [`view`] -- which cells the camera sees, and drawing them.

pub mod error;
pub mod map;
pub mod view;

#[cfg(test)]
mod tests;

pub use error::MapError;
pub use map::TileMap;
