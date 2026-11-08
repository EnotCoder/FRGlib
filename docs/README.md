# Документация FRGLib

Версия крейта: **0.4.0**  
Минимальный Rust: **1.82**  
Основа: **SDL3** (крейт `sdl3` 0.20)

FRGLib — небольшая 2D-библиотека поверх SDL3: окно, рендерер, события,
рисование примитивов и текстур, камера, интерфейсные виджеты, тайлмапы и
немного математики. Опциональная фича `wgpu` добавляет параллельное окно с
современным GPU-бэкендом (Vulkan, Metal, DX12): 3D-сцену, 2D-примитивы,
текстуры, текст, UI и тайлмапы. Отдельного Vulkan-рендерера вне `wgpu` нет.

Это оглавление документации. Каждый файл — отдельная тема; ссылки ниже ведут
на подробности.

## Оглавление

| Файл | О чём |
|---|---|
| [01-architecture.md](01-architecture.md) | Как устроен проект целиком: слои, жизненный цикл кадра, владение, координаты, ошибки, фичи |
| [02-window.md](02-window.md) | `Window`, `WindowBuilder`, `Draw`, событийный насос, рисование и геометрия |
| [03-graphics.md](03-graphics.md) | `Color`, `Image`, `Object`, `Camera`, `Texture`, `Animation` |
| [04-math.md](04-math.md) | `Vec2`, `Rect`, `Mat3`, `Ease`, `Lerp`, `sin`/`cos` |
| [05-events-and-timing.md](05-events-and-timing.md) | `Event`, трансляция событий SDL, `FrameClock` |
| [06-ui.md](06-ui.md) | `Label`, `Anchor`, `TextCache`, `Button`, `Slider` |
| [07-tilemap.md](07-tilemap.md) | `TileMap<T>`, `MapError`, видимость, отрисовка тайлов |
| [08-examples.md](08-examples.md) | Каталог примеров и ассетов |
| [09-build-and-dev.md](09-build-and-dev.md) | Сборка, фичи, тесты, правила разработки, известные особенности |
| [10-wgpu.md](10-wgpu.md) | Полное руководство по `wgpu`: разбор каждого файла `src/gpu`, API, 3D- и 2D-рендереры, текст, UI, тайлмапы, свои проходы |

## За 60 секунд

```rust
use frglib::{Color, Event, FrameClock, Object, Window};

fn main() -> Result<(), frglib::Error> {
    let mut window = Window::builder("FRGLib")
        .size(800, 600)
        .background(Color::FON)
        .build()?;

    let mut clock = FrameClock::new(60.0);

    loop {
        let dt = clock.dt();

        for event in window.poll()? {
            if event.should_close() {
                return Ok(());
            }
        }

        window.frame_with(|d| {
            let square = Object::from_xywh(400.0, 300.0, 120.0, 120.0);
            d.fill_object(&square, Color::rgb(220, 60, 60))?;
            Ok(())
        })?;

        let _ = dt; // здесь двигали бы объекты
        clock.tick();
    }
}
```

Порядок почти всегда один и тот же:

1. `Window::builder(...).build()` — открыть окно и рендерер.
2. `clock.dt()` — взять время прошлого кадра.
3. `window.poll()` — забрать события.
4. Обновить состояние игры.
5. `window.frame_with(|d| ...)` — нарисовать кадр.
6. `clock.tick()` — доспать остаток кадра.

## Структура репозитория

```text
FRGLib/
├── Cargo.toml          # пакет, фичи, dev-зависимости, список примеров
├── Cargo.lock
├── README.md           # пользовательская документация и таблицы API
├── LICENSE             # MIT
├── logo.svg
├── src/
│   ├── lib.rs          # корень крейта: модули и публичные реэкспорты
│   ├── event.rs        # Event + трансляция из SDL
│   ├── timing.rs       # FrameClock
│   ├── gpu/            # окно и рендереры на wgpu (фича wgpu)
│   │   ├── mod.rs
│   │   ├── window.rs   # WgpuWindow, Gpu, Frame
│   │   ├── renderer.rs # Renderer3D
│   │   ├── renderer2d.rs # Renderer2D, GpuTexture
│   │   ├── font.rs     # FontId, атлас глифов, раскладка текста
│   │   ├── camera.rs
│   │   ├── mesh.rs
│   │   └── shaders/
│   ├── window/         # окно, рендерер, события окна, рисование
│   │   ├── mod.rs
│   │   ├── builder.rs
│   │   ├── error.rs
│   │   ├── pump.rs
│   │   └── draw/
│   │       ├── mod.rs
│   │       ├── object.rs
│   │       └── geometry.rs
│   ├── graphics/       # что рисуется
│   │   ├── mod.rs
│   │   ├── color.rs
│   │   ├── image.rs
│   │   ├── object.rs
│   │   ├── camera.rs
│   │   ├── texture.rs  # только с фичей cached-textures
│   │   └── animation.rs# только с фичей cached-textures
│   ├── math/           # Vec2, Rect, Mat3, Ease, Lerp
│   │   ├── mod.rs
│   │   ├── rect.rs
│   │   ├── mat3.rs
│   │   └── ease.rs
│   ├── ui/             # интерфейс поверх кадра
│   │   ├── mod.rs
│   │   ├── text.rs
│   │   ├── button.rs
│   │   └── slider.rs
│   └── tilemap/        # сетка клеток
│       ├── mod.rs
│       ├── map.rs
│       ├── view.rs
│       ├── error.rs
│       └── tests.rs
└── examples/           # примеры, в публикуемый пакет не входят
```

