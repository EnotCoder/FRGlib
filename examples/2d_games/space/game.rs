//! Игровой цикл `space`.
//!
//! Вынесен из `main.rs`, чтобы тот же код собирался и как обычный пример, и
//! как Android-библиотека: `space_android` экспортирует `SDL_main`, который
//! просто вызывает [`run`].
//!
//! Ввод общий для мыши и пальцев. На Android касания приходят событиями
//! `Finger*` (синтез мыши отключён), на десктопе -- обычными `Mouse*`.

use std::collections::HashMap;
use std::error::Error;
use std::time::{Duration, Instant};

use frglib::{Camera, Event, FrameClock, Keycode, MouseButton, Vec2, Window};

use crate::assets::{font_path, load_solar_system};
use crate::bodies::{
    get_asteroid_belt, get_kuiper_belt, get_orte_belt, get_stars, Planet, PlanetPose, SystemPose,
};
use crate::consts::*;
use crate::draw::*;
use crate::selection::{body_position, pick_body, select_planet};
use crate::ui::{collect_name_labels, BodyInfo, HudState, Ui};
use crate::view::fit_zoom;

/// Палец, который ещё касается экрана.
struct Finger {
    /// Где палец коснулся экрана: по смещению от этой точки отличаем тап
    /// от перетаскивания.
    start: Vec2,
    /// Позиция на прошлом кадре, чтобы считать дельту перемещения.
    last: Vec2,
    started: Instant,
    moved: bool,
}

/// Сдвиг пальца, после которого это уже перетаскивание, а не тап.
const TAP_SLOP: f32 = 12.0;

/// Дольше этого касание считается удержанием и не выбирает тело.
const TAP_MAX_MS: u64 = 450;

