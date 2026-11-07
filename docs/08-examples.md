# 08. Примеры

Все примеры лежат в `examples/` и **не входят** в публикуемый пакет крейта
(см. `include` в `Cargo.toml`). Они нужны как живая документация и как ручная
проверка API.

## Почему примеры перечислены вручную

Cargo сам находит только `examples/*.rs` и `examples/*/main.rs` — на одну
уровень вглубь. Файлы вроде `examples/basic/draw.rs` лежат глубже, поэтому
каждый объявлен отдельной секцией `[[example]]` в `Cargo.toml`:

```toml
[[example]]
name = "basic-draw"
path = "examples/basic/draw.rs"
```

Имя target — это же имя бинарника, поэтому `/` в нём нельзя: используется
`basic-texture`, а не `basic/texture`.

Примеры, которым нужна фича, помечены:

```toml
required-features = ["cached-textures"]
```

## Каталог

### basic

| Target | Файл | Фича | Что показывает |
|---|---|---|---|
| `basic-window` | `examples/basic/window.rs` | — | Окно, фон, `poll()`, печать событий, `FrameClock` |
| `basic-draw` | `examples/basic/draw.rs` | — | Заливка объектов, подгонка квадрата под размер окна |
| `basic-texture` | `examples/basic/texture.rs` | — | Логотип как `Image` и `fill_textured_object` |
| `basic-polygon` | `examples/basic/polygon.rs` | — | Заливка многоугольника, контур, текстура на многоугольнике |
| `basic-object` | `examples/basic/object.rs` | — | Три объекта с поворотом: цвет, текстура, многоугольник |
| `basic-mouse` | `examples/basic/mouse.rs` | — | Состояние кнопок мыши между кадрами, `FocusLost` |
| `basic-camera` | `examples/basic/camera.rs` | — | Движение камеры, зум к курсору, `to_world` |
| `basic-cached` | `examples/basic/cached.rs` | `cached-textures` | Одинаковая жаба: заливка каждый кадр против одной загрузки |
| `basic-animation` | `examples/basic/animation.rs` | `cached-textures` | 8 кадров, `Animation::advance`, пауза, скорость, цикл |

### input

| Target | Файл | Фича | Что показывает |
|---|---|---|---|
| `input-relative` | `examples/input/relative.rs` | — | Абсолютная мышь (`MouseMotion`) против `relative_mouse()` |

### math

| Target | Файл | Фича | Что показывает |
|---|---|---|---|
| `math-lerp` | `examples/math/lerp.rs` | — | `lerp` для позиции, размера и цвета с одним весом |
| `math-sin-cos` | `examples/math/sin_cos.rs` | — | Круговое движение через `sin`/`cos` |
| `math-jumper` | `examples/math/jumper.rs` | — | Цепочка пружин, фиксированный шаг физики `1/240` |
| `math-matrix` | `examples/math/matrix.rs` | — | Иерархия `Mat3`: планета и спутник, `determinant` |
| `math-aabb` | `examples/math/aabb.rs` | — | AABB-коллизии: `Rect::overlaps`, цвет игрока при пересечении |

### ui

| Target | Файл | Фича | Что показывает |
|---|---|---|---|
| `ui-text` | `examples/ui/text.rs` | — | SDL3_ttf → `Image::from_surface` → `Label`, кириллица |
| `ui-button` | `examples/ui/button.rs` | — | Три состояния, клик по отпусканию, интерфейс поверх камеры |
| `ui-slider` | `examples/ui/slider.rs` | — | Непрерывный, дробный и целочисленный шаг; перетаскивание |

### 2d_games

| Target | Файл | Фича | Что показывает |
|---|---|---|---|
| `2d_games-fgoty` | `examples/2d_games/fgoty/main.rs` | `cached-textures` | RPG на `TileMap`, нарезка атласа, камера, коллизии |
| `2d_games-fgoty-dash` | `examples/2d_games/dash/main.rs` | `cached-textures` | Прыжковая физика, вращение куба, уровень строкой, меню |
| `2d_games-gallery` | `examples/2d_games/gallery.rs` | — | Листалка картинок по индексу, круговой проход |
| `2d_games-space` | `examples/2d_games/space/main.rs` | — | Солнечная система: орбиты, зум, перетаскивание, пояса |

## Запуск

