//! Интерполяция: квадрат ездит между двумя точками и меняет цвет.
//!
//! Позиция, размер и цвет считаются одной и той же функцией `lerp` с одним и
//! тем весом `t`, поэтому всё движется согласованно. Четвёртый аргумент -- тип
//! интерполяции; пока это `Ease::Linear`, который ведёт равномерно, без
//! ускорений и замедлений.
//!
//! Вес специально не зажимается: здесь он ходит в `0..=1`, но значение за
//! пределами этого диапазона просто продолжило бы путь за точку `to` -- как в
//! Godot.
//!
//! Управление: пробел -- пауза, Escape -- выход.
//!
//! Запуск: `cargo run --example basic-lerp`

use frglib::{lerp, Color, Ease, Event, FrameClock, Keycode, Lerp, Object, Vec2, Window};

/// Секунд на полный проход «туда-обратно».
const PERIOD: f32 = 3.0;

fn main() -> Result<(), frglib::Error> {
    let mut window = Window::builder("FRGLib lerp")
        .size(800, 600)
        .background(Color::FON)
        .build()?;

    let mut paused = false;
    let mut elapsed = 0.0f32;
    let mut clock = FrameClock::new(60.0);

    loop {
        // Реальное время кадра: скорость не должна зависеть от того, сколько
        // кадров в секунду тянет машина.
        let dt = clock.dt();

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
                } => paused = !paused,
                _ => {}
            }
        }

        // Пауза -- это «время не двигается». Сама интерполяция ничего про паузу
        // не знает: ей всё равно, что такое секунды, ей нужен только вес.
        if !paused {
            elapsed += dt;
        }

        // Треугольная волна: 0 в начале, 1 на середине периода, снова 0 в
        // конце -- квадрат доезжает до конца и возвращается тем же ходом.
        let phase = (elapsed % PERIOD) / PERIOD;
        let t = 1.0 - (2.0 * phase - 1.0).abs();

        // Концы пути берём от размера окна, а не из констант: при ресайзе точка
        // `to` не должна уезжать за край.
        let (w, h) = window.size();
        let (w, h) = (w as f32, h as f32);
        let from = Vec2::new(w * 0.2, h / 2.0);
        let to = Vec2::new(w * 0.8, h / 2.0);

        // Три вызова одного `lerp` с одним весом: позиция и размер -- через
        // свободную функцию, цвет -- методом, которому нужен trait `Lerp` в
        // области видимости. Оба способа -- один и тот же вызов.
        let position = lerp(from, to, t, Ease::Linear);
        let size = lerp(
            Vec2::new(50.0, 50.0),
            Vec2::new(130.0, 130.0),
            t,
            Ease::Linear,
        );
        let color = Color::rgb(220, 80, 60).lerp(Color::rgb(60, 130, 220), t, Ease::Linear);

        // Трасса и стойки на её концах. Стойка выше квадрата, поэтому видна,
        // даже когда тот доехал до конца и накрыл её середину.
        let track = Object::from_xywh((from.x + to.x) / 2.0, from.y, to.x - from.x, 4.0);
        let stand = Object::from_xywh(from.x, from.y, 4.0, 220.0);
        let stands = [
            stand,
            Object::from_xywh(to.x, to.y, stand.size.x, stand.size.y),
        ];
        let runner = Object::new(position, size);

        window.frame_with(|d| {
            d.fill_object(&track, Color::rgb(70, 75, 85))?;
            for stand in stands {
                d.fill_object(&stand, Color::rgb(95, 100, 110))?;
            }
            d.fill_object(&runner, color)?;
            Ok(())
        })?;

        clock.tick();
    }
}
