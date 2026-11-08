# 10. Wgpu: полное руководство

Фича `wgpu` добавляет в FRGLib второй тип окна — `WgpuWindow`. В нём SDL3
по-прежнему создаёт окно, приносит события и отдаёт нативный handle, но кадром
владеет `wgpu`: свопчейн, командный энкодер, пайплайны, буферы и текстуры.

Рендерить можно 3D и 2D, текст, интерфейс и тайлмапы. Для этого в модуле
`frglib::gpu` есть два независимых высокоуровневых рендерера — `Renderer3D` и
`Renderer2D` — плюс сырой слой из `Gpu` и `Frame`, в который можно вписать свои
`wgpu`-проходы.

Фича не включена по умолчанию. Без неё библиотека не тянет GPU-стек вообще.

```bash
cargo run --features wgpu --example wgpu-cube
cargo run --features wgpu --example wgpu-hud
```

`wgpu-cube` — только 3D: куб, сетка, орбитальная камера. `wgpu-hud` — оба
рендерера в одном кадре: 3D-сцена, а поверх неё панель, текст, кнопка,
слайдер и текстурированный тайлмап.

## Что важно понять сразу

1. **`Window` и `WgpuWindow` — разные типы окна.** В одном окне нельзя
   одновременно держать `SDL_Renderer` и `wgpu`. `Draw`, `Window::frame` и
   SDL-методы `draw` к `WgpuWindow` не применяются.
2. **`Renderer2D` — это GPU-аналог `Draw`.** Он рисует в экранных пикселях или
   в мировых единицах через обычную 2D-камеру `frglib::Camera`.
3. **Шрифты разбирает `fontdue`, но только под фичей `wgpu`.** SDL-окно
   по-прежнему не зависит от SDL3_ttf: там текст рисует вызывающий код.
4. **Кадр явный:** `begin_frame` → рисование → `present`. Пропущенный кадр —
   норма, а не ошибка.
5. **3D и 2D делят один кадр, но не один проход.** `Renderer3D::finish`
   очищает вложения, поэтому 2D поверх 3D начинается с `begin_overlay`.

## Включение фичи

В `Cargo.toml` крейта:

```toml
[dependencies]
frglib = { version = "0.4", features = ["wgpu"] }
```

Фича включает зависимости:

| Крейт | Версия | Зачем |
|---|---|---|
| `wgpu` | 30 | Сам GPU-бэкенд: Vulkan, Metal, DX12, GLES |
| `pollster` | 1 | Блокирующее ожидание запросов адаптера и устройства |
| `bytemuck` | 1 | Вершины и юниформы как POD-структуры |
| `glam` | 0.34 | Математика 3D: `Vec3`, `Mat4` |
| `fontdue` | 0.9 | Разбор TTF/OTF и растрирование глифов |

`wgpu` и `glam` реэкспортированы через модуль, поэтому свои версии в проект
добавлять не нужно и не стоит:

```rust
use frglib::gpu::glam::{Mat4, Vec3};
use frglib::gpu::wgpu;

let _ = wgpu::TextureFormat::Rgba8UnormSrgb;
```

Версии входят в публичный API: их повышение — ломающее изменение для тех, кто
пользуется фичей.

## Карта модуля

```text
src/gpu/
├── mod.rs            реэкспорты, DEPTH_FORMAT, конвертация цвета
├── builder.rs        WgpuWindowBuilder
├── window.rs         WgpuWindow, Gpu, Frame
├── camera.rs         Camera3D
├── mesh.rs           Vertex, Mesh, GpuMesh
├── renderer.rs       Renderer3D, Material
├── renderer2d.rs     Renderer2D, GpuTexture
├── font.rs           FontId, TextAlign, атлас глифов, раскладка текста
├── error.rs          Error
└── shaders/
    ├── lit.wgsl      3D: освещённый и unlit-фрагмент
    └── sprite.wgsl   2D: фигуры, спрайты и текст
```

| Файл | Роль | Публичные имена |
|---|---|---|
| `mod.rs` | Точка входа модуля. Собирает дерево, реэкспортирует типы, хранит формат глубины и преобразование sRGB → linear | `gpu::*`, `gpu::wgpu`, `gpu::glam` |
| `builder.rs` | Настройка и создание окна | `WgpuWindowBuilder` |
| `window.rs` | Окно, GPU-ручки и один кадр | `WgpuWindow`, `Gpu`, `Frame` |
| `camera.rs` | Перспективная 3D-камера | `Camera3D` |
| `mesh.rs` | Геометрия на CPU и на GPU | `Vertex`, `Mesh`, `GpuMesh` |
| `renderer.rs` | 3D-рендерер с освещением | `Renderer3D`, `Material` |
| `renderer2d.rs` | Батчевый 2D-рендерер | `Renderer2D`, `GpuTexture` |
| `font.rs` | Шрифты, глифы и раскладка строк | `FontId`, `TextAlign` |
| `error.rs` | Ошибки окна, GPU и рисования | `Error` |
| `shaders/lit.wgsl` | WGSL для 3D | — |
| `shaders/sprite.wgsl` | WGSL для 2D | — |

Дальше — каждый файл по порядку.

## mod.rs: точка входа

Файл объявляет подмодули и реэкспортирует публичные типы, поэтому всё
доступно как `frglib::gpu::Renderer2D`, а не по внутренним путям. Кроме того,
он держит три вещи, общие для всего слоя.

### `DEPTH_FORMAT`

```rust
pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
```

Формат буфера глубины. Один и тот же у окна (`create_depth_view`) и у обоих
встроенных пайплайнов. Если писать свой пайплайн в тот же кадр, его
`depth_stencil` должен использовать этот формат, иначе `wgpu` откажется
привязывать проход к вложению.

