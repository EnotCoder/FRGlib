# 02. Окно и рисование

Модуль: `src/window/`

```text
src/window/
├── mod.rs        Window: создание, poll, frame/frame_with, размер, фон
├── builder.rs    WindowBuilder: настройка и build()
├── error.rs      Error: единый тип ошибок окна/рендерера/рисования
├── pump.rs       thread-local EventPump
└── draw/
    ├── mod.rs    Draw + приватные SDL-примитивы
    ├── object.rs публичные *_object методы
    └── geometry.rs чистая геометрия и вершины
```

## `Window`

Объявление — `src/window/mod.rs`.

| Поле | Тип | Зачем |
|---|---|---|
| `textures` | `graphics::texture::Store` | Только с `cached-textures`. Хранилище загруженных текстур |
| `canvas` | `sdl3::render::WindowCanvas` | Окно + рендерер SDL |
| `scale` | `sdl3::render::ScaleMode` | `Linear` или `Nearest`, применяется к каждой создаваемой текстуре |
| `background` | `Color` | Цвет, которым `frame`/`frame_with` заливают окно |
| `_sdl` | `sdl3::Sdl` | Держит SDL инициализированным, удаляется последним |

### Методы

| Метод | Что делает |
|---|---|
| `Window::builder(title)` | Возвращает `WindowBuilder` |
| `poll() -> Result<Vec<Event>, Error>` | Вычитывает очередь событий, фильтрует по id окна |
| `relative_mouse() -> Result<Vec2, Error>` | Смещение мыши с прошлого вызова в пикселях |
| `size() -> (u32, u32)` | Текущий drawable-размер окна |
| `pixel_art() -> bool` | Был ли включён `Nearest` при создании |
| `set_background(Color)` | Меняет цвет фона на лету |
| `background() -> Color` | Текущий цвет фона |
| `frame() -> Result<(), Error>` | Залить фон и показать кадр |
| `frame_with(closure) -> Result<(), Error>` | Залить фон, выполнить рисование, показать кадр |
| `vulkan_instance_extensions() -> Result<Vec<String>, Error>` | Расширения Vulkan, нужные окну |
| `upload(&Image) -> Result<Texture, Error>` | Только с `cached-textures`; заливает текстуру на GPU один раз |

`Window::create` — приватный конструктор, вызываемый только из builder.

### Особенности `frame` и `frame_with`

- Фон заливается **каждый кадр**: у SDL нет API «цвет окна по умолчанию».
- `frame_with` разделяет заимствования полей (`canvas`, `scale`, `textures`)
  через деструктуризацию, чтобы одновременно отдать `&mut canvas` и `&textures`.
- Внутри замыкания можно вернуть `Err`, и кадр не будет показан.
- `sdl3::set_draw_color`/`clear` паникуют при ошибке рендерера, а `present`
  возвращает `Error::Present`. Это задокументировано в rustdoc.

## `WindowBuilder`

Файл `src/window/builder.rs`. Все методы — цепочкой, `build()` последний.

| Метод | Умолчание | Смысл |
|---|---|---|
| `WindowBuilder::new(title)` | — | Создаёт builder |
| `.size(w, h)` | `800 × 600` | Размер в пикселях |
| `.resizable(bool)` | `true` | Можно ли менять размер |
| `.vulkan(bool)` | `false` | Флаг `SDL_WINDOW_VULKAN` |
| `.background(Color)` | `Color::GREEN` | Цвет фона |
| `.pixel_art(bool)` | `false` | `Nearest` вместо `Linear` |
| `.build()` | — | Открывает окно, возвращает `Result<Window, Error>` |

`pixel_art(true)` нужен для пиксель-арта: иначе SDL интерполирует соседние
тексели и спрайт «мылится». `Nearest` хорошо выглядит только при целочисленном
масштабе; при `zoom = 1.5` часть текселей удвоится, а часть нет — это
свойство режима, а не баг.

Внутри builder хранит не `bool`, а `ScaleMode`, чтобы применять режим одной
безусловной строкой во всех трёх местах создания текстуры.

## Событийный насос

`src/window/pump.rs`:

```rust
thread_local! {
    static PUMP: RefCell<Option<Box<EventPump>>> = const { RefCell::new(None) };
}

pub(crate) fn with_pump<R>(sdl: &Sdl, f: impl FnOnce(&mut EventPump) -> Result<R, Error>) -> Result<R, Error>
```

- Pump создаётся при первом обращении и живёт в потоке.
- `poll()` и `relative_mouse()` используют его.
- Если SDL откажется выдать pump, вернётся `Error::Sdl`.

`relative_mouse()` читает `pump.relative_mouse_state()`. Это **смещение**, а не
позиция: его надо прибавлять к текущему положению. Повторное чтение в том же
кадре вернёт ноль, потому что SDL сбрасывает счётчик.

## `Draw`

`Draw<'a>` живёт только внутри `frame_with`. Поля:

| Поле | Смысл |
|---|---|
| `canvas` | `&mut WindowCanvas` |
| `camera: Option<Camera>` | Камера текущего кадра |
| `scale: ScaleMode` | Режим фильтрации для текстур, создаваемых на лету |
| `textures: &Store` | Только с `cached-textures` |

### Управление камерой