```bash
cargo run --example basic-window
cargo run --example basic-draw
cargo run --example basic-texture
cargo run --example basic-polygon
cargo run --example basic-object
cargo run --example basic-mouse
cargo run --example basic-camera
cargo run --example input-relative
cargo run --example math-lerp
cargo run --example math-sin-cos
cargo run --example math-jumper
cargo run --example math-matrix
cargo run --example math-aabb
cargo run --example ui-text
cargo run --example ui-button
cargo run --example ui-slider
cargo run --example 2d_games-gallery
cargo run --example 2d_games-space

# с фичей
cargo run --features cached-textures --example basic-cached
cargo run --features cached-textures --example basic-animation
cargo run --features cached-textures --example 2d_games-fgoty
cargo run --features cached-textures --example 2d_games-fgoty-dash
```

## Ассеты

```text
examples/
├── basic/assets/           # логотип, жабы, кадры rotate_rust
├── ui/assets/              # font.otf, картинки для кнопок
└── 2d_games/assets/
    ├── font.otf            # один шрифт на все игры
    ├── fgoty/              # tilemap_forest.png, rpg_fgoty/*
    ├── dash/               # куб, шип, пол
    ├── gallery/            # картинки для листалки
    └── space/              # солнце, планеты, луны
```

Пути строятся от `CARGO_MANIFEST_DIR`, а не от текущего каталога:

```rust
fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/basic/assets")
        .join(name)
}
```

Так `cargo run --example` работает из любой директории.

Ассеты — обычные файлы в репозитории; они не пакуются в crate.

## Разбор двух больших примеров

### `2d_games-fgoty`

Файлы:

| Файл | Роль |
|---|---|
| `main.rs` | Окно, ввод, камера, цикл кадра |
| `map.rs` | Нарезка атласа 16×16, описание тайлов, генерация карты |
| `player.rs` | Направления, кадры ходьбы, скорость |
| `game.rs` | `World`: позиция игрока, обновление, коллизии через `TileMap` |
| `draw/mod.rs` | Порядок отрисовки сцены |

Ключевые решения:

- атлас режется `Image::crop`, каждый тайл заливается один раз через
  `Window::upload`;
- `TileMap::from_rows` принимает сгенерированную карту;
- `map.clamp_camera` не даёт камере уехать за лес;
- камера плавно догоняет игрока: `camera.position += (target - camera.position) * min(8*dt, 1)`;
- `pixel_art(true)`, потому что тайлы 16×16 рисуются в 32×32.

### `2d_games-fgoty-dash`

Файлы:

| Файл | Роль |
|---|---|
| `main.rs` | Два экрана (`Menu`/`Play`), ввод, камера, цикл |
| `level.rs` | Уровень строкой (`.` пусто, `#` шип, `=` платформа), геометрия |
| `player.rs` | Прыжок, гравитация, вращение, состояния |
| `game.rs` | Обновление и столкновения |
| `menu.rs` | Меню на SDL3_ttf + `Button`/`Label` |
| `draw/mod.rs` | Отрисовка тайлов, игрока, неба |

Ключевые решения:

- мир едет влево, а куб держится на фиксированной экранной `X`;
- земля ставится на долю высоты окна, а не на фиксированную мировую
  координату, потому что WM может выдать окно другого размера;
- меню забирает события себе, пока открыто, и не даёт игре реагировать на
  прыжки;
- шрифт — dev-зависимость SDL3_ttf, в библиотеку не попадает.

## `math-aabb`

Пример показывает проверку пересечения двух AABB-прямоугольников:

- неподвижный `Rect` стоит в центре;
- второй `Rect` следует за мышью;
- `Rect::overlaps` решает, пересекаются ли они;
- цвет игрока меняется, когда пересечение есть.

Так как `Draw` рисует через `Object` (центр + размер), а `Rect` хранит левый
верхний угол, в примере есть маленький помощник `object_of(rect)`. Он
переводит одно представление в другое; `Draw::fill_rect` намеренно остаётся
приватным.

## Что проверять примером

| Возможность | Пример |
|---|---|
| Окно и события | `basic-window` |
| Прямоугольники и альфа | `basic-draw` |
| Текстуры | `basic-texture` |
| Полигоны | `basic-polygon` |
| Поворот | `basic-object` |
| Камера и мышь | `basic-camera`, `input-relative` |
| Кеш текстур | `basic-cached`, `basic-animation` |
| Тайлмап | `2d_games-fgoty`, `2d_games-fgoty-dash` |
| UI | `ui-button`, `ui-slider`, `ui-text` |
| AABB-коллизии | `math-aabb` |
| Математика | `math-*` |
