# 05. События и тайминг

Два независимых модуля:

- `src/event.rs` — собственное перечисление событий;
- `src/timing.rs` — `FrameClock`.

## `Event`

SDL отдаёт один огромный `sdl3::event::Event`. FRGLib переводит его в
маленькое перечисление, чтобы пользователь не зависел от `sdl3`.

```rust
pub enum Event {
    Quit,
    CloseRequested,
    Resized { width: u32, height: u32 },
    Moved { x: i32, y: i32 },
    FocusGained,
    FocusLost,
    Minimized,
    Maximized,
    Restored,
    KeyDown { key: Keycode, repeat: bool, mods: Mod },
    KeyUp { key: Keycode, mods: Mod },
    TextInput(String),
    MouseMotion { x: f32, y: f32 },
    MouseButtonDown { button: MouseButton, x: f32, y: f32, clicks: u8 },
    MouseButtonUp { button: MouseButton, x: f32, y: f32, clicks: u8 },
    MouseWheel { x: f32, y: f32, direction: MouseWheelDirection },
}
```

Реэкспорты из `sdl3`:

| Тип | Откуда |
|---|---|
| `Keycode` | `sdl3::keyboard` |
| `Mod` | `sdl3::keyboard` |
| `MouseButton` | `sdl3::mouse` |
| `MouseWheelDirection` | `sdl3::mouse` |

`Event::should_close()` возвращает `true` для `Quit` и `CloseRequested`. Это
удобный способ не писать `match` в каждой программе.

### Как SDL-событие превращается в `Event`

Функция `translate(sdl, window_id)` — `pub(crate)`, вызывается из
`Window::poll`.

| SDL-событие | Результат |
|---|---|
| `Quit` | `Event::Quit` (без фильтра по окну) |
| `Window::CloseRequested` | `Event::CloseRequested` |
| `Window::Resized(w, h)` / `PixelSizeChanged(w, h)` | `Event::Resized`, отрицательные значения зажимаются в 0 |
| `Window::Moved(x, y)` | `Event::Moved` |
| `Window::FocusGained` / `FocusLost` | Соответствующий вариант |
| `Window::Minimized` / `Maximized` / `Restored` | Соответствующий вариант |
| `KeyDown` | `Event::KeyDown`; событие без `keycode` отбрасывается |
| `KeyUp` | `Event::KeyUp`; без `keycode` отбрасывается |
| `TextInput` | `Event::TextInput(String)` |
| `MouseMotion` | `Event::MouseMotion` |
| `MouseButtonDown` / `Up` | `Event::MouseButtonDown` / `Up` |
| `MouseWheel` | `Event::MouseWheel` |
| Всё остальное | `None` (отбрасывается) |

### Фильтрация по окну

У SDL **одна очередь на процесс**. Без фильтра одно окно получало бы мышь и
клавиатуру другого. Поэтому каждое оконное событие проверяется на
`window_id == self.canvas.window().id()`. Исключение — `Quit`: он относится к
процессу, а не к окну.

### Получение событий

```rust
for event in window.poll()? {
    match event {
        Event::Quit | Event::CloseRequested => return Ok(()),
        Event::KeyDown { key, repeat, .. } => { /* ... */ }
        Event::MouseMotion { x, y } => { /* ... */ }
        _ => {}
    }
}
```

`poll()` вычитывает очередь **до конца** и возвращает `Vec<Event>`. Вызывайте
его раз в кадр.

`relative_mouse()` — отдельный метод, не событие: он читает накопленное
SDL-смещение мыши. Это не позиция, а дельта; подробнее в
[02-window.md](02-window.md).

## `FrameClock`

Файл `src/timing.rs`.

Наивный цикл `sleep(16ms)` даёт не 60 FPS, а «время работы кадра + 16 мс».
Чем тяжелее кадр, тем ниже FPS. `FrameClock` спит **остаток** до дедлайна.

```rust
pub const DEFAULT_MAX_DT: Duration = Duration::from_millis(100);

pub struct FrameClock {
    period: Duration,
    last: Instant,
    next: Instant,
    max_dt: Duration,
}
```

| Метод | Что делает |
|---|---|
| `FrameClock::new(hz)` | Цель `hz` кадров в секунду; некорректная частота → 60 |
| `.hz(hz)` | Перецелиться, отсчёт от текущего момента |
| `.period(duration)` | То же, но периодом |
| `.max_dt(duration)` | Ограничить сверху значение `dt` |
| `.period_len()` | Текущий период |
| `.dt() -> f32` | Секунды с прошлого чтения, не больше `max_dt` |
| `.tick()` | Доспать остаток кадра и сдвинуть дедлайн |

### Как работает `dt`

```text
elapsed = min(last.elapsed(), max_dt)
last = now
return elapsed.as_secs_f32()
```

- `dt` включает и работу, и сон: иначе всё, что умножается на `dt`, двигалось
  бы в сотни раз медленнее.
- Чтение `dt` сдвигает часы: второй вызов в том же кадре вернёт почти ноль.
- `max_dt` защищает от телепорта после alt-tab, брейкпоинта или долгой
  загрузки. По умолчанию 100 мс.

### Как работает `tick`

```text
если next в будущем: sleep(next - now)
next = max(next + period, now)
```

- Если кадр уже вылез за бюджет, `tick` не спит.
- Отставание не копится: после пропущенного дедлайна следующий ставится от
  текущего момента, а не серией догоняющих кадров.
- Период `0` означает «не спать».

### Типичный цикл

```rust
let mut clock = FrameClock::new(60.0);

loop {
    let dt = clock.dt();
    // события, логика, отрисовка
    clock.tick();
}
```

## Тесты

В `event.rs` тестов нет: трансляция проверяется косвенно через примеры.
В `timing.rs` 12 тестов на:

- перевод частоты в период и fallback на 60;
- `dt` на первом чтении;
- включение сна в `dt`;
- второе чтение в кадре;
- отсутствие сна при просроченном дедлайне;
- отсутствие накопления долга;
- ограничение `max_dt`.
