//! Демонстрация костной анимации glTF.
//!
//! Запуск: `cargo run --example wgpu-skinned --features wgpu`
//!
//! Маленькая лента из `skinned.glb` привязана к двум костям. В файле лежит
//! клип, который поворачивает верхнюю кость на 90°, и `GltfAnimationPlayer`
//! каждый кадр пересчитывает матрицы костей. Рендерер передаёт их в шейдер,
//! который деформирует вершины по `JOINTS_0` и `WEIGHTS_0`.

use frglib::gpu::glam::{Mat4, Vec3};
use frglib::gpu::{Camera3D, GltfAnimationPlayer, GltfModel, Renderer3D, WgpuWindow};
use frglib::{Color, Event, FrameClock};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = WgpuWindow::builder("FRGLib skinned glTF")
        .size(800, 600)
        .background(Color::rgb(20, 22, 30))
        .build()?;

    let mut renderer = Renderer3D::new(window.gpu());

    let model = GltfModel::from_file("examples/wgpu/assets/skinned.glb")?;
    let gpu_model = renderer.create_gltf(&model)?;
    // Нулевой клип выбран по умолчанию; `set_clip` и `set_speed` меняют
    // воспроизведение, а `set_time` позволяет перемотать анимацию вручную.
    let mut player = GltfAnimationPlayer::new(&model);

    renderer.set_lights([
        frglib::gpu::Light::directional(Vec3::new(-0.4, -1.0, -0.5), Color::rgb(255, 235, 200))
            .with_intensity(1.0),
        frglib::gpu::Light::directional(Vec3::new(0.6, -0.2, 0.7), Color::rgb(120, 170, 255))
            .with_intensity(0.5),
    ]);

    let mut clock = FrameClock::new(60.0);
    let mut angle = 0.0f32;

    loop {
        for event in window.poll()? {
            if matches!(event, Event::Quit | Event::CloseRequested) {
                return Ok(());
            }
        }

        // Ограничиваем шаг, чтобы после долгой паузы клип не прыгал вперёд.
        player.update(clock.dt().min(0.05));

        let Some(mut frame) = window.begin_frame()? else {
            clock.tick();
            continue;
        };

        let camera = Camera3D::orbit(Vec3::new(0.0, 0.8, 0.0), 4.0, angle, -0.15);
        angle += 0.004;

        renderer.begin(&camera);
        renderer.draw_grid(20, 1.0, Color::rgb(60, 70, 82));
        // Масштаб модели не трогает кости: анимированный mesh-узел в glTF
        // игнорируется, поэтому placement остаётся единичным.
        renderer.draw_gltf_animated(&gpu_model, &player, Mat4::IDENTITY);
        renderer.finish(&mut frame);
        window.present(frame)?;

        clock.tick();
    }
}
