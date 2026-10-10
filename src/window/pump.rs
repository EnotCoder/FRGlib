use std::cell::RefCell;

use sdl3::{EventPump, Sdl};

use super::error::Error;

thread_local! {
    static PUMP:
        RefCell<Option<Box<EventPump>>> =
            const { RefCell::new(None) };
}

/// Runs `f` against the thread's event pump, creating it on first use.
///
/// `pub(crate)` because `super` imports it: a private item is visible only in its
/// own module and its descendants, never in the parent.
pub(crate) fn with_pump<R>(
    sdl: &Sdl,
    f: impl FnOnce(&mut EventPump) -> Result<R, Error>,
) -> Result<R, Error> {
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
