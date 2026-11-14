use frglib::{
    Anchor, Camera, Color, Event, FrameClock, Image, Keycode, Label, MouseButton, Object,
    TextCache, Vec2, Window,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};

mod bodies;
mod draw;

use bodies::*;
use draw::*;

/// Сторона спрайта Солнца в мировых единицах.
const SUN_SIZE: f32 = 120.0;

/// Радиус, в который вписывается система при сбросе камеры: чуть больше
/// орбиты Нептуна и пояса Койпера.
const SYSTEM_RADIUS: f32 = 1_080.0;

const MIN_ZOOM: f32 = 0.05;
const MAX_ZOOM: f32 = 12.0;

/// Множители времени. Пауза -- отдельный флаг, чтобы не терять скорость.
const TIME_SPEEDS: [f32; 6] = [0.25, 0.5, 1.0, 2.0, 5.0, 10.0];
const DEFAULT_SPEED: usize = 2;

const CONTROLS: &str = "LMB select   double-click follow\n\
MMB drag   wheel zoom   arrows move\n\
1-8 planet   0 sun   F follow\n\
SPACE pause   +/- speed\n\
O orbits   B belts   L labels   M moons\n\
R reset view   ESC back   Q quit";

/// То, что показываем в панели после клика по телу.
#[derive(Clone, Copy)]
struct BodyInfo {
    name: &'static str,
    kind: &'static str,
    /// Радиус в мировых единицах -- для кольца выделения.
    radius: f32,
    radius_km: f32,
    distance_km: Option<f32>,
    period_days: Option<f32>,
    moon_count: u32,
    position: Vec2,
}

impl BodyInfo {
    fn text(&self) -> String {
        let mut text = format!(
            "{}\n{}\nRadius: {} km",
            self.name,
            self.kind,
            group_digits(self.radius_km)
        );

        if let Some(distance) = self.distance_km {
            text.push_str(&format!("\nDistance: {}", format_distance(distance)));
        }

        if let Some(period) = self.period_days {
            text.push_str(&format!("\nPeriod: {}", format_period(period)));
        }

        if self.moon_count > 0 {
            text.push_str(&format!("\nMoons: {}", self.moon_count));
        }

        text
    }
}

/// `1234567` -> `1 234 567`: так большие километры читаются.
fn group_digits(value: f32) -> String {
    let digits = value.abs().round().max(0.0).to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);

    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(' ');
        }

        out.push(ch);
    }

    out
}

fn format_distance(km: f32) -> String {
    if km >= 1_000_000.0 {
        format!("{:.1} mln km", km / 1_000_000.0)
    } else {
        format!("{} km", group_digits(km))
    }
}

fn format_period(days: f32) -> String {
    if days >= 365.0 {
        format!("{:.1} years", days / 365.25)
    } else {
        format!("{:.1} days", days)
    }
}

fn planet_info(planet: &Planet, position: Vec2) -> BodyInfo {
    BodyInfo {
        name: planet.name,
        kind: "planet",
        radius: planet.radius(),
        radius_km: planet.radius_km,
        distance_km: Some(planet.distance_km),
        period_days: Some(planet.year_days),
        moon_count: planet.moon_count,
        position,
    }
}

fn moon_info(moon: &Moon, position: Vec2) -> BodyInfo {
    BodyInfo {
        name: moon.name,
        kind: "moon",
        radius: moon.radius(),
        radius_km: moon.radius_km,
        distance_km: Some(moon.orbit_km),
        period_days: Some(moon.period_days),
        moon_count: 0,
        position,
    }
}

