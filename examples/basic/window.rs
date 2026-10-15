//! Opens a window, fills it with a colour, and prints what happens.
//!
//! Запуск: `cargo run --example basic-window`

use frglib::{Color, Event, FrameClock, Window};

fn main() -> Result<(), frglib::Error> {
    let mut window = Window::builder("FRGLib")
        .size(800, 600)
        .resizable(true)
        .vulkan(false)
        .background(Color::FON)
        .build()?;

    println!(
        "size {:?}, background {:?}",
        window.size(),
        window.background()
    );

    let mut clock = FrameClock::new(60.0);

    loop {
        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::Resized { width, height } => println!("resized to {width}x{height}"),
                Event::KeyDown { key, repeat, .. } => {
                    println!(
                        "key {key:?}{} (repeat={repeat})",
                        if repeat { " held" } else { "" }
                    )
                }
                Event::MouseMotion { x, y } => println!("mouse at {x:.0},{y:.0}"),
                other => println!("event: {other:?}"),
            }
        }

        window.frame()?;
        clock.tick();
    }
}
