//! Абсолютная мышь против относительной.
//!
//! Два маркера: тёмный едет за координатами из `Event::MouseMotion`, светлый
//! накапливает `window.relative_mouse()` в свою позицию. Пока курсор в центре
//! они рядом; у края окна расходятся -- и остаются разъехавшимися, пока не
//! вернуться в центр. В этом и разница: абсолютная мышь говорит, где курсор,
//! относительная -- насколько он сдвинулся.
//!
//! Дельта печатается в консоль раз в кадр. Обратите внимание, что у края окна
//! она нулевая: мышь упирается и перестаёт двигаться.
//!
//! Управление: движение мышью, Escape -- выход.
//!
//! Запуск: `cargo run --example basic-relative`

use frglib::{Color, Event, FrameClock, Object, Vec2, Window};

fn main() -> Result<(), frglib::Error> {
    let mut window = Window::builder("FRGLib relative mouse")
        .size(800, 600)
        .background(Color::FON)
        .build()?;

    // Две независимые позиции. Абсолютная переписывается каждый кадр, потому
    // что это координата; относительная только накапливается, потому что это
    // приращение -- заменить ею позицию значит выбросить всё, что было.
    let mut absolute = Vec2::new(400.0, 300.0);
    let mut relative = Vec2::new(400.0, 300.0);

    let mut clock = FrameClock::new(60.0);

    loop {
        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::MouseMotion { x, y } => absolute = Vec2::new(x, y),
                _ => {}
            }
        }

        // Сдвиг за кадр, накопленный в позицию. Читается ровно один раз:
        // повторный вызов в том же кадре дал бы ноль -- SDL отдаёт дельту и
        // тут же обнуляет счётчик, как и `FrameClock::dt`.
        let step = window.relative_mouse()?;
        relative += step;

        // Ненулевая дельта печатается, нулевая -- нет: движение видно по
        // строкам, а у края окна наступает тишина. Это и есть тот самый ноль.
        if step != Vec2::ZERO {
            println!("дельта: {step:?}");
        }

        let (w, h) = window.size();
        let (w, h) = (w as f32, h as f32);
        relative.x = relative.x.clamp(20.0, w - 20.0);
        relative.y = relative.y.clamp(20.0, h - 20.0);

        let dark = Object::from_xywh(absolute.x, absolute.y, 40.0, 40.0);
        let light = Object::from_xywh(relative.x, relative.y, 40.0, 40.0);

        window.frame_with(|d| {
            d.fill_object(&dark, Color::rgb(90, 95, 110))?;
            d.fill_object(&light, Color::rgb(235, 240, 250))?;
            Ok(())
        })?;

        clock.tick();
    }
}
