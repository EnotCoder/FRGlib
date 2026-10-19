//! Interface pieces drawn on top of the game.
//!
//! Widgets here know nothing about fonts. A button takes its caption as an
//! [`Image`] the caller already rendered, which is what keeps SDL3_ttf and
//! every other text tool on the caller's side of the line: the one thing this
//! crate has always refused to do is make everybody who wants a caption pay
//! for a font parser they never asked to use.

mod button;
mod slider;

pub use button::{Button, ButtonState};
pub use slider::{Slider, SliderState};
