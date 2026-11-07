# 03. Графика: данные, которые рисуются

Модуль: `src/graphics/`

```text
src/graphics/
├── mod.rs        объявления и реэкспорты
├── color.rs      Color
├── image.rs      Image (пиксели на CPU)
├── object.rs     Object (позиция, размер, поворот, скорость)
├── camera.rs     Camera (что и как видно)
├── texture.rs    Texture + Store (фича cached-textures)
└── animation.rs  Animation (фича cached-textures)
```

Здесь лежат **описания** того, что рисуется. Само рисование — в
`src/window/draw/`. Такое разделение позволяет, например, одному `Object`
рисоваться разными картинками.

## `Color`

Файл `src/graphics/color.rs`.

```rust
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}
```

Собственный тип, а не `sdl3::pixels::Color`: публичный API не должен меняться
при обновлении `sdl3`.

| Элемент | Значение / смысл |
|---|---|
| `Color::BLACK` | `(0, 0, 0, 255)` |
| `Color::WHITE` | `(255, 255, 255, 255)` |
| `Color::FON` | `(20, 25, 30, 255)` — тёмный фон, используемый в примерах |
| `Color::RED` | `(200, 0, 0, 255)` |
| `Color::GREEN` | `(0, 200, 0, 255)` — фон окна по умолчанию |
| `Color::BLUE` | `(0, 0, 200, 255)` |
| `Color::rgb(r, g, b)` | Непрозрачный цвет (`a = 255`) |
| `Color::rgba(r, g, b, a)` | Любой цвет |
| `Color::to_array()` | `[r, g, b, a]` |

Преобразования: `From<(u8, u8, u8)>`, `From<(u8, u8, u8, u8)>`,
`From<[u8; 4]>`, `From<Color> for sdl3::pixels::Color`.

`Color` реализует `Lerp`. В отличие от `f32` и `Vec2`, каналы **зажимаются** в
`0..=255`, потому что `u8` не вмещает больше. Значение округляется к целому.

Альфа работает в рисовании:

- сплошная заливка: `a == 255` → `BlendMode::None` (замена);
- полупрозрачная: `a < 255` → `BlendMode::Blend`.

## `Image`

Файл `src/graphics/image.rs`.

```rust
pub struct Image {
    pub(crate) surface: sdl3::surface::Surface<'static>,
}
```

Пиксели лежат в памяти CPU, не на GPU. `Image` не занимает ресурсов окна и
может жить дольше его. Загрузка на GPU происходит при каждом вызове
`fill_textured_object` — если только не используется `Texture`.

| Метод | Что делает |
|---|---|
| `Image::from_file(path)` | Загружает PNG/BMP (другие форматы — если SDL собран с библиотекой формата) |
| `crop(x, y, w, h)` | Вырезает прямоугольник в новую `Image` (копия пикселей) |
| `Image::from_surface(surface)` | Принимает готовую поверхность от внешнего рендерера (например, SDL3_ttf) |
| `size() -> (u32, u32)` | Ширина и высота |
| `Clone` | Клонирование — это `crop(0, 0, w, h)`; при ошибке паникует через `expect` |

`from_surface` — ключевая точка расширения: библиотека не знает о шрифтах, но
принимает результат любого кода, который умеет сделать `Surface`. Пример:

```rust
let image = frglib::Image::from_surface(font.render("Играть").solid(white)?);
```

`Debug` печатает только размер: `Image { 128x64 }`.

### Нарезка атласа

```rust
let sheet = Image::from_file("tilemap_forest.png")?;
let tile = sheet.crop(0, 0, 16, 16)?;
```

`crop` создаёт новую поверхность `RGBA32` и копирует туда кусок через
`blit`. Это именно копия, а не ссылка на исходник.

## `Object`

Файл `src/graphics/object.rs`.

```rust
pub struct Object {
    pub position: Vec2,   // центр в мировых координатах
    pub size: Vec2,
    pub rotation: f32,    // градусы, по часовой стрелке
    pub velocity: Vec2,   // пиксели/единицы мира в секунду
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
}
```

Текстуры внутри нет специально: один объект можно рисовать разными картинками.
Так работают наборы спрайтов по направлениям (см. пример `fgoty`).

| Метод | Что делает |
|---|---|
| `Object::new(position, size)` | Центр в `position`, без поворота и скорости |
| `Object::from_xywh(x, y, w, h)` | Сахар для `new` |
| `.with_velocity(v)` | Задаёт скорость |
| `.with_rotation(degrees)` | Задаёт поворот |
| `.with_flip(h, v)` | Задаёт отражение |
| `.update(dt)` | `position += velocity * dt` |
| `.rect()` | Неповёрнутый прямоугольник вокруг центра |
| `.corners()` | Четыре угла с учётом поворота, по часовой стрелке от левого верхнего |
| `.turn(point)` | Поворачивает точку вокруг центра объекта |

Формулы:

