# 07. Тайлмапы

Модуль: `src/tilemap/`

```text
src/tilemap/
├── mod.rs    объявления, реэкспорт TileMap и MapError
├── map.rs    TileMap<T>: хранение, конструкторы, поиск клеток
├── view.rs   видимые клетки, clamp_camera, draw
├── error.rs  MapError
└── tests.rs  интеграционные тесты модуля
```

Тайлмап — прямоугольная сетка клеток одинакового размера. Библиотека отвечает
за три вещи:

1. где лежат клетки и как мир связан с индексами;
2. какие клетки видит камера;
3. как нарисовать клетку текстурой.

Что клетка **значит** — решает игра. `TileMap<T>` не требует от `T` ни
`Copy`, ни какого-либо трейта.

## `TileMap<T>`

```rust
pub struct TileMap<T> {
    pub(super) cells: Vec<T>, // плоское хранение, row-major
    pub(super) cols: usize,
    pub(super) rows: usize,
    pub(super) cell: f32,
    pub(super) origin: Vec2,
}
```

Индекс клетки: `cells[row * cols + col]`. Плоский `Vec` выбран, чтобы карта
была одним выделением, а строка — одним срезом.

Поля `pub(super)`, потому что `view.rs` — сосед `map.rs`, а не потомок;
приватные поля были бы недоступны. За пределы `tilemap` они всё равно не
выходят.

### Конструкторы

| Метод | Что принимает |
|---|---|
| `TileMap::from_ascii(rows, cell, parse)` | `&[&str]`, размер клетки, `FnMut(char) -> Option<T>` |
| `TileMap::from_rows(rows, cell)` | `Vec<Vec<T>>` — для карт, сгенерированных кодом |

`from_ascii`:

- каждая строка должна быть одинаковой длины, иначе `MapError::RaggedRow`;
- `parse` возвращает `None` для неизвестного символа →
  `MapError::UnknownChar`;
- пустой список строк или строки нулевой длины → `MapError::NoRows`;
- размер клетки должен быть конечным и `> 0`, иначе `MapError::BadCell`.

Опечатка в карте останавливает программу, а не превращается в пустую клетку,
сквозь которую игрок провалится.

Пример:

```rust
#[derive(Debug)]
enum Cell { Floor, Water }

let map = frglib::TileMap::from_ascii(
    &["......", "..~~..", "......"],
    32.0,
    |c| Some(match c {
        '~' => Cell::Water,
        _ => Cell::Floor,
    }),
)?;
```

`from_rows` проверяет те же правила: непустая карта, все строки одной длины.
Так делает пример `2d_games-fgoty`, где карта генерируется.

### Размеры и позиция

| Метод | Смысл |
|---|---|
| `.origin(x, y)` | Куда в мире встаёт левый верхний угол клетки `(0, 0)` |
| `.cols()`, `.rows()` | Количество клеток по осям |
| `.cell_size()` | Сторона клетки в мировых единицах |
| `.width()` | `cols * cell` |
| `.height()` | `rows * cell` |

`origin` полезен, когда уровень привязан к земле, а не к мировому нулю:

```rust
let map = map.origin(0.0, 400.0 - map.height());
```

### Поиск клеток

| Метод | Что возвращает |
|---|---|
| `.contains(col, row)` | Есть ли такая клетка |
| `.get(col, row)` | `Option<&T>` |
| `.cell_at(world)` | `Option<(col, row)>` по мировой точке |
| `.get_at(world)` | `Option<&T>` сразу |
| `.cell_rect(col, row)` | `Rect` клетки в мире (даже за границей карты) |
| `.cell_center(col, row)` | `Vec2` центра клетки в мире |

`cell_at`:

```text
local = world - origin
col = floor(local.x / cell)
row = floor(local.y / cell)
```

Используется `floor`, а не `round`: точка на границе принадлежит клетке, в
которую она только что вошла. При движении вправо ответ меняется ровно на
границе. `NaN` и бесконечности отбрасываются до приведения к `usize`.

`cell_rect` за границей карты всё равно возвращает прямоугольник: вызывающему
обычно нужно рисовать или проверять пересечение, а вопрос «входит ли клетка в
карту» решает `contains`.

