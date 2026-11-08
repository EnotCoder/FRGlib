//! 3D-куб и 2D-интерфейс с текстом поверх него на `wgpu`.
//!
//! Запуск: `cargo run --example wgpu-hud --features wgpu`
//!
//! В одном кадре работают оба рендерера: [`Renderer3D`] рисует сцену, затем
//! [`Renderer2D`] кладёт поверх неё панель, текст, кнопку, слайдер и
//! тайлмап. Шрифт разбирает `fontdue` на стороне библиотеки, поэтому SDL3_ttf
//! примеру не нужен.
//!
//! Стрелки не нужны: камера облетает куб сама. Escape закрывает окно.

use std::error::Error;
use std::path::PathBuf;

use frglib::gpu::glam::{Mat4, Vec3};
use frglib::gpu::{Camera3D, Renderer2D, Renderer3D, WgpuWindow};
use frglib::{
    Button, Camera, Color, Event, FrameClock, Image, Keycode, Rect, Slider, TileMap, Vec2,
};

/// Шрифт из примеров интерфейса: обычный OTF, который `fontdue` читает сам.
const FONT: &[u8] = include_bytes!("../ui/assets/font.otf");

fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/2d_games/assets/space/planets")
        .join(name)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut window = WgpuWindow::builder("FRGLib wgpu — 2D и текст")
        .size(1000, 690)
        .background(Color::rgb(16, 18, 26))
        .build()?;

    let info = window.gpu().adapter_info();
    println!("{} ({:?} {})", info.name, info.backend, info.driver);

    let mut scene = Renderer3D::new(window.gpu());
    let mut ui = Renderer2D::new(window.gpu());
    let font = ui.load_font(FONT.to_vec())?;

    // Две текстуры для тайлмапа: любые PNG, которые умеет читать SDL.
    let tiles = vec![
        ui.create_texture(&Image::from_file(asset("neptune.png"))?)?,
        ui.create_texture(&Image::from_file(asset("mars.png"))?)?,
    ];

    let map = TileMap::from_rows(
        vec![
            vec![0usize, 1, 0, 1, 0],
            vec![1, 0, 1, 0, 1],
            vec![0, 1, 0, 1, 0],
        ],
        44.0,
    )?
    .origin(650.0, 430.0);

    let panel = Rect::new(24.0, 24.0, 430.0, 286.0);
    let mut button = Button::new(Rect::new(44.0, 240.0, 170.0, 44.0));
    let mut slider = Slider::new(Rect::new(44.0, 186.0, 360.0, 24.0))
        .with_range(0.0, 1.0, 0.05)
        .with_colors(
            Color::rgb(38, 42, 54),
            Color::rgb(92, 146, 210),
            Color::rgb(226, 232, 244),
            Color::rgb(232, 168, 92),
        );
    slider.set_value(0.5);

    let mut clock = FrameClock::new(60.0);
    let mut angle = 0.0f32;
    let mut fps = 60.0f32;
    let mut clicks = 0u32;

    loop {
        let events = window.poll()?;

        for event in &events {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::KeyDown {
                    key: Keycode::Escape,
                    ..
                } => return Ok(()),
                _ => {}
            }
        }

        if button.update(&events) {
            clicks += 1;
        }
        slider.update(&events);

        let dt = clock.dt();
        if dt > 0.0 {
            fps = fps * 0.9 + (1.0 / dt) * 0.1;
        }

        // Слайдер задаёт скорость вращения, а не сам угол: иначе куб прыгал
        // бы каждый раз, когда ручку двигают.
        angle += dt * (0.35 + slider.fraction() * 1.8);

        let Some(mut frame) = window.begin_frame()? else {
            clock.tick();
            continue;
        };

        // --- 3D ---------------------------------------------------------
        let camera = Camera3D::orbit(Vec3::new(0.0, 0.9, 0.0), 6.5, 0.7 + angle * 0.15, 0.34);

        scene.begin(&camera);
        scene.draw_grid(20, 1.0, Color::rgb(58, 68, 84));

        let cube = Mat4::from_translation(Vec3::new(0.0, 1.0, 0.0))
            * Mat4::from_rotation_y(angle)
            * Mat4::from_scale(Vec3::splat(1.5));
        scene.draw_cube_model(cube, Color::rgb(146, 197, 141));

        scene.finish(&mut frame);

        // --- 2D поверх 3D ----------------------------------------------
        let (width, height) = frame.size;

        ui.begin_overlay(&frame);

        ui.fill_rect(panel, Color::rgba(12, 14, 22, 210));
        ui.draw_rect(panel, 1.0, Color::rgba(120, 140, 180, 120));

        ui.draw_text(font, "FRGLib + wgpu", 44.0, 42.0, 30.0, Color::WHITE)?;
        ui.draw_text(
            font,
            "3D-сцена, 2D-панель, текст и тайлмап",
            44.0,
            84.0,
            17.0,
            Color::rgb(164, 176, 198),
        )?;

        let fps_line = format!("{fps:.0} fps");
        ui.draw_text(
            font,
            &fps_line,
            44.0,
            112.0,
            16.0,
            Color::rgb(120, 200, 140),
        )?;

        let value = format!("яркость {:.2}", slider.fraction());
        ui.draw_text(font, &value, 44.0, 156.0, 18.0, Color::WHITE)?;

        slider.draw_gpu(&mut ui);

        ui.draw_text(
            font,
            &format!("нажатий: {clicks}"),
            236.0,
            252.0,
            18.0,
            Color::rgb(164, 176, 198),
        )?;
        button.draw_gpu(&mut ui, font, "Нажми меня", 20.0)?;

        // Тайлмап рисуется в мировых координатах. Камера, смотрящая в центр
        // экрана без зума, делает один мир = один пиксель, поэтому карта
        // встаёт ровно туда, куда её поставил `origin`.
        ui.draw_text(font, "TileMap на GPU", 650.0, 382.0, 22.0, Color::WHITE)?;
        let flat_camera = Camera::new(Vec2::new(width as f32 / 2.0, height as f32 / 2.0));
        map.draw_gpu(&mut ui, flat_camera, |tile| tiles.get(*tile).cloned());

        ui.finish(&mut frame);

        window.present(frame)?;
        clock.tick();
    }
}