### `color_to_linear`

```rust
pub(crate) fn color_to_linear(color: crate::graphics::Color) -> [f32; 4]
```

Превращает 8-битный sRGB-цвет библиотеки в линейный RGBA, с которым работают
шейдеры. Альфа не гамма-корректируется: она и так линейна. Функция приватная,
но именно она объясняет, почему `Color::rgb(128, 128, 128)` в шейдере — это
около `0.216`, а не `0.5`.

### `color_to_wgpu`

Тот же цвет в форме `wgpu::Color` — для `LoadOp::Clear`. При sRGB-таргете
`wgpu` сам переводит линейное значение обратно при записи в свопчейн.

## builder.rs: `WgpuWindowBuilder`

Сознательно меньше, чем `WindowBuilder`: опции SDL-рендерера (пиксель-арт,
флаги Vulkan) к свопчейну `wgpu` не применяются, а режим показа GPU выбирает
сам.

| Метод | По умолчанию | Что делает |
|---|---|---|
| `new(title)` | — | Начинает сборку |
| `size(width, height)` | `800 × 600` | Размер в логических пикселях; поверхность следует физическому размеру с учётом HiDPI |
| `resizable(bool)` | `true` | Можно ли менять размер; при изменении поверхность и глубина пересоздаются на следующем кадре |
| `background(Color)` | `Color::BLACK` | Цвет, которым очищается кадр |
| `build()` | — | Открывает окно и инициализирует GPU |

```rust
use frglib::Color;
use frglib::gpu::WgpuWindow;

let window = WgpuWindow::builder("FRGLib")
    .size(1280, 720)
    .resizable(true)
    .background(Color::rgb(16, 18, 26))
    .build()?;
```

`build` возвращает `Result<WgpuWindow, Error>`: ошибка здесь означает, что не
создалось окно, поверхность или не нашёлся подходящий адаптер.

## window.rs: `WgpuWindow`, `Gpu`, `Frame`

Самый важный файл для понимания жизненного цикла.

### Создание окна

`WgpuWindow::create` делает по шагам:

1. Ставит подсказку X11, чтобы композитор не отключался (окно сохраняет
   скругления и тень).
2. Инициализирует SDL и видео подсистему.
3. Создаёт окно SDL. На macOS вызывает `.metal_view()` — без этого `wgpu` не
   получит нативный handle; на других платформах вызов ничего не делает.
4. Создаёт `wgpu::Instance` и поверхность из нативного handle.
5. Просит адаптер с `PowerPreference::HighPerformance` и поддержкой этой
   поверхности.
6. Открывает устройство и очередь с лимитами по умолчанию.
7. Выбирает sRGB-формат свопчейна, если он есть; иначе первый доступный.
8. Настраивает поверхность: `PresentMode::AutoVsync`, максимальная задержка
   кадра 2.
9. Создаёт буфер глубины `Depth32Float`.

Порядок полей структуры — это порядок уничтожения. Поверхность объявлена
раньше окна, потому что она привязана к его нативному handle; SDL-окно в свою
очередь должно пережить видео подсистему.

### Методы окна

| Метод | Что делает |
|---|---|
| `builder(title)` | Возвращает `WgpuWindowBuilder` |
| `poll() -> Result<Vec<Event>, Error>` | Забирает события SDL за этот кадр. Работает как `Window::poll`, фильтруя очередь по id окна |
| `gpu() -> &Gpu` | GPU-ручки: instance, adapter, device, queue, конфигурация |
| `size() -> (u32, u32)` | Текущий физический размер окна |
| `render_size() -> (u32, u32)` | Размер, с которым будет рисоваться следующий кадр; на один шаг отстаёт от `size()` между ресайзом и `begin_frame` |
| `set_background(Color)` | Меняет цвет очистки |
| `background() -> Color` | Текущий цвет очистки |
| `begin_frame() -> Result<Option<Frame>, Error>` | Забирает изображение свопчейна и открывает кадр |
| `present(frame) -> Result<(), Error>` | Отправляет команды в очередь и показывает кадр |

`begin_frame` возвращает `Ok(None)`, когда кадр рисовать некуда:

| Состояние поверхности | Что делает окно |
|---|---|
| `Success`, `Suboptimal` | Отдаёт кадр |
| `Timeout`, `Occluded` | `Ok(None)`: окно свёрнуто или перекрыто |
| `Outdated`, `Lost` | Переконфигурирует поверхность и возвращает `Ok(None)` |
| `Validation` | `Err(Error::SurfaceValidation)`: это баг в коде рисования, а не повод повторять |

Пропускать `None` нужно через `continue`: рисовать в такой кадр не во что.

### `Gpu`

`Gpu` — это набор дешёвых reference-counted ручек. Клонировать `Device` и
`Queue` можно, чем и пользуются рендереры: они не borrow-ят окно.

| Метод | Возвращает |
|---|---|
| `instance()` | `&wgpu::Instance` — например, чтобы создать дополнительную поверхность |
| `adapter()` | `&wgpu::Adapter` — фичи и лимиты |
| `adapter_info()` | `&wgpu::AdapterInfo` — имя, бэкенд, версия драйвера |
| `device()` | `&wgpu::Device` — на нём создаются свои пайплайны |
| `queue()` | `&wgpu::Queue` — туда уходят свои проходы |
| `format()` | `wgpu::TextureFormat` свопчейна; встроенные пайплайны построены под него |
| `size()` | `(u32, u32)` — размер последней конфигурации |
| `config()` | `&wgpu::SurfaceConfiguration` |

### `Frame`

Один кадр: изображение свопчейна плюс энкодер.

