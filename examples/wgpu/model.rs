//! Демонстрация загрузки OBJ-модели и наложения текстуры в 3D.
//!
//! Запуск: `cargo run --example wgpu-model --features wgpu`
//!
//! Мы загружаем модель куба из файла, применяем к ней текстуру логотипа
//! FRGLib и вращаем её.

use frglib::gpu::glam::{Mat4, Vec3};
use frglib::gpu::{Camera3D, Mesh, Renderer3D, Texture, WgpuWindow, Material};
use frglib::{Color, Event, FrameClock, Image};

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let mut window = WgpuWindow::builder("FRGLib 3D Model")
        .size(800, 600)
        .background(Color::rgb(20, 22, 30))
        .build()?;

    let mut renderer = Renderer3D::new(window.gpu());
    
    // Загружаем меш из OBJ-файла
    let mesh = Mesh::from_obj_file("examples/wgpu/assets/box.obj")?;
    let gpu_mesh = renderer.create_mesh(&mesh);
    
    // Загружаем текстуру логотипа
    let logo_image = Image::from_file("examples/wgpu/assets/box.png")?;
    let texture = Texture::from_image(window.gpu(), &logo_image)?;

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

        // Камера облетает центр. OBJ куб у нас от 0 до 1, так что центр в 0.5
        let camera = Camera3D::orbit(Vec3::new(0.0, 0.5, 0.0), 3.0, 0.0, 0.5);
        
        renderer.begin(&camera);
        
        // Рисуем нашу модель с текстурой
        let transform = Mat4::from_rotation_y(angle * 0.5);
        // renderer.draw_mesh_textured(&gpu_mesh, &texture, transform, Color::WHITE);

        let my_material = Material::Unlit;
        renderer.draw_mesh_textured_with(&gpu_mesh, &texture, transform, my_material, Color::WHITE);
        
        renderer.finish(&mut frame);
        window.present(frame)?;
        
        angle += 0.02;
        clock.tick();
    }
}
