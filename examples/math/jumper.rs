//! Пружины: волна бежит по цепочке грузиков.
//!
//! Каждый грузик связан пружиной с соседями, поэтому толчок на одном конце
//! передаётся дальше и отражается от свободного края. У грузика есть только
//! вертикальное смещение: горизонтальные расстояния считаются постоянными, а
//! сама волна -- достаточно мелкой, чтобы этим можно было пренебречь.
//!
//! Время физики фиксированное (`1/240` с), а `dt` кадра только накапливается.
//! С шагом, равным кадру, пружина при разном FPS вела бы себя по-разному, а на
//! большом `dt` явный метод Эйлера просто разошёлся бы.
//!
//! Управление: пробел -- толчок, клик мышью -- толкнуть ближайший грузик,
//! R -- сброс, Escape -- выход.
//!
//! Запуск: `cargo run --example math-jumper`

use frglib::{lerp, Color, Ease, Event, FrameClock, Keycode, MouseButton, Object, Vec2, Window};

/// Сколько грузиков в цепочке.
const NODES: usize = 30;

/// Жёсткость пружин между соседями.
const STIFFNESS: f32 = 260.0;

/// Затухание: скорость, теряемая за секунду.
const DAMPING: f32 = 0.35;

/// Слабое притяжение к линии равновесия, чтобы цепочка не уезжала.
const ANCHOR: f32 = 0.8;

/// Шаг физики в секундах.
const STEP: f32 = 1.0 / 240.0;

/// Импульс толчка, пикселей в секунду.
const KICK: f32 = 600.0;

/// Пауза между автоматическими толчками, секунды.
const AUTO_KICK: f32 = 2.4;

/// Смещение, при котором грузик считается «раскалённым».
const HOT_OFFSET: f32 = 100.0;

/// Цвет грузика в покое.
const CALM: Color = Color::rgb(90, 150, 230);

/// Цвет грузика на пике волны.
const HOT: Color = Color::rgb(255, 120, 60);

/// Грузик цепочки: смещение от линии равновесия и его скорость.
///
/// Положительное смещение -- вниз, как и ось `y` окна.
#[derive(Debug, Clone, Copy, Default)]
struct Node {
    offset: f32,
    velocity: f32,
}

/// Цепочка пружин.
struct Chain {
    nodes: [Node; NODES],
    /// Время, ещё не отданное физике.
    accumulator: f32,
    /// Сколько осталось до следующего автоматического толчка.
    until_kick: f32,
    /// С какого конца толкать в следующий раз.
    from_left: bool,
}

impl Chain {
    fn new() -> Self {
        Chain {
            nodes: [Node::default(); NODES],
            accumulator: 0.0,
            until_kick: AUTO_KICK,
            from_left: true,
        }
    }

    fn reset(&mut self) {
        *self = Chain::new();
    }

    /// Продвигает физику на `dt` фиксированными подшагами.
    fn update(&mut self, dt: f32) {
        // Верхняя граница: после паузы окна не надо догонять секунды физики
        // за один кадр -- это выглядело бы как взрыв.
        self.accumulator = (self.accumulator + dt).min(0.1);

        while self.accumulator >= STEP {
            self.step();
            self.accumulator -= STEP;
        }

        self.until_kick -= dt;
        if self.until_kick <= 0.0 {
            self.kick(self.from_left);
            self.from_left = !self.from_left;
            self.until_kick = AUTO_KICK;
        }
    }

    /// Один шаг полунеявного Эйлера.
    ///
    /// Сначала считаются все ускорения по старым смещениям, потом обновляются
    /// все скорости и позиции. Иначе сосед, обновлённый первым, успевал бы
    /// повлиять на второго внутри одного шага.
    fn step(&mut self) {
        let mut acceleration = [0.0f32; NODES];

        for i in 0..NODES {
            // У свободного конца соседа нет. Подставляем собственное смещение:
            // разница с ним нулевая, и край ни за что не тянет.
            let left = if i > 0 {
                self.nodes[i - 1].offset
            } else {
                self.nodes[i].offset
            };
            let right = if i + 1 < NODES {
                self.nodes[i + 1].offset
            } else {
                self.nodes[i].offset
            };

            acceleration[i] = STIFFNESS * (left + right - 2.0 * self.nodes[i].offset)
                - DAMPING * self.nodes[i].velocity
                - ANCHOR * self.nodes[i].offset;
        }

        for (node, acceleration) in self.nodes.iter_mut().zip(acceleration) {
            node.velocity += acceleration * STEP;
            node.offset += node.velocity * STEP;
        }
    }

