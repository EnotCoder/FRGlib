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
use frglib::window::Window;
use std::time::Duration;

fn main() -> Result<(), frglib::window::Error> {
    let mut window = Window::new("FRGLib", 800, 600, true, false)?;

    window.set_background(0, 200, 0, 255);
    while window.poll()? {
        window.frame()?;
        std::thread::sleep(Duration::from_millis(16));
    }
    Ok(())
}
```

## API

| Вызов | Описание |
|---|---|
| `Window::new(title, width, height, resizable, vulkan)` | Открывает окно с рендерером |
| `poll() -> Result<bool, Error>` | Разбирает очередь событий. `false` — пора закрываться |
| `frame() -> Result<(), Error>` | Заливает окно текущим цветом фона и показывает кадр |
| `set_background(r, g, b, a)` | Задаёт цвет фона |
| `background() -> (u8, u8, u8, u8)` | Возвращает текущий цвет фона |
| `size() -> (u32, u32)` | Текущий размер окна |
| `vulkan_instance_extensions() -> Result<Vec<String>, Error>` | Расширения Vulkan, нужные этому окну |

Дефолтный цвет фона — зелёный, `(0, 200, 0, 255)`.

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
