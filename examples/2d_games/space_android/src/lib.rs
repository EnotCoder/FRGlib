//! Android entry point for the `space` example.
//!
//! The game itself is not duplicated: the same source files are compiled into
//! this cdylib through `#[path]`, and [`game::run`] returns when the player
//! quits. Only the entry symbol differs -- SDL's Java layer dlsym()s
//! `SDL_main`, so a Rust `main` (or `#[sdl3_main::main]`) would not be found.
//!
//! Build it with `./android/build.sh`, which reads `android/game.properties`.

#[path = "../../space/assets.rs"]
mod assets;
#[path = "../../space/bodies/mod.rs"]
mod bodies;
#[path = "../../space/consts.rs"]
mod consts;
#[path = "../../space/draw/mod.rs"]
mod draw;
#[path = "../../space/game.rs"]
mod game;
#[path = "../../space/selection.rs"]
mod selection;
#[path = "../../space/ui/mod.rs"]
mod ui;
#[path = "../../space/view.rs"]
mod view;

use std::ffi::c_char;

/// Android-specific hints that have to be in place before the window exists.
///
/// SDLActivity chooses the activity orientation at runtime and overrides
/// `android:screenOrientation` from the manifest when the window is resizable,
/// so the hint has to agree with `orientation` in `android/game.properties`;
/// `build.sh` passes it here at compile time.
///
/// Trapping the back button delivers it to the game as `Keycode::AcBack`
/// instead of finishing the activity immediately. The game then decides:
/// close the panel, clear the selection, quit.
fn apply_android_hints() {
    let orientation = option_env!("FRGLIB_ORIENTATION").unwrap_or("landscape");
    let hint = match orientation {
        "landscape" => "LandscapeLeft LandscapeRight",
        "portrait" => "Portrait PortraitUpsideDown",
        _ => "",
    };

    sdl3::hint::set("SDL_ORIENTATIONS", hint);
    sdl3::hint::set("SDL_ANDROID_TRAP_BACK_BUTTON", "1");
}

/// Entry point looked up by `SDLActivity` through `dlsym`.
///
/// # Safety
///
/// Called by SDL with valid `argc`/`argv`, on the thread that already set up
/// the Android JNI environment.
#[no_mangle]
pub unsafe extern "C" fn SDL_main(_argc: i32, _argv: *mut *mut c_char) -> i32 {
    apply_android_hints();

    match game::run() {
        Ok(()) => 0,
        Err(err) => {
            sdl3::log::log_error(
                sdl3::log::Category::Application,
                &format!("space: {err}"),
            );
            1
        }
    }
}
