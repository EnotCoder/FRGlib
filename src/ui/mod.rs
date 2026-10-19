//! Interface pieces drawn on top of the game.
//!
//! Widgets here know nothing about fonts. A button takes its caption as an
//! [`Image`] the caller already rendered, and a [`Label`] is that same idea
//! pushed further: it is not a widget at all, just a place for a picture. That
//! keeps SDL3_ttf and every other text tool on the caller's side of the line:
//! the one thing this crate has always refused to do is make everybody who
//! wants a caption pay for a font parser they never asked to use.

mod button;
mod slider;
mod text;

pub use button::{Button, ButtonState};
pub use slider::{Slider, SliderState};
pub use text::{Anchor, Label, TextCache};
