//! Windowing and event handling on top of SDL3.
//!
//! Deliberately free of any Python types, so the same code can back a native
//! executable later without duplication.

use std::cell::RefCell;
use std::fmt;

use sdl3::event::{Event, WindowEvent};
use sdl3::video::Window as SdlWindow;
use sdl3::{EventPump, Sdl};

/// An SDL failure, carrying the `SDL_GetError()` text.
///
/// The C++ version returned a bare `0` on failure and dropped the message on
/// the floor; keeping it means Python gets a real exception.
#[derive(Debug, Clone)]
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<sdl3::Error> for Error {
    fn from(err: sdl3::Error) -> Self {
        Error(err.to_string())
    }
}

thread_local! {
    /// SDL permits exactly one `EventPump` per process: the `sdl3` crate flips a
    /// global flag on creation and never clears it, so a second attempt fails.
    /// That restriction matches reality anyway — SDL has a single global event
    /// queue, so one pump shared by every window is the correct model.
    static PUMP: RefCell<Option<Box<EventPump>>> = const { RefCell::new(None) };
}

/// Runs `f` against the thread's event pump, creating it on first use.
fn with_pump<R>(sdl: &Sdl, f: impl FnOnce(&mut EventPump) -> Result<R, Error>) -> Result<R, Error> {
    PUMP.with(|slot| {
        let mut guard = slot.borrow_mut();
        if guard.is_none() {
            let pump = sdl.event_pump()?;
            *guard = Some(Box::new(pump));
        }
        // Cannot be `None`: either it already was, or we just filled it in.
        let pump = guard.as_mut().expect("event pump was just initialised");
        f(pump.as_mut())
    })
}

/// An SDL3 window.
///
/// Must be created on, and used from, the thread that first initialised SDL —
/// SDL itself has that restriction and the `sdl3` crate enforces it.
pub struct Window {
    // Declaration order is drop order. The SDL window has to be destroyed
    // before the `Sdl` handle is released, so it is declared first.
    window: SdlWindow,
    _sdl: Sdl,
}

impl Window {
    /// Opens a window.
    ///
    /// `resizable` actually takes effect here: `builder.resizable()` is applied
    /// to a flag set that starts empty, and the resulting flags are what gets
    /// handed to `SDL_CreateWindow`. The C++ version built `flags` and then
    /// passed a hardcoded `SDL_WINDOW_RESIZABLE` instead.
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
        let window = builder.build().map_err(|err| Error(err.to_string()))?;

        Ok(Self { window, _sdl: sdl })
    }

    /// Drains the event queue.
    ///
    /// Returns `false` once this window has been asked to close, or once SDL
    /// has been told to quit, after which the caller should stop looping.
    pub fn poll(&mut self) -> Result<bool, Error> {
        let window_id = self.window.id();
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
        self.window.size()
    }

    /// The Vulkan instance extensions this window needs.
    ///
    /// Feed this straight into `ash::Instance::enumerate_instance_extensions`
    /// surface-extension checks.
    pub fn vulkan_instance_extensions(&self) -> Result<Vec<String>, Error> {
        Ok(self.window.vulkan_instance_extensions()?)
    }
}
