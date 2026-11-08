//! Простая 3D-игра на `wgpu`: собери золотые кубы и не дай красным догнать.
//!
//! Запуск: `cargo run --example 3d_games-runner --features wgpu`
//!
//! WASD или стрелки двигают синий куб по арене, R перезапускает игру,
//! Escape закрывает окно. Каждый собранный куб даёт очки; когда золотые
//! кончаются, начинается следующая волна с более быстрыми врагами. Три
//! столкновения — и игра окончена.
//!
//! Пример показывает оба рендерера в одном кадре: `Renderer3D` рисует сцену,
//! `Renderer2D` — HUD с текстом поверх неё.

use std::f32::consts::TAU;

use frglib::gpu::glam::{Mat4, Vec2, Vec3};
use frglib::gpu::{Camera3D, FontId, Renderer2D, Renderer3D, WgpuWindow};
use frglib::{Color, Event, FrameClock, Keycode, Rect};

/// Шрифт HUD из примеров интерфейса.
const FONT: &[u8] = include_bytes!("../assets/font.otf");

/// Половина стороны арены в мировых единицах.
const ARENA: f32 = 11.0;
/// Сколько золотых кубов появляется в каждой волне.
const COINS: usize = 8;
/// Скорость игрока, единиц в секунду.
const PLAYER_SPEED: f32 = 7.5;
/// Сколько столкновений выдерживает игрок.
const LIVES: u32 = 3;

fn main() -> Result<(), frglib::gpu::Error> {
    let mut window = WgpuWindow::builder("FRGLib 3D — собери кубы")
        .size(1000, 700)
        .background(Color::rgb(14, 16, 24))
        .build()?;

    let info = window.gpu().adapter_info();
    println!("{} ({:?} {})", info.name, info.backend, info.driver);

    let mut scene = Renderer3D::new(window.gpu());
    let mut ui = Renderer2D::new(window.gpu());
    let font = ui.load_font(FONT.to_vec())?;

    let mut game = Game::new(seed());
    let mut keys = Keys::default();
    let mut clock = FrameClock::new(60.0);

    loop {
        let events = window.poll()?;

        for event in &events {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::KeyDown {
                    key: Keycode::Escape,
                    ..
                } => return Ok(()),
                Event::KeyDown { key, .. } => {
                    if matches!(key, Keycode::R) || (game.over && matches!(key, Keycode::Space)) {
                        game = Game::new(seed());
                        keys = Keys::default();
                    } else {
                        keys.press(*key, true);
                    }
                }
                Event::KeyUp { key, .. } => keys.press(*key, false),
                // Иначе после alt-tab куб продолжит ехать сам по себе.
                Event::FocusLost => keys = Keys::default(),
                _ => {}
            }
        }

        // Кадр мог задержаться: длинный шаг физики проскочил бы столкновение.
        let dt = clock.dt().min(0.05);
        game.update(dt, &keys);

        let Some(mut frame) = window.begin_frame()? else {
            clock.tick();
            continue;
        };

        scene.begin(&follow_camera(&game));
        draw_scene(&game, &mut scene);
        scene.finish(&mut frame);

        ui.begin_overlay(&frame);
        draw_hud(&mut ui, font, &game, frame.size)?;
        ui.finish(&mut frame);

        window.present(frame)?;
        clock.tick();
    }
}

/// Камера сверху и немного сзади, всегда на игроке.
fn follow_camera(game: &Game) -> Camera3D {
    Camera3D::perspective(55.0, 0.1, 200.0)
        .position(game.player + Vec3::new(0.0, 16.0, 11.0))
        .target(game.player + Vec3::new(0.0, 0.5, 0.0))
}

