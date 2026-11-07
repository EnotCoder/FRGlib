# 01. Архитектура

## Слои

Зависимости идут сверху вниз: верхние слои знают о нижних, нижние — нет.

```text
        пример / игра пользователя
                   │
     ┌─────────────┼──────────────┐
     │             │              │
   ui/         tilemap/       (ваш код)
     │             │
     └──────┬──────┘
            │
        window/  ── рисует и владеет окном
            │
        graphics/ ── что рисуется
            │
          math/  ── числа и преобразования
            │
          sdl3 / SDL3
```

- `math` не знает ни о SDL, ни об окне.
- `graphics` знает о `math`; `Image` хранит `sdl3::surface::Surface`, но
  остальные типы от SDL не зависят.
- `window` знает обо всём: именно он превращает `Object`, `Camera`, `Color` и
  `Image` в вызовы SDL-рендерера.
- `ui` и `tilemap` построены поверх `window::Draw`, `graphics` и `math`.

## Точка входа и реэкспорты

`src/lib.rs` объявляет модули:

```rust
pub mod event;
pub mod graphics;
pub mod math;
pub mod timing;
pub mod tilemap;
pub mod ui;
pub mod window;
```

и выносит самое частое на верхний уровень, чтобы пользователь писал
`frglib::Window`, а не `frglib::window::Window`. Часть реэкспортов закрыта
фичей `cached-textures`:

```rust
#[cfg(feature = "cached-textures")]
pub use graphics::{Animation, Texture};
pub use graphics::{Camera, Color, Image, Object};
pub use math::{cos, lerp, sin, Ease, Lerp, Mat3, Rect, Vec2};
pub use timing::FrameClock;
pub use tilemap::{MapError, TileMap};
pub use ui::{Anchor, Button, ButtonState, Label, Slider, SliderState, TextCache};
pub use window::{Draw, Error, Window, WindowBuilder};
```

Если добавляете новый публичный тип, сначала объявите его в модуле, затем
реэкспортируйте в `src/lib.rs` (и, если нужно, в `mod.rs` своего модуля).

## Старт приложения

`Window::builder(title)` создаёт `WindowBuilder` с умолчаниями:

| Параметр | Значение по умолчанию |
|---|---|
| Заголовок | обязателен |
| Размер | 800 × 600 |
| Изменяемый размер | `true` |
| Флаг Vulkan | `false` |
| Цвет фона | `Color::GREEN` |
| `pixel_art` | `false` (линейная фильтрация) |

`build()` вызывает `Window::create` (`src/window/mod.rs`), который:

1. Выставляет hint `SDL_VIDEO_X11_NET_WM_BYPASS_COMPOSITOR=0`, чтобы окно не
   теряло эффекты композитора (блюр, скругления, тени).
2. Инициализирует SDL: `sdl3::init()` → видео подсистема.
3. Создаёт окно; при необходимости добавляет `resizable`, `vulkan` и всегда
   `opengl` (SDL на Linux выбирает OpenGL-рендерер, и флаг окна должен с ним
   совпадать).
4. Создаёт рендерер через `create_renderer(window, None)`.
5. Кладёт в структуру `textures` (если есть фича), `canvas`, `scale`,
   `background` и `_sdl`.

`_sdl` хранится последним полем: порядок объявления — это порядок удаления.
Текстуры удаляются раньше canvas, а canvas — раньше `Sdl`. Иначе
`SDL_DestroyTexture` мог бы вызваться на уже мёртвом рендерере.

## Жизненный цикл кадра

```text
clock.dt()                 время прошлого кадра (секунды)
window.poll()              все события этого окна за кадр
[обновление состояния]     игровая логика, движение, камера
window.frame_with(|d| {    очистка фона
    d.set_camera(camera)   (необязательно) перевод world -> screen
    d.fill_...()           вызовы SDL-рендерера
})                         present
clock.tick()               сон до дедлайна следующего кадра
```

`frame()` — упрощённый вариант: только залить фон и показать кадр.
`frame_with` делает то же, но сначала выполняет замыкание.

Важно: `poll()` и `frame_with()` берут `&mut Window`, поэтому окно нельзя
держать заимствованным между кадрами. Именно поэтому `Texture` — это индекс, а
не объект, владеющий пикселями (см. [03-graphics.md](03-graphics.md)).