## `MapError`

```rust
pub enum MapError {
    NoRows,
    RaggedRow { row: usize, expected: usize, got: usize },
    UnknownChar { row: usize, col: usize, ch: char },
    BadCell { cell: f32 },
}
```

| Вариант | Причина |
|---|---|
| `NoRows` | Нет строк или все строки пустые |
| `RaggedRow` | Длина строки отличается от первой |
| `UnknownChar` | `parse` вернул `None` |
| `BadCell` | Размер клетки `<= 0`, `NaN` или бесконечность |

`MapError` отдельный от `Error` намеренно: добавление варианта в `Error`
сломало бы все `match` по нему без `_`. Реализованы `Display`,
`std::error::Error`, `Debug`, `Clone`, `PartialEq`.

## Видимость и отрисовка

Файл `src/tilemap/view.rs`.

### `visible(camera, view)`

Возвращает итератор `(col, row, &T)` по видимым клеткам, построчно слева
направо. Диапазон расширен на одну клетку за каждый край, чтобы тайл,
наполовину попавший в кадр, не появлялся скачком.

Алгоритм:

```text
zoom   = camera.zoom > EPSILON ? camera.zoom : 1.0
half_w = view.width  / 2 / zoom
half_h = view.height / 2 / zoom

col0..col1 = strip(camera.x - half_w, camera.x + half_w, origin.x, cols)
row0..row1 = strip(camera.y - half_h, camera.y + half_h, origin.y, rows)

strip(from, to, origin, len):
    start = floor((from - origin) / cell) - 1
    end   = ceil((to - origin) / cell) + 1
    start = clamp(start, 0, len)
    end   = clamp(end, 0, len)
    вернуть start..max(end, start)
```

Если карта целиком за экраном, `start == end`, и итератор пуст.

### `clamp_camera(&mut camera, view)`

Не даёт камере показать пустоту за краем карты.

```text
half_w = view.width  / 2 / zoom
half_h = view.height / 2 / zoom

если map_width <= 2 * half_w:
    camera.x = origin.x + map_width / 2    // карта уже экрана — центрируем
иначе:
    camera.x = clamp(camera.x, origin.x + half_w, origin.x + map_width - half_w)

то же для y
```

Камера с зумом меняет `half_w`/`half_h`, поэтому ограничение учитывает
масштаб.

### `draw(d, camera, texture_of)` (фича `cached-textures`)

```rust
map.draw(d, camera, |cell| match cell {
    Cell::Water => Some(water_tex),
    Cell::Floor => Some(grass_tex),
})?;
```

- рисует только клетки из `visible`;
- `texture_of` возвращает `Option<Texture>`; `None` пропускает клетку — так
  пустая клетка остаётся пустой без знания карты о том, что такое «пусто»;
- камера должна быть той же, что передана в `Draw::set_camera`;
- метод под фичей, потому что иначе каждая клетка заливала бы текстуру на GPU
  каждый кадр.

Если клетки рисуются цветом, а не текстурой, используйте `visible` напрямую:

```rust
for (col, row, cell) in map.visible(camera, window.size()) {
    let rect = map.cell_rect(col, row);
    // d.fill_object(...)
}
```

## Мини-рецепт

```rust
let map = TileMap::from_ascii(rows, 32.0, parse)?;
let mut camera = Camera::new(map.cell_center(0, 0));

map.clamp_camera(&mut camera, window.size());

window.frame_with(|d| {
    d.set_camera(camera);
    map.draw(d, camera, |cell| tiles.get(cell))
})?;
```

## Тесты

`src/tilemap/tests.rs` — 335 строк. Проверяются:

- парсинг ASCII и ошибки (`NoRows`, `RaggedRow`, `UnknownChar`, `BadCell`);
- `from_rows`;
- `origin`, размеры, `contains`, `get`;
- `cell_at` на границах и за пределами, включая нечисловые значения;
- `cell_rect`/`cell_center`;
- `visible` с камерой, зумом и картой меньше экрана;
- `clamp_camera` для карт больше и меньше окна.
