# 10. Wgpu: 3D, 2D и текст (опционально)

Фича `wgpu` добавляет в FRGLib второй тип окна — `WgpuWindow`. В нём SDL3
по-прежнему создаёт окно и приносит события, но кадром владеет `wgpu`:
свопчейн, командный энкодер, пайплайны и буферы. Рендерить можно и 3D, и 2D,
и текст: для этого есть два независимых высокоуровневых рендерера.

Фича не включена по умолчанию. Без неё библиотека остаётся такой же лёгкой,
как раньше, и вообще не тянет GPU-стек.

```bash
cargo run --example wgpu-cube --features wgpu
cargo run --example wgpu-hud  --features wgpu
```

## Два рендерера

| Тип | Для чего | Ключевые методы |
|---|---|---|
| `Renderer3D` | Меши, кубы, плоскости, сетки и отладочные линии | `begin`, `draw_cube`, `draw_mesh`, `draw_grid`, `draw_line`, `finish` |
| `Renderer2D` | Прямоугольники, линии, круги, текстуры и текст | `begin`, `fill_rect`, `draw_texture`, `draw_text`, `finish` |
| `Camera3D` | Перспективная камера с орбитальным вращением | `perspective`, `orbit`, `position`, `target` |
| `Camera` | Обычная 2D-камера: позиция и зум | `new`, `with_zoom` |

`Renderer2D` — это GPU-аналог `Draw`. Он рисует в экранных пикселях или в
мировых единицах через обычную `Camera` из `frglib::graphics`.

## Жизненный цикл кадра

Кадр начинается с `window.begin_frame()`, который может вернуть `None`, если
окно свёрнуто или поверхность не готова. Пропуск такого кадра — норма, а не
ошибка.

### Только 2D

```rust
let Some(mut frame) = window.begin_frame()? else { continue };

renderer.begin(&frame);                 // очищает цвет и глубину
renderer.fill_rect(Rect::new(20.0, 20.0, 200.0, 60.0), Color::BLUE);
renderer.draw_text(font, "Привет", 28.0, 32.0, 24.0, Color::WHITE)?;
renderer.finish(&mut frame);

window.present(frame)?;
```

### 3D и 2D поверх него

`Renderer3D::finish` всегда очищает экран цветом фона окна. Поэтому 2D-слой
после него должен использовать `begin_overlay`, а не `begin`:

```rust
scene.begin(&camera3d);
scene.draw_cube(Vec3::ZERO, Vec3::ONE, Color::GREEN);
scene.finish(&mut frame);

ui.begin_overlay(&frame);               // LoadOp::Load: 3D остаётся на месте
ui.draw_text(font, "HUD", 24.0, 24.0, 28.0, Color::WHITE)?;
ui.finish(&mut frame);
```

Глубина в 2D-пайплайне всегда проходит тест и не пишется, так что интерфейс
рисуется поверх сцены независимо от её содержимого.

### Сырой API

`Gpu` отдаёт `Instance`, `Adapter`, `Device`, `Queue` и конфигурацию
поверхности. `Frame` отдаёт `encoder`, `color`, `depth`, `size` и
`background`, поэтому в тот же кадр можно записать собственные проходы.
Свои проходы после встроенных должны использовать `LoadOp::Load`.

```rust
let gpu = window.gpu();
let device = gpu.device();
let queue = gpu.queue();

let Some(mut frame) = window.begin_frame()? else { continue };
// ...свои wgpu-команды через frame.encoder...
window.present(frame)?;
```

## Renderer2D

### Примитивы

```rust
renderer.fill_rect(Rect::new(10.0, 10.0, 120.0, 40.0), Color::RED);
renderer.draw_rect(Rect::new(10.0, 10.0, 120.0, 40.0), 2.0, Color::WHITE);
renderer.draw_line(Vec2::new(10.0, 80.0), Vec2::new(130.0, 80.0), 3.0, Color::WHITE);
renderer.fill_circle(Vec2::new(80.0, 160.0), 24.0, Color::BLUE);
```

`Rect.x` и `Rect.y` — левый верхний угол. Ось Y направлена вниз, как в
остальной библиотеке.

### Экранное и мировое пространство

После `begin` рендерер работает в экранных пикселях. `set_camera` включает
мировые координаты:

```rust
renderer.set_camera(Camera::new(Vec2::new(400.0, 300.0)).with_zoom(2.0));
renderer.fill_rect(Rect::new(-20.0, -20.0, 40.0, 40.0), Color::RED);
renderer.set_screen_space();
```

`screen_space(|r| ...)` временно возвращает экранные координаты и
восстанавливает прежний режим, как одноимённый метод у `Draw`.

### Текстуры

Текстуры создаются из `Image` или из сырых RGBA-байтов:

