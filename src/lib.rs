//! Rust bindings to SDL3.
//!
//! A window with a renderer, window and input events, and a [`TileMap`] for
//! laying tiles out over a world.
//!
//! With the `wgpu` feature the crate also grows a second window flavour,
//! [`gpu::WgpuWindow`], where SDL3 still owns the window and input but `wgpu`
//! owns the swapchain and the drawing. The two do not mix in one window: a
//! [`Window`] draws through `SDL_Renderer`, a [`gpu::WgpuWindow`] does not.
//!
//! # Example
//!
//! ```no_run
//! use frglib::{Color, Event, Window};
//! use std::time::Duration;
//!
//! # fn main() -> Result<(), frglib::Error> {
//! let mut window = Window::builder("FRGLib")
//!     .size(800, 600)
//!     .background(Color::GREEN)
//!     .build()?;
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

pub mod event;
#[cfg(feature = "wgpu")]
pub mod gpu;
pub mod graphics;
pub mod math;
pub mod tilemap;
pub mod timing;
pub mod ui;
pub mod window;

pub use event::{Event, Keycode, Mod, MouseButton, MouseWheelDirection};
#[cfg(feature = "wgpu")]
pub use gpu::Error as GpuError;
#[cfg(feature = "wgpu")]
pub use gpu::{
    Camera3D, Frame, Gpu, GpuMesh, Material, Mesh, Renderer3D, Vertex, WgpuWindow,
    WgpuWindowBuilder,
};
#[cfg(feature = "cached-textures")]
pub use graphics::{Animation, Texture};
pub use graphics::{Camera, Color, Image, Object};
pub use math::{cos, lerp, sin, Ease, Lerp, Mat3, Rect, Vec2};
pub use tilemap::{MapError, TileMap};
pub use timing::FrameClock;
pub use ui::{Anchor, Button, ButtonState, Label, Slider, SliderState, TextCache};
pub use window::{Draw, Error, Window, WindowBuilder};
