// Windowing and event handling on top of SDL3.
mod error;
mod event;

pub use error::Error;

use event::with_pump;

use sdl3::event::{Event, WindowEvent};
use sdl3::video::Window as SdlWindow;
use sdl3::Sdl;

pub struct Window {
    window: SdlWindow,
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

        Ok(Self { window, _sdl: sdl })
    }

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