    /// Толчок мягкой «шапкой» на нескольких узлах с одного конца.
    ///
    /// Одиночный узел завёл бы высокочастотную рябь: волна получилась бы
    /// дёрганой, а не плавной.
    fn kick(&mut self, from_left: bool) {
        for k in 0..6 {
            let index = if from_left { k } else { NODES - 1 - k };
            let weight = (-(k as f32) * 0.8).exp();
            self.nodes[index].velocity -= KICK * weight;
        }
    }

    /// Толкает окрестность узла `index` в направлении `direction`.
    fn poke(&mut self, index: usize, direction: f32) {
        for offset in -2i32..=2 {
            let neighbour = index as i32 + offset;
            if !(0..NODES as i32).contains(&neighbour) {
                continue;
            }

            // Ближние узлы получают больше, дальние -- меньше.
            let weight = 1.0 - offset.abs() as f32 * 0.25;
            self.nodes[neighbour as usize].velocity += direction * KICK * 0.8 * weight;
        }
    }
}

fn main() -> Result<(), frglib::Error> {
    let mut window = Window::builder("FRGLib spring wave")
        .size(900, 520)
        .background(Color::FON)
        .build()?;

    let mut clock = FrameClock::new(60.0);
    let mut chain = Chain::new();
    chain.kick(true);

    loop {
        let dt = clock.dt();

        let (width, height) = window.size();
        let (width, height) = (width as f32, height as f32);

        // Раскладка считается от размера окна: после ресайза цепочка остаётся
        // на экране, а не уезжает за край.
        let margin = 60.0;
        let step = (width - 2.0 * margin) / (NODES - 1) as f32;
        let base_y = height * 0.5;
        let node_x = |i: usize| margin + i as f32 * step;

        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::KeyDown {
                    key: Keycode::Escape,
                    ..
                } => return Ok(()),
                Event::KeyDown {
                    key: Keycode::Space,
                    ..
                } => {
                    chain.kick(true);
                    chain.until_kick = AUTO_KICK;
                }
                Event::KeyDown { key: Keycode::R, .. } => chain.reset(),
                Event::MouseButtonDown {
                    button: MouseButton::Left,
                    x,
                    y,
                    ..
                } => {
                    // Ближайший по горизонтали узел получает импульс в сторону
                    // от курсора: ткнул выше -- грузик пошёл вниз.
                    let index = ((x - margin) / step).round().clamp(0.0, (NODES - 1) as f32);
                    let index = index as usize;
                    let direction = (base_y + chain.nodes[index].offset - y).signum();
                    chain.poke(index, direction);
                }
                _ => {}
            }
        }

        chain.update(dt);

        window.frame_with(|d| {
            // Пружина рисуется зигзагом между соседними грузиками. Нормаль
            // берётся от самого отрезка, поэтому витки наклоняются вместе с
            // волной.
            let origin = Object::default();
            let coil_color = Color::rgb(85, 100, 120);
            let coils = 7;
            let amplitude = (step * 0.28).min(7.0);

            for i in 0..NODES - 1 {
                let a = Vec2::new(node_x(i), base_y + chain.nodes[i].offset);
                let b = Vec2::new(node_x(i + 1), base_y + chain.nodes[i + 1].offset);
                let along = b - a;
                let normal = Vec2::new(-along.y, along.x).normalized();

                let mut previous = a;
                for turn in 1..coils {
                    let t = turn as f32 / coils as f32;
                    let side = if turn % 2 == 0 { -1.0 } else { 1.0 };
                    let point = a + along * t + normal * (amplitude * side);

                    d.draw_object_polygon(
                        &origin,
                        &[(previous.x, previous.y), (point.x, point.y)],
                        coil_color,
                    )?;
                    previous = point;
                }
                d.draw_object_polygon(
                    &origin,
                    &[(previous.x, previous.y), (b.x, b.y)],
                    coil_color,
                )?;
            }

            // Сами грузики. Цвет -- интерполяция от спокойного к горячему по
            // величине смещения, поэтому гребни волны видно без легенды.
            let side = (step * 0.42).min(14.0);
            for (i, node) in chain.nodes.iter().enumerate() {
                let heat = (node.offset.abs() / HOT_OFFSET).clamp(0.0, 1.0);
                let color = lerp(CALM, HOT, heat, Ease::Linear);
                let block = Object::new(
                    Vec2::new(node_x(i), base_y + node.offset),
                    Vec2::new(side, side),
                );
                d.fill_object(&block, color)?;
            }

            Ok(())
        })?;

        clock.tick();
    }
}