/// Запускает игру до закрытия окна.
pub fn run() -> Result<(), Box<dyn Error>> {
    // textures
    let (sun_image, planets) = load_solar_system()?;

    // create window
    let builder = Window::builder(WINDOW_TITLE)
        .size(WINDOW_SIZE.0, WINDOW_SIZE.1)
        .pixel_art(true)
        .background(WINDOW_BACKGROUND)
        .high_pixel_density(true);

    // На Android пальцы обрабатываются событиями `Finger*`, поэтому
    // синтезированные клики мышью только дублировали бы выбор.
    #[cfg(target_os = "android")]
    let builder = builder.touch_as_mouse(false);

    let mut window = builder.build()?;

    // Масштаб интерфейса: на телефоне и HiDPI-мониторе шрифты, панели и
    // отступы растут вместе, иначе интерфейс становится нечитаемым.
    //
    // На Android `pixel_density()` всегда 1.0: размер окна и drawable там
    // совпадают, а реальная плотность экрана приходит через `display_scale()`.
    // Ограничиваем значение, чтобы случайный 0 или NaN не сломал вёрстку.
    let scale = {
        let scale = window.display_scale();

        if scale.is_finite() && scale > 0.0 {
            scale.clamp(1.0, 3.0)
        } else {
            1.0
        }
    };
    let touch_input = cfg!(target_os = "android");

    // Палец грубее курсора: радиус попадания по телу немного увеличиваем.
    let min_hit_radius = MIN_HIT_RADIUS * scale * if touch_input { 1.6 } else { 1.0 };
    let tap_slop = TAP_SLOP * scale;
    let double_radius = DOUBLE_CLICK_RADIUS * scale * if touch_input { 1.5 } else { 1.0 };

    // Шрифты: крупный для карточки, помельче для HUD и подписей.
    let ttf = sdl3::ttf::init()?;
    let font = ttf.load_font(font_path(), FONT_TITLE * scale)?;
    let small_font = ttf.load_font(font_path(), FONT_HUD * scale)?;

    let mut ui = Ui::new(font, small_font, scale);

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

    // Касания. Пальцы живут отдельно от мыши: даже если SDL когда-нибудь
    // включит синтез кликов, жест не должен сломаться.
    let mut fingers: HashMap<u64, Finger> = HashMap::new();
    // Последнее состояние щипка: расстояние между пальцами и их середина.
    let mut pinch: Option<(f32, Vec2)> = None;

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
                    // На Android системная кнопка "назад" приходит как AcBack
                    // и ведёт себя как Esc: закрывает помощь, снимает выбор,
                    // и только потом выходит из игры.
                    Keycode::Escape | Keycode::AcBack => {
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
                        selected = Some(crate::ui::sun_info());
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
                    tap(
                        Vec2::new(x, y),
                        camera,
                        window.size(),
                        &planets,
                        &poses.planets,
                        center,
                        min_hit_radius,
                        double_radius,
                        &mut last_click,
                        &mut selected,
                        &mut follow,
                    );
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

                Event::FingerDown { id, x, y, .. } => {
                    let position = Vec2::new(x, y);

                    fingers.insert(
                        id,
                        Finger {
                            start: position,
                            last: position,
                            started: Instant::now(),
                            moved: false,
                        },
                    );

                    // Второй палец превращает жест в щипок: одиночные тапы
                    // после него уже не должны выбирать тела.
                    if fingers.len() >= 2 {
                        for finger in fingers.values_mut() {
                            finger.moved = true;
                        }

                        pinch = pinch_state(&fingers);
                        follow = None;
                    }
                }

                Event::FingerMotion { id, x, y, .. } => {
                    let position = Vec2::new(x, y);

                    // Предыдущая позиция нужна для дельты, а `was_moved` --
                    // чтобы камера не прыгнула в момент пересечения порога.
                    let previous = fingers.get(&id).map(|finger| finger.last);
                    let was_moved = fingers.get(&id).is_some_and(|finger| finger.moved);

                    if let Some(finger) = fingers.get_mut(&id) {
                        finger.last = position;

                        if (position - finger.start).length() > tap_slop {
                            finger.moved = true;
                        }
                    }

                    if fingers.len() >= 2 {
                        let next = pinch_state(&fingers);

                        if let (Some((previous_distance, previous_mid)), Some((distance, mid))) =
                            (pinch, next)
                        {
                            let size = window.size();

                            // Два пальца тянут сцену за собой.
                            camera.position = camera.position - (mid - previous_mid) / camera.zoom;

                            // И одновременно масштабируют её вокруг середины:
                            // точка под пальцами остаётся на месте.
                            if previous_distance > 0.0 {
                                let before = camera.to_world(mid, size);
                                camera.zoom = (camera.zoom * (distance / previous_distance))
                                    .clamp(MIN_ZOOM, MAX_ZOOM);
                                let after = camera.to_world(mid, size);
                                camera.position += before - after;
                            }

                            follow = None;
                        }

                        pinch = next;
                    } else if was_moved {
                        if let Some(previous) = previous {
                            camera.position = camera.position - (position - previous) / camera.zoom;
                            follow = None;
                        }
                    }
                }

                Event::FingerUp { id, x, y, .. } => {
                    end_finger(
                        id,
                        Vec2::new(x, y),
                        false,
                        &mut fingers,
                        &mut pinch,
                        camera,
                        window.size(),
                        &planets,
                        &poses.planets,
                        center,
                        min_hit_radius,
                        double_radius,
                        &mut last_click,
                        &mut selected,
                        &mut follow,
                    );
                }

                Event::FingerCanceled { id, x, y } => {
                    end_finger(
                        id,
                        Vec2::new(x, y),
                        true,
                        &mut fingers,
                        &mut pinch,
                        camera,
                        window.size(),
                        &planets,
                        &poses.planets,
                        center,
                        min_hit_radius,
                        double_radius,
                        &mut last_click,
                        &mut selected,
                        &mut follow,
                    );
                }

                // Потеря фокуса -- например, звонок или уведомление: сбрасываем
                // зажатые клавиши и незавершённые жесты, иначе камера уедет.
                Event::FocusLost => {
                    up_pressed = false;
                    down_pressed = false;
                    left_pressed = false;
                    right_pressed = false;
                    dragging = false;
                    last_cursor = None;
                    cursor = None;
                    fingers.clear();
                    pinch = None;
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

            pick_body(
                &planets,
                &poses.planets,
                world,
                center,
                camera.zoom,
                min_hit_radius,
            )
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
            scale,
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

/// Расстояние между первыми двумя пальцами и их середина.
///
/// Порядок пальцев в `HashMap` не определён, но расстояние и середина от него
/// не зависят, поэтому специально сортировать ничего не нужно.
fn pinch_state(fingers: &HashMap<u64, Finger>) -> Option<(f32, Vec2)> {
    let mut values = fingers.values();
    let first = values.next()?;
    let second = values.next()?;

    Some((
        (first.last - second.last).length(),
        (first.last + second.last) * 0.5,
    ))
}

/// Завершает касание одним пальцем.
///
/// Если до этого на экране было два пальца, жест был щипком: оставшийся палец
/// продолжает панорамирование, но тапом уже не станет.
#[allow(clippy::too_many_arguments)]
fn end_finger(
    id: u64,
    position: Vec2,
    canceled: bool,
    fingers: &mut HashMap<u64, Finger>,
    pinch: &mut Option<(f32, Vec2)>,
    camera: Camera,
    window: (u32, u32),
    planets: &[Planet],
    poses: &[PlanetPose],
    center: Vec2,
    min_hit_radius: f32,
    double_radius: f32,
    last_click: &mut Option<(Instant, Vec2)>,
    selected: &mut Option<BodyInfo>,
    follow: &mut Option<&'static str>,
) {
    let fingers_before = fingers.len();
    let finger = fingers.remove(&id);

    if fingers_before >= 2 {
        for finger in fingers.values_mut() {
            finger.moved = true;
        }

        *pinch = pinch_state(fingers);
    } else if let Some(finger) = finger {
        // Тап -- это одиночное короткое касание без движения. Отменённое
        // касание (системный жест, уведомление) тапом не считается.
        if !canceled
            && !finger.moved
            && finger.started.elapsed() < Duration::from_millis(TAP_MAX_MS)
        {
            tap(
                position,
                camera,
                window,
                planets,
                poses,
                center,
                min_hit_radius,
                double_radius,
                last_click,
                selected,
                follow,
            );
        }
    }
}

/// Один клик или тап: выбор тела и, если он повторный, слежение за ним.
///
/// Общий код для мыши и пальцев: оба приходят в экранных координатах, а
/// отличие только в радиусе двойного нажатия.
#[allow(clippy::too_many_arguments)]
fn tap(
    screen: Vec2,
    camera: Camera,
    window: (u32, u32),
    planets: &[Planet],
    poses: &[PlanetPose],
    center: Vec2,
    min_hit_radius: f32,
    double_radius: f32,
    last_click: &mut Option<(Instant, Vec2)>,
    selected: &mut Option<BodyInfo>,
    follow: &mut Option<&'static str>,
) {
    let world = camera.to_world(screen, window);
    let picked = pick_body(planets, poses, world, center, camera.zoom, min_hit_radius);

    // Двойной клик по телу -- следить за ним камерой.
    let double_click = last_click.is_some_and(|(time, position)| {
        time.elapsed() < Duration::from_millis(DOUBLE_CLICK_MS)
            && (screen - position).length() < double_radius
    });

    if double_click {
        *follow = picked.map(|body| body.name);
    }

    if picked.is_none() {
        *follow = None;
    }

    *selected = picked;
    *last_click = Some((Instant::now(), screen));
}