/// Арена, золотые кубы, враги и игрок.
fn draw_scene(game: &Game, scene: &mut Renderer3D) {
    // Пол на волосок ниже сетки: иначе линии и плоскость спорят за одну
    // глубину и сетка мерцает.
    scene.draw_plane(
        Vec3::new(0.0, -0.02, 0.0),
        Vec2::new(ARENA * 2.0, ARENA * 2.0),
        Color::rgb(30, 34, 46),
    );
    scene.draw_grid((ARENA as u32) * 2, 1.0, Color::rgb(52, 60, 78));

    // Стены обозначают границу, за которую игрок не выйдет.
    let wall = Color::rgb(46, 52, 70);
    let height = 0.8;
    let thickness = 0.4;
    let long = ARENA * 2.0 + thickness;

    scene.draw_cube(
        Vec3::new(0.0, height / 2.0, -ARENA),
        Vec3::new(long, height, thickness),
        wall,
    );
    scene.draw_cube(
        Vec3::new(0.0, height / 2.0, ARENA),
        Vec3::new(long, height, thickness),
        wall,
    );
    scene.draw_cube(
        Vec3::new(-ARENA, height / 2.0, 0.0),
        Vec3::new(thickness, height, long),
        wall,
    );
    scene.draw_cube(
        Vec3::new(ARENA, height / 2.0, 0.0),
        Vec3::new(thickness, height, long),
        wall,
    );

    for coin in &game.coins {
        // Кубы покачиваются и медленно вращаются, чтобы их было видно.
        let bob = (game.time * 2.2 + coin.phase).sin() * 0.15;
        let model = Mat4::from_translation(coin.pos + Vec3::new(0.0, 0.7 + bob, 0.0))
            * Mat4::from_rotation_y(game.time * 1.6 + coin.phase)
            * Mat4::from_scale(Vec3::splat(0.45));

        scene.draw_cube_model(model, Color::rgb(240, 196, 84));
    }

    for enemy in &game.enemies {
        let model = Mat4::from_translation(enemy.pos + Vec3::new(0.0, 0.4, 0.0))
            * Mat4::from_rotation_y(game.time * 2.4 + enemy.phase)
            * Mat4::from_scale(Vec3::splat(0.8));

        scene.draw_cube_model(model, Color::rgb(214, 84, 84));
    }

    // Пока действует неуязвимость, игрок мигает.
    let blink = game.invulnerable > 0.0 && (game.time * 12.0) as u32 % 2 == 0;

    if !blink {
        scene.draw_cube(
            game.player + Vec3::new(0.0, 0.45, 0.0),
            Vec3::splat(0.9),
            Color::rgb(96, 178, 240),
        );
    }
}

/// Счёт, жизни и сообщения поверх сцены.
fn draw_hud(
    ui: &mut Renderer2D,
    font: FontId,
    game: &Game,
    size: (u32, u32),
) -> Result<(), frglib::gpu::Error> {
    let (width, height) = (size.0 as f32, size.1 as f32);

    let panel = Rect::new(18.0, 18.0, 252.0, 96.0);
    ui.fill_rect(panel, Color::rgba(10, 12, 18, 200));
    ui.draw_rect(panel, 1.0, Color::rgba(96, 116, 158, 150));

    ui.draw_text(
        font,
        &format!("Счёт: {}", game.score),
        34.0,
        30.0,
        26.0,
        Color::WHITE,
    )?;
    ui.draw_text(
        font,
        &format!("Жизни: {}", game.lives),
        34.0,
        66.0,
        18.0,
        Color::rgb(226, 128, 128),
    )?;
    ui.draw_text(
        font,
        &format!("Волна: {}", game.wave),
        166.0,
        66.0,
        18.0,
        Color::rgb(156, 178, 214),
    )?;

    if game.over {
        ui.fill_rect(
            Rect::new(0.0, 0.0, width, height),
            Color::rgba(6, 8, 14, 170),
        );

        ui.draw_text_centered(
            font,
            "Игра окончена",
            width / 2.0,
            height / 2.0 - 54.0,
            44.0,
            Color::WHITE,
        )?;
        ui.draw_text_centered(
            font,
            &format!("Счёт: {}", game.score),
            width / 2.0,
            height / 2.0 + 4.0,
            24.0,
            Color::rgb(236, 200, 110),
        )?;
        ui.draw_text_centered(
            font,
            "R или пробел — заново",
            width / 2.0,
            height / 2.0 + 52.0,
            18.0,
            Color::rgb(170, 184, 208),
        )?;
    } else {
        ui.draw_text_centered(
            font,
            "WASD / стрелки — двигаться · собирай золотые кубы · R — заново",
            width / 2.0,
            height - 28.0,
            16.0,
            Color::rgba(190, 200, 220, 210),
        )?;
    }

    Ok(())
}

/// Состояние игры: всё, что нужно для одного кадра и одного шага физики.
struct Game {
    player: Vec3,
    coins: Vec<Coin>,
    enemies: Vec<Enemy>,
    score: u32,
    lives: u32,
    wave: u32,
    /// Секунды, оставшиеся до конца мигания после удара.
    invulnerable: f32,
    /// Время с начала партии: вращает кубы и качает золото.
    time: f32,
    over: bool,
    rng: Rng,
}

struct Coin {
    pos: Vec3,
    /// Сдвиг фазы, чтобы кубы не качались синхронно.
    phase: f32,
}

struct Enemy {
    pos: Vec3,
    phase: f32,
}

impl Game {
    fn new(seed: u32) -> Self {
        let mut game = Game {
            player: Vec3::ZERO,
            coins: Vec::new(),
            enemies: Vec::new(),
            score: 0,
            lives: LIVES,
            wave: 1,
            invulnerable: 0.0,
            time: 0.0,
            over: false,
            rng: Rng::new(seed),
        };

        game.spawn_wave();
        game
    }