| Поле | Тип | Что это |
|---|---|---|
| `encoder` | `wgpu::CommandEncoder` | Энкодер этого кадра |
| `color` | `wgpu::TextureView` | Изображение свопчейна как render target |
| `depth` | `wgpu::TextureView` | Буфер глубины, совпадает по размеру с `color` |
| `size` | `(u32, u32)` | Размер обоих вложений в физических пикселях |
| `background` | `Color` | Цвет, которым встроенные рендереры очищают кадр |

Метод `aspect()` возвращает `width / height` — готовое значение для
`Camera3D::view_projection`.

## camera.rs: `Camera3D`

Перспективная камера 3D-мира. В отличие от `crate::Camera` (центр и зум в 2D),
у неё есть позиция, цель и направление «вверх».

Система координат: правая, Y вверх, камера смотрит в `-Z` в view space.

| Поле | Смысл |
|---|---|
| `position: Vec3` | Где стоит камера |
| `target: Vec3` | Куда смотрит |
| `up: Vec3` | Что считается верхом; по умолчанию `Vec3::Y` |
| `fov_y: f32` | Вертикальный угол обзора в градусах |
| `z_near`, `z_far` | Плоскости отсечения |

| Метод | Что делает |
|---|---|
| `perspective(fov_y, z_near, z_far)` | Камера в начале координат, смотрит в `-Z` |
| `.position(v)`, `.target(v)`, `.up(v)`, `.fov_y(deg)`, `.clip(near, far)` | Билдеры, возвращают `Self` |
| `orbit(centre, radius, yaw, pitch)` | Ставит камеру на сферу вокруг `centre`; углы в радианах |
| `view()` | View-матрица |
| `projection(aspect)` | Проекция; нулевой или отрицательный aspect зажимается, чтобы матрица осталась конечной |
| `view_projection(aspect)` | `projection * view` — то, что умножает вершинный шейдер |

```rust
use frglib::gpu::Camera3D;
use frglib::gpu::glam::Vec3;

// Облёт начала координат.
let camera = Camera3D::orbit(Vec3::new(0.0, 1.0, 0.0), 6.0, 0.7, 0.35);

// Или вручную.
let camera = Camera3D::perspective(60.0, 0.1, 500.0)
    .position(Vec3::new(3.0, 2.0, 5.0))
    .target(Vec3::ZERO);
```

## mesh.rs: `Vertex`, `Mesh`, `GpuMesh`

### `Vertex`

Одна вершина: позиция, нормаль и цветовой множитель.

```rust
#[repr(C)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
}
```

Размер — 40 байт. `Vertex::new` делает белую вершину, `Vertex::with_color` —
цветную. Встроенные примитивы белые, поэтому цвет draw-вызова их и
окрашивает; своя геометрия может запечь цвет в вершинах.

### `Mesh`

Геометрия на CPU: вершины, индексы и топология.

| Поле | Тип |
|---|---|
| `vertices` | `Vec<Vertex>` |
| `indices` | `Vec<u32>` |
| `topology` | `wgpu::PrimitiveTopology` |

| Конструктор | Что создаёт |
|---|---|
| `cube(size)` | Куб с ребром `size` в начале координат, 24 вершины и 36 индексов: у каждой грани свои нормали |
| `plane(width, depth)` | Плоский quad в плоскости XZ, смотрит вверх |
| `grid(slices, spacing)` | Проволочная сетка в XZ; `slices` клеток на ось, `spacing` — ширина клетки |
| `from_vertices(vertices, indices)` | Треугольный меш из готовых данных |
| `from_line_list(vertices)` | Список независимых отрезков: каждая пара вершин — линия |
| `line(from, to, color)` | Один отрезок с уже линейным цветом |

Все примитивы возвращаются в локальном пространстве; место им задаёт
модельная матрица в draw-вызове.

### `GpuMesh`

Геометрия, загруженная на GPU. Создаётся через
`Renderer3D::create_mesh(&Mesh)` и передаётся в draw-вызовы. Клонирование
дешёвое: копируются только handle-ы буферов, память общая.

Поля `GpuMesh` приватные: снаружи это непрозрачный handle. Топология берётся
из исходного `Mesh`, поэтому линии и треугольники рисуются разными
пайплайнами автоматически.

## renderer.rs: `Renderer3D`

Высокоуровневый 3D-рендерер. Рисует меши, кубы, плоскости, сетки и
отладочные линии с одним направленным светом или без освещения.

### `Material`

| Значение | Что делает |
|---|---|
| `Material::Lit` | Направленный свет + ambient 0.18 + блик Блинна-Фонга (степень 32, вклад 0.25). По умолчанию |
| `Material::Unlit` | Цвет уходит на экран без изменений. Для сеток, осей и отладки |

### Методы

| Метод | Что делает |
|---|---|
| `new(&Gpu)` | Создаёт пайплайны, буферы юниформ и встроенные куб с плоскостью |
| `create_mesh(&Mesh) -> GpuMesh` | Загружает свою геометрию; handle можно хранить между кадрами |
| `begin(&Camera3D)` | Начинает кадр с камерой и сбрасывает очередь |
| `draw_mesh(&GpuMesh, Mat4, Color)` | Ставит lit-меш модельной матрицей |
| `draw_mesh_with(&GpuMesh, Mat4, Material, Color)` | То же с явным материалом |
| `draw_cube(center, size, Color)` | Куб, сдвинутый и растянутый; внутри `translation * scale` |
| `draw_cube_model(Mat4, Color)` | Единичный куб с произвольной матрицей — для поворотов |
| `draw_plane(center, size, Color)` | Плоский quad в XZ |
| `draw_grid(slices, spacing, Color)` | Проволочная сетка; меш кешируется по `(slices, spacing)` |
| `draw_line(from, to, Color)` | Один отрезок; копится в общий буфер кадра |
| `draw_lines(&[Vec3], Color)` | Связная ломаная через точки |
| `finish(&mut Frame)` | Записывает все накопленные draw в один render pass и очищает очередь |