fn sun_info(center: Vec2) -> BodyInfo {
    BodyInfo {
        name: "Sun",
        kind: "star",
        radius: SUN_SIZE * 0.5,
        radius_km: 696_340.0,
        distance_km: None,
        period_days: None,
        moon_count: 0,
        position: center,
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

/// Текущее положение тела по имени. Выбранное тело движется вместе с орбитой.
fn body_position(
    planets: &[Planet],
    poses: &[PlanetPose],
    name: &str,
    center: Vec2,
) -> Option<Vec2> {
    if name == "Sun" {
        return Some(center);
    }

    for (planet, pose) in planets.iter().zip(poses) {
        if planet.name == name {
            return Some(pose.position);
        }

        for (moon, moon_position) in planet.moons.iter().zip(&pose.moons) {
            if moon.name == name {
                return Some(*moon_position);
            }
        }
    }

    None
}

/// Ищет тело под курсором. Из нескольких попаданий выбирает то, чей край
/// ближе всего к точке клика.
fn pick_body(
    planets: &[Planet],
    poses: &[PlanetPose],
    world: Vec2,
    center: Vec2,
    zoom: f32,
) -> Option<BodyInfo> {
    let mut best: Option<(f32, BodyInfo)> = None;
    let mut consider = |distance: f32, radius: f32, info: BodyInfo| {
        // Мелкие спутники иначе почти невозможно поймать мышью.
        let hit_radius = radius.max(6.0 / zoom.max(0.001));

        if distance <= hit_radius {
            let offset = distance / hit_radius;

            if best.is_none_or(|(best_offset, _)| offset < best_offset) {
                best = Some((offset, info));
            }
        }
    };

    for (planet, pose) in planets.iter().zip(poses) {
        for (moon, moon_position) in planet.moons.iter().zip(&pose.moons) {
            consider(
                (*moon_position - world).length(),
                moon.radius(),
                moon_info(moon, *moon_position),
            );
        }

        consider(
            (pose.position - world).length(),
            planet.radius(),
            planet_info(planet, pose.position),
        );
    }

    consider((center - world).length(), SUN_SIZE * 0.5, sun_info(center));

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

/// Зум, при котором система целиком влезает в окно.
fn fit_zoom(size: (u32, u32)) -> f32 {
    let half_width = size.0 as f32 * 0.5;
    let half_height = size.1 as f32 * 0.5;

    (half_width / SYSTEM_RADIUS).min(half_height / (SYSTEM_RADIUS * ORBIT_TILT)) * 0.95
}

fn world_to_screen(camera: Camera, world: Vec2, size: (u32, u32)) -> Vec2 {
    (world - camera.position) * camera.zoom + Vec2::new(size.0 as f32 * 0.5, size.1 as f32 * 0.5)
}

fn on_off(value: bool) -> &'static str {
    if value {
        "on"
    } else {
        "off"
    }
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
        .background(Color::rgb(7, 9, 20))
        .build()?;

    // Шрифты: крупный для карточки, мелкий для HUD и подписей.
    let ttf = sdl3::ttf::init()?;
    let font = ttf.load_font(shared_asset("font.otf"), 20.0)?;
    let small_font = ttf.load_font(shared_asset("font.otf"), 14.0)?;

    let text_color = sdl3::pixels::Color::RGBA(232, 236, 244, 255);
    let hint_color = sdl3::pixels::Color::RGBA(196, 205, 222, 255);

    let mut info_cache =
        TextCache::new(|text: &str| -> Result<Image, Box<dyn std::error::Error>> {
            Ok(Image::from_surface(font.render(text).blended(text_color)?))
        });
    let mut hud_cache = TextCache::new(|text: &str| -> Result<Image, Box<dyn std::error::Error>> {
        Ok(Image::from_surface(
            small_font.render(text).blended(hint_color)?,
        ))
    });

    let mut clock = FrameClock::new(60.0);
    let mut elapsed = 0.0f32;
    let mut real_elapsed = 0.0f32;
    let mut fps = 60.0f32;

    // Камера смотрит в начало координат -- там Солнце.
    let mut camera = Camera::new(Vec2::ZERO).with_zoom(fit_zoom(window.size()));

    let mut cursor: Option<Vec2> = None;
    let mut last_cursor: Option<Vec2> = None;
    let mut dragging = false;
    let mut last_click: Option<(Instant, Vec2)> = None;

    let mut camera_velocity = Vec2::ZERO;
    let move_speed = 1000.0;
    let friction = 10.0;

    let mut up_pressed = false;
    let mut down_pressed = false;
    let mut left_pressed = false;
    let mut right_pressed = false;

    let mut selected: Option<BodyInfo> = None;
    let mut follow: Option<&'static str> = None;

    let mut paused = false;
    let mut speed_index = DEFAULT_SPEED;
    let mut show_orbits = true;
    let mut show_belts = true;
    let mut show_labels = true;
    let mut show_moons = true;

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

    let asteroid_belt = get_asteroid_belt();
    let kuiper_belt = get_kuiper_belt();
    let cloud_orte = get_orte_belt();
    let stars = get_stars();

    // Солнце и все орбиты живут в начале координат.
    let center = Vec2::ZERO;

    // Подписи тел кэшируются один раз: имена не меняются.
    for name in [
        "Sun", "Mercury", "Venus", "Earth", "Mars", "Jupiter", "Saturn", "Uranus", "Neptune",
    ] {
        hud_cache.keep(name)?;
    }
    for planet in &planets {
        for moon in &planet.moons {
            hud_cache.keep(moon.name)?;
        }
    }
    hud_cache.keep(CONTROLS)?;

    loop {
        let dt = clock.dt();
        real_elapsed += dt;

        if dt > 0.0 {
            fps += (1.0 / dt - fps) * 0.1;
        }

        let time_scale = if paused {
            0.0
        } else {
            TIME_SPEEDS[speed_index]
        };

        elapsed += dt * time_scale;

        // Все положения считаются один раз на кадр: отрисовка, выбор мышью,
        // панель и камера дальше только читают их.
        let poses = SystemPose::new(&planets, elapsed, center);

        // Определяем желаемое направление движения на основе зажатых клавиш.
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

        // Если есть ввод, ускоряем камеру, иначе замедляем.
        if move_dir.length_squared() > 0.0 {
            let target_vel = move_dir.normalized() * move_speed;
            camera_velocity = camera_velocity + (target_vel - camera_velocity) * (friction * dt);
        } else {
            camera_velocity = camera_velocity * (1.0 - friction * dt).max(0.0);
        }

        camera.position += camera_velocity * (dt / camera.zoom);

        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),

                Event::KeyDown { key, .. } => match key {
                    Keycode::Escape => {
                        // Первый Esc снимает выбор, второй выходит.
                        if selected.is_some() || follow.is_some() {
                            selected = None;
                            follow = None;
                        } else {
                            return Ok(());
                        }
                    }
                    Keycode::Q => return Ok(()),

                    Keycode::Up => {
                        up_pressed = true;
                        follow = None;
                    }
                    Keycode::Down => {
                        down_pressed = true;
                        follow = None;
                    }
                    Keycode::Left => {
                        left_pressed = true;
                        follow = None;
                    }
                    Keycode::Right => {
                        right_pressed = true;
                        follow = None;
                    }

                    Keycode::Space => paused = !paused,
                    Keycode::Equals | Keycode::KpPlus => {
                        speed_index = (speed_index + 1).min(TIME_SPEEDS.len() - 1);
                        paused = false;
                    }
                    Keycode::Minus | Keycode::KpMinus => {
                        speed_index = speed_index.saturating_sub(1);
                        paused = false;
                    }

                    Keycode::O => show_orbits = !show_orbits,
                    Keycode::B => show_belts = !show_belts,
                    Keycode::L => show_labels = !show_labels,
                    Keycode::M => show_moons = !show_moons,

                    Keycode::F => {
                        follow = if follow.is_some() {
                            None
                        } else {
                            selected.map(|body| body.name)
                        };
                    }

                    Keycode::R => {
                        camera = Camera::new(Vec2::ZERO).with_zoom(fit_zoom(window.size()));
                        camera_velocity = Vec2::ZERO;
                        follow = None;
                    }

                    Keycode::_0 => {
                        selected = Some(sun_info(center));
                        follow = Some("Sun");
                    }
                    Keycode::_1 => select_planet(&planets, 0, &poses, &mut selected, &mut follow),
                    Keycode::_2 => select_planet(&planets, 1, &poses, &mut selected, &mut follow),
                    Keycode::_3 => select_planet(&planets, 2, &poses, &mut selected, &mut follow),
                    Keycode::_4 => select_planet(&planets, 3, &poses, &mut selected, &mut follow),
                    Keycode::_5 => select_planet(&planets, 4, &poses, &mut selected, &mut follow),
                    Keycode::_6 => select_planet(&planets, 5, &poses, &mut selected, &mut follow),
                    Keycode::_7 => select_planet(&planets, 6, &poses, &mut selected, &mut follow),
                    Keycode::_8 => select_planet(&planets, 7, &poses, &mut selected, &mut follow),

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
                            follow = None;
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
                    let screen = Vec2::new(x, y);
                    let world = camera.to_world(screen, window.size());
                    let picked = pick_body(&planets, &poses.planets, world, center, camera.zoom);

                    // Двойной клик по телу -- следить за ним камерой.
                    let double_click = last_click.is_some_and(|(time, position)| {
                        time.elapsed() < Duration::from_millis(350)
                            && (screen - position).length() < 24.0
                    });

                    if double_click {
                        follow = picked.map(|body| body.name);
                    }

                    if picked.is_none() {
                        follow = None;
                    }

                    selected = picked;
                    last_click = Some((Instant::now(), screen));
                }

                Event::MouseButtonDown {
                    button: MouseButton::Middle,
                    x,
                    y,
                    ..
                } => {
                    dragging = true;
                    last_cursor = Some(Vec2::new(x, y));
                    follow = None;
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
                        camera.zoom = (camera.zoom * (1.0 + y * 0.1)).clamp(MIN_ZOOM, MAX_ZOOM);
                        let after = camera.to_world(mouse, size);
                        camera.position += before - after;
                    }

                    // Горизонтальный скролл двигает камеру вбок.
                    if x != 0.0 {
                        camera.position.x += x * 60.0 / camera.zoom;
                        follow = None;
                    }
                }

                _ => {}
            }
        }

        // Камера плавно догоняет выбранное тело.
        if let Some(name) = follow {
            if let Some(target) = body_position(&planets, &poses.planets, name, center) {
                camera.position =
                    camera.position + (target - camera.position) * (dt * 4.0).min(1.0);
            }
        }

        let (w, h) = window.size();
        let window_size = Vec2::new(w as f32, h as f32);

        let hovered = cursor.and_then(|screen| {
            let world = camera.to_world(screen, (w, h));

            pick_body(&planets, &poses.planets, world, center, camera.zoom)
        });

        if let Some(body) = &mut selected {
            if let Some(position) = body_position(&planets, &poses.planets, body.name, center) {
                body.position = position;
            }
        }

        // Тексты панелей. HUD-строки меняются редко, поэтому кэш не растёт:
        // вариантов ограниченное число.
        let info_text = selected.map(|body| body.text());
        if let Some(info) = &info_text {
            info_cache.keep(info)?;
        }

        let time_text = if paused {
            "TIME   paused".to_string()
        } else {
            format!("TIME   x{}", TIME_SPEEDS[speed_index])
        };
        let layers_text = format!(
            "orbits {}   belts {}\nlabels {}   moons {}",
            on_off(show_orbits),
            on_off(show_belts),
            on_off(show_labels),
            on_off(show_moons)
        );
        let fps_bucket = ((fps / 5.0).round() * 5.0) as u32;
        let fps_text = format!("FPS    {fps_bucket}");

        hud_cache.keep(&time_text)?;
        hud_cache.keep(&layers_text)?;
        hud_cache.keep(&fps_text)?;

        let name_labels = collect_name_labels(
            &planets,
            &poses.planets,
            center,
            camera,
            (w, h),
            show_labels,
            show_moons,
        );

        window.frame_with(|d| {
            d.set_camera(camera);

            // Фон, звёзды и интерфейс живут в экранных координатах.
            let _ = d.screen_space(|d| {
                draw_background(d, window_size)?;
                draw_stars(d, &stars, window_size, camera, real_elapsed)
            });

            if show_orbits {
                draw_orbits(d, &planets, &poses.planets, center, show_moons, camera.zoom)?;
            }

            if show_belts {
                draw_belt(d, &asteroid_belt, elapsed, center, camera.zoom)?;
                draw_belt(d, &kuiper_belt, elapsed, center, camera.zoom)?;
                draw_belt(d, &cloud_orte, elapsed, center, camera.zoom)?;
            }

            // Солнце: корона и лучи до спрайта.
            draw_sun(d, center, SUN_SIZE * 0.5, elapsed)?;

            let sun =
                Object::new(center, Vec2::new(SUN_SIZE, SUN_SIZE)).with_rotation(elapsed * 20.0);

            d.fill_textured_object(&sun, &sun_image)?;

            // Свечение под телами: подсказывает, что по ним можно кликнуть.
            for (planet, pose) in planets.iter().zip(&poses.planets) {
                draw_glow(
                    d,
                    pose.position,
                    planet.radius(),
                    camera.zoom,
                    body_glow(planet.name, hovered.as_ref(), selected.as_ref(), elapsed),
                )?;

                for (moon, moon_position) in planet.moons.iter().zip(&pose.moons) {
                    draw_glow(
                        d,
                        *moon_position,
                        moon.radius(),
                        camera.zoom,
                        body_glow(moon.name, hovered.as_ref(), selected.as_ref(), elapsed),
                    )?;
                }
            }

            draw_bodies(d, &planets, &poses.planets, elapsed, center)?;

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

            // Подписи планет (и лун на близком зуме).
            for label in &name_labels {
                let position = world_to_screen(camera, label.position, (w, h));
                let y = position.y - label.radius * camera.zoom - 6.0;

                if position.x < -80.0
                    || position.x > window_size.x + 80.0
                    || y < -20.0
                    || y > window_size.y + 20.0
                {
                    continue;
                }

                let text = Label::new(hud_cache.get(label.name))
                    .at(position.x, y)
                    .anchored(Anchor::Bottom);

                text.draw(d)?;
            }

            // Панель с информацией о выбранном теле.
            if let Some(info) = &info_text {
                let label = Label::new(info_cache.get(info))
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

                d.screen_space(|d| {
                    d.fill_object(&panel, Color::rgba(12, 14, 22, 228))?;
                    d.draw_object(&panel, Color::rgba(90, 120, 170, 90))?;
                    label.draw(d)
                })?;
            }

            // HUD в правом верхнем углу.
            let panel_width = 220.0;
            let panel_height = 100.0;
            let panel_x = window_size.x - panel_width - 18.0;
            let panel_y = 18.0;

            let panel = Object::from_xywh(
                panel_x + panel_width * 0.5,
                panel_y + panel_height * 0.5,
                panel_width,
                panel_height,
            );

            d.screen_space(|d| {
                d.fill_object(&panel, Color::rgba(12, 14, 22, 200))?;
                d.draw_object(&panel, Color::rgba(90, 120, 170, 70))?;

                Label::new(hud_cache.get(&time_text))
                    .at(panel_x + 12.0, panel_y + 10.0)
                    .anchored(Anchor::TopLeft)
                    .draw(d)?;
                Label::new(hud_cache.get(&layers_text))
                    .at(panel_x + 12.0, panel_y + 32.0)
                    .anchored(Anchor::TopLeft)
                    .draw(d)?;
                Label::new(hud_cache.get(&fps_text))
                    .at(panel_x + 12.0, panel_y + 74.0)
                    .anchored(Anchor::TopLeft)
                    .draw(d)?;

                Ok(())
            })?;

            // Подсказка по управлению в левом нижнем углу.
            let controls = Label::new(hud_cache.get(CONTROLS))
                .at(18.0, window_size.y - 18.0)
                .anchored(Anchor::BottomLeft);
            let controls_rect = controls.rect();
            let controls_panel = Object::from_xywh(
                controls_rect.x + controls_rect.w / 2.0 - 10.0,
                controls_rect.y + controls_rect.h / 2.0 - 10.0,
                controls_rect.w + 20.0,
                controls_rect.h + 20.0,
            );

            d.screen_space(|d| {
                d.fill_object(&controls_panel, Color::rgba(12, 14, 22, 190))?;
                controls.draw(d)
            })?;

            Ok(())
        })?;

        clock.tick();
    }
}

