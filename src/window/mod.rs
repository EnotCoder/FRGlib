// Windowing and event handling on top of SDL3.
mod builder;
mod draw;
mod error;
mod pump;

pub use builder::WindowBuilder;
pub use draw::Draw;
pub use error::Error;

use pump::with_pump;

use crate::graphics::Color;
use sdl3::render::{create_renderer, ScaleMode, WindowCanvas};
use sdl3::Sdl;

pub struct Window {
    // Declaration order is drop order. The textures are declared first, so they
    // are destroyed before the canvas that owns the renderer -- destroying them
    // afterwards would call `SDL_DestroyTexture` on a dead renderer. The canvas
    // itself comes next, and has to go while `_sdl` is still alive.
    #[cfg(feature = "cached-textures")]
    pub(crate) textures: crate::graphics::texture::Store,
    pub(crate) canvas: WindowCanvas,
    // How every texture drawn on this window is sampled when it lands at a size
    // other than its own. There is no renderer-wide default in SDL3, so this is
    // applied to each texture as it is created rather than once at startup.
    pub(crate) scale: ScaleMode,
    background: Color,
    _sdl: Sdl,
}

impl Window {
    /// Starts building a window. See [`WindowBuilder`].
    pub fn builder(title: impl Into<String>) -> WindowBuilder {
        WindowBuilder::new(title)
    }

    /// Opens the window. Called by [`WindowBuilder::build`].
    fn create(
        title: String,
        width: u32,
        height: u32,
        resizable: bool,
        vulkan: bool,
        background: Color,
        scale: ScaleMode,
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

        let mut builder = video.window(&title, width, height);
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
            #[cfg(feature = "cached-textures")]
            textures: crate::graphics::texture::Store::new(),
            canvas,
            scale,
            background,
            _sdl: sdl,
        })
    }

    /// Drains the event queue and returns everything that happened this frame.
    ///
    /// SDL keeps one queue for the whole process, so events are filtered by
    /// window id and only this window's reach the caller. `Event::Quit` is the
    /// exception, as it is not tied to any window.
    ///
    /// Use [`crate::Event::should_close`] to decide whether to keep looping.
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

    /// How far the mouse moved since the last call, in pixels.
    ///
    /// Read exactly like [`FrameClock::dt`](crate::FrameClock::dt): one call
    /// per frame, and a second call in the same frame returns zero, because
    /// SDL hands out the movement and clears its own counter on the way.
    ///
    /// `x` grows to the right, `y` grows down. The result is a movement, not a
    /// position, so it has to be added to where the thing already is:
    ///
    /// ```no_run
    /// # fn main() -> Result<(), frglib::Error> {
    /// # let mut window = frglib::Window::builder("demo").build()?;
    /// # let mut at = frglib::Vec2::ZERO;
    /// let step = window.relative_mouse()? * 0.1;
    /// at += step;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// While the cursor is against an edge of the window the movement is zero:
    /// the mouse stops there, so there is nothing to report. Getting movement
    /// past the edge needs the cursor captured, which this does not do yet.
    ///
    /// The error is the same one [`poll`](Self::poll) can return: the event
    /// pump is created on first use and SDL can refuse to hand it over.
    pub fn relative_mouse(&mut self) -> Result<crate::math::Vec2, Error> {
        with_pump(&self._sdl, |pump| {
            let state = pump.relative_mouse_state();
            Ok(crate::math::Vec2::new(state.x(), state.y()))
        })
    }

    /// Current drawable size, as `(width, height)`.
    pub fn size(&self) -> (u32, u32) {
        self.canvas.window().size()
    }

    /// Whether textures on this window keep hard pixel edges when scaled.
    ///
    /// Reports what [`WindowBuilder::pixel_art`] asked for at startup. There is
    /// no way to change it afterwards, since textures are filtered as they are
    /// created and the ones already uploaded would have to be revisited.
    pub fn pixel_art(&self) -> bool {
        self.scale == ScaleMode::Nearest
    }

    /// Sets the colour `frame` fills the window with.
    pub fn set_background(&mut self, color: crate::graphics::Color) {
        self.background = color;
    }

    /// The colour `frame` currently fills the window with.
    pub fn background(&self) -> crate::graphics::Color {
        self.background
    }

    /// Fills the window with the background colour without presenting it.
    fn clear(&mut self) {
        self.canvas.set_draw_color(self.background);
        self.canvas.clear();
    }

    /// Presents whatever has been drawn.
    fn present(&mut self) -> Result<(), Error> {
        if !self.canvas.present() {
            return Err(Error::Present(sdl3::get_error()));
        }
        Ok(())
    }

    /// Draws one frame: fills with the background colour, then presents it.
    ///
    /// SDL has no API for the background of a window, so this has to run every
    /// frame -- there is nothing to set once at startup.
    ///
    /// Use [`Window::frame_with`] to put shapes on top of the background.
    ///
    /// # Panics
    ///
    /// `set_draw_color` and `clear` `panic!` in `sdl3` instead of returning a
    /// `Result`, so a broken renderer surfaces as a panic rather than as an
    /// `Error`. `present` does report failure, and that case is handled.
    pub fn frame(&mut self) -> Result<(), Error> {
        self.clear();
        self.present()
    }

    /// Draws one frame with shapes on top of the background.
    ///
    /// The background is cleared first, then `draw` runs, then the frame is
    /// presented. `draw` may return an error to abort the frame.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), frglib::Error> {
    /// # let mut window = frglib::Window::builder("demo").build()?;
    /// window.frame_with(|d| {
    ///     let square = frglib::Object::from_xywh(200.0, 200.0, 200.0, 200.0);
    ///     d.fill_object(&square, frglib::Color::rgb(220, 60, 60))?;
    ///     Ok(())
    /// })?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Same caveat as [`Window::frame`]: `clear` and `set_draw_color` panic
    /// inside `sdl3` rather than returning an error.
    pub fn frame_with(
        &mut self,
        draw: impl FnOnce(&mut Draw<'_>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        self.clear();
        {
            // Destructured rather than read as two separate fields: the drawing code
            // needs `&mut canvas` and `&textures` at once, which only works if
            // the borrows are split up front. The `textures` binding is gated so
            // the default build, which has no store, still compiles. `scale`
            // binds by reference too, but it is `Copy`, so `*scale` hands over a
            // plain value and the field keeps being borrowed at once.
            let Self {
                canvas,
                scale,
                #[cfg(feature = "cached-textures")]
                textures,
                ..
            } = self;
            let mut handle = Draw::new(
                canvas,
                *scale,
                #[cfg(feature = "cached-textures")]
                textures,
            );
            draw(&mut handle)?;
        }
        self.present()
    }

    /// The Vulkan instance extensions this window needs.
    ///
    /// Feed this straight into `ash::Instance::enumerate_instance_extensions`
    /// surface-extension checks.
    pub fn vulkan_instance_extensions(&self) -> Result<Vec<String>, Error> {
        Ok(self.canvas.window().vulkan_instance_extensions()?)
    }
}
