use frglib::{
    Anchor, Camera, Color, Event, FrameClock, Image, Keycode, Label, MouseButton, Object,
    TextCache, Vec2, Window,
};
use std::path::PathBuf;

mod bodies;
mod draw;

use bodies::*;
use draw::*;

/// Сторона спрайта Солнца в мировых единицах.
const SUN_SIZE: f32 = 120.0;

/// То, что показываем в панели после клика по телу.
#[derive(Clone, Copy)]
struct BodyInfo {
    name: &'static str,
    kind: &'static str,
    radius: f32,
    orbit_radius: Option<f32>,
    position: Vec2,
}

impl BodyInfo {
    fn text(&self) -> String {
        let mut text = format!(
            "{}\nType: {}\nRadius: {:.1}",
            self.name, self.kind, self.radius
        );

        if let Some(orbit_radius) = self.orbit_radius {
            text.push_str(&format!("\nOrbit radius: {:.1}", orbit_radius));
        }

        text
    }
}

/// Цвет свечения тела: обычное, выбранное или подсвеченное курсором.
fn body_glow(
    name: &str,
    hovered: Option<&BodyInfo>,
    selected: Option<&BodyInfo>,
    elapsed: f32,
) -> Color {
    if hovered.is_some_and(|body| body.name == name) {
        let pulse = (elapsed * 6.0).sin() * 0.5 + 0.5;

        Color::rgba(185, 225, 255, (150.0 + pulse * 70.0) as u8)
    } else if selected.is_some_and(|body| body.name == name) {
        Color::rgba(155, 210, 255, 110)
    } else {
        Color::rgba(130, 175, 255, 40)
    }
}

/// Текущее положение тела по имени: выбранное тело движется вместе с орбитой.
fn body_position(planets: &[Planet], name: &str, elapsed: f32, center: Vec2) -> Option<Vec2> {
    if name == "Sun" {
        return Some(center);
    }

    for planet in planets {
        if planet.name == name {
            return Some(planet.position(elapsed, center));
        }

        let planet_pos = planet.position(elapsed, center);

        for moon in &planet.moons {
            if moon.name == name {
                return Some(moon.position(planet_pos, elapsed));
            }
        }
    }

    None
}

/// Ищет тело под курсором. Из нескольких попаданий выбирает то, чей край
/// ближе всего к точке клика.
fn pick_body(
    planets: &[Planet],
    world: Vec2,
    elapsed: f32,
    center: Vec2,
    zoom: f32,
) -> Option<BodyInfo> {
    let mut best: Option<(f32, BodyInfo)> = None;
    let mut consider = |distance: f32, radius: f32, info: BodyInfo| {
        // Мелкие спутники иначе почти невозможно поймать мышью.
        let hit_radius = radius.max(6.0 / zoom.max(0.001));

        if distance <= hit_radius {
            let offset = distance / hit_radius;

            if best.map_or(true, |(best_offset, _)| offset < best_offset) {
                best = Some((offset, info));
            }
        }
    };

    for planet in planets {
        let planet_pos = planet.position(elapsed, center);

        for moon in &planet.moons {
            let moon_pos = moon.position(planet_pos, elapsed);

            consider(
                (moon_pos - world).length(),
                moon.radius(),
                BodyInfo {
                    name: moon.name,
                    kind: "moon",
                    radius: moon.radius(),
                    orbit_radius: Some(moon.orbit_radius),
                    position: moon_pos,
                },
            );
        }

        consider(
            (planet_pos - world).length(),
            planet.radius(),
            BodyInfo {
                name: planet.name,
                kind: "planet",
                radius: planet.radius(),
                orbit_radius: Some(planet.orbit_radius),
                position: planet_pos,
            },
        );
    }

    let sun_radius = SUN_SIZE * 0.5;

    consider(
        (center - world).length(),
        sun_radius,
        BodyInfo {
            name: "Sun",
            kind: "star",
            radius: sun_radius,
            orbit_radius: None,
            position: center,
        },
    );

    best.map(|(_, info)| info)
}

/// Путь к картинке из общей папки ассетов.
fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/2d_games/assets/space")
        .join(name)
}

