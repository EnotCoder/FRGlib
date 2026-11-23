//! Камера свободного полёта: осматриваем ландшафт со всех сторон.
//!
//! WASD двигают камеру по горизонту, Space поднимает, C опускает, стрелки
//! поворачивают её, а зажатая мышь (правая кнопка или левая вне кнопки HUD)
//! смотрит по направлению движения. Колесо меняет скорость. Камера не
//! проваливается под рельеф: её высота ограничена [`Terrain::height_at`].

use frglib::gpu::glam::Vec3;
use frglib::gpu::Camera3D;
use frglib::{Event, Keycode, MouseButton, MouseWheelDirection, Rect, Vec2};

use crate::terrain::{Terrain, SIZE};

/// Минимальный зазор камеры над рельефом: радиус её «коллизии» для отладки.
pub const MIN_HEIGHT_ABOVE_GROUND: f32 = 2.5;
/// Граница полёта по X/Z: половина карты плюс запас.
pub const HALF_SIZE: f32 = SIZE * 0.5 + 20.0;

pub struct FlyCamera {
    position: Vec3,
    yaw: f32,
    pitch: f32,
    speed: f32,
    pressed: Vec<Keycode>,
    dragging: bool,
    last_mouse: Vec2,
}

impl FlyCamera {
    pub fn new() -> Self {
        FlyCamera {
            position: Vec3::new(0.0, 46.0, 92.0),
            yaw: 0.0,
            pitch: -0.42,
            speed: 34.0,
            pressed: Vec::new(),
            dragging: false,
            last_mouse: Vec2::ZERO,
        }
    }

    /// Позиция камеры для HUD.
    pub fn position(&self) -> Vec3 {
        self.position
    }

    /// Текущая скорость полёта для HUD.
    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// Разбирает события кадра. `button` — область кнопки в координатах окна:
    /// нажатие на неё не должно поворачивать камеру.
    pub fn handle(&mut self, event: &Event, button: &Rect) {
        match event {
            Event::KeyDown { key, .. } => {
                if !self.pressed.contains(key) {
                    self.pressed.push(*key);
                }
            }
            Event::KeyUp { key, .. } => self.pressed.retain(|held| held != key),
            Event::FocusLost => {
                self.pressed.clear();
                self.dragging = false;
            }
            Event::MouseMotion { x, y } => {
                if self.dragging {
                    let delta = Vec2::new(*x, *y) - self.last_mouse;
                    // По горизонтали знак инвертирован: тянем вправо — смотрим
                    // вправо. По вертикали обычный порядок: вверх — вверх.
                    self.yaw += delta.x * 0.005;
                    self.pitch = (self.pitch - delta.y * 0.005).clamp(-1.48, 1.48);
                }
                self.last_mouse = Vec2::new(*x, *y);
            }
            Event::MouseButtonDown { button: key, x, y, .. } => {
                let over_button = button.contains(*x, *y);
                if *key == MouseButton::Right || (*key == MouseButton::Left && !over_button) {
                    self.dragging = true;
                    self.last_mouse = Vec2::new(*x, *y);
                }
            }
            Event::MouseButtonUp { button: key, .. } => {
                if *key == MouseButton::Right || *key == MouseButton::Left {
                    self.dragging = false;
                }
            }
            Event::MouseWheel { y, direction, .. } => {
                let sign = if *direction == MouseWheelDirection::Flipped {
                    -1.0
                } else {
                    1.0
                };
                if *y > 0.0 {
                    self.speed = (self.speed * 1.15 * sign).clamp(6.0, 180.0);
                } else if *y < 0.0 {
                    self.speed = (self.speed / 1.15 * sign).clamp(6.0, 180.0);
                }
            }
            _ => {}
        }
    }

    /// Обновляет положение и возвращает камеру для отрисовки кадра.
    pub fn update(&mut self, dt: f32, terrain: &Terrain) -> Camera3D {
        let held = |key| self.pressed.contains(&key);

        // Стрелки поворачивают камеру, если мышь не задействована.
        if held(Keycode::Left) {
            self.yaw += 1.6 * dt;
        }
        if held(Keycode::Right) {
            self.yaw -= 1.6 * dt;
        }
        if held(Keycode::Up) {
            self.pitch = (self.pitch + 1.2 * dt).clamp(-1.48, 1.48);
        }
        if held(Keycode::Down) {
            self.pitch = (self.pitch - 1.2 * dt).clamp(-1.48, 1.48);
        }

        let forward = Vec3::new(self.yaw.sin(), 0.0, -self.yaw.cos());
        let right = Vec3::new(self.yaw.cos(), 0.0, self.yaw.sin());

        let mut move_dir = Vec3::ZERO;
        if held(Keycode::W) {
            move_dir += forward;
        }
        if held(Keycode::S) {
            move_dir -= forward;
        }
        if held(Keycode::D) {
            move_dir += right;
        }
        if held(Keycode::A) {
            move_dir -= right;
        }
        if held(Keycode::Space) {
            move_dir += Vec3::Y;
        }
        if held(Keycode::C) {
            move_dir -= Vec3::Y;
        }

        let boost = if held(Keycode::LShift) || held(Keycode::RShift) {
            3.0
        } else {
            1.0
        };

        if move_dir.length_squared() > 0.0 {
            self.position += move_dir.normalize() * self.speed * boost * dt;
        }

        // Не улетаем за край карты и не проваливаемся внутрь гор.
        self.position.x = self.position.x.clamp(-HALF_SIZE, HALF_SIZE);
        self.position.z = self.position.z.clamp(-HALF_SIZE, HALF_SIZE);
        let floor = terrain.height_at(self.position.x, self.position.z) + MIN_HEIGHT_ABOVE_GROUND;
        self.position.y = self.position.y.max(floor);

        let look = forward * self.pitch.cos() + Vec3::Y * self.pitch.sin();
        Camera3D::perspective(60.0, 0.1, 600.0)
            .position(self.position)
            .target(self.position + look)
    }
}