### Как это устроено внутри

- **Два bind group.** Group 0 — камера: `view_proj`, позиция камеры,
  направление и цвет света. Group 1 — конкретный draw: модельная матрица и
  цвет.
- **Динамические offset-ы.** Все per-draw юниформы лежат в одном буфере;
  между draw-ами меняется только offset. Шаг выравнивается по
  `min_uniform_buffer_offset_alignment` устройства. Буфер стартует с 256
  записей и растёт вдвое, когда их не хватает.
- **Три пайплайна.** Lit-треугольники, unlit-треугольники (оба с отсечением
  задних граней) и unlit-линии (без отсечения).
- **Кеш сетки.** `draw_grid` с теми же `slices` и `spacing` переиспользует
  буферы; биты `f32` сравниваются напрямую.
- **Линии.** `draw_line`/`draw_lines` не создают буфер на каждый вызов: за
  кадр они собираются в один `LineList`-меш.

`finish` записывает **один** проход и очищает цвет и глубину цветом
`frame.background`. Это главное правило порядка: всё, что должно оказаться
под 3D-сценой, рисуйте до `finish`; всё, что поверх, — после, через
`Renderer2D::begin_overlay`.

## renderer2d.rs: `Renderer2D` и `GpuTexture`

Батчевый 2D-рендерер: фигуры, спрайты, интерфейс и текст. Это GPU-аналог
`Draw`.

### Как устроен один кадр

```
begin / begin_overlay
  ├── fill_rect, draw_line, fill_circle   → 1×1 белая текстура
  ├── draw_texture, draw_texture_region   → своя текстура
  ├── draw_text*                          → атлас глифов
  └── ...
finish  → один render pass, по одному draw на каждую смену текстуры
```

Все draw-вызовы только складывают вершины в `Vec`. `finish` заливает их в
вершинный буфер и записывает проход.

- **Один пайплайн на всё.** Фигуры, спрайты и текст отличаются только
  текстурой. Смена текстуры — это новый bind group, а не новый пайплайн.
- **Флаг пространства.** У каждой вершины есть `space`: `1.0` — экранные
  пиксели, `0.0` — мировые единицы. Поэтому в одном кадре можно смешивать
  мировую карту и экранный HUD.
- **Белая текстура 1×1.** Заливка фигур — это текстура, умноженная на цвет;
  отдельного шейдера для фигур нет.
- **Батчи.** Соседние квады с одной текстурой сливаются в один `draw`.
  Порядок рисования сохраняется, так что прозрачность ведёт себя как в SDL.
- **Вершинный буфер** стартует с 64 КиБ и растёт вдвое, когда не хватает.
- **Глубина** всегда проходит тест и не пишется: 2D — это overlay.
- **Смешивание** — обычное альфа-смешивание, отсечения граней нет.

### Начало и конец кадра

| Метод | Загрузка вложений | Когда использовать |
|---|---|---|
| `begin(&Frame)` | `Clear` цветом `frame.background`, глубина в 1.0 | Чисто 2D-кадр |
| `begin_overlay(&Frame)` | `Load` | Поверх уже записанной 3D-сцены |
| `finish(&mut Frame)` | Записывает накопленное и очищает очередь | Всегда в конце |

`begin` также сбрасывает камеру в центр экрана с зумом 1 и включает экранное
пространство, чтобы координаты совпадали с пикселями, пока вызывающий не
поставил свою камеру.

`finish` с пустой очередью в режиме overlay не создаёт проход вообще — пустой
HUD ничего не стоит. В режиме `begin` проход создаётся всегда: кадр обязан
быть очищен, даже если ничего не нарисовано.

### Пространство и камера

| Метод | Что делает |
|---|---|
| `set_screen_space()` | Экранные пиксели, начало координат — левый верхний угол |
| `set_camera(Camera)` | Мировые единицы через 2D-камеру |
| `camera() -> Camera` | Текущая камера |
| `is_screen_space() -> bool` | Текущий режим |
| `size() -> (u32, u32)` | Размер кадра, который дали `begin`/`begin_overlay` |
| `screen_space(\|r\| ...)` | Выполняет замыкание в экранном режиме и восстанавливает прежний |

```rust
// Мир: камера смотрит в (400, 300) с зумом 2.
renderer.set_camera(Camera::new(Vec2::new(400.0, 300.0)).with_zoom(2.0));
renderer.fill_rect(Rect::new(-20.0, -20.0, 40.0, 40.0), Color::RED);

// Экранный HUD, не зависящий от камеры.
renderer.set_screen_space();
renderer.fill_rect(Rect::new(8.0, 8.0, 120.0, 32.0), Color::BLACK);
```

**Текст всегда экранный.** `draw_text*` игнорирует текущую камеру: подпись —
это элемент интерфейса, а не объект сцены.

### Примитивы

| Метод | Что рисует |
|---|---|
| `fill_rect(Rect, Color)` | Залитый прямоугольник; `x`, `y` — левый верхний угол |
| `draw_rect(Rect, thickness, Color)` | Рамку толщиной `thickness` внутри прямоугольника |
| `draw_line(Vec2, Vec2, thickness, Color)` | Отрезок заданной толщины |
| `fill_circle(Vec2, radius, Color)` | Круг из треугольника-веера; 12–64 сегмента в зависимости от радиуса |

