//! Все настраиваемые значения примера в одном месте.
//!
//! Логика и отрисовка читают отсюда размеры, скорости и цвета панелей:
//! так поведение сцены можно подкрутить, не разыскивая числа по файлам.

use frglib::{Color, Vec2};

// --- Мир -----------------------------------------------------------------

/// Сторона спрайта Солнца в мировых единицах.
pub const SUN_SIZE: f32 = 120.0;

/// Радиус, в который вписывается система при сбросе камеры: чуть больше
/// орбиты Нептуна и пояса Койпера.
pub const SYSTEM_RADIUS: f32 = 1_080.0;

/// Насколько орбиты сплющены по вертикали. Меньше `y` -- дальше тело.
pub const ORBIT_TILT: f32 = 0.38;

// --- Камера --------------------------------------------------------------

pub const MIN_ZOOM: f32 = 0.05;
pub const MAX_ZOOM: f32 = 12.0;
pub const ZOOM_STEP: f32 = 0.1;
pub const FIT_MARGIN: f32 = 0.95;

pub const CAMERA_MOVE_SPEED: f32 = 1_000.0;
pub const CAMERA_FRICTION: f32 = 10.0;
pub const CAMERA_FOLLOW_SPEED: f32 = 4.0;
pub const SCROLL_MOVE_SPEED: f32 = 60.0;

/// Минимальный радиус попадания мышью, в пикселях экрана.
pub const MIN_HIT_RADIUS: f32 = 6.0;

// --- Ввод ---------------------------------------------------------------

pub const DOUBLE_CLICK_MS: u64 = 350;
pub const DOUBLE_CLICK_RADIUS: f32 = 24.0;

// --- Время --------------------------------------------------------------

/// Множители времени. Пауза -- отдельный флаг, чтобы не терять скорость.
pub const TIME_SPEEDS: [f32; 6] = [0.25, 0.5, 1.0, 2.0, 5.0, 10.0];
pub const DEFAULT_SPEED: usize = 2;

pub const CONTROLS: &str = "LMB select   double-click follow\n\
MMB drag   wheel zoom   arrows move\n\
1-8 planet   0 sun   F follow\n\
SPACE pause   +/- speed\n\
O orbits   B belts   L labels   M moons\n\
R reset view   ESC back   Q quit";

// --- Окно ---------------------------------------------------------------

pub const WINDOW_TITLE: &str = "FRGlib: Space";
pub const WINDOW_SIZE: (u32, u32) = (1200, 900);
pub const WINDOW_BACKGROUND: Color = Color::rgb(7, 9, 20);

// --- Интерфейс ----------------------------------------------------------

pub const INFO_PANEL_ORIGIN: Vec2 = Vec2::new(24.0, 24.0);
pub const INFO_PANEL_PADDING: f32 = 12.0;
pub const INFO_PANEL_BG: Color = Color::rgba(12, 14, 22, 228);
pub const INFO_PANEL_BORDER: Color = Color::rgba(90, 120, 170, 90);

pub const HUD_PANEL_SIZE: Vec2 = Vec2::new(220.0, 100.0);
pub const HUD_PANEL_MARGIN: Vec2 = Vec2::new(18.0, 18.0);
pub const HUD_PANEL_BG: Color = Color::rgba(12, 14, 22, 200);
pub const HUD_PANEL_BORDER: Color = Color::rgba(90, 120, 170, 70);
pub const HUD_TEXT_X: f32 = 12.0;
pub const HUD_TIME_Y: f32 = 10.0;
pub const HUD_LAYERS_Y: f32 = 32.0;
pub const HUD_FPS_Y: f32 = 74.0;

pub const CONTROLS_MARGIN: Vec2 = Vec2::new(18.0, 18.0);
pub const CONTROLS_PADDING: Vec2 = Vec2::new(10.0, 10.0);
pub const CONTROLS_BG: Color = Color::rgba(12, 14, 22, 190);

pub const LABEL_GAP: f32 = 6.0;
pub const LABEL_CULL_X: f32 = 80.0;
pub const LABEL_CULL_Y: f32 = 20.0;
pub const LABEL_SPAWN_MARGIN: f32 = 120.0;
pub const PLANET_LABEL_ZOOM: f32 = 0.45;
pub const MOON_LABEL_ZOOM: f32 = 1.4;

/// Шаг, с которым показывается FPS: значение округляется до пяти.
pub const FPS_BUCKET: f32 = 5.0;
