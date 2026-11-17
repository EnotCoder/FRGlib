//! Пример "Solar System" на FRGLib.
//!
//! Игровой цикл живёт в `game.rs`: тот же файл собирает Android-библиотека
//! `space_android`, которой нужен экспортированный `SDL_main`, а не `main`.

mod assets;
mod bodies;
mod consts;
mod draw;
mod game;
mod selection;
mod ui;
mod view;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    game::run()
}