```rust
renderer.fill_rect(Rect::new(10.0, 10.0, 120.0, 40.0), Color::RED);
renderer.draw_rect(Rect::new(10.0, 10.0, 120.0, 40.0), 2.0, Color::WHITE);
renderer.draw_line(Vec2::new(10.0, 80.0), Vec2::new(130.0, 80.0), 3.0, Color::WHITE);
renderer.fill_circle(Vec2::new(80.0, 160.0), 24.0, Color::BLUE);
```

### Текстуры

```rust
// Из Image: любой формат, который умеет читать SDL.
let texture = renderer.create_texture(&Image::from_file("sprite.png")?)?;

// Или из сырых RGBA8-байтов.
let pixels = [
    255, 0, 0, 255,   0, 255, 0, 255,
    0, 0, 255, 255,   255, 255, 255, 255,
];
let raw = renderer.create_texture_rgba8(2, 2, &pixels)?;

renderer.draw_texture(&texture, Rect::new(40.0, 40.0, 128.0, 128.0), Color::WHITE);
renderer.draw_texture_region(
    &texture,
    Rect::new(0.0, 0.0, 32.0, 32.0),   // кусок в пикселях исходника
    Rect::new(200.0, 40.0, 64.0, 64.0),
    Color::rgba(255, 255, 255, 180),    // подкраска и прозрачность
);
```

| Метод | Что делает |
|---|---|
| `create_texture(&Image)` | Конвертирует поверхность в RGBA32, перепаковывает строки по `pitch` и заливает текстуру |
| `create_texture_rgba8(w, h, &[u8])` | Заливает плотно упакованные RGBA8; длина обязана быть `w * h * 4` |
| `draw_texture(&GpuTexture, dest, tint)` | Вся текстура, растянутая на `dest` |
| `draw_texture_region(&GpuTexture, source, dest, tint)` | Часть текстуры; `source` зажимается по границам, чтобы не залезть в соседний кадр спрайт-листа |

`GpuTexture` — дешёвый handle: `Clone` копирует только ссылки на GPU-память.
У него есть `size()`, `width()`, `height()`, `texture()` и `view()` — последние
два нужны, если строишь свой bind group.

Текстура принадлежит рендереру, который её создал: внутри неё лежит bind
group, собранный под layout и сэмплер этого рендерера. Переносить
`GpuTexture` в другой `Renderer2D` нельзя.

**Сэмплер всегда линейный.** Пиксель-арт на GPU будет сглаживаться; для
пиксель-арта пока лучше подходит SDL-окно с `pixel_art(true)`.

### Текст

```rust
let font = renderer.load_font(std::fs::read("font.otf")?)?;

renderer.draw_text(font, "Слева", 40.0, 40.0, 24.0, Color::WHITE)?;
renderer.draw_text_aligned(
    font, "По центру", 400.0, 40.0, 24.0, Color::WHITE, TextAlign::Center,
)?;
renderer.draw_text_centered(font, "В кнопке", 400.0, 200.0, 20.0, Color::WHITE)?;

let size = renderer.measure_text(font, "Строка\nв две строки", 20.0)?;
println!("{} × {}", size.x, size.y);
```

| Метод | Что делает |
|---|---|
| `load_font(bytes) -> Result<FontId, Error>` | Разбирает TTF/OTF и запоминает шрифт |
| `draw_text(font, text, x, y, size, color)` | Левый верх текстового блока в `(x, y)` |
| `draw_text_aligned(font, text, x, y, size, color, align)` | `x` — левый край, центр или правый край каждой строки |
| `draw_text_centered(font, text, x, y, size, color)` | Блок центрируется по `(x, y)` и по горизонтали, и по вертикали |
| `measure_text(font, text, size) -> Result<Vec2, Error>` | Ширина самой длинной строки и высота всего блока |

Правила:

- `size` — размер em в пикселях. Глифы растрируются именно в этом размере, а
  не масштабируются из одного базового.
- `\n` начинает новую строку. Автоматического переноса по ширине нет:
  сначала `measure_text`, потом своя разбивка.
- При `Center` и `Right` каждая строка выравнивается независимо.
- `measure_text` возвращает ширину самой длинной строки, а не сумму.
- Пробелы и другие пустые глифы пропускаются при рисовании, но участвуют в
  раскладке.
- Кириллица и любые другие символы работают, если они есть в шрифте.

`FontId` — маленький `Copy`-handle, но, как и текстура, он имеет смысл только
в том рендерере, который его выдал.

## font.rs: шрифты и атлас

Внутренний файл: публично видны только `FontId` и `TextAlign`, но понимать,
что происходит, полезно.

### Разбор и раскладка

`FontData::new` читает шрифт через `fontdue`. Дальше `layout`:

1. Режет текст по `\n`.
2. Считает ширину каждой строки с учётом кернинга.
3. Берёт метрики строки из шрифта; если таблицы нет — подставляет
   `ascent = 0.8 * size`, `descent = -0.2 * size`, `line_height = size`.
4. Ставит baseline первой строки в `y + ascent`, каждой следующей — плюс
   высота строки.
5. Раскладывает глифы по pen-позиции, прибавляя кернинг и advance.

### Атлас глифов

- Квадрат **2048×2048**, формат `Rgba8UnormSrgb`.
- Глифы пакуются полками (shelf packing): слева направо, при переполнении
  полки — на следующую строку.
- Вокруг каждого глифа 1 пиксель прозрачного поля, чтобы линейная фильтрация
  не подтягивала соседа.
- В атласе лежит **белый RGB и покрытие в альфе** — поэтому тот же шейдер,
  что рисует спрайт, умеет тонировать текст.
