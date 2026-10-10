//! Открывает окно и заливает его сплошным цветом.
//!
//! Запуск: `cargo run --example window`

use std::time::Duration;

use frglib::window::Window;

fn main() -> Result<(), frglib::window::Error> {
    // title, width, height, resizable, vulkan
    let mut window = Window::new("FRGLib", 800, 600, true, false)?;

    window.set_background(0, 200, 0, 255);
    println!(
        "size {:?}, background {:?}",
        window.size(),
        window.background()
    );

    while window.poll()? {
        window.frame()?;
        std::thread::sleep(Duration::from_millis(16));
    }

    Ok(())
}
