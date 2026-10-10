## FRGLib

Rust-обвязка над SDL3: окно, рендерер и обработка событий. Слой для Vulkan
впереди.

## Требования

- Rust 1.82+
- SDL3 (для сборки по умолчанию берётся системная библиотека)
- `cmake` и C-компилятор — только если включаешь фичу `bundled`

## Сборка и запуск

```bash
cargo build
cargo run --example window
```

## Использование

```rust
use frglib::{Color, Event, Window};
use std::time::Duration;

fn main() -> Result<(), frglib::Error> {
    let mut window = Window::builder("FRGLib")
        .size(800, 600)
        .resizable(true)
        .vulkan(false)
        .background(Color::GREEN)
        .build()?;

    loop {
        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::Resized { width, height } => println!("{width}x{height}"),
                _ => {}
            }
        }

        window.frame()?;
        std::thread::sleep(Duration::from_millis(16));
    }
}
```

## API

| Вызов | Описание |
|---|---|
| `Window::builder(title)` | Начинает сборку окна. Заголовок обязателен, остальное необязательно |
| `.size(w, h)` | Размер в пикселях. По умолчанию 800x600 |
| `.resizable(bool)` | Разрешить ресайз. По умолчанию `true` |
| `.vulkan(bool)` | Флаг `SDL_WINDOW_VULKAN`. По умолчанию `false` |
| `.background(Color)` | Цвет фона. По умолчанию `Color::GREEN` |
| `.build()` | Открывает окно, возвращает `Result<Window, Error>` |
| `poll() -> Result<Vec<Event>, Error>` | Разбирает очередь событий за кадр |
| `frame() -> Result<(), Error>` | Заливает окно текущим цветом и показывает кадр |
| `set_background(Color)` | Меняет цвет фона на лету |
| `background() -> Color` | Текущий цвет фона |
| `size() -> (u32, u32)` | Текущий размер окна |
| `vulkan_instance_extensions() -> Result<Vec<String>, Error>` | Расширения Vulkan, нужные этому окну |

`Event::should_close()` отвечает на вопрос, стоит ли продолжать цикл.

### События

`poll()` возвращает `Vec<Event>`: `Quit`, `CloseRequested`, `Resized`, `Moved`,
`FocusGained`/`FocusLost`, `Minimized`/`Maximized`/`Restored`, `KeyDown`,
`KeyUp`, `TextInput`, `MouseMotion`, `MouseButtonDown`, `MouseButtonUp`,
`MouseWheel`.

У SDL одна очередь событий на весь процесс, поэтому события фильтруются по
идентификатору окна: чужое окно не получит твои события мыши и клавиатуры.
Исключение — `Event::Quit`, он не привязан к окну.

## SDL3 без зависимости от системы

По умолчанию линкуется системная SDL3. Фича `bundled` собирает её из исходников
и линкует статически — полезно, если итоговый бинарник должен таскать SDL3 с
собой:

```bash
cargo build --release --features bundled
```

Оговорка про `bundled`: в `sdl3-sys` флаг `build-from-source` сам по себе не
переключает линковку на статическую — за это отвечает отдельный `link-static`.
Нужен именно `build-from-source-static`, как и прописано в фиче.

## Ограничения

- SDL требует, чтобы окно создавалось и использовалось в том же потоке, который
  первым инициализировал SDL. `Window` не `Send` и не `Sync`, и рендерер
  создаётся только на главном потоке — так что `frame()` тоже оттуда.
- События ввода и клавиатуры не диспетчеризуются: `poll()` сообщает только о
  том, что окно закрывается, остальное выбрасывает. Это следующий шаг.
- `set_draw_color` и `clear` в крейте `sdl3` `panic!` вместо `Result`, так что
  битый рендерер вылезет паникой, а не `Error`. `present` ошибку возвращает
  честно, этот случай обработан.
