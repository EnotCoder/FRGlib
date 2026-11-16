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

/// Свёрнутая подсказка: одна строка вместо шести, полный список -- по `H`.
pub const CONTROLS: &str = "H help   LMB select   MMB pan   wheel zoom   SPACE pause   R reset";

// --- Окно ---------------------------------------------------------------

pub const WINDOW_TITLE: &str = "FRGlib: Space";
pub const WINDOW_SIZE: (u32, u32) = (1200, 900);
pub const WINDOW_BACKGROUND: Color = Color::rgb(7, 9, 20);

// --- Интерфейс ----------------------------------------------------------

/// Размеры шрифтов: заголовок карточки крупнее остального текста.
pub const FONT_TITLE: f32 = 24.0;
pub const FONT_HUD: f32 = 18.0;

/// Общая палитра панелей: один фон, одна рамка и один акцентный цвет.
pub const PANEL_BG: Color = Color::rgba(9, 12, 22, 224);
pub const PANEL_BORDER: Color = Color::rgba(92, 124, 178, 84);
pub const PANEL_SEPARATOR: Color = Color::rgba(92, 124, 178, 58);
pub const PANEL_ACCENT: Color = Color::rgba(120, 176, 255, 210);
pub const PANEL_ACCENT_WIDTH: f32 = 3.0;

/// Цвета текста. Текст кэшируется вместе с цветом, поэтому их несколько.
pub const TEXT_TITLE: Color = Color::rgba(236, 242, 252, 255);
pub const TEXT_BODY: Color = Color::rgba(190, 201, 220, 255);
pub const TEXT_BRIGHT: Color = Color::rgba(242, 246, 252, 255);
pub const TEXT_DIM: Color = Color::rgba(132, 146, 172, 255);
pub const TEXT_ACCENT: Color = Color::rgba(126, 180, 255, 255);
pub const TEXT_GOOD: Color = Color::rgba(124, 226, 154, 255);
pub const TEXT_WARN: Color = Color::rgba(255, 193, 104, 255);
pub const TEXT_SHADOW: Color = Color::rgba(4, 6, 12, 255);

/// Акцент карточки зависит от типа тела: звезда, планета или спутник.
pub const ACCENT_STAR: Color = Color::rgba(255, 197, 112, 255);
pub const ACCENT_PLANET: Color = Color::rgba(128, 184, 255, 255);
pub const ACCENT_MOON: Color = Color::rgba(190, 202, 222, 255);

// Карточка тела в левом верхнем углу.
pub const INFO_PANEL_ORIGIN: Vec2 = Vec2::new(24.0, 24.0);
pub const INFO_PANEL_WIDTH: f32 = 320.0;
pub const INFO_PANEL_PADDING: f32 = 16.0;
pub const INFO_TITLE_GAP: f32 = 12.0;
pub const INFO_ROW_HEIGHT: f32 = 24.0;
pub const INFO_SECTION_GAP: f32 = 12.0;
pub const INFO_FOOTER_GAP: f32 = 10.0;

// HUD в правом верхнем углу.
pub const HUD_PANEL_SIZE: Vec2 = Vec2::new(272.0, 216.0);
pub const HUD_PANEL_MARGIN: Vec2 = Vec2::new(18.0, 18.0);
pub const HUD_TEXT_X: f32 = 18.0;
pub const HUD_HEADER_Y: f32 = 14.0;
pub const HUD_SEPARATOR_1_Y: f32 = 44.0;
pub const HUD_SPEED_Y: f32 = 58.0;
pub const HUD_SEPARATOR_2_Y: f32 = 88.0;
pub const HUD_TOGGLES_Y: f32 = 102.0;
pub const HUD_ROW_HEIGHT: f32 = 28.0;

// Компактная строка состояния в правом нижнем углу.
pub const STATS_MARGIN: Vec2 = Vec2::new(18.0, 18.0);
pub const STATS_PADDING: Vec2 = Vec2::new(14.0, 10.0);
pub const STATS_GAP: f32 = 9.0;
pub const STATS_BLOCK_GAP: f32 = 22.0;

// Подсказка и полный список управления в левом нижнем углу.
pub const CONTROLS_MARGIN: Vec2 = Vec2::new(18.0, 18.0);
pub const CONTROLS_PADDING: Vec2 = Vec2::new(12.0, 10.0);

pub const HELP_TITLE: &str = "CONTROLS";
pub const HELP_PANEL_WIDTH: f32 = 420.0;
pub const HELP_PANEL_PADDING: f32 = 16.0;
pub const HELP_TITLE_GAP: f32 = 12.0;
pub const HELP_SECTION_HEIGHT: f32 = 26.0;
pub const HELP_SECTION_GAP: f32 = 12.0;
pub const HELP_ROW_HEIGHT: f32 = 24.0;
pub const HELP_KEY_X: f32 = 5.0;
pub const HELP_DESC_X: f32 = 170.0;

const VIEW_ROWS: &[(&str, &str)] = &[
    ("MMB drag", "pan camera"),
    ("arrows", "pan camera"),
    ("wheel", "zoom to cursor"),
    ("R", "reset view"),
];

const SELECT_ROWS: &[(&str, &str)] = &[
    ("LMB", "inspect a body"),
    ("double-click", "follow a body"),
    ("1-8 / 0", "planet / sun"),
    ("F", "follow selection"),
];

const TIME_ROWS: &[(&str, &str)] = &[
    ("SPACE", "pause / resume"),
    ("+ / -", "time speed"),
    ("O / B / L / M", "orbits / belts / labels / moons"),
];

const APP_ROWS: &[(&str, &str)] = &[("H", "hide this panel"), ("ESC", "back"), ("Q", "quit")];

pub const HELP_SECTIONS: &[(&str, &[(&str, &str)])] = &[
    ("VIEW", VIEW_ROWS),
    ("SELECT", SELECT_ROWS),
    ("TIME", TIME_ROWS),
    ("APP", APP_ROWS),
];

pub const LABEL_GAP: f32 = 6.0;
pub const LABEL_CULL_X: f32 = 80.0;
pub const LABEL_CULL_Y: f32 = 20.0;
pub const LABEL_SPAWN_MARGIN: f32 = 120.0;
pub const PLANET_LABEL_ZOOM: f32 = 0.45;
pub const MOON_LABEL_ZOOM: f32 = 1.4;

/// Шаг, с которым показывается FPS: значение округляется до пяти.
pub const FPS_BUCKET: f32 = 5.0;
