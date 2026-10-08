// Windowing and event handling on top of SDL3.
mod error;
mod event;

pub use error::Error;

use event::with_pump;

use sdl3::event::{Event, WindowEvent};
use sdl3::pixels::Color;
use sdl3::render::{create_renderer, WindowCanvas};
use sdl3::Sdl;

pub struct Window {
    // Declaration order is drop order: `WindowCanvas` owns both the renderer and
    // the SDL window, so it has to be torn down while `_sdl` is still alive.
    canvas: WindowCanvas,
    background: Color,
    _sdl: Sdl,
}

impl Window {
    pub fn new(
        title: &str,
        width: u32,
        height: u32,
        resizable: bool,
        vulkan: bool,
    ) -> Result<Self, Error> {
        let sdl = sdl3::init()?;
        let video = sdl.video()?;

        let mut builder = video.window(title, width, height);
        if resizable {
            builder.resizable();
        }
        if vulkan {
            builder.vulkan();
        }
        let window = builder.build()?;

        // `create_renderer` takes the window by value and hands back a Result,
        // which is why we use it instead of `Window::into_canvas`: that one is
        // infallible in its signature and `panic!`s internally on failure. SDL
        // docs suggest `SDL_CreateWindowAndRenderer` to avoid startup flicker,
        // but it takes no window flags, so `resizable`/`vulkan` would be lost.
        let canvas = create_renderer(window, None)?;

        Ok(Self {
            canvas,
            background: Color {
                r: 0,
                g: 200,
                b: 0,
                a: 255,
            },
            _sdl: sdl,
        })
    }

    pub fn poll(&mut self) -> Result<bool, Error> {
        let window_id = self.canvas.window().id();
        let sdl = &self._sdl;

        with_pump(sdl, |pump| {
            let mut running = true;
            while let Some(event) = pump.poll_event() {
                match event {
                    Event::Quit { .. } => running = false,
                    // Matched on `window_id`: the C++ version closed on any
                    // window's close request, including other windows'.
                    Event::Window {
                        window_id: id,
                        win_event: WindowEvent::CloseRequested,
                        ..
                    } if id == window_id => running = false,
                    _ => {}
                }
            }
            Ok(running)
        })
    }

    /// Current drawable size, as `(width, height)`.
    pub fn size(&self) -> (u32, u32) {
        self.canvas.window().size()
    }

    /// Sets the colour `frame` fills the window with.
    pub fn set_background(&mut self, r: u8, g: u8, b: u8, a: u8) {
        self.background = Color { r, g, b, a };
    }

    /// The colour `frame` currently fills the window with.
    pub fn background(&self) -> (u8, u8, u8, u8) {
        let c = self.background;
        (c.r, c.g, c.b, c.a)
    }

    /// Draws one frame: fills with the background colour, then presents it.
    ///
    /// SDL has no API for the background of a window, so this has to run every
    /// frame -- there is nothing to set once at startup.
    ///
    /// # Panics
    ///
    /// `set_draw_color` and `clear` `panic!` in `sdl3` instead of returning a
    /// `Result`, so a broken renderer surfaces as a panic (PyO3 turns that into
    /// `PanicException`) rather than as an `Error`. `present` does report
    /// failure, and that case is handled.
    pub fn frame(&mut self) -> Result<(), Error> {
        self.canvas.set_draw_color(self.background);
        self.canvas.clear();
        if !self.canvas.present() {
            return Err(sdl3::get_error().into());
        }
        Ok(())
    }

    /// The Vulkan instance extensions this window needs.
    ///
    /// Feed this straight into `ash::Instance::enumerate_instance_extensions`
    /// surface-extension checks.
    pub fn vulkan_instance_extensions(&self) -> Result<Vec<String>, Error> {
        Ok(self.canvas.window().vulkan_instance_extensions()?)
    }
}
