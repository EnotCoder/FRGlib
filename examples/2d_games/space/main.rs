use frglib::{Camera, Event, FrameClock, Keycode, MouseButton, Vec2, Window};
use std::time::{Duration, Instant};

mod assets;
mod bodies;
mod consts;
mod draw;
mod selection;
mod ui;
mod view;

use assets::{font_path, load_solar_system};
use bodies::*;
use consts::*;
use draw::*;
use selection::{body_position, pick_body, select_planet};
use ui::{collect_name_labels, BodyInfo, HudState, Ui};
use view::fit_zoom;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // textures
    let (sun_image, planets) = load_solar_system()?;

    // create window
    let mut window = Window::builder(WINDOW_TITLE)
        .size(WINDOW_SIZE.0, WINDOW_SIZE.1)
        .pixel_art(true)
        .background(WINDOW_BACKGROUND)
        .build()?;

    // Шрифты: крупный для карточки, мелкий для HUD и подписей.
    let ttf = sdl3::ttf::init()?;
    let font = ttf.load_font(font_path(), 20.0)?;
    let small_font = ttf.load_font(font_path(), 14.0)?;

    let mut ui = Ui::new(font, small_font);

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
    let mut help_open = false;

    let asteroid_belt = get_asteroid_belt();
    let kuiper_belt = get_kuiper_belt();
    let cloud_orte = get_orte_belt();
    let stars = get_stars();

    // Солнце и все орбиты живут в начале координат.
    let center = Vec2::ZERO;

    // Имена тел и подсказка по управлению не меняются: кэшируем один раз.
    ui.keep_body_names(&planets)?;
    ui.keep_controls()?;

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
            let target_vel = move_dir.normalized() * CAMERA_MOVE_SPEED;
            camera_velocity =
                camera_velocity + (target_vel - camera_velocity) * (CAMERA_FRICTION * dt);
        } else {
            camera_velocity = camera_velocity * (1.0 - CAMERA_FRICTION * dt).max(0.0);
        }

        camera.position += camera_velocity * (dt / camera.zoom);

        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),

                Event::KeyDown { key, .. } => match key {
                    Keycode::Escape => {
                        // Первый Esc закрывает помощь, затем снимает выбор,
                        // и только последний выходит из игры.
                        if help_open {
                            help_open = false;
                        } else if selected.is_some() || follow.is_some() {
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
                    Keycode::H => help_open = !help_open,

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
                        selected = Some(ui::sun_info());
                        follow = Some("Sun");
                    }
                    Keycode::_1 => select_planet(&planets, 0, &mut selected, &mut follow),
                    Keycode::_2 => select_planet(&planets, 1, &mut selected, &mut follow),
                    Keycode::_3 => select_planet(&planets, 2, &mut selected, &mut follow),
                    Keycode::_4 => select_planet(&planets, 3, &mut selected, &mut follow),
                    Keycode::_5 => select_planet(&planets, 4, &mut selected, &mut follow),
                    Keycode::_6 => select_planet(&planets, 5, &mut selected, &mut follow),
                    Keycode::_7 => select_planet(&planets, 6, &mut selected, &mut follow),
                    Keycode::_8 => select_planet(&planets, 7, &mut selected, &mut follow),

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
                        time.elapsed() < Duration::from_millis(DOUBLE_CLICK_MS)
                            && (screen - position).length() < DOUBLE_CLICK_RADIUS
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
                        camera.zoom =
                            (camera.zoom * (1.0 + y * ZOOM_STEP)).clamp(MIN_ZOOM, MAX_ZOOM);
                        let after = camera.to_world(mouse, size);
                        camera.position += before - after;
                    }

                    // Горизонтальный скролл двигает камеру вбок.
                    if x != 0.0 {
                        camera.position.x += x * SCROLL_MOVE_SPEED / camera.zoom;
                        follow = None;
                    }
                }

                _ => {}
            }
        }

        // Камера плавно догоняет выбранное тело.
        if let Some(name) = follow {
            if let Some(target) = body_position(&planets, &poses.planets, name, center) {
                camera.position = camera.position
                    + (target - camera.position) * (dt * CAMERA_FOLLOW_SPEED).min(1.0);
            }
        }

        let (w, h) = window.size();
        let window_size = Vec2::new(w as f32, h as f32);

        let hovered = cursor.and_then(|screen| {
            let world = camera.to_world(screen, (w, h));

            pick_body(&planets, &poses.planets, world, center, camera.zoom)
        });

        // Тексты панелей. HUD-строки меняются редко, поэтому кэш не растёт:
        // вариантов ограниченное число.
        if let Some(info) = &selected {
            ui.keep_info(info)?;
        }

        let hud_state = HudState {
            paused,
            speed: TIME_SPEEDS[speed_index],
            zoom: camera.zoom,
            show_orbits,
            show_belts,
            show_labels,
            show_moons,
            fps,
        };
        let hud_texts = hud_state.texts();
        ui.keep_hud(&hud_texts)?;

        let name_labels = collect_name_labels(
            &planets,
            &poses.planets,
            center,
            camera,
            window_size,
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

            // Все тела -- включая Солнце -- рисуются одной сценой,
            // отсортированной по глубине: дальние перекрываются ближними.
            draw_bodies(
                d,
                &planets,
                &poses.planets,
                elapsed,
                center,
                camera.zoom,
                SUN_SIZE,
                &sun_image,
                show_moons,
                hovered.as_ref().map(|body| body.name),
                selected.as_ref().map(|body| body.name),
            )?;

            // Подписи планет (и лун на близком зуме).
            ui.draw_labels(d, &name_labels, camera, window_size)?;

            // Панель с информацией о выбранном теле.
            if let Some(info) = &selected {
                ui.draw_info_card(d, info, follow.is_some())?;
            }

            // HUD в правом верхнем углу.
            ui.draw_hud(d, &hud_texts, window_size)?;

            // FPS и зум в правом нижнем углу.
            ui.draw_stats(d, &hud_texts, window_size)?;

            // Подсказка по управлению в левом нижнем углу.
            ui.draw_controls(d, window_size, help_open)?;

            Ok(())
        })?;

        clock.tick();
    }
}
