// Windowing and event handling on top of SDL3.
mod error;
mod pump;

pub use error::Error;

use pump::with_pump;

use crate::color::Color;
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
        // SDL3 asks the X11 compositor to bypass our window by default, which
        // sets _NET_WM_BYPASS_COMPOSITOR=1 and makes the window get drawn
        // straight to the framebuffer. A bypassed window never receives
        // compositor effects, so under picom it loses the blur, the rounded
        // corners, the shadow and the open/close animations. SDL says to set
        // this before creating a window.
        sdl3::hint::set("SDL_VIDEO_X11_NET_WM_BYPASS_COMPOSITOR", "0");

        let sdl = sdl3::init()?;
        let video = sdl.video()?;

        let mut builder = video.window(title, width, height);
        if resizable {
            builder.resizable();
        }
        if vulkan {
            builder.vulkan();
        }
        // SDL picks the first available renderer driver, and on Linux that is
        // `opengl`. SDL expects a window to carry SDL_WINDOW_OPENGL before a GL
        // context is attached to it, so setting it here keeps the flag and the
        // backend that `create_renderer` below is about to choose in agreement.
        builder.opengl();
        let window = builder.build()?;

        // `create_renderer` takes the window by value and hands back a Result,
        // which is why we use it instead of `Window::into_canvas`: that one is
        // infallible in its signature and `panic!`s internally on failure. SDL
        // docs suggest `SDL_CreateWindowAndRenderer` to avoid startup flicker,
        // but it takes no window flags, so `resizable`/`vulkan` would be lost.
        let canvas = create_renderer(window, None)?;

        Ok(Self {
            canvas,
            background: Color::GREEN,
            _sdl: sdl,
        })
    }

    /// Drains the event queue and returns everything that happened this frame.
    ///
    /// SDL keeps one queue for the whole process, so events are filtered by
    /// window id and only this window's reach the caller. `Event::Quit` is the
    /// exception, as it is not tied to any window.
    ///
    /// Use [`Event::should_close`] to decide whether to keep looping.
    pub fn poll(&mut self) -> Result<Vec<crate::event::Event>, Error> {
        let window_id = self.canvas.window().id();
        let sdl = &self._sdl;

        with_pump(sdl, |pump| {
            let mut events = Vec::new();
            while let Some(event) = pump.poll_event() {
                if let Some(event) = crate::event::translate(event, window_id) {
                    events.push(event);
                }
            }
            Ok(events)
        })
    }

    /// Current drawable size, as `(width, height)`.
    pub fn size(&self) -> (u32, u32) {
        self.canvas.window().size()
    }

    /// Sets the colour `frame` fills the window with.
    pub fn set_background(&mut self, color: crate::color::Color) {
        self.background = color;
    }

    /// The colour `frame` currently fills the window with.
    pub fn background(&self) -> crate::color::Color {
        self.background
    }

    /// Draws one frame: fills with the background colour, then presents it.
    ///
    /// SDL has no API for the background of a window, so this has to run every
    /// frame -- there is nothing to set once at startup.
    ///
    /// # Panics
    ///
    /// `set_draw_color` and `clear` `panic!` in `sdl3` instead of returning a
    /// `Result`, so a broken renderer surfaces as a panic rather than as an
    /// `Error`. `present` does report failure, and that case is handled.
    pub fn frame(&mut self) -> Result<(), Error> {
        self.canvas.set_draw_color(self.background);
        self.canvas.clear();
        if !self.canvas.present() {
            return Err(Error::Present(sdl3::get_error()));
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
