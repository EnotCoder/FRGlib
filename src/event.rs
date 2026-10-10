//! Window and input events.

// Re-exported so callers can match on them without depending on `sdl3`
// themselves; only these four leak out, not SDL's whole event type.
pub use sdl3::keyboard::{Keycode, Mod};
pub use sdl3::mouse::{MouseButton, MouseWheelDirection};

/// Something that happened, translated out of SDL's own event type.
///
/// SDL's queue is process-wide, so [`Window::poll`](crate::window::Window::poll)
/// filters by window id and only these events reach here.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// The application was asked to quit. Process-wide, not tied to a window.
    Quit,

    // --- window ---
    CloseRequested,
    Resized {
        width: u32,
        height: u32,
    },
    Moved {
        x: i32,
        y: i32,
    },
    FocusGained,
    FocusLost,
    Minimized,
    Maximized,
    Restored,

    // --- keyboard ---
    /// Keycode-less events are dropped during translation.
    KeyDown {
        key: Keycode,
        repeat: bool,
        mods: Mod,
    },
    KeyUp {
        key: Keycode,
        mods: Mod,
    },
    TextInput(String),

    // --- mouse ---
    MouseMotion {
        x: f32,
        y: f32,
    },
    MouseButtonDown {
        button: MouseButton,
        x: f32,
        y: f32,
        clicks: u8,
    },
    MouseButtonUp {
        button: MouseButton,
        x: f32,
        y: f32,
        clicks: u8,
    },
    MouseWheel {
        x: f32,
        y: f32,
        direction: MouseWheelDirection,
    },
}

impl Event {
    /// Whether the loop should stop after this event.
    pub fn should_close(&self) -> bool {
        matches!(self, Event::Quit | Event::CloseRequested)
    }
}

/// Translates one SDL event for `window_id`, or drops it.
///
/// Everything is filtered by window id because SDL has a single shared queue:
/// without this, one window would receive another's mouse and key events.
/// `Event::Quit` is the exception, as it belongs to the process rather than to
/// any window.
pub(crate) fn translate(sdl: sdl3::event::Event, window_id: u32) -> Option<Event> {
    use sdl3::event::{Event as Sdl, WindowEvent};

    match sdl {
        Sdl::Quit { .. } => Some(Event::Quit),

        Sdl::Window {
            window_id: id,
            win_event,
            ..
        } if id == window_id => match win_event {
            WindowEvent::CloseRequested => Some(Event::CloseRequested),
            // SDL reports i32 here; clamp rather than cast a negative into u32.
            WindowEvent::Resized(w, h) | WindowEvent::PixelSizeChanged(w, h) => {
                Some(Event::Resized {
                    width: w.max(0) as u32,
                    height: h.max(0) as u32,
                })
            }
            WindowEvent::Moved(x, y) => Some(Event::Moved { x, y }),
            WindowEvent::FocusGained => Some(Event::FocusGained),
            WindowEvent::FocusLost => Some(Event::FocusLost),
            WindowEvent::Minimized => Some(Event::Minimized),
            WindowEvent::Maximized => Some(Event::Maximized),
            WindowEvent::Restored => Some(Event::Restored),
            _ => None,
        },

        Sdl::KeyDown {
            window_id: id,
            keycode,
            keymod,
            repeat,
            ..
        } if id == window_id => keycode.map(|key| Event::KeyDown {
            key,
            repeat,
            mods: keymod,
        }),

        Sdl::KeyUp {
            window_id: id,
            keycode,
            keymod,
            ..
        } if id == window_id => keycode.map(|key| Event::KeyUp { key, mods: keymod }),

        Sdl::TextInput {
            window_id: id,
            text,
            ..
        } if id == window_id => Some(Event::TextInput(text)),

        Sdl::MouseMotion {
            window_id: id,
            x,
            y,
            ..
        } if id == window_id => Some(Event::MouseMotion { x, y }),

        Sdl::MouseButtonDown {
            window_id: id,
            mouse_btn,
            clicks,
            x,
            y,
            ..
        } if id == window_id => Some(Event::MouseButtonDown {
            button: mouse_btn,
            x,
            y,
            clicks,
        }),

        Sdl::MouseButtonUp {
            window_id: id,
            mouse_btn,
            clicks,
            x,
            y,
            ..
        } if id == window_id => Some(Event::MouseButtonUp {
            button: mouse_btn,
            x,
            y,
            clicks,
        }),

        Sdl::MouseWheel {
            window_id: id,
            x,
            y,
            direction,
            ..
        } if id == window_id => Some(Event::MouseWheel { x, y, direction }),

        _ => None,
    }
}