fn select_planet(
    planets: &[Planet],
    index: usize,
    poses: &SystemPose,
    selected: &mut Option<BodyInfo>,
    follow: &mut Option<&'static str>,
) {
    if let (Some(planet), Some(pose)) = (planets.get(index), poses.planets.get(index)) {
        *selected = Some(planet_info(planet, pose.position));
        *follow = Some(planet.name);
    }
}

struct ScreenLabel {
    name: &'static str,
    position: Vec2,
    radius: f32,
}

#[allow(clippy::too_many_arguments)]
fn collect_name_labels(
    planets: &[Planet],
    poses: &[PlanetPose],
    center: Vec2,
    camera: Camera,
    size: (u32, u32),
    show_labels: bool,
    show_moons: bool,
) -> Vec<ScreenLabel> {
    if !show_labels {
        return Vec::new();
    }

    let mut labels = Vec::new();
    let mut push = |name: &'static str, position: Vec2, radius: f32| {
        let screen = world_to_screen(camera, position, size);
        let margin = 120.0;

        if screen.x > -margin
            && screen.x < size.0 as f32 + margin
            && screen.y > -margin
            && screen.y < size.1 as f32 + margin
        {
            labels.push(ScreenLabel {
                name,
                position,
                radius,
            });
        }
    };

    push("Sun", center, SUN_SIZE * 0.5);

    // На общем плане подписи планет, спутники -- только при приближении.
    if camera.zoom > 0.45 {
        for (planet, pose) in planets.iter().zip(poses) {
            push(planet.name, pose.position, planet.radius());
        }
    }

    if show_moons && camera.zoom > 1.4 {
        for (planet, pose) in planets.iter().zip(poses) {
            for (moon, moon_position) in planet.moons.iter().zip(&pose.moons) {
                push(moon.name, *moon_position, moon.radius());
            }
        }
    }

    labels
}
