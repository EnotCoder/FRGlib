# 09. Сборка, тесты и разработка

## `Cargo.toml` верхним взглядом

```toml
[package]
name = "frglib"
version = "0.4.0"
edition = "2021"
rust-version = "1.82"
license = "MIT"
include = ["src/**", "Cargo.toml", "README.md", "LICENSE", "logo.svg"]
```

`include` означает: в публикуемый `.crate` попадут только исходники,
манифест, README, лицензия и логотип. Примеры и ассеты остаются в репозитории,
но не в пакете.

## Зависимости

| Зависимость | Версия | Когда |
|---|---|---|
| `sdl3` | 0.20 | Всегда; включён `raw-window-handle` |
| `sdl3` (dev) | 0.20 | Только примеры; включены `ttf` и `build-from-source-static` |

Библиотека не тянет SDL3_ttf. Примеры с текстом используют dev-зависимость,
поэтому пользователь крейта не получает парсер шрифтов, если текст ему не
нужен.

`build-from-source-static` у dev-зависимости нужен потому, что SDL3_ttf в
системе обычно не установлен. Первая сборка примера с текстом занимает около
минуты — она собирает и SDL3.

## Фичи

### `cached-textures`

```toml
cached-textures = ["sdl3/unsafe_textures"]
```

Включает:

- `graphics/texture.rs` — `Texture`, `Store`, `Window::upload`;
- `graphics/animation.rs` — `Animation`;
- `TileMap::draw`;
- `Draw::fill_object_texture`, `fill_object_texture_polygon`,
  `fill_object_animation`;
- реэкспорты `Texture` и `Animation` из корня.

Зачем: без неё текстура привязана к создателю (`TextureCreator`), и хранить её
между кадрами нельзя — `Window` начал бы ссылаться на собственное поле.
`sdl3/unsafe_textures` снимает проверку времени жизни и разрешает использовать
текстуру с чужим canvas (UB). FRGLib гарантирует, что этого не происходит:
`Store` лежит в окне, поле объявлено до canvas, рисование достаёт текстуру из
того же окна. Но фича opt-in, чтобы крейт оставался sound по умолчанию.

### `bundled`

```toml
bundled = ["sdl3/build-from-source-static"]
```

Собирает SDL3 из исходников и линкует статически. Нужны `cmake` и C-компилятор.

```bash
cargo build --release --features bundled
```

Тонкость: `sdl3-sys` переключается на статическую линковку только под
`link-static`, поэтому нужен именно `build-from-source-static`. Одного
`build-from-source` недостаточно.

### Комбинация

```bash
cargo build --release --features bundled,cached-textures
```

## Обычные команды

```bash
cargo build
cargo test
cargo test --lib          # только unit-тесты
cargo test --doc          # только doctest'ы
cargo doc --open
cargo clippy
cargo fmt
```

Запуск примеров:

```bash
cargo run --example basic-window
cargo run --features cached-textures --example basic-cached
```

## Тесты

На момент написания документации:

- `cargo test --lib` — **143 теста**, все проходят;
- `cargo test --lib --features cached-textures` — **163 теста** (добавляются
  20 тестов `Animation`);
- `cargo test --doc` — **14 doctest'ов**, все проходят.

Тесты не требуют окна и SDL: проверяется чистая логика.

| Область | Что тестируется |
|---|---|
| `math/mod.rs` | `Vec2`, операторы, длины, поворот, `lerp`, `direction_to` |
| `math/rect.rs` | `contains` (полуоткрытость), `overlaps`, пустой прямоугольник |
| `math/mat3.rs` | identity, перенос, поворот, масштаб, композиция, определитель |
| `math/ease.rs` | `Linear` пропускает вес, значения вне `0..=1`, `Default` |
| `graphics/color.rs` | именованные цвета, RGB/RGBA, конверсии, `Lerp`, зажим каналов |
| `graphics/object.rs` | `rect`, `corners`, поворот вокруг центра, `update` |
| `graphics/camera.rs` | `to_screen`/`to_world`, зум, нулевой и отрицательный зум |
| `graphics/animation.rs` | С фичей `cached-textures`: время вместо кадров, циклы, one-shot, пустая анимация, `fps` |
| `window/builder.rs` | `pixel_art` и умолчания |
| `window/draw/geometry.rs` | проекция без камеры, с камерой, порядок поворота/зума |
| `timing.rs` | период, `dt`, `tick`, `max_dt`, отсутствие накопления долга |
| `ui/button.rs` | наведение, клик по отпусканию, уход курсора, фокус, правая кнопка |
| `ui/slider.rs` | старт, нажатие, края, шаг, перетаскивание, фокус |
| `ui/text.rs` | все `Anchor`, сохранение размера |
| `tilemap/tests.rs` | парсинг, ошибки, координаты, видимость, `clamp_camera` |