- Ключ кеша — `(символ, биты размера)`. Размер входит в ключ, поэтому
  анимировать размер каждый кадр нельзя: атлас забьётся, и вызов вернёт
  `Error::AtlasFull`.
- Глифы не вытесняются. Это рассчитано на несколько фиксированных размеров
  шрифта в интерфейсе.

## error.rs: `Error`

Ошибка всего GPU-слоя. Отдельная от `frglib::Error`, потому что у двух типов
окон разные точки отказа.

| Вариант | Когда возникает |
|---|---|
| `Sdl(sdl3::Error)` | SDL или видео подсистема не стартовали |
| `WindowBuild(WindowBuildError)` | Не создалось окно |
| `Window(WindowError)` | Ошибка насоса событий в `poll` |
| `Surface(CreateSurfaceError)` | Из нативного handle не создалась поверхность |
| `NoAdapter(RequestAdapterError)` | Нет адаптера с поддержкой поверхности |
| `Device(RequestDeviceError)` | Адаптер не дал устройство |
| `NoSurfaceFormat` | Поверхность не сообщила ни одного формата |
| `SurfaceValidation` | `get_current_texture` поймал validation error — баг в коде рисования |
| `Font(String)` | `fontdue` не разобрал байты шрифта |
| `UnknownFont` | `FontId` не из этого рендерера |
| `AtlasFull` | В атласе глифов кончилось место |
| `EmptyTexture` | Текстуру просят с нулевой шириной или высотой |
| `PixelData { expected, actual }` | Длина RGBA-среза не равна `width * height * 4` |

`Display` даёт человекочитаемое сообщение, `source()` возвращает вложенную
ошибку там, где она есть. `frglib::GpuError` — тот же тип под коротким
именем.

## shaders/: WGSL

### `lit.wgsl`

Один шейдер на все три 3D-пайплайна.

- Group 0 — камера: `view_proj`, `camera_pos`, `light_dir`, `light_color`.
- Group 1 — draw: `model`, `color`; с dynamic offset.
- `vs_main` умножает позицию на `model`, затем на `view_proj`; нормаль
  поворачивается модельной матрицей. Это предполагает равномерный масштаб —
  при растяжении нормали нужно было бы трансформировать обратной
  транспонированной матрицей.
- `fs_lit` считает diffuse и блик Блинна-Фонга, добавляет ambient и
  возвращает цвет с исходной альфой.
- `fs_unlit` возвращает цвет как есть.

Цвета приходят уже линейными. При sRGB-таргете `wgpu` конвертирует их при
записи, поэтому в шейдере нет гамма-кода.

### `sprite.wgsl`

Один шейдер на все 2D-вызовы.

- Group 0 — view: центр камеры, размер экрана и зум.
- Group 1 — текстура и сэмплер.
- Если `space < 0.5`, позиция мировая и переводится в пиксели:
  `(p - camera) * zoom + screen / 2`.
- Затем пиксели переводятся в clip space с переворотом Y (экран растёт вниз,
  clip — вверх).
- Фрагмент возвращает `texel * color`. Для фигур текстура белая, для текста
  в альфе лежит покрытие, для спрайтов — свой цвет.

## Как этим пользоваться

Примеры ниже возвращают `Result<(), frglib::gpu::Error>`, когда в них только
GPU-вызовы. Если в игре смешаны `std::fs`, `Image` и `TileMap`, удобнее
`main() -> Result<(), Box<dyn std::error::Error>>`: так устроен `wgpu-hud`, и
тогда `?` работает для всех ошибок сразу.

### Минимальное 2D-приложение

```rust
use frglib::gpu::{Renderer2D, WgpuWindow};
use frglib::{Color, Event, Rect};

const FONT: &[u8] = include_bytes!("font.otf");

fn main() -> Result<(), frglib::gpu::Error> {
    let mut window = WgpuWindow::builder("2D on wgpu")
        .size(960, 540)
        .background(Color::rgb(18, 20, 28))
        .build()?;

    let mut ui = Renderer2D::new(window.gpu());
    let font = ui.load_font(FONT.to_vec())?;

    loop {
        for event in window.poll()? {
            if matches!(event, Event::Quit | Event::CloseRequested) {
                return Ok(());
            }
        }

        let Some(mut frame) = window.begin_frame()? else {
            continue;
        };

        ui.begin(&frame);
        ui.fill_rect(Rect::new(40.0, 40.0, 240.0, 64.0), Color::rgb(52, 86, 140));
        ui.draw_text(font, "Hello, wgpu", 56.0, 58.0, 24.0, Color::WHITE)?;
        ui.finish(&mut frame);

        window.present(frame)?;
    }
}
```

### 3D-сцена

```rust
use frglib::gpu::glam::{Mat4, Vec3};
use frglib::gpu::{Camera3D, Renderer3D, WgpuWindow};
use frglib::{Color, Event};

fn main() -> Result<(), frglib::gpu::Error> {
    let mut window = WgpuWindow::builder("3D on wgpu").build()?;
    let mut scene = Renderer3D::new(window.gpu());
    let mut angle = 0.0f32;

    loop {
        for event in window.poll()? {
            if matches!(event, Event::Quit | Event::CloseRequested) {
                return Ok(());
            }
        }

        let Some(mut frame) = window.begin_frame()? else {
            continue;
        };

        let camera = Camera3D::orbit(Vec3::new(0.0, 1.0, 0.0), 6.0, angle, 0.35);

        scene.begin(&camera);
        scene.draw_grid(20, 1.0, Color::rgb(60, 70, 88));
        scene.draw_cube_model(
            Mat4::from_translation(Vec3::new(0.0, 1.0, 0.0))
                * Mat4::from_rotation_y(angle)
                * Mat4::from_scale(Vec3::splat(1.5)),
            Color::GREEN,
        );
        scene.finish(&mut frame);

        window.present(frame)?;
        angle += 0.01;
    }
}
```