## Владение и потоки

- `Window` намеренно **не** `Send` и **не** `Sync`: SDL требует создавать и
  использовать окно в том потоке, где SDL был инициализирован (обычно главный).
- Рендерер тоже привязан к главному потоку, поэтому `frame()` и `frame_with()`
  вызываются оттуда же.
- Очередь событий SDL одна на процесс. `Window::poll()` фильтрует события по
  `window_id`, кроме `Event::Quit`.
- Event pump создаётся лениво и хранится в `thread_local!` (`src/window/pump.rs`),
  в `RefCell<Option<Box<EventPump>>>`. Повторный вызов переиспользует его.

## Системы координат

В библиотеке две системы:

| Система | Где используется | Ориентация |
|---|---|---|
| Экранные координаты | события мыши, `Draw` без камеры, UI, `Label`, `Button`, `Slider` | `x` вправо, `y` вниз, начало в левом верхнем углу |
| Мировые координаты | `Object`, `Camera::position`, `TileMap`, рисование с камерой | `x` вправо, `y` вниз |

Камера хранит **центр** обзора в мире и зум. Преобразования
(`src/graphics/camera.rs`):

```text
scale = zoom > EPSILON ? zoom : 1.0
centre = (width / 2, height / 2)

to_screen(world) = (world - camera.position) * scale + centre
to_world(screen) = (screen - centre) / scale + camera.position
```

`to_world` — то, что нужно для мыши: событие приходит в экранных координатах, а
игроку нужна точка мира.

`Camera::project_rect` проецирует только левый верхний угол и отдельно
масштабирует размер. Это корректно, потому что зум одинаков по обеим осям, но
только для **неповёрнутых** прямоугольников. Повёрнутые углы проецируются по
одному.

Углы поворота:

- `Object::rotation` — в **градусах**, положительное значение визуально
  поворачивает по часовой стрелке (потому что `y` растёт вниз).
- `Vec2::rotated` и `Mat3::rotation` — в **радианах**, стандартная
  математическая матрица.

## Поток рисования

```text
fill_object / fill_object_polygon / fill_textured_object / ...
        │
        ├── без поворота ──► приватный fill_rect / fill_polygon
        └── с поворотом  ──► geometry::polygon_vertices
                                  │
                                  ▼
                          canvas.render_geometry
```

`src/window/draw/` разделён на три части:

- `mod.rs` — приватные SDL-примитивы (`fill_rect`, `draw_polygon`,
  `fill_polygon`), камера, `screen_space`, `camera_rect`.
- `object.rs` — публичный API `*_object`, который добавляет к примитивам
  поворот, размер и отражение из `Object`.
- `geometry.rs` — чистая математика без canvas: индексы квада, поворот точек,
  проекция, сборка `Vertex`.

## Ошибки

Два независимых типа:

| Тип | Файл | Когда |
|---|---|---|
| `frglib::Error` | `src/window/error.rs` | SDL, создание окна/рендерера/текстуры, индексы полигона, `present` |
| `frglib::MapError` | `src/tilemap/error.rs` | битая карта: нет строк, рваные строки, неизвестный символ, плохой размер клетки |

`Error` реализует `std::error::Error` и `From` для ошибок `sdl3`, поэтому `?`
работает напрямую. `MapError` отдельный специально: добавление варианта в
`Error` сломало бы все `match` без `_`.

## Фичи

| Фича | Что включает | Зачем |
|---|---|---|
| `default` | пусто | Библиотека работает без дополнительных зависимостей |
| `cached-textures` | `sdl3/unsafe_textures`, модули `graphics/texture.rs`, `graphics/animation.rs`, `TileMap::draw` | Позволяет хранить текстуру между кадрами |
| `bundled` | `sdl3/build-from-source-static` | Собрать SDL3 из исходников и слинковать статически |

`cached-textures` — opt-in, потому что `sdl3` при отключении проверки времени
жизни текстуры разрешает использовать её с чужим canvas (UB). FRGLib этого не
делает: текстуры лежат внутри окна, которое их создало, и удаляются до
рендерера. Но риск должен быть осознанным выбором пользователя.

Подробнее о сборке — в [09-build-and-dev.md](09-build-and-dev.md).
