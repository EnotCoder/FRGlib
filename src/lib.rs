//! Rust bindings to SDL3.
//!
//! A window with a renderer, plus window and input events. A Vulkan layer is
//! still to come.
//!
//! # Example
//!
//! ```no_run
//! use frglib::{Color, Event, Window};
//! use std::time::Duration;
//!
//! # fn main() -> Result<(), frglib::Error> {
//! let mut window = Window::new("FRGLib", 800, 600, true, false)?;
//! window.set_background(Color::GREEN);
//!
//! loop {
//!     for event in window.poll()? {
//!         match event {
//!             Event::Quit | Event::CloseRequested => return Ok(()),
//!             Event::Resized { width, height } => println!("now {width}x{height}"),
//!             _ => {}
//!         }
//!     }
//!
//!     window.frame()?;
//!     std::thread::sleep(Duration::from_millis(16));
//! }
//! # }
//! ```
//!
//! # Limitations
//!
//! [`Window`] is intentionally neither `Send` nor `Sync`: SDL requires a window
//! to be created and used on the thread that initialised SDL first, and `sdl3`
//! keeps its types `!Send` for the same reason. That is normally the main
//! thread. The renderer, like the window, can only be created on the main
//! thread, so [`Window::frame`] has to be called from there too.

pub mod color;
pub mod event;
pub mod window;

pub use color::Color;
pub use event::Event;
pub use window::{Error, Window};