Добавляя публичное поведение, добавляйте unit-тест. Если для теста нужен SDL
или окно, скорее всего, логику можно вынести в чистую функцию (так сделаны
`place`, `polygon_vertices`, `clamp_camera`).

## Правила и стиль

- Комментарии и rustdoc — по-английски; пользовательская документация
  (`README.md`, эта папка) — по-русски.
- Публичные элементы документируются `///`, модули — `//!`.
- Собственные типы вместо типов `sdl3` в публичном API, кроме явных
  исключений (`Image::from_surface`, реэкспорты клавиш/мыши).
- Ошибки возвращаются, а не паникуют. Паника допустима только там, где
  паникует `sdl3`, или где ошибка — нарушение контракта (`TextCache::get`).
- Для `cfg(feature = ...)` поля и методы объявляются условно, чтобы сборка без
  фичи не имела мёртвых веток.
- Имена файлов — по смыслу: `map.rs`, `view.rs`, `error.rs`, а не `mod2.rs`.

## Куда добавлять код

| Задача | Куда |
|---|---|
| Новый цвет/картинка/объект/камера | `src/graphics/`, затем реэкспорт в `graphics/mod.rs` и `lib.rs` |
| Новый метод рисования | Публичный — `src/window/draw/object.rs`; если нужен новый SDL-примитив — приватный в `src/window/draw/mod.rs`; чистая математика — `src/window/draw/geometry.rs` |
| Новое событие | Вариант в `src/event.rs` + ветка в `translate`; `Window::poll` менять не нужно |
| Новый виджет | Новый файл в `src/ui/`, `mod.rs`, реэкспорт в `lib.rs` |
| Новый математический тип | Новый файл в `src/math/`, `mod.rs`, реэкспорт в `lib.rs` |
| Новое поведение тайлмапа | Хранение/координаты — `tilemap/map.rs`; видимость/рисование — `tilemap/view.rs`; ошибки — `tilemap/error.rs` |
| Новый пример | Файл в `examples/<категория>/<имя>.rs` + секция `[[example]]` в `Cargo.toml` (+ `required-features`, если нужно) |

После добавления публичного типа не забудьте:

1. `pub use` в `mod.rs` своего модуля;
2. `pub use` в `src/lib.rs`;
3. rustdoc с примером, если поведение неочевидно;
4. тест;
5. строку в `README.md` и, при необходимости, в `docs/`.

## Публикация

Пакет собирается с `include`, поэтому примеры и `docs/` в `.crate` не
попадают. Перед публикацией полезно:

```bash
cargo package --list
cargo publish --dry-run
```

`README.md` и `LICENSE` обязательны: они указаны в манифесте.

## Известные особенности и ограничения

| Особенность | Детали |
|---|---|
| Vulkan не реализован | Только флаг окна и `vulkan_instance_extensions()` |
| `Window` не `Send`/`Sync` | Требование SDL: окно и рендерер живут в потоке инициализации |
| Одна очередь событий | `poll()` фильтрует по `window_id`, `Quit` — исключение |
| Текстуры по умолчанию заливаются каждый кадр | Для повторного рисования нужна фича `cached-textures` |
| `TileMap::draw` и `Animation` только с фичей | Без неё доступны `visible`, ручное рисование и `Image` |
| `Image::clone` может паниковать | Внутри `crop(...).expect(...)` |
| `TextCache::get` паникует | Если строку не вызывали через `keep` |
| `frame`/`frame_with` могут паниковать | Внутри `sdl3` на `set_draw_color`/`clear` |
| `Color::FON` | Публичное имя константы; менять его — ломающее изменение |
| Смешанные языки | Код и rustdoc — английский, README и `docs/` — русский, часть комментариев в примерах — русская |

## Полезные ссылки внутри репозитория

- Пользовательское API и таблицы методов — `README.md`.
- Публичная точка входа — `src/lib.rs`.
- Сгенерированная rustdoc-документация — `cargo doc --open` (или `target/doc`).
- Примеры — `examples/`.