```rust
let texture = renderer.create_texture(&Image::from_file("sprite.png")?)?;
let raw = renderer.create_texture_rgba8(2, 2, &[
    255, 0, 0, 255,   0, 255, 0, 255,
    0, 0, 255, 255,   255, 255, 255, 255,
])?;

renderer.draw_texture(&texture, Rect::new(40.0, 40.0, 128.0, 128.0), Color::WHITE);
renderer.draw_texture_region(
    &texture,
    Rect::new(0.0, 0.0, 32.0, 32.0),    // кусок в пикселях исходника
    Rect::new(200.0, 40.0, 64.0, 64.0),
    Color::rgba(255, 255, 255, 180),     // подкраска и прозрачность
);
```

`Image` конвертируется в RGBA32, поэтому подходит любой формат, который
умеет читать SDL. Созданная текстура — дешёвая ссылка на GPU-память, её можно
клонировать и хранить между кадрами.

### Текст и шрифты

Шрифты разбирает `fontdue` — чистый Rust, без SDL3_ttf. Это осознанное
исключение из правила «в библиотеке нет разбора шрифтов»: оно действует
только под фичей `wgpu`, где текст рисует сама библиотека, а не вызывающий
код.

```rust
let font = renderer.load_font(std::fs::read("font.otf")?)?;

renderer.draw_text(font, "Слева", 40.0, 40.0, 24.0, Color::WHITE)?;
renderer.draw_text_aligned(
    font, "По центру", 400.0, 40.0, 24.0, Color::WHITE, TextAlign::Center,
)?;
renderer.draw_text_centered(font, "В кнопке", 400.0, 200.0, 20.0, Color::WHITE)?;

let size = renderer.measure_text(font, "Строка\nв две строки", 20.0)?;
```

* `\n` начинает новую строку; автоматического переноса нет.
* `size` — размер em в пикселях.
* Текст всегда рисуется в экранных пикселях, даже если рендерер сейчас в
  мировом режиме: это интерфейсный элемент, а не объект сцены.
* `TextAlign` — `Left`, `Center`, `Right`; выравнивание задаёт точку, в
  которую попадает `x`.
* `measure_text` возвращает ширину самой длинной строки и высоту всего блока.
* Кириллица и любые другие глифы работают, если они есть в шрифте.

Глифы растрируются при первом использовании и складываются в атлас
2048×2048. Размер входит в ключ кеша, поэтому шрифт, нарисованный в десяти
разных размерах, занимает место десяти наборов. Для интерфейса это нормально;
анимировать размер каждый кадр не стоит — атлас переполнится и вернёт
`Error::AtlasFull`.

## UI и TileMap на GPU

Существующие виджеты не изменили свой SDL-API. У них появились
дополнительные методы под фичей `wgpu`:

```rust
slider.draw_gpu(&mut renderer);
button.draw_gpu(&mut renderer, font, "Play", 22.0)?;
```

`Button::draw_gpu` рисует фон и центрированную текстовую подпись, а
`Slider::draw_gpu` — дорожку, заполненную часть и ручку. Оба временно
переключают рендерер в экранное пространство и возвращают прежнюю камеру.

`TileMap::draw_gpu` рисует только видимые клетки в мировых координатах:

```rust
let tiles = [
    renderer.create_texture(&Image::from_file("water.png")?)?,
    renderer.create_texture(&Image::from_file("sand.png")?)?,
];

map.draw_gpu(&mut renderer, camera, |tile| tiles.get(*tile).cloned());
```

Камера одновременно решает, какие клетки видимы, и как они проецируются.
Если камера стоит в центре экрана с зумом 1, мировые координаты совпадают с
экранными пикселями — удобно для интерфейсных карт.

## Координаты и математика

* **2D:** ось Y вниз, экранные пиксели или мировые единицы через `Camera`.
* **3D:** правая система, Y вверх; `glam` 0.34. `Vec3` и `Mat4`
  реэкспортированы через `frglib::gpu::glam`.
* **Цвета:** конвертируются из sRGB в линейное пространство на CPU. Целевой
  формат свопчейна — sRGB, поэтому GPU переводит результат обратно.

## Освещение

`lit.wgsl` реализует один направленный источник света, ambient и
specular-блики (Блинн-Фонг). Для сеток, осей и отладочных линий используйте
`Material::Unlit`.

## Ограничения

* **Фильтрация:** сэмплер всегда линейный. Пиксель-арт на GPU будет
  сглаживаться; для него пока лучше остаётся SDL-окно с `pixel_art(true)`.
* **Атлас шрифтов:** 2048×2048, без вытеснения глифов. Это рассчитано на
  несколько фиксированных размеров шрифта.
* **Сглаживание:** MSAA не включён; круги и текст сглаживаются самой
  растеризацией и линейной фильтрацией.
* **Смешивание бэкендов:** в одном окне нельзя одновременно использовать
  `SDL_Renderer` и `wgpu`. Это разные типы окна: `Window` и `WgpuWindow`.
* **macOS:** `WgpuWindowBuilder` по умолчанию вызывает `.metal_view()`;
  без этого `wgpu` не получит нативный handle.
