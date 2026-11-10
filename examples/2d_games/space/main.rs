use frglib::{Camera, Color, Event, FrameClock, Image, Keycode, MouseButton, Object, Vec2, Window};
use std::path::PathBuf;

mod bodies;
mod draw;

use bodies::*;
use draw::*;

/// Путь к картинке из общей папки ассетов.
fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/2d_games/assets/space")
        .join(name)
}

fn main() -> Result<(), frglib::Error> {
    // textures
    let sun_image = Image::from_file(asset("sun.png"))?;

    let mercury_image = Image::from_file(asset("planets/mercury.png"))?;
    let vinera_image = Image::from_file(asset("planets/vinera.png"))?;
    let earth_image = Image::from_file(asset("planets/earth.png"))?;
    let mars_image = Image::from_file(asset("planets/mars.png"))?;

    let jupiter_image = Image::from_file(asset("planets/jupiter.png"))?;
    let saturn_image = Image::from_file(asset("planets/saturn.png"))?;
    let uran_image = Image::from_file(asset("planets/uran.png"))?;
    let neptune_image = Image::from_file(asset("planets/neptune.png"))?;

    let moon_textures = [
        "moons/moon.png",
        "moons/thomas_deimos.png",
        "moons/io.png",
        "moons/callisto.png",
        "moons/ganymede.png",
        "moons/titan.png",
        "moons/rhea.png",
        "moons/titania.png",
        "moons/triton.png",
    ]
    .iter()
    .map(|name| Image::from_file(asset(name)).ok())
    .collect::<Vec<_>>();

    // create window
    let mut window = Window::builder("FRGlib: Space")
        .size(1200, 900)
        .pixel_art(true)
        .background(Color::rgb(20, 20, 25))
        .build()?;

    let mut clock = FrameClock::new(60.0);
    let mut elapsed = 0.0f32;

    // Камера смотрит в начало координат -- там Солнце.
    // Колесо мыши: вертикально -- зум к курсору,
    // горизонтально -- движение вбок.
    // Средняя кнопка мыши -- перетаскивание сцены.
    let mut camera = Camera::new(Vec2::ZERO);
    let mut cursor: Option<Vec2> = None;
    let mut last_cursor: Option<Vec2> = None;
    let mut dragging = false;

    let mut camera_velocity = Vec2::ZERO;
    let move_speed = 1000.0;
    let friction = 10.0;

    let mut up_pressed = false;
    let mut down_pressed = false;
    let mut left_pressed = false;
    let mut right_pressed = false;

    let orbit_thickness = 2.0;
    let orbit_color = Color::rgb(80, 80, 85);

    let planets = get_planets(
        mercury_image,
        vinera_image,
        earth_image,
        mars_image,
        jupiter_image,
        saturn_image,
        uran_image,
        neptune_image,
        &moon_textures,
    );

    // Пояса создаём один раз: между Марсом и Юпитером
    // и сразу за Нептуном.
    let asteroid_belt = get_asteroid_belt();
    let kuiper_belt = get_kuiper_belt();

    let cloud_orte = get_orte_belt();

    // Создаём звёзды один раз.
    let stars = get_stars();

    loop {
        let dt = clock.dt();
        elapsed += dt;

        // Определяем желаемое направление движения на основе зажатых клавиш
        let mut move_dir = Vec2::ZERO;
        if up_pressed {
            move_dir.y -= 1.0;
        }
        if down_pressed {
            move_dir.y += 1.0;
        }
        if left_pressed {
            move_dir.x -= 1.0;
        }
        if right_pressed {
            move_dir.x += 1.0;
        }

        // Если есть ввод, ускоряем камеру, иначе замедляем
        if move_dir.length_squared() > 0.0 {
            // Нормализуем, чтобы по диагонали не двигаться быстрее
            let target_vel = move_dir.normalized() * move_speed;
            // Плавное приближение к целевой скорости (lerp-like)
            camera_velocity = camera_velocity + (target_vel - camera_velocity) * (friction * dt);
        } else {
            // Плавное торможение
            camera_velocity = camera_velocity * (1.0 - friction * dt).max(0.0);
        }

        camera.position += camera_velocity * (dt / camera.zoom);

        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),

                Event::KeyDown { key, .. } => match key {
                    Keycode::Escape => return Ok(()),
                    Keycode::Up => up_pressed = true,
                    Keycode::Down => down_pressed = true,
                    Keycode::Left => left_pressed = true,
                    Keycode::Right => right_pressed = true,
                    _ => {}
                },

                Event::KeyUp { key, .. } => match key {
                    Keycode::Up => up_pressed = false,
                    Keycode::Down => down_pressed = false,
                    Keycode::Left => left_pressed = false,
                    Keycode::Right => right_pressed = false,
                    _ => {}
                },

                Event::MouseMotion { x, y } => {
                    let position = Vec2::new(x, y);

                    if dragging {
                        if let Some(last) = last_cursor {
                            // Тянем сцену за собой, поэтому камера
                            // едет в обратную сторону.
                            camera.position = camera.position - (position - last) / camera.zoom;
                        }
                    }

                    last_cursor = Some(position);
                    cursor = Some(position);
                }

                Event::MouseButtonDown {
                    button: MouseButton::Middle,
                    x,
                    y,
                    ..
                } => {
                    dragging = true;
                    last_cursor = Some(Vec2::new(x, y));
                }

                Event::MouseButtonUp {
                    button: MouseButton::Middle,
                    ..
                } => {
                    dragging = false;
                    last_cursor = None;
                }

                Event::MouseWheel { x, y, .. } => {
                    let size = window.size();
                    let mouse = cursor
                        .unwrap_or_else(|| Vec2::new(size.0 as f32 / 2.0, size.1 as f32 / 2.0));

                    // Вертикальный скролл -- зум к положению курсора,
                    // чтобы точка под мышью оставалась на месте.
                    if y != 0.0 {
                        let before = camera.to_world(mouse, size);
                        camera.zoom = (camera.zoom * (1.0 + y * 0.1)).clamp(0.001, 10.0);
                        let after = camera.to_world(mouse, size);
                        camera.position += before - after;
                    }

                    // Горизонтальный скролл двигает камеру вбок.
                    if x != 0.0 {
                        camera.position.x += x * 60.0 / camera.zoom;
                    }
                }

                _ => {}
            }
        }

        let (w, h) = window.size();
        let window_size = Vec2::new(w as f32, h as f32);

        // Солнце и все орбиты живут в начале координат.
        let center = Vec2::ZERO;

        window.frame_with(|d| {
            d.set_camera(camera);

            // Звёзды рисуем в экранных координатах:
            // они не должны уезжать вместе с камерой.
            let _ = d.screen_space(|d| {
                let _ = draw_stars(d, &stars, &window_size);
                Ok(())
            });

            // Орбиты планет.
            let _ = draw_orbit(d, &planets, center, orbit_thickness, orbit_color);

            // Пояса астероидов и Койпера.
            let _ = draw_belt(d, &asteroid_belt, elapsed, center);
            let _ = draw_belt(d, &kuiper_belt, elapsed, center);
            let _ = draw_cloud(d, &cloud_orte, elapsed, center);

            // Солнце.
            let sun = Object::new(center, Vec2::new(120.0, 120.0))
                .with_rotation(elapsed * 20.0);

            d.fill_textured_object(&sun, &sun_image)?;

            // Планеты

            let _ = draw_bodies(d, &planets, elapsed, center, orbit_thickness, orbit_color);

            Ok(())
        })?;

        clock.tick();
    }
}