```text
rect:
  x = position.x - size.x / 2
  y = position.y - size.y / 2
  w = size.x
  h = size.y

corners:
  offset ∈ {(-w/2,-h/2), (w/2,-h/2), (w/2,h/2), (-w/2,h/2)}
  corner = position + offset.rotated(rotation.to_radians())

turn(point):
  position + (point - position).rotated(rotation.to_radians())
```

`rect()` **игнорирует** поворот: это ось-выровненная коробка. Если нужна
реальная форма, берите `corners()`. `update` не учитывает столкновения и не
ограничивает скорость — это чистое интегрирование.

У `Object` есть `Debug`, `Clone`, `Copy`, `PartialEq`, `Default`.

## `Camera`

Файл `src/graphics/camera.rs`.

```rust
pub struct Camera {
    pub position: Vec2, // центр обзора в мире
    pub zoom: f32,      // 1.0 = один мир = один пиксель
}
```

| Метод | Что делает |
|---|---|
| `Camera::new(position)` | Зум `1.0` |
| `.with_zoom(zoom)` | Задаёт зум |
| `to_world(screen, size)` | Экранная точка → мировая (для мыши) |
| `to_screen(world, size)` | Мировая → экранная (внутренний, `pub(crate)`) |
| `project_rect(rect, size)` | Проекция прямоугольника (внутренний) |

Если `zoom <= f32::EPSILON` или не положительный, используется `1.0`. Это
защищает `to_world` от деления на ноль и бесконечностей. Отрицательный зум
трактуется как отсутствие зума, а не как зеркалирование.

Камера не ограничена границами мира. Ограничение — отдельная операция
`TileMap::clamp_camera` (см. [07-tilemap.md](07-tilemap.md)).

## `Texture` и `Store` (фича `cached-textures`)

Файл `src/graphics/texture.rs`.

```rust
pub struct Texture {
    slot: usize, // индекс в Store окна
}

pub(crate) struct Store {
    textures: Vec<sdl3::render::Texture>,
}
```

`Texture` — дешёвый `Copy`-индекс. Пиксели принадлежат окну, которое их
загрузило, и удаляются вместе с ним. Получить `Texture` можно только через
`Window::upload(&Image)`.

Зачем фича, а не всегда:

- `sdl3` связывает текстуру с создателем, поэтому хранить её между кадрами
  напрямую нельзя: `Window` начал бы ссылаться на собственное поле.
- Обходной путь — `sdl3/unsafe_textures`, который снимает проверку и разрешает
  использовать текстуру с чужим canvas (UB).
- FRGLib этого не делает: `Store` лежит в `Window`, поле объявлено до canvas,
  и рисование разрешает индекс обратно в тот же canvas. Но риск opt-in, чтобы
  крейт оставался sound по умолчанию.

| Элемент | Смысл |
|---|---|
| `Window::upload(&Image)` | Создаёт текстуру, применяет `ScaleMode`, возвращает `Texture` |
| `Store::push(texture)` | Кладёт текстуру, возвращает индекс |
| `Store::get(Texture)` | Достаёт `&sdl3::render::Texture` |
| `Texture::at(slot)` | Только для тестов; в публичном API отсутствует |

`Texture` реализует `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`.

## `Animation` (фича `cached-textures`)

Файл `src/graphics/animation.rs`.

```rust
pub struct Animation {
    frames: Vec<Texture>,
    fps: f32,
    at: f32,      // секунды с начала, уже свёрнутые/ограниченные
    looped: bool,
}
```

Анимация хранит только индексы уже загруженных текстур и позицию во времени.
Проигрывание — выбор кадра, а не загрузка. Именно это убирает «минуту на
старте» из старых примеров.

| Метод | Что делает |
|---|---|
| `Animation::new(fps)` | Пустая анимация; `fps <= 0` замораживает на первом кадре |
| `.with_frames(iter)` | Добавляет кадры, позиция не сбрасывается |
| `.push(frame)` | Добавляет один кадр |
| `.looped(bool)` | Включает/выключает зацикливание (по умолчанию включено) |
| `.is_looped()` | Текущий режим |
| `.set_looped(bool)` | Меняет режим; при выключении подрезает позицию до конца |
| `.advance(dt)` | Двигает время; `dt < 0` отматывает, но не раньше старта |
| `.rewind()` | В начало |
| `.len()`, `.is_empty()` | Количество кадров |
| `.fps()`, `.set_fps(fps)` | Частота; позиция во времени сохраняется |
| `.duration()` | `frames.len() / fps`, `0` при плохом fps |
| `.frame_index()` | Текущий индекс кадра |
| `.frame() -> Option<Texture>` | Текущая текстура или `None` |

Алгоритм `advance`:

```text
at = max(0, at + dt)
duration = frames.len() / fps
если duration <= 0: at = 0
иначе если looped:  at %= duration
иначе:              at = min(at, duration)
```

`frame_index` считает `(at * fps) as usize`, а не `(at / duration) * len`:
второй вариант в `f32` для трёх кадров может дать `0.99999994` и на кадр
отстать. Результат ограничивается `len - 1`.

Пустая анимация — валидное состояние: `frame()` вернёт `None`, рисование
ничего не сделает. Это позволяет собрать анимацию позже без заглушки.
