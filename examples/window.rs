//! Opens a window, fills it with a colour, and prints what happens.
//!
//! Запуск: `cargo run --example window`

use std::time::Duration;

use frglib::{Color, Event, Window};

fn main() -> Result<(), frglib::Error> {
    let mut window = Window::builder("FRGLib")
        .size(800, 600)
        .resizable(true)
        .vulkan(false)
        .background(Color::WHITE)
        .build()?;

    println!(
        "size {:?}, background {:?}",
        window.size(),
        window.background()
    );

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
        std::thread::sleep(Duration::from_millis(16));
    }
}
