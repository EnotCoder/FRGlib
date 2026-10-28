## FRGLib

![Логотип библиотеки](logo.svg)

[![crates.io](https://img.shields.io/crates/v/frglib.svg)](https://crates.io/crates/frglib)
[![docs.rs](https://img.shields.io/docsrs/frglib/latest.svg)](https://docs.rs/frglib/latest)
[![license](https://img.shields.io/crates/l/frglib.svg)](https://github.com/EnotCoder/FRGlib/blob/main/LICENSE)
![Rust](https://img.shields.io/badge/rust-1.82%2B-blue)

Установка:

```bash
cargo add frglib
```

Rust-обвязка над SDL3: окно, рендерер и обработка событий. Слой для Vulkan
впереди.

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
| `relative_mouse() -> Vec2` | Насколько мышь сдвинулась за кадр, в пикселях |
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
| `lerp(from, to, t, Ease)` | Интерполяция `f32`, `Vec2` и `Color`, как в Godot |
| `pixel_art(bool)` | Флаг окна: текстуры без сглаживания |
| `Window::pixel_art()` | Что выбрано при создании окна |
| `screen_space(f)` | Рисует в координатах экрана, камеру временно снимает |
| `Button::new(Rect)` | Кнопка с тремя состояниями, клик по отпусканию |
| `Button::with_colors(idle, hover, pressed)` | Своя палитра кнопки |
| `Button::update(&[Event]) -> bool` | Кормит события, `true` в кадре клика |
| `Button::draw(d, &Image)` | Рисует кнопку с подписью поверх камеры |
| `Label::new(&Image).at(x, y)` | Готовая картинка текста, по центру на `(x, y)` |
| `Label::anchored(Anchor)` | Какой точкой текст стоит на позиции: `TopRight`, `Bottom`, `Center`… |
| `Label::draw(d)` | Рисует текст поверх камеры, сам снимает её |
| `TextCache::new(render)` | Кеш строк: `render` — замыкание вызывающего, картинками |
| `TextCache::keep(text)` | Рендерит строку, если её ещё нет |
| `TextCache::get(text) -> &Image` | Уже отрендеренная картинка |
| `Rect::contains(x, y)` | Точка внутри прямоугольника, края не входят |
| `TileMap::from_ascii(rows, cell, parse)` | Карта из строк текста; символ решает `parse` |
| `TileMap::from_rows(rows, cell)` | Карта из уже собранных клеток |
| `TileMap::get(col, row)` / `get_at(world)` | Клетка по индексу или по точке мира |
| `TileMap::cell_rect(col, row)` / `cell_center` | Прямоугольник и центр клетки в мире |
| `TileMap::origin(x, y)` | Куда в мире встаёт клетка `(0, 0)` |
| `TileMap::visible(camera, view)` | Только клетки в кадре, с запасом в одну |
| `TileMap::clamp_camera(&mut cam, view)` | Держит камеру в границах карты |
| `TileMap::draw(d, camera, texture_of)` | Рисует видимые клетки (фича `cached-textures`) |
| `MapError` | Что не так с картой: рваные строки, лишний символ |

У методов объекта:

| Вызов | Описание |
|---|---|
| `corners()` | Четыре повёрнутых угла объекта |
| `turn(point)` | Поворачивает точку вокруг центра объекта |
| `update(dt)` | Двигает объект вдоль `velocity`, `dt` в секундах |

У `Camera` — `position`, `zoom`, `to_world(screen, size)` для мыши и
`with_zoom`.

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

### Постоянные текстуры

Фича `cached-textures` убирает загрузку текстуры на каждом кадре:

```toml
[dependencies]
frglib = { version = "0.4", features = ["cached-textures"] }
```

```rust
let logo = window.upload(&logo_image)?;      // один раз за жизнь окна

window.frame_with(|d| {
    d.fill_object_texture(&sprite, logo)?;    // а это каждый кадр
    Ok(())
})?;
```

### Пиксель-арт против гладкой графики

По умолчанию SDL сглаживает текстуры: между соседними точками подставляет
промежуточные цвета. Для фотографий и векторов это правильно, а мелкий
пиксель-арт от этого превращается в мыло. Лечится флагом при создании окна:

```rust
let window = Window::builder("game")
    .pixel_art(true)   // каждая точка либо остаётся, либо пропадает целиком
    .build()?;
```

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

Спать фиксированные 16 мс после каждого кадра.

`dt` ограничен сверху: после паузы или сворачивания окна без ограничения
`update` получил бы на несколько секунд за раз и унёс всё за экран.

### Интерполяция

`lerp` ведёт значение от одного к другому, как в Godot. Четвёртый аргумент —
тип интерполяции, пока это только `Ease::Linear`:

```rust
use frglib::{lerp, Ease, Lerp, Vec2};

let from = Vec2::new(10.0, 20.0);
let to = Vec2::new(30.0, 60.0);

// Середина пути: то же самое двумя способами.
let halfway = lerp(from, to, 0.5, Ease::Linear);
let halfway = from.lerp(to, 0.5, Ease::Linear);
```

Работает с `f32`, `Vec2` и `Color`. Методу нужен `use frglib::Lerp;`, потому
что это функция из трейта, а не метод структуры.

Вес не зажимается: за пределами `0..=1` значение продолжает движение мимо
цели — как в Godot, и `Ease::Linear` в этом не мешает. Исключение — `Color`:
его каналы останавливаются на `0` и `255`, так как `u8` большего не вмещает.

Когда-нибудь здесь появятся `Ease::In`, `Ease::Out` и остальные — сигнатура
не изменится, добавится только новый вариант enum.

### Кнопки

```rust
use frglib::{Button, Rect};

let mut play = Button::new(Rect::new(40.0, 200.0, 180.0, 44.0))
    .with_colors(idle, hover, pressed);

// ...в цикле кадра:
window.frame_with(|d| {
    d.set_camera(camera);
    play.draw(d, &label)?;        // кнопка сама снимает камеру
    Ok(())
})?;

if play.update(&events) {
    // клик состоялся именно в этом кадре
}
```

Состояния — `Idle`, `Hover`, `Pressed`, цвет берётся из палитры. Клик случается
**по отпусканию** и только если курсор всё ещё над кнопкой: нажал, увёл вбок,
отпустил — клика не будет. Реагирует только левая кнопка мыши, а потеря фокуса
окна сбрасывает наведение и зажатость, чтобы после alt-tab кнопка не осталась
залитой.

Подпись — это `Image`, а не строка. Кнопка ничего не знает про шрифты: нарисуйте
слово чем угодно и передайте готовую картинку. Именно поэтому в библиотеке нет
ни SDL3_ttf, ни `ab_glyph`, ни `rusttype`, и пользователю крейта они тоже не
достаются.

### Текст и шрифты

Текст рисует то, чем удобно автору, а библиотека только показывает результат:

```rust
let font = sdl3::ttf::init()?.load_font("assets/font.otf", 28.0)?;
let image = frglib::Image::from_surface(font.render("Играть").solid(white)?);
```

`Image::from_surface` — вход в API, всё остальное уже было.
Шрифт подключается через `[dev-dependencies]`, поэтому в публикуемый пакет он не
попадает: читающий README без нужды в тексте его не собирает и не тащит.

Режим `solid` вместо `blended` даёт заливку пиксель в пиксель, без сглаживания,
что и нужно при `pixel_art(true)`.

### Тайлмапы

Карта — это сетка клеток одинакового размера. `TileMap<T>` держит три вещи и
не лезет в четвёртую: где клетки, какие из них в кадре, и как одна рисуется.
Чем клетка *является* — вода это или шип — решает вызывающий: тип `T`
вообще ничем не ограничен, ни `Copy`, ни трейтом.

```rust
#[derive(Debug)]
enum Cell { Floor, Water }

let map = frglib::TileMap::from_ascii(
    &["......", "..~~..", "......"],
    32.0,                          // сторона клетки в мире
    |c| Some(match c {
        '~' => Cell::Water,
        _ => Cell::Floor,
    }),
)?;

// Карта стоит в мире не в нуле, если уровень привязан к земле:
let map = map.origin(0.0, 400.0 - map.height());

// Мир -> клетка и обратно:
let (col, row) = map.cell_at(world).unwrap();
let tile = map.get_at(world);       // или сразу содержимое
let rect = map.cell_rect(col, row); // и центр: cell_center
```

Строки разной длины — [`MapError::RaggedRow`], неизвестный символ —
[`MapError::UnknownChar`]. Опечатка в карте останавливает программу, а не
тихо превращается в пустую клетку, сквозь которую игрок провалится.

Карта, собранная кодом, принимается тем же типом через `TileMap::from_rows` —
так делает пример `2d_games-fgoty`, где карта генерируется, а не пишется рукой.

В кадре рисуется только то, что видно, и камера не выходит за край карты:

```rust
map.clamp_camera(&mut camera, window.size());

window.frame_with(|d| {
    d.set_camera(camera);
    map.draw(d, camera, |cell| match cell {
        Cell::Water => Some(water_tex),
        Cell::Floor => Some(grass_tex),
    })?;
    Ok(())
})?;
```

`draw` стоит под фичей `cached-textures`: без неё каждый тайл заливался бы на
GPU заново каждый кадр. `visible(camera, view)` остаётся открытым — им
пользуются, когда клетки рисуются не текстурой, а цветом.

Картинку на экран ставит `Label` — он берёт готовую `Image` и сам снимает камеру,
как `Button`. Ставить вручную через `Object::from_xywh` тоже можно, просто
длиннее:

```rust
let hello = frglib::Label::new(&image).at(360.0, 240.0);           // центр текста
let score = frglib::Label::new(&image).anchored(Anchor::TopRight)  // правый верх
    .at(700.0, 20.0);

window.frame_with(|d| {
    hello.draw(d)?;
    score.draw(d)
})?;
```

`Anchor` выбирает, какой точкой текст стоит на позиции: `Center` по умолчанию,
плюс восемь краёв и углов. Без него пришлось бы каждый раз измерять картинку и
самому вычитать половину ширины.

Для строк, которые меняются, есть `TextCache`: он держит `HashMap` строка →
`Image` и не рендерит одно и то же заново шестьдесят раз в секунду. Рендерит
вызывающий, поэтому шрифт в библиотеку по-прежнему не попадает:

```rust
let mut cache = frglib::TextCache::new(|text: &str| {
    Ok(frglib::Image::from_surface(font.render(text).solid(white)?))
});

cache.keep("Очки: 120")?;                  // до кадра
// d.fill_textured_object(&obj, cache.get("Очки: 120"))?;   // внутри кадра
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
| `cargo run --example basic-gallery` | Листалка картинок: выбор по индексу, круговой проход |
| `cargo run --features cached-textures --example basic-animation` | Анимация из кадров: залитые текстуры, `advance(dt)`, пауза и скорость |
| `cargo run --example basic-lerp` | Интерполяция: `lerp` позиции, размера и цвета, `Ease::Linear` |
| `cargo run --example basic-sin-cos` | Круговое движение с помощью `sin` и `cos` |
| `cargo run --example basic-relative` | Абсолютная мышь против относительной: `relative_mouse()` |
| `cargo run --features cached-textures --example 2d_games-fgoty` | RPG на тайловой карте: `TileMap`, нарезка атласа, камера, коллизии |
| `cargo run --features cached-textures --example 2d_games-fgoty-dash` | Dash: физика прыжка, вращение куба, уровень строкой |
| `cargo run --example ui-text` | Текст через SDL3_ttf, кириллица, кеш строк |
| `cargo run --example ui-button` | Кнопки: три состояния, клик по отпусканию, интерфейс поверх камеры |
| `cargo run --example ui-slider` | Слайдеры: диапазон, шаг, перетаскивание, раскладка от размера окна |