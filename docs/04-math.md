# 04. Математика

Модуль: `src/math/`

```text
src/math/
├── mod.rs   Vec2, Lerp, lerp(), sin(), cos(), deg_to_rad(), rad_to_deg()
├── rect.rs  Rect
├── mat3.rs  Mat3
└── ease.rs  Ease
```

Этот модуль не зависит ни от SDL, ни от окна. Его можно использовать в тестах
и в игровой логике отдельно от графики.

## Углы

| Функция | Формула |
|---|---|
| `deg_to_rad(deg)` | `deg * (PI / 180)` |
| `rad_to_deg(rad)` | `rad * (180 / PI)` |

Соглашение:

- `Object::rotation` — **градусы**;
- `Vec2::rotated`, `Mat3::rotation` — **радианы**;
- положительный поворот при `y` вниз выглядит как поворот по часовой стрелке.

`sin` и `cos` — тонкие обёртки над `f32::sin`/`f32::cos`, добавлены «как в
Godot»; они принимают радианы.

## `Vec2`

```rust
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}
```

| Элемент | Смысл |
|---|---|
| `Vec2::ZERO` | `(0, 0)` |
| `Vec2::new(x, y)` | Конструктор |
| `length_squared()` | `x² + y²`, без `sqrt` — для сравнения расстояний |
| `length()` | `sqrt(x² + y²)` |
| `dot(other)` | `x1*x2 + y1*y2` |
| `direction_to(other)` | Единичный вектор от `self` к `other`; для совпадающих точек — `ZERO` |
| `normalized()` | Единичная длина; нулевой вектор остаётся нулевым |
| `rotated(radians)` | Поворот вокруг начала координат |
| `to_array()` | `[x, y]` |

Формула поворота:

```text
x' = x * cos(a) - y * sin(a)
y' = x * sin(a) + y * cos(a)
```

Операторы: `Add`, `Sub`, `Mul<f32>`, `Div<f32>`, `Neg`, `AddAssign`.
Преобразования: `From<(f32, f32)>`, `From<[f32; 2]>`.
`Default` даёт `ZERO`.

## `Rect`

```rust
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
```

Прямоугольник, выровненный по осям. `x`, `y` — левый верхний угол.

| Метод | Смысл |
|---|---|
| `Rect::new(x, y, w, h)` | Конструктор |
| `Rect::square(x, y, side)` | Квадрат с углом в `(x, y)` |
| `Rect::square_centered(cx, cy, side)` | Квадрат с центром в `(cx, cy)` |
| `to_array()` | `[x, y, w, h]` |
| `contains(x, y)` | Точка внутри; правый и нижний край **не входят** |
| `overlaps(&other)` | Пересечение AABB |
| `intersects(other)` | Алиас `overlaps` |
| `contains_vec(v)` | `contains(v.x, v.y)` |

`contains` полуоткрытый:

```text
x >= self.x && x < self.x + self.w &&
y >= self.y && y < self.y + self.h
```

Это важно для интерфейса: две кнопки, стоящие вплотную, не делят пиксель на
шве, и клик всегда попадает ровно в одну.

`overlaps` — стандартная проверка AABB:

```text
self.x < other.x + other.w &&
self.x + self.w > other.x &&
self.y < other.y + other.h &&
self.y + self.h > other.y
```

Преобразования: `From<(f32, f32, f32, f32)>`, `From<[f32; 4]>`,
`From<(i32, i32, u32, u32)>`, `From<Rect> for sdl3::render::FRect`.
`Default` — нулевой прямоугольник.

## `Mat3`

Файл `src/math/mat3.rs`. Матрица 3×3 для 2D-аффинных преобразований.

```rust
pub struct Mat3 {
    pub rows: [[f32; 3]; 3],
}
```

Хранится построчно. Последняя строка обычно `[0, 0, 1]`.

| Элемент | Смысл |
|---|---|
| `Mat3::IDENTITY` | Не меняет точку |
| `Mat3::translation(v)` | Перенос на `v` |
| `Mat3::rotation(radians)` | Поворот вокруг начала координат |
| `Mat3::scaling(v)` | Масштаб по осям |
| `transform_point(v)` | Применяет матрицу к 2D-точке |
| `determinant()` | Определитель 3×3 |
| `Mul` | Композиция матриц |
| `Default` | `IDENTITY` |

`transform_point` использует только первые две строки:

```text
x' = m00*x + m01*y + m02
y' = m10*x + m11*y + m12
```

Порядок композиции: `a * b` означает «сначала `b`, потом `a`». Пример
иерархии из `examples/math/matrix.rs`:

```rust
let planet = Mat3::translation(center)
    * Mat3::rotation(planet_angle)
    * Mat3::translation(Vec2::new(radius, 0.0));

let moon = planet
    * Mat3::rotation(moon_angle)
    * Mat3::translation(Vec2::new(moon_radius, 0.0));
```

`planet` задаёт систему координат планеты, `moon` — вложенную систему
спутника.

## `Ease` и `Lerp`

Файлы `src/math/ease.rs` и `src/math/mod.rs`.

```rust
pub enum Ease {
    Linear, // пока единственный вариант
}
```

`Ease::apply(t)` пропускает вес через кривую. Сейчас `Linear` возвращает `t`
без изменений, но точка расширения уже есть: добавление `In`, `Out` и т.п. не
изменит сигнатуры.

```rust
pub trait Lerp: Sized {
    fn lerp(self, other: Self, t: f32, ease: Ease) -> Self;
}

pub fn lerp<T: Lerp>(from: T, to: T, t: f32, ease: Ease) -> T {
    from.lerp(to, t, ease)
}
```

Реализации:

| Тип | Поведение |
|---|---|
| `f32` | `self + (other - self) * ease(t)`, вес не зажимается |
| `Vec2` | То же по каждой координате |
| `Color` | По каждому каналу, округление, зажим в `0..=255` |

Вес `t` намеренно не ограничен `0..=1`: `2.0` уводит значение за цель, `-1.0`
— до цели и дальше назад, как в Godot. Исключение — `Color`, где `u8` не может
хранить значение вне диапазона.

Два способа вызова эквивалентны:

```rust
use frglib::{lerp, Ease, Lerp, Vec2};

let from = Vec2::new(10.0, 20.0);
let to = Vec2::new(30.0, 60.0);

let a = lerp(from, to, 0.5, Ease::Linear);
let b = from.lerp(to, 0.5, Ease::Linear); // нужен `use frglib::Lerp`
```

## Где это используется

| Тип | Кто использует |
|---|---|
| `Vec2` | `Object`, `Camera`, `TileMap`, события, весь игровой код |
| `Rect` | `Object::rect`, `Camera::project_rect`, `Button`, `Slider`, `Label`, AABB-коллизии |
| `Mat3` | Иерархические трансформации в примерах; в самом рендере не участвует |
| `Ease`/`Lerp` | Интерполяция позиции, размера, цвета; примеры `math-lerp` |
| `sin`/`cos` | Круговое движение, орбиты, волны |