## Карта модулей

| Модуль / файл | Назначение | Ключевые типы |
|---|---|---|
| `src/lib.rs` | Собирает модули и делает удобные реэкспорты | `Window`, `Draw`, `Event`, `Color`, `Object`, `Camera`, `Rect`, `Vec2`, `TileMap`, `Button`, `Slider`, `Label` |
| `src/window/` | Всё, что связано с окном и рендерером SDL | `Window`, `WindowBuilder`, `Draw`, `Error` |
| `src/window/draw/` | Публичный API рисования и его геометрия | методы `Draw::fill_*`, `Draw::draw_*` |
| `src/graphics/` | Данные, которые попадают на экран | `Color`, `Image`, `Object`, `Camera`, `Texture`, `Animation` |
| `src/math/` | 2D-математика и интерполяция | `Vec2`, `Rect`, `Mat3`, `Ease`, `Lerp` |
| `src/ui/` | Виджеты и размещение готового текста | `Button`, `Slider`, `Label`, `Anchor`, `TextCache` |
| `src/tilemap/` | Сетка клеток, координаты, видимость | `TileMap<T>`, `MapError` |
| `src/event.rs` | Своё перечисление событий вместо SDL-типа | `Event` |
| `src/timing.rs` | Держит цикл на заданной частоте | `FrameClock` |
| `src/gpu/` | 3D и 2D через `wgpu` | `WgpuWindow`, `Renderer3D`, `Renderer2D`, `Camera3D`, `FontId`, `GpuTexture` |

## Главные принципы

1. **Публичный API не течёт типами `sdl3`.** `Color`, `Rect`, `Vec2` и
   `Object` — собственные типы, чтобы обновление `sdl3` не ломало API
   библиотеки. Исключения: `Image::from_surface` (принимает `sdl3::surface::Surface`)
   и реэкспорт `Keycode`, `Mod`, `MouseButton`, `MouseWheelDirection`.
2. **Шрифтов в SDL-слое нет.** Текст рисует вызывающий код (например,
   SDL3_ttf), а `frglib` только принимает готовую `Image`. Поэтому `Button` и
   `Label` работают с картинками, а не со строками. Исключение — опциональный
   `wgpu`-слой: там `Renderer2D` сам разбирает TTF/OTF через `fontdue`, потому
   что SDL-рендерера в этом окне нет.
3. **Значение клетки тайлмапа — дело игры.** `TileMap<T>` не требует от `T`
   ни `Copy`, ни трейтов: библиотека знает, где клетка, но не что она значит.
4. **Дорогие текстуры — опционально.** По умолчанию `Image` заливается на GPU
   при каждом рисовании. Фича `cached-textures` позволяет залить её один раз и
   рисовать по дешёвому индексу (`Texture`).
5. **Ошибки — значения, а не паника.** Там, где SDL позволяет, методы
   возвращают `Result`. Паники остаются только в местах, где сам `sdl3`
   паникует, и в `TextCache::get` при обращении к незакешированной строке.
6. **Vulkan — через `wgpu`, и по желанию.** `WindowBuilder::vulkan(true)`
   ставит флаг окна, а `Window::vulkan_instance_extensions()` отдаёт
   расширения, но собственного Vulkan-рендерера у SDL-окна нет. Современный
   бэкенд живёт в `WgpuWindow` под фичей `wgpu`.

## Быстрые команды

```bash
cargo build
cargo test
cargo run --example basic-window
cargo run --features cached-textures --example basic-cached
cargo run --features wgpu --example wgpu-cube
cargo run --features wgpu --example wgpu-hud
cargo doc --open
```

Подробнее — в [09-build-and-dev.md](09-build-and-dev.md).