/// Ассет, общий для примеров: шрифт лежит один на всех.
fn shared_asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/2d_games/assets")
        .join(name)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    // Шрифт для панели с информацией о теле.
    let ttf = sdl3::ttf::init()?;
    let font = ttf.load_font(shared_asset("font.otf"), 20.0)?;
    let text_color = sdl3::pixels::Color::RGBA(232, 236, 244, 255);
    let mut labels = TextCache::new(|text: &str| -> Result<Image, Box<dyn std::error::Error>> {
        Ok(Image::from_surface(font.render(text).blended(text_color)?))
    });
    let mut selected: Option<BodyInfo> = None;

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

    // Солнце и все орбиты живут в начале координат.
    let center = Vec2::ZERO;

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
                    button: MouseButton::Left,
                    x,
                    y,
                    ..
                } => {
                    let world = camera.to_world(Vec2::new(x, y), window.size());

                    selected = pick_body(&planets, world, elapsed, center, camera.zoom);
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

        let hovered = cursor.and_then(|screen| {
            let world = camera.to_world(screen, (w, h));

            pick_body(&planets, world, elapsed, center, camera.zoom)
        });

        if let Some(body) = &mut selected {
            if let Some(position) = body_position(&planets, body.name, elapsed, center) {
                body.position = position;
            }
        }

        let info = selected.map(|body| body.text());
        if let Some(info) = &info {
            labels.keep(info)?;
        }

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

            // Свечение под телами: подсказывает, что по ним можно кликнуть.
            for planet in &planets {
                let planet_pos = planet.position(elapsed, center);

                draw_glow(
                    d,
                    planet_pos,
                    planet.radius(),
                    camera.zoom,
                    body_glow(planet.name, hovered.as_ref(), selected.as_ref(), elapsed),
                )?;

                for moon in &planet.moons {
                    let moon_pos = moon.position(planet_pos, elapsed);

                    draw_glow(
                        d,
                        moon_pos,
                        moon.radius(),
                        camera.zoom,
                        body_glow(moon.name, hovered.as_ref(), selected.as_ref(), elapsed),
                    )?;
                }
            }

            let sun_radius = SUN_SIZE * 0.5;
            draw_glow(
                d,
                center,
                sun_radius,
                camera.zoom,
                body_glow("Sun", hovered.as_ref(), selected.as_ref(), elapsed),
            )?;

            // Солнце.
            let sun =
                Object::new(center, Vec2::new(SUN_SIZE, SUN_SIZE)).with_rotation(elapsed * 20.0);

            d.fill_textured_object(&sun, &sun_image)?;

            // Планеты

            let _ = draw_bodies(d, &planets, elapsed, center, orbit_thickness, orbit_color);

            // Выбранное тело остаётся обведённым, а тело под курсором
            // подсвечивается ярче и пульсирует.
            if let Some(body) = &selected {
                draw_ring(
                    d,
                    body.position,
                    body.radius,
                    camera.zoom,
                    Color::rgba(155, 210, 255, 170),
                )?;
            }

            if let Some(body) = &hovered {
                let pulse = (elapsed * 6.0).sin() * 0.5 + 0.5;

                draw_ring(
                    d,
                    body.position,
                    body.radius,
                    camera.zoom,
                    Color::rgba(205, 235, 255, (150.0 + pulse * 80.0) as u8),
                )?;
            }

            // Панель с информацией о выбранном теле.
            if let Some(info) = &info {
                let label = Label::new(labels.get(info))
                    .at(24.0, 24.0)
                    .anchored(Anchor::TopLeft);
                let rect = label.rect();
                let padding = 12.0;

                let panel = Object::from_xywh(
                    rect.x + rect.w / 2.0,
                    rect.y + rect.h / 2.0,
                    rect.w + padding * 2.0,
                    rect.h + padding * 2.0,
                );

                d.screen_space(|d| d.fill_object(&panel, Color::rgba(16, 18, 26, 220)))?;

                label.draw(d)?;
            }

            Ok(())
        })?;

        clock.tick();
    }
}
