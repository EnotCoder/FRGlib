//! Куб и всё, что его касается: прыжок, гравитация, столкновения.

use crate::level::{BODY, GROUND, TILE};
use frglib::Vec2;

/// С чего игрок начинает, в мире.
pub const START_X: f32 = 3.0 * TILE;

/// Где куб стоит на экране. Мир едет влево, поэтому и камера едет вправо.
pub const SCREEN_X: f32 = -200.0;

/// Скорость бега, пикселей в секунду.
pub const SPEED: f32 = 320.0;

/// Тяжесть вниз.
pub const GRAVITY: f32 = 2400.0;

/// Начальная скорость прыжка вверх.
pub const JUMP: f32 = 780.0;

/// Сколько от прыжка остаётся, если отпустить кнопку на подъёме.
pub const JUMP_CUT: f32 = 0.42;

/// Насколько можно промахнуться мимо верхушки платформы и всё равно
/// приземлиться. Без этого на быстром падении пересечение верхней плоскости
/// находится ровно в одной точке и легко проскакивает.
pub const LAND_TOLERANCE: f32 = 1.5;

/// Шип проверяем не всем тайлом, а чуть меньшим квадратом: треугольник в
/// файле уже сужается кверху, и полный тайл убивал бы впритык.
pub const SPIKE_INSET: f32 = 5.0;

/// Что происходит с игроком.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum State {
    Run,
    Dead,
    Won,
}

/// Всё про одного игрока.
pub struct Player {
    pub x: f32,
    pub y: f32,
    pub vy: f32,
    pub grounded: bool,
    /// Прыжок уже урезан отпусканием — второй раз резать нельзя.
    pub cut: bool,
    pub state: State,
    pub spin: f32,
}

impl Player {
    pub fn new() -> Self {
        Player {
            x: START_X,
            y: GROUND - BODY / 2.0,
            vy: 0.0,
            grounded: true,
            cut: false,
            state: State::Run,
            spin: 0.0,
        }
    }

    pub fn center(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    /// Тайлы, которых касается куб по горизонтали.
    pub fn columns(&self) -> (isize, isize) {
        let half = BODY / 2.0;
        (
            ((self.x - half) / TILE).floor() as isize,
            ((self.x + half) / TILE).floor() as isize,
        )
    }
}
