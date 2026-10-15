## FRGLib

![Логотип библиотеки](logo.svg)

[![crates.io](https://img.shields.io/crates/v/frglib.svg)](https://crates.io/crates/frglib)
[![docs.rs](https://img.shields.io/docsrs/frglib/latest.svg)](https://docs.rs/frglib/latest)
[![license](https://img.shields.io/crates/l/frglib.svg)](https://github.com/EnotCoder/FRGlib/blob/main/LICENSE)
![Rust](https://img.shields.io/badge/rust-1.82%2B-blue)

Установка:

```toml
[dependencies]
frglib = "0.2"
```

Rust-обвязка над SDL3: окно, рендерер и обработка событий. Слой для Vulkan
впереди.

## Требования

- Rust 1.82+
- SDL3 (для сборки по умолчанию берётся системная библиотека)
- `cmake` и C-компилятор — только если включаешь фичу `bundled`

## Сборка и запуск

```bash
cargo build
cargo run --example basic-window
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
| `frame_with(impl FnOnce(&mut Draw) -> Result<(), Error>)` | То же, но позволяет рисовать поверх фона |
| `set_background(Color)` | Меняет цвет фона на лету |
| `background() -> Color` | Текущий цвет фона |
| `size() -> (u32, u32)` | Текущий размер окна |
| `vulkan_instance_extensions() -> Result<Vec<String>, Error>` | Расширения Vulkan, нужные этому окну |

`Event::should_close()` отвечает на вопрос, стоит ли продолжать цикл.

### Рисование

`frame_with` очищает окно фоном, зовёт твой код, показывает кадр:

```rust
window.frame_with(|d| {
    let square = Object::from_xywh(200.0, 200.0, 200.0, 200.0);
    d.fill_object(&square, Color::rgb(220, 60, 60))?;
    Ok(())
})?;
```

| Вызов | Описание |
|---|---|
| `fill_object(&Object, Color)` | Заливает прямоугольник объекта, с поворотом |
| `draw_object(&Object, Color)` | Обводит тот же прямоугольник |
| `fill_textured_object(&Object, &Image)` | Рисует объект текстурой, с поворотом и отражением |
| `fill_object_polygon(&Object, points, indices, Color)` | Заливает многоугольник, повёрнутый вокруг центра объекта |
| `draw_object_polygon(&Object, points, Color)` | Обводит тот же повёрнутый многоугольник |
| `fill_textured_object_polygon(&Object, points, indices, &Image)` | Заливает повёрнутый многоугольник текстурой |
| `upload(&Image) → Texture` | Заливает текстуру один раз (фича `cached-textures`) |
| `fill_object_texture(&Object, Texture)` | Рисует залитую текстуру (фича `cached-textures`) |
| `fill_object_texture_polygon(&Object, points, indices, Texture)` | То же для многоугольника (фича `cached-textures`) |
| `set_camera(Camera)` | Задаёт камеру на текущий кадр |
| `size()` | Размер окна, полезно для центрирования |
| `FrameClock::new(hz)` | Таймер кадров: `dt()` и `tick()` в цикле |

Всё рисование идёт через `Object` — метода без него в API нет. У методов
объекта:

| Вызов | Описание |
|---|---|
| `corners()` | Четыре повёрнутых угла объекта |
| `turn(point)` | Поворачивает точку вокруг центра объекта |
| `update(dt)` | Двигает объект вдоль `velocity`, `dt` в секундах |

У `Camera` — `position`, `zoom`, `to_world(screen, size)` для мыши и
`with_zoom`.

Альфа работает сама: цвет с `a < 255` смешивается с тем, что под ним, с `a == 255`
затирает. Режим проходимости переключается сам, только когда он реально меняется.

### Текстуры

```rust
let logo = Image::from_file("assets/logo.png")?;
let sprite = Object::from_xywh(200.0, 200.0, 200.0, 200.0);

window.frame_with(|d| {
    d.fill_textured_object(&sprite, &logo)?;
    Ok(())
})?;
```

`Image` держит пиксели в памяти CPU и ничего не занимает у окна, так что может
жить дольше него. SDL сам определяет PNG и BMP; остальные форматы требуют
сборки SDL с соответствующей библиотекой.

Атлас с несколькими кадрами или тайлами режется на части через `crop`:

```rust
let sheet = Image::from_file("assets/tilemap_forest.png")?;
let grass = sheet.crop(0, 0, 16, 16)?;   // копия пикселей, не ссылка
```

Общего с исходной картинкой ничего не остаётся, так что атлас можно сразу
отпустить. Рисовать части по одной можно и напрямую, если атлас небольшой;
`crop` нужен там, где деталей много, как в примере с RPG.

Текстура заливается на GPU **на каждый вызов** и сразу удаляется, то есть
стоимость — одна загрузка на кадр. Для спрайтов это нормально, для крупной
картинки в цикле будет заметно — тогда понадобится постоянная текстура, см. фичу
`cached-textures` ниже.

`fill_textured_object_polygon` растягивает текстуру по bounding box многоугольника
**до поворота**, так что полигон закрывает её целиком, пропорции картинки не
сохраняются, а сама картинка поворачивается вместе с фигурой.

### Постоянные текстуры

Фича `cached-textures` убирает загрузку текстуры на каждом кадре:

```toml
[dependencies]
frglib = { version = "0.2", features = ["cached-textures"] }
```

```rust
let logo = window.upload(&logo_image)?;      // один раз за жизнь окна

window.frame_with(|d| {
    d.fill_object_texture(&sprite, logo)?;    // а это каждый кадр
    Ok(())
})?;
```

`Texture` — просто ручка: `Copy`, ничего не стоит, копируется вместе со ссылкой
на пиксели. Пиксели живут в окне и уходят вместе с ним. Замер на 800×600 RGBA,
60 кадров, два пути подряд:

| Спрайтов на кадре | `fill_textured_object` | `fill_object_texture` | Разница |
|---|---|---|---|
| 1 | 1 мс | 0.6 мс | 1.6× |
| 8 | 6 мс | 0.6 мс | 9.5× |
| 32 | 22 мс | 0.3 мс | 74× |

Чем больше спрайтов, тем заметнее: на одном выигрыш скромный, на десятках он
уже в разы.

**Почему это отдельная фича.** `sdl3` привязывает текстуру к создавшему её
холсту, поэтому текстура не может пережить кадр, а чтобы она жила дольше окна,
пришлось бы обращаться к собственному полю — безопасный Rust так не умеет.
Выход — фича `sdl3/unsafe_textures`, которая снимает эту связь и признаёт, что
использование текстуры с чужим холстом есть UB.

Мы так не делаем: текстуры лежат внутри окна, которое их создало, и поле
объявлено так, что они уничтожаются раньше рендерера. Но риск остаётся, поэтому
фича выключена по умолчанию, и без неё крейт остаётся sound. Включай, если
сцена тяжелее пары спрайтов.

### Объекты

`Object` — это положение, размер, поворот, скорость и отражение. Текстуру он не
хранит: картинка передаётся отдельно, поэтому один объект можно рисовать разными
картинками — так работают наборы спрайтов по направлениям.

```rust
use frglib::{Object, Vec2};

let mut frog = Object::from_xywh(400.0, 300.0, 64.0, 64.0)
    .with_velocity(Vec2::new(120.0, 0.0))
    .with_rotation(15.0);

frog.update(dt);  // dt в секундах, двигает вдоль velocity

window.frame_with(|d| {
    d.fill_textured_object(&frog, &sprite)?;
    Ok(())
})?;
```

Поворот идёт вокруг `position` — это центр объекта, — в градусах, положительный
знак означает поворот по часовой стрелке. `flip_horizontal` и `flip_vertical`
имеют смысл только для текстур, у заливки цветом они игнорируются.

У полигонов объект задаёт только точку, вокруг которой они поворачиваются;
`size` на них не действует.

### Камера

Рисование идёт в координатах мира, а `Camera` решает, какая часть мира попадёт
на экран. Ставится один раз в начале кадра:

```rust
let mut camera = Camera::new(Vec2::new(100.0, 200.0)).with_zoom(1.5);

window.frame_with(|d| {
    d.set_camera(camera);       // первым делом: всё дальше идёт через неё
    d.fill_object(&frog, Color::WHITE)?;
    Ok(())
})?;
```

Позиции и `Object::size` теперь в единицах мира — при зуме 2 объект 64×64
нарисуется 128×128. Границ у камеры нет, она ездит куда угодно.

Обратное преобразование — для мыши. Координаты из `MouseMotion` приходят в
экранных, а игроку нужен мир:

```rust
Event::MouseMotion { x, y } => {
    let at = camera.to_world(Vec2::new(x, y), window.size());
    // at — точка мира под курсором
}
```

### Кадры и время

`FrameClock` держит цикл на заданной частоте и отдаёт, сколько длился прошлый
кадр:

```rust
let mut clock = FrameClock::new(60.0);

loop {
    let dt = clock.dt();
    for event in window.poll()? { ... }
    // ... движение со скоростью на dt, отрисовка ...
    clock.tick();
}
```

Спать фиксированные 16 мс после каждого кадра — это не 60 fps, а «сколько занял
кадр, плюс 16 мс»:

| Тело кадра | Фиксированный сон | `FrameClock` на 60 |
|---|---|---|
| 1 мс | 58.8 fps | 60 fps |
| 5 мс | 47.6 fps | 60 fps |
| 16 мс | 31.2 fps | 60 fps |
| 20 мс (перегрузка) | 27.8 fps | 50 fps |

`FrameClock` спит остаток до следующего кадра, поэтому цель держится, пока работа
в неё укладывается, а при перегрузке просто не спит — без спирали смерти.

`dt` — это длина всего кадра, вместе с ожиданием, а не только времени работы.
На 60 fps кадр спит около шестнадцати миллисекунд из семнадцати, так что `dt`,
меряющий одну работу, отдавал бы микросекунды и всё, что на него умножается, ехало
бы в тысячу раз медленнее. Читать `dt` нужно один раз на кадр: он двигает часы, и
второе чтение в том же кадре вернёт почти ноль.

`dt` ограничен сверху: после паузы или сворачивания окна без ограничения
`update` получил бы на несколько секунд за раз и унёс всё за экран.

### Примеры

| Пример | Что показывает |
|---|---|
| `cargo run --example basic-window` | Окно, ввод, события |
| `cargo run --example basic-draw` | Прямоугольники, квадраты, альфа |
| `cargo run --example basic-texture` | Логотип библиотеки как текстура |
| `cargo run --example basic-polygon` | Многоугольники: заливка, контур, текстура |
| `cargo run --example basic-mouse` | Состояние кнопок мыши: `Down`/`Up`, потеря фокуса |
| `cargo run --example basic-object` | Объекты: поворот заливки, текстуры и многоугольника |
| `cargo run --example basic-camera` | Камера: движение, зум, картинка на сцене |
| `cargo run --features cached-textures --example basic-cached` | Текстура, залитая один раз, против заливки на кадр |
| `cargo run --features cached-textures --example 2d_games-frogy` | RPG на тайловой карте: нарезка атласа, камера, коллизии |

### Что изменилось в 0.2.0

`0.2.0` ломает API относительно `0.1.0` — рисование придётся переписать.

| Было в 0.1.0 | Стало в 0.2.0 |
|---|---|
| `fill_rect`, `fill_square`, `fill_texture`, `fill_polygon` и подобные | `fill_object`, `draw_object`, `fill_textured_object` и подобные — всё через `Object` |
| координаты считались экранными | координаты в мире, размещение решает `Camera` |
| поворота не было | `Object::rotation`, в градусах, вокруг центра |
| `Vec2` не было | `Vec2` в корне крейта |
| — | `Camera` с перемещением и зумом |
| `Keycode` и `Mod` лежали в `frglib::event` | реэкспортированы в корень, как и `MouseButton` |

Методы, принимавшие координаты напрямую, убраны: осевой прямоугольник — это
`Object` без поворота, а квадрат — просто `Object` с равными сторонами.

```rust
// было
d.fill_square(100.0, 100.0, 200.0, color)?;

// стало: объект задаёт центр, а не левый верхний угол
let square = Object::from_xywh(200.0, 200.0, 200.0, 200.0);
d.fill_object(&square, color)?;
```

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
