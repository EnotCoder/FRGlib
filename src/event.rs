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

    // --- touch ---
    //
    // SDL reports fingers in normalized `0..1` coordinates; these are already
    // multiplied by the window size, so `x`/`y` share the mouse's coordinate
    // space. A finger is identified by `id`, which stays the same for as long
    // as that finger touches the screen.
    FingerDown {
        id: u64,
        x: f32,
        y: f32,
        pressure: f32,
    },
    FingerMotion {
        id: u64,
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
        pressure: f32,
    },
    FingerUp {
        id: u64,
        x: f32,
        y: f32,
        pressure: f32,
    },
    /// The system took the touch away: a notification, a gesture, or the app
    /// losing focus. Treat it like [`Event::FingerUp`] without a click.
    FingerCanceled {
        id: u64,
        x: f32,
        y: f32,
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
///
/// `width` and `height` are the window size in the same units as mouse events.
/// SDL normalizes finger coordinates to `0..1`, so the size is what turns them
/// back into a position a caller can compare with `MouseMotion`.
pub(crate) fn translate(
    sdl: sdl3::event::Event,
    window_id: u32,
    width: u32,
    height: u32,
) -> Option<Event> {
    use sdl3::event::{Event as Sdl, WindowEvent};

    let w = width as f32;
    let h = height as f32;

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

        Sdl::FingerDown {
            window_id: id,
            finger_id,
            x,
            y,
            pressure,
            ..
        } if id == window_id => Some(Event::FingerDown {
            id: finger_id,
            x: x * w,
            y: y * h,
            pressure,
        }),

        Sdl::FingerMotion {
            window_id: id,
            finger_id,
            x,
            y,
            dx,
            dy,
            pressure,
            ..
        } if id == window_id => Some(Event::FingerMotion {
            id: finger_id,
            x: x * w,
            y: y * h,
            dx: dx * w,
            dy: dy * h,
            pressure,
        }),

        Sdl::FingerUp {
            window_id: id,
            finger_id,
            x,
            y,
            pressure,
            ..
        } if id == window_id => Some(Event::FingerUp {
            id: finger_id,
            x: x * w,
            y: y * h,
            pressure,
        }),

        Sdl::FingerCanceled {
            window_id: id,
            finger_id,
            x,
            y,
            ..
        } if id == window_id => Some(Event::FingerCanceled {
            id: finger_id,
            x: x * w,
            y: y * h,
        }),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finger_coordinates_are_scaled_to_the_window() {
        let sdl = sdl3::event::Event::FingerDown {
            timestamp: 0,
            touch_id: 1,
            finger_id: 7,
            x: 0.5,
            y: 0.25,
            dx: 0.0,
            dy: 0.0,
            pressure: 1.0,
            window_id: 3,
        };

        assert_eq!(
            translate(sdl, 3, 800, 600),
            Some(Event::FingerDown {
                id: 7,
                x: 400.0,
                y: 150.0,
                pressure: 1.0,
            })
        );
    }

    #[test]
    fn finger_motion_scales_the_delta_too() {
        let sdl = sdl3::event::Event::FingerMotion {
            timestamp: 0,
            touch_id: 1,
            finger_id: 7,
            x: 0.75,
            y: 0.5,
            dx: 0.25,
            dy: -0.5,
            pressure: 0.5,
            window_id: 3,
        };

        assert_eq!(
            translate(sdl, 3, 800, 600),
            Some(Event::FingerMotion {
                id: 7,
                x: 600.0,
                y: 300.0,
                dx: 200.0,
                dy: -300.0,
                pressure: 0.5,
            })
        );
    }

    #[test]
    fn fingers_from_another_window_are_dropped() {
        let sdl = sdl3::event::Event::FingerUp {
            timestamp: 0,
            touch_id: 1,
            finger_id: 7,
            x: 0.5,
            y: 0.5,
            dx: 0.0,
            dy: 0.0,
            pressure: 0.0,
            window_id: 4,
        };

        assert_eq!(translate(sdl, 3, 800, 600), None);
    }
}
