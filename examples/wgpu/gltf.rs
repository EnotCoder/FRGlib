//! Демонстрация загрузки glTF-модели в 3D.
//!
//! Запуск: `cargo run --example wgpu-gltf --features wgpu`
//!
//! Загружаем куб из `box.gltf`: геометрию, материал и встроенную текстуру
//! берём прямо из файла. Модель вращается вокруг своей оси, а освещают её
//! три источника разного цвета.

use frglib::gpu::glam::{Mat4, Vec3};
use frglib::gpu::{Camera3D, GltfModel, Light, Renderer3D, WgpuWindow};
use frglib::{Color, Event, FrameClock};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = WgpuWindow::builder("FRGLib glTF")
        .size(800, 600)
        .background(Color::rgb(20, 22, 30))
        .build()?;

    let mut renderer = Renderer3D::new(window.gpu());

    // .gltf и .glb читаются одинаково; внешние буферы и картинки
    // подтягиваются относительно самого файла.
    let model = GltfModel::from_file("examples/wgpu/assets/box.gltf")?;
    let gpu_model = renderer.create_gltf(&model)?;

    // Три источника: тёплый и холодный направленные плюс красная точка.
    // `set_lights` заменяет встроенный свет по умолчанию.
    renderer.set_lights([
        Light::directional(Vec3::new(-0.5, -1.0, -0.3), Color::rgb(255, 220, 160))
            .with_intensity(1.1),
        Light::directional(Vec3::new(0.8, -0.4, -0.6), Color::rgb(120, 170, 255))
            .with_intensity(0.7),
        Light::point(Vec3::new(0.0, 1.2, 2.0), Color::rgb(255, 70, 70))
            .with_range(5.0)
            .with_intensity(0.8),
    ]);
    renderer.set_ambient(Color::rgb(30, 30, 40)); // Слегка синеватый эмбиент

    let mut clock = FrameClock::new(60.0);
    let mut angle = 0.0f32;

    loop {
        for event in window.poll()? {
            if matches!(event, Event::Quit | Event::CloseRequested) {
                return Ok(());
            }
        }

        let Some(mut frame) = window.begin_frame()? else {
            continue;
        };

        let camera = Camera3D::orbit(Vec3::ZERO, 3.0, 0.0, 0.5);

        renderer.begin(&camera);
        renderer.draw_gltf(&gpu_model, Mat4::from_rotation_y(angle));
        renderer.finish(&mut frame);
        window.present(frame)?;

        angle += 0.02;
        clock.tick();
    }
}
