//! Minimal Android entry point.
//!
//! SDL's Java layer dlopen()s `libmain.so` and looks up a C symbol named
//! `SDL_main`, so this is the Android counterpart of `fn main`. A Rust `main`
//! or `#[sdl3_main::main]` is not enough: the macro generates a local `main`,
//! while Java asks for `SDL_main` by name.
//!
//! The template exists to prove that the whole chain works before a real game
//! is involved: Rust cdylib -> libSDL3.so -> Gradle -> APK. Swap `run` for the
//! game loop and update `android/game.properties`.

use std::ffi::c_char;
use std::time::Duration;

use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use sdl3::pixels::Color;

/// Entry point looked up by `SDLActivity` through `dlsym`.
///
/// # Safety
///
/// Called by SDL with a valid `argc`/`argv`, on the thread that already set up
/// the Android JNI environment.
#[no_mangle]
pub unsafe extern "C" fn SDL_main(_argc: i32, _argv: *mut *mut c_char) -> i32 {
    match run() {
        Ok(()) => 0,
        Err(err) => {
            sdl3::log::log_error(
                sdl3::log::Category::Application,
                &format!("FRGLib Android template: {err}"),
            );
            1
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let sdl = sdl3::init()?;
    let video = sdl.video()?;

    // On Android the window is the device surface; the requested size is only
    // a hint. Desktop builds of this template still get an 800x600 window.
    let window = video.window("FRGLib Android template", 800, 600).build()?;
    let mut canvas = sdl3::render::create_renderer(window, None)?;
    let mut pump = sdl.event_pump()?;

    canvas.set_draw_color(Color::RGB(12, 16, 28));
    canvas.clear();
    canvas.present();

    loop {
        let event = pump.wait_event_timeout(Duration::from_millis(250));
        if let Some(event) = event {
            match event {
                Event::Quit { .. } => break,
                Event::KeyDown {
                    keycode: Some(Keycode::Escape | Keycode::AcBack),
                    ..
                } => break,
                _ => {}
            }
        }

        canvas.clear();
        canvas.present();
    }

    Ok(())
}