### 3D-сцена и 2D-HUD в одном кадре

Порядок обязателен: сначала 3D (он очищает кадр), потом 2D в режиме overlay.

```rust
scene.begin(&camera3d);
scene.draw_grid(20, 1.0, Color::rgb(60, 70, 88));
scene.draw_cube(Vec3::ZERO, Vec3::ONE, Color::GREEN);
scene.finish(&mut frame);          // Clear: 3D-сцена записана

ui.begin_overlay(&frame);          // Load: ничего не стираем
ui.fill_rect(Rect::new(16.0, 16.0, 260.0, 96.0), Color::rgba(0, 0, 0, 160));
ui.draw_text(font, "HUD", 28.0, 24.0, 22.0, Color::WHITE)?;
ui.finish(&mut frame);

window.present(frame)?;
```

Если перепутать и вызвать `begin` после 3D, сцена исчезнет: `begin` очищает
цвет и глубину.

### Своя текстура и спрайт-лист

```rust
let sheet = ui.create_texture(&Image::from_file("tiles.png")?)?;

// Кадр 32×32 в левом верхнем углу листа.
ui.draw_texture_region(
    &sheet,
    Rect::new(0.0, 0.0, 32.0, 32.0),
    Rect::new(100.0, 100.0, 64.0, 64.0),
    Color::WHITE,
);
```

Одна текстура может рисоваться сколько угодно раз за кадр: каждая копия —
шесть вершин в общем буфере.

### Кнопка и слайдер

```rust
use frglib::{Button, Rect, Slider};

let mut play = Button::new(Rect::new(40.0, 200.0, 180.0, 44.0));
let mut volume = Slider::new(Rect::new(40.0, 150.0, 240.0, 24.0))
    .with_range(0.0, 1.0, 0.05);

// События приходят из window.poll().
let events = window.poll()?;
if play.update(&events) {
    // клик
}
volume.update(&events);

ui.begin(&frame);
volume.draw_gpu(&mut ui);
play.draw_gpu(&mut ui, font, "Play", 22.0)?;
ui.finish(&mut frame);
```

`draw_gpu` временно включает экранное пространство и возвращает прежнюю
камеру. `Button::draw_gpu` рисует фон и центрированную подпись; `Slider` —
дорожку, заполненную часть и ручку. SDL-методы `draw` у этих типов не
изменились.

### `TileMap` на GPU

```rust
use frglib::{Camera, Image, TileMap, Vec2};

let tiles = vec![
    ui.create_texture(&Image::from_file("water.png")?)?,
    ui.create_texture(&Image::from_file("sand.png")?)?,
];

let map = TileMap::from_rows(
    vec![
        vec![0usize, 1, 0, 1],
        vec![1, 0, 1, 0],
    ],
    32.0,
)
.map_err(|err| format!("bad map: {err}"))?
.origin(100.0, 100.0);

let camera = Camera::new(Vec2::new(480.0, 270.0));

map.draw_gpu(&mut ui, camera, |tile| tiles.get(*tile).cloned());
```

`draw_gpu` рисует только видимые клетки, в мировых координатах, и
восстанавливает камеру после вызова. `texture_of` возвращает
`Option<GpuTexture>` — `None` пропускает клетку. Камера одновременно решает,
что видно, и как клетки проецируются.

`Option<GpuTexture>` вместо ссылки выбран намеренно: замыкание, возвращающее
`&GpuTexture`, не даёт нормально жить массиву текстур рядом с вызовом, а
`GpuTexture` — дешёвый clone.

### Свой проход `wgpu` в том же кадре

`Frame` отдаёт энкодер и оба вложения. Свои проходы после встроенных должны
использовать `LoadOp::Load`, иначе они затрут сцену.

```rust
use frglib::gpu::wgpu;

let Some(mut frame) = window.begin_frame()? else {
    continue;
};

// Сначала встроенные проходы: они очищают вложения и задают глубину.
scene.begin(&camera3d);
scene.draw_grid(20, 1.0, Color::rgb(60, 70, 88));
scene.finish(&mut frame);

{
    let mut pass = frame.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("my pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &frame.color,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });

    // pass.set_pipeline(&my_pipeline);
    // pass.draw(0..3, 0..1);
}

window.present(frame)?;
```

Проход нужно закрыть (отпустить `pass`) до `present`: энкодер нельзя
финализировать, пока на него есть заимствование.

Если свой проход идёт **первым**, он сам отвечает за очистку: используйте
`LoadOp::Clear(wgpu::Color::BLACK)` или любой другой цвет в линейном
пространстве. Публичного конвертера `Color → wgpu::Color` нет — встроенные
рендереры делают это приватно.

### Ресайз, свёрнутое окно и vsync

- `begin_frame` сам пересоздаёт поверхность и буфер глубины, когда меняется
  физический размер окна.
- Свёрнутое или перекрытое окно даёт `Ok(None)` — просто пропустите кадр и
  не считайте это ошибкой.
- Режим показа — `AutoVsync`: драйвер сам выбирает поддерживаемый
  vsync-режим. Своего переключателя нет.
- `render_size()` отстаёт от `size()` на один кадр: между событием ресайза и
  следующим `begin_frame` они могут отличаться.

## Порядок вызовов в кадре

```text
window.poll()
      │
      ▼
window.begin_frame() ── None ──► continue
      │ Some(frame)
      ▼
scene.begin(&camera3d)        2D: ui.begin(&frame)
scene.draw_*                  ui.fill_rect / draw_texture / draw_text
scene.finish(&mut frame)      ui.finish(&mut frame)
      │                            │
      └──────────┬─────────────────┘
                 ▼
      свои проходы через frame.encoder (LoadOp::Load)
                 ▼
      window.present(frame)
```