    /// Раскладывает золото и врагов подальше от игрока.
    fn spawn_wave(&mut self) {
        self.coins.clear();

        for _ in 0..COINS {
            let pos = self.point_away(1.5);
            let phase = self.rng.range(0.0, TAU);
            self.coins.push(Coin { pos, phase });
        }

        self.enemies.clear();

        // 3 врага на первой волне, дальше до шести.
        let count = 2 + self.wave.min(4) as usize;

        for _ in 0..count {
            let pos = self.point_away(6.0);
            let phase = self.rng.range(0.0, TAU);
            self.enemies.push(Enemy { pos, phase });
        }
    }

    /// Случайная точка внутри арены, не ближе `distance` к игроку.
    fn point_away(&mut self, distance: f32) -> Vec3 {
        for _ in 0..64 {
            let point = self.point();

            if (point - self.player).length() >= distance {
                return point;
            }
        }

        // Арена большая, а точки случайные: до сюда очередь почти не доходит.
        self.point()
    }

    fn point(&mut self) -> Vec3 {
        Vec3::new(
            self.rng.range(-ARENA + 1.2, ARENA - 1.2),
            0.0,
            self.rng.range(-ARENA + 1.2, ARENA - 1.2),
        )
    }

    fn update(&mut self, dt: f32, keys: &Keys) {
        self.time += dt;

        if self.over {
            return;
        }

        if self.invulnerable > 0.0 {
            self.invulnerable = (self.invulnerable - dt).max(0.0);
        }

        let mut direction = Vec3::ZERO;
        if keys.left {
            direction.x -= 1.0;
        }
        if keys.right {
            direction.x += 1.0;
        }
        if keys.up {
            direction.z -= 1.0;
        }
        if keys.down {
            direction.z += 1.0;
        }

        if direction.length_squared() > 0.0 {
            self.player += direction.normalize() * (PLAYER_SPEED * dt);
            self.player.x = self.player.x.clamp(-ARENA + 0.6, ARENA - 0.6);
            self.player.z = self.player.z.clamp(-ARENA + 0.6, ARENA - 0.6);
        }

        let player = self.player;
        let mut gained = 0;

        self.coins.retain(|coin| {
            if (coin.pos - player).length() < 0.85 {
                gained += 10;
                false
            } else {
                true
            }
        });
        self.score += gained;

        if self.coins.is_empty() {
            self.wave += 1;
            self.score += 50;
            self.spawn_wave();
        }

        let speed = (2.2 + self.wave as f32 * 0.25).min(5.5);
        let mut hit = false;

        for enemy in &mut self.enemies {
            let to_player = player - enemy.pos;
            let distance = to_player.length();

            if distance > 0.001 {
                // Не перескакиваем игрока на большом шаге.
                let step = (speed * dt).min(distance);
                enemy.pos += to_player / distance * step;
            }

            if distance < 0.9 {
                hit = true;
            }
        }

        if hit && self.invulnerable <= 0.0 {
            self.lives = self.lives.saturating_sub(1);
            self.invulnerable = 1.8;

            if self.lives == 0 {
                self.over = true;
            } else {
                // Разбрасываем врагов, иначе следующий удар придёт сразу же.
                let mut spots = Vec::with_capacity(self.enemies.len());
                for _ in 0..self.enemies.len() {
                    spots.push(self.point_away(7.0));
                }

                for (enemy, spot) in self.enemies.iter_mut().zip(spots) {
                    enemy.pos = spot;
                }
            }
        }
    }
}

/// Какие клавиши сейчас зажаты.
#[derive(Default)]
struct Keys {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

impl Keys {
    fn press(&mut self, key: Keycode, down: bool) {
        match key {
            Keycode::W | Keycode::Up => self.up = down,
            Keycode::S | Keycode::Down => self.down = down,
            Keycode::A | Keycode::Left => self.left = down,
            Keycode::D | Keycode::Right => self.right = down,
            _ => {}
        }
    }
}

/// Крошечный xorshift32: игра не тянет `rand` ради восьми кубов.
struct Rng(u32);

impl Rng {
    fn new(seed: u32) -> Self {
        // Ноль — неподвижная точка xorshift, поэтому состояние всегда нечётное.
        Rng(seed | 1)
    }

    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    fn range(&mut self, min: f32, max: f32) -> f32 {
        let t = self.next() as f32 / u32::MAX as f32;
        min + t * (max - min)
    }
}

/// Разный расклад на каждый запуск и перезапуск.
fn seed() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos() as u32 ^ elapsed.subsec_nanos())
        .unwrap_or(0x9E37_79B9)
}
