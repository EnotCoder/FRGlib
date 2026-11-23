//! 3D-пример: ландшафт, сгенерированный шумом Перлина.
//!
//! Запуск: `cargo run --features wgpu --example 3d_games-terrain`
//!
//! Кнопка «Перегенерировать» (или R) создаёт новый рельеф с новым seed.
//! Камеру можно вести в любую сторону: WASD — полёт, Space/C — вверх и вниз,
//! зажатая мышь или стрелки — обзор, колесо — скорость, Shift — ускорение,
//! Escape — выход. Камера не проваливается под поверхность.
//!
//! Отладочные опции движка: F1 переключает `set_show_collision` (сфера камеры
//! над рельефом и границы карты), F2 — `set_wireframe` (каркас моделей).
//!
//! Меш строится из сетки вершин: высота — сумма октав [`frglib::Perlin`],
//! цвет зависит от высоты и крутизны склона. По поверхности случайно
//! рассеиваются простые ёлки. Небо и вода тоже процедурные, поэтому примеру
//! не нужны внешние ассеты.

mod camera;
mod hud;
mod props;
mod sky;
mod terrain;

use std::time::{SystemTime, UNIX_EPOCH};

use frglib::gpu::glam::{Mat4, Vec3};
use frglib::gpu::{Light, Material, Mesh, Renderer2D, Renderer3D, WgpuWindow};
use frglib::{Button, Color, Event, FrameClock, Keycode, Rect};

use camera::{FlyCamera, HALF_SIZE, MIN_HEIGHT_ABOVE_GROUND};
use props::Props;
use terrain::{Terrain, SIZE};

/// Шрифт HUD из общих ассетов примеров 3D-игр.
const FONT: &[u8] = include_bytes!("../assets/font.ttf");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = WgpuWindow::builder("FRGLib 3D — ландшафт Перлина")
        .size(1100, 720)
        .background(Color::rgb(18, 28, 44))
        .build()?;

    println!("{}", window.gpu().adapter_info().name);

    let mut scene = Renderer3D::new(window.gpu());
    let mut ui = Renderer2D::new(window.gpu());
    let font = ui.load_font(FONT.to_vec())?;

    scene.set_skybox(sky::create(window.gpu())?);
    // Тёплое солнце с той стороны, где оно нарисовано на панораме, и холодная
    // подсветка с другой: рельеф читается даже в тенях.
    scene.set_lights([
        Light::directional(
            Vec3::new(-0.55, -0.28, 0.78),
            Color::rgba(255, 238, 210, 255),
        ),
        Light::directional(
            Vec3::new(0.45, -0.7, -0.5),
            Color::rgba(124, 162, 220, 78),
        ),
    ]);
    scene.set_ambient(Color::rgb(64, 78, 100));

    // Квадрат воды на уровне моря. Полупрозрачный материал рисуется после
    // ландшафта, поэтому подводные участки просвечивают сквозь неё.
    let water = scene.create_mesh(&Mesh::plane(SIZE, SIZE));
    let water_model = Mat4::from_translation(Vec3::new(0.0, 0.15, 0.0));

    let mut seed = 4242u64;
    let mut regenerations = 0u32;
    let mut terrain = Terrain::generate(&scene, seed);
    let mut props = Props::generate(&scene, &terrain, seed);

    let mut button = Button::new(Rect::new(24.0, 152.0, 250.0, 48.0)).with_colors(
        Color::rgb(46, 64, 94),
        Color::rgb(78, 118, 168),
        Color::rgb(156, 108, 60),
    );

    let mut camera = FlyCamera::new();
    let mut clock = FrameClock::new(60.0);

    loop {
        let events = window.poll()?;
        let mut regenerate = false;

        for event in &events {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::KeyDown {
                    key: Keycode::Escape,
                    ..
                } => return Ok(()),
                Event::KeyDown {
                    key: Keycode::R, ..
                } => regenerate = true,
                // Отладочные опции движка; удержание клавиши не щёлкает ими.
                Event::KeyDown {
                    key: Keycode::F1,
                    repeat: false,
                    ..
                } => scene.set_show_collision(!scene.show_collision()),
                Event::KeyDown {
                    key: Keycode::F2,
                    repeat: false,
                    ..
                } => scene.set_wireframe(!scene.wireframe()),
                _ => {}
            }

            // Клик по кнопке не должен одновременно поворачивать камеру.
            camera.handle(event, &button.rect());
        }

        if button.update(&events) {
            regenerate = true;
        }

        if regenerate {
            seed = fresh_seed(regenerations);
            regenerations += 1;
            terrain = Terrain::generate(&scene, seed);
            props = Props::generate(&scene, &terrain, seed);
        }

        let dt = clock.dt().min(0.05);
        let view = camera.update(dt, &terrain);

        let Some(mut frame) = window.begin_frame()? else {
            clock.tick();
            continue;
        };

        scene.begin(&view);
        scene.draw_mesh_with(&terrain.mesh, Mat4::IDENTITY, Material::Lit, Color::WHITE);
        props.draw(&mut scene);
        scene.draw_mesh_with(
            &water,
            water_model,
            Material::Transparent,
            Color::rgba(48, 128, 184, 145),
        );

        if scene.show_collision() {
            draw_collision_debug(&mut scene, &camera, &terrain);
        }
        scene.finish(&mut frame);

        ui.begin_overlay(&frame);
        hud::draw(
            &mut ui,
            font,
            &button,
            terrain.seed,
            regenerations,
            camera.position(),
            camera.speed(),
            frame.size,
        )?;
        ui.finish(&mut frame);

        window.present(frame)?;
        clock.tick();
    }
}

/// Отладочная опция движка `show_collision`: «коллизия» камеры — сфера
/// минимального зазора над рельефом, точка опоры под ней и границы карты.
fn draw_collision_debug(scene: &mut Renderer3D, camera: &FlyCamera, terrain: &Terrain) {
    let pos = camera.position();

    // Камера не проваливается под поверхность: кламп высоты — это сфера
    // радиуса MIN_HEIGHT_ABOVE_GROUND вокруг позиции камеры.
    scene.draw_sphere(pos, MIN_HEIGHT_ABOVE_GROUND, Color::rgb(90, 210, 255));

    // Точка рельефа прямо под камерой и отрезок до неё: видно, откуда
    // берётся ограничение высоты.
    let ground = Vec3::new(pos.x, terrain.height_at(pos.x, pos.z), pos.z);
    scene.draw_circle(ground + Vec3::Y * 0.05, 1.0, Color::rgb(255, 214, 116));
    scene.draw_line(ground, pos, Color::rgb(230, 235, 245));

    // Границы полёта по X/Z, на текущей высоте камеры.
    let y = pos.y;
    scene.draw_lines(
        &[
            Vec3::new(-HALF_SIZE, y, -HALF_SIZE),
            Vec3::new(HALF_SIZE, y, -HALF_SIZE),
            Vec3::new(HALF_SIZE, y, HALF_SIZE),
            Vec3::new(-HALF_SIZE, y, HALF_SIZE),
            Vec3::new(-HALF_SIZE, y, -HALF_SIZE),
        ],
        Color::rgb(230, 235, 245),
    );
}

/// Seed для следующей перегенерации: время плюс счётчик, чтобы два нажатия
/// подряд не дали один и тот же ландшафт.
fn fresh_seed(regenerations: u32) -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);
    nanos ^ (regenerations as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
}