| Этап | Что делает с цветом | Что делает с глубиной |
|---|---|---|
| `Renderer3D::begin` | ничего | ничего |
| `Renderer3D::finish` | `Clear(background)` | `Clear(1.0)` |
| `Renderer2D::begin` | `Clear(background)` | `Clear(1.0)` |
| `Renderer2D::begin_overlay` | `Load` | `Load` |
| `Renderer2D::finish` | рисует, не меняя load-op | тест всегда проходит, запись выключена |

## Координаты и цвет

| Что | Правило |
|---|---|
| 2D, экран | Y вниз, `(0, 0)` — левый верхний угол, единица — физический пиксель |
| 2D, мир | Ось Y вниз, `Camera::position` — центр вида, `zoom` — пикселей на мировую единицу |
| Текст | Всегда экранные пиксели, даже в мировом режиме |
| 3D | Правая система, Y вверх, камера смотрит в `-Z`; `glam` 0.34 |
| Цвет на входе | `Color` — 8-битный sRGB |
| Цвет в шейдере | Линейный RGBA; альфа не гамма-корректируется |
| Цвет на выходе | sRGB-таргет свопчейна, `wgpu` конвертирует сам |
| Глубина | `Depth32Float`; 2D её не пишет |

## Ограничения и подводные камни

- **Фильтрация.** Сэмплер всегда линейный. Пиксель-арт на GPU сглаживается;
  для него пока лучше SDL-окно с `pixel_art(true)`.
- **Атлас шрифтов.** 2048×2048, без вытеснения. Несколько фиксированных
  размеров — нормально; анимация размера каждый кадр забьёт атлас и вернёт
  `Error::AtlasFull`.
- **Сглаживание.** MSAA не включён. Круги и текст сглаживаются
  растеризацией и линейной фильтрацией.
- **Один бэкенд на окно.** `Window` и `WgpuWindow` не смешиваются: в одном
  окне нельзя одновременно `SDL_Renderer` и `wgpu`.
- **Владение ресурсами.** `FontId` и `GpuTexture` принадлежат создавшему их
  рендереру. Чужой `FontId` даёт `UnknownFont`; чужую текстуру рисовать не
  следует.
- **Нет переноса текста.** Только `\n`; ширину считайте сами через
  `measure_text`.
- **Нормали 3D.** `lit.wgsl` поворачивает нормаль модельной матрицей, что
  верно для равномерного масштаба. При растяжении освещение будет неточным.
- **`Renderer3D::finish` очищает кадр.** Всё, что должно быть под сценой,
  рисуйте до него; 2D поверх — только через `begin_overlay`.
- **Пустой overlay бесплатен, пустой `begin` — нет.** `begin` обязан
  очистить кадр, поэтому создаёт проход даже без draw-вызовов.
- **macOS.** `WgpuWindowBuilder` всегда вызывает `.metal_view()`; без этого
  `wgpu` не получит нативный handle.
- **Версии `wgpu` и `glam` — часть API.** Обновление любого из них меняет
  публичные типы в сигнатурах.

## Тесты и проверка

```bash
cargo fmt --check
cargo check --all-targets --features wgpu

# Библиотека с GPU-слоем и без него.
cargo test --lib --features wgpu
cargo test --lib

# Примеры документации.
cargo test --doc --features wgpu

# Живые примеры.
cargo run --features wgpu --example wgpu-cube
cargo run --features wgpu --example wgpu-hud
```

Юнит-тесты GPU-слоя не требуют видеокарты: они проверяют раскладку текста,
аллокатор атласа, размеры вершин и юниформ, математику камеры и конвертацию
цвета. Всё, что реально трогает `Device` или `Queue`, живёт в примерах.

## Шпаргалка по API

### Окно и кадр

```rust
WgpuWindow::builder(title) -> WgpuWindowBuilder
WgpuWindowBuilder::{size, resizable, background, build}
WgpuWindow::{poll, gpu, size, render_size, set_background, background, begin_frame, present}
Gpu::{instance, adapter, adapter_info, device, queue, format, size, config}
Frame::{encoder, color, depth, size, background} + Frame::aspect
```

### 3D

```rust
Camera3D::{perspective, position, target, up, fov_y, clip, orbit, view, projection, view_projection}
Mesh::{cube, plane, grid, from_vertices, from_line_list, line}
Vertex::{new, with_color}
Renderer3D::{new, create_mesh, begin, draw_mesh, draw_mesh_with, draw_cube, draw_cube_model,
             draw_plane, draw_grid, draw_line, draw_lines, finish}
Material::{Lit, Unlit}
```

### 2D

```rust
Renderer2D::{new, load_font, create_texture, create_texture_rgba8,
             begin, begin_overlay, finish, size, camera, set_camera,
             set_screen_space, is_screen_space, screen_space,
             fill_rect, draw_rect, draw_line, fill_circle,
             draw_texture, draw_texture_region,
             draw_text, draw_text_aligned, draw_text_centered, measure_text}
GpuTexture::{size, width, height, texture, view}
FontId
TextAlign::{Left, Center, Right}
```

### UI и TileMap

```rust
Button::{new, with_colors, update, state, rect, draw, draw_gpu}
Slider::{new, with_range, with_colors, update, state, rect, value, set_value,
         fraction, knob_side, draw, draw_gpu}
TileMap::{from_rows, origin, visible, draw, draw_gpu}
```

### Ошибки

```rust
use frglib::GpuError; // то же, что frglib::gpu::Error
```