| Метод | Что делает |
|---|---|
| `set_camera(camera)` | Ставит камеру на остаток кадра |
| `screen_space(closure)` | Временно снимает камеру, выполняет код, возвращает камеру |
| `size()` | Размер окна в пикселях |
| `camera_rect()` | Прямоугольник мира, видимый камерой; без камеры — весь экран |

`screen_space` сделан замыканием, а не парой `take_camera`/`restore_camera`:
забыть восстановить камеру слишком легко, и тогда интерфейс начнёт ездить
вместе с миром. Так делают `Button::draw`, `Slider::draw` и `Label::draw`.

### Публичные методы рисования

Все объявлены в `src/window/draw/object.rs`.

| Метод | Что рисует | Фича |
|---|---|---|
| `fill_object(&Object, Color)` | Залитый прямоугольник с поворотом | — |
| `draw_object(&Object, Color)` | Контур прямоугольника с поворотом | — |
| `fill_object_polygon(&Object, points, indices, Color)` | Залитый многоугольник, повёрнутый вокруг центра `Object` | — |
| `draw_object_polygon(&Object, points, Color)` | Контур многоугольника | — |
| `fill_textured_object(&Object, &Image)` | Текстура в прямоугольнике, с поворотом и отражением | — |
| `fill_textured_object_polygon(&Object, points, indices, &Image)` | Текстура в многоугольнике | — |
| `fill_object_texture(&Object, Texture)` | То же, но текстура уже на GPU | `cached-textures` |
| `fill_object_texture_polygon(&Object, points, indices, Texture)` | Текстура-многоугольник с готовой текстурой | `cached-textures` |
| `fill_object_animation(&Object, &Animation)` | Текущий кадр анимации | `cached-textures` |

Что важно знать:

- `fill_object` при `rotation == 0.0` идёт через быстрый `fill_rect`; при
  повороте — через `fill_polygon` и `Object::corners()`.
- `fill_object_polygon` использует `Object` только как центр поворота. `size`
  не действует.
- У `fill_object` флаги `flip_*` игнорируются: сплошной цвет нечего отражать.
- Текстурные методы безусловно включают `BlendMode::Blend`, чтобы альфа
  картинки работала.
- `fill_textured_object` и `fill_textured_object_polygon` создают текстуру на
  каждый вызов и удаляют после. Это дорого: заливка 800×600 RGBA — около
  1.9 МБ на вызов. Для повторяющихся картинок нужен `Texture`/`Animation`.
- Анимация сама не двигается: `Animation::advance(dt)` вызывает пользователь.
- Пустая анимация ничего не рисует (не ошибка).

### Приватные примитивы (`draw/mod.rs`)

| Метод | Назначение |
|---|---|
| `size_px()` | Размер окна как `(u32, u32)` |
| `place_rect(rect)` | Проекция прямоугольника через камеру, если она есть |
| `fill_rect(rect, color)` | `canvas.fill_rect` + управление blend mode |
| `draw_polygon(points, color)` | Линии между соседними точками + замыкание контура |
| `fill_polygon(points, indices, color)` | Сборка `Vertex` и `render_geometry` |

Blend mode переключается только при необходимости: `Blend` для `a < 255`,
`None` для непрозрачного цвета. Каждое переключение — изменение состояния GL.

### Геометрия (`draw/geometry.rs`)

| Элемент | Смысл |
|---|---|
| `QUAD_INDICES` | `[0, 1, 2, 0, 2, 3]` — два треугольника по четырём углам |
| `WHITE` | `FColor` без модуляции, чтобы текстура шла как есть |
| `to_f_rect` | `Rect` → `sdl3::render::FRect` |
| `rotate_points` | Поворот набора точек вокруг центра `Object` |
| `project` | Перевод точек мира в экран через камеру |
| `polygon_vertices` | Проверка индексов, сборка `Vertex`, текстурные координаты |

`polygon_vertices` возвращает `Error::IndicesNotTriples`, если число индексов
не кратно трём, и `Error::IndexOutOfBounds`, если индекс ссылается за пределы
точек. Раньше это были `assert!`, которые убивали процесс.

Текстурные координаты считаются по bounding box **до** поворота: картинка
растягивается на форму, а затем поворачивается вместе с ней.

## Ошибки

`src/window/error.rs`:

| Вариант | Причина |
|---|---|
| `Error::Sdl` | Инициализация SDL, видео, запросы к окну |
| `Error::WindowBuild` | Не удалось создать окно |
| `Error::Renderer` | Не удалось создать рендерер |
| `Error::Texture` | Не удалось создать текстуру из поверхности |
| `Error::IndicesNotTriples { count }` | Индексов не кратно трём |
| `Error::IndexOutOfBounds { index, len }` | Индекс вне списка точек |
| `Error::Present` | Не удалось показать кадр |

Реализованы `Display`, `std::error::Error`, `source()` и `From` для ошибок
`sdl3`, поэтому в приложении достаточно `Result<(), frglib::Error>` и `?`.

## Vulkan

На данный момент поддержка ограничена:

1. `WindowBuilder::vulkan(true)` ставит `SDL_WINDOW_VULKAN`.
2. `Window::vulkan_instance_extensions()` возвращает список расширений,
   которые нужны окну для создания `VkSurface`.

У этого окна нет своего Vulkan-контекста, swapchain и рендеринга. Готовый
современный бэкенд живёт в отдельном типе окна `WgpuWindow` под фичей `wgpu`:
он использует Vulkan, Metal, DX12 или GLES и умеет и 3D, и 2D, и текст.
