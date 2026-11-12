//! Состояние партии: игрок, монеты, боты, частицы и светлячки.
//!
//! Вся логика живёт здесь и ничего не знает о рендере: `render` читает поля
//! `Game`, `hud` рисует по ним интерфейс, а `main` только передаёт ввод.

use std::f32::consts::{PI, TAU};

use frglib::gpu::glam::Vec3;
use frglib::{Color, Keycode};

/// Половина стороны арены в мировых единицах.
pub const ARENA: f32 = 8.0;
/// Сколько монет появляется в каждой волне.
pub const COINS: usize = 8;
/// Скорость игрока, единиц в секунду.
const PLAYER_SPEED: f32 = 6.0;
/// Сколько столкновений выдерживает игрок.
const LIVES: u32 = 3;
/// Радиус, в котором монета считается собранной.
const COIN_RADIUS: f32 = 0.9;
/// Радиус столкновения с ботом.
const HIT_RADIUS: f32 = 0.95;
/// Секунды неуязвимости после удара.
const INVULNERABLE: f32 = 1.8;

/// Всё, что нужно для одного кадра и одного шага физики.
pub struct Game {
    pub player: Vec3,
    /// Куда смотрит Феррис, радианы вокруг Y.
    pub facing: f32,
    /// Двигался ли игрок в этом кадре: рендер качает модель сильнее.
    pub moving: bool,
    pub coins: Vec<Coin>,
    pub enemies: Vec<Enemy>,
    pub particles: Vec<Particle>,
    pub fireflies: Vec<Firefly>,
    pub score: u32,
    pub lives: u32,
    pub wave: u32,
    /// Секунды, оставшиеся до конца мигания после удара.
    pub invulnerable: f32,
    /// Короткая пауза в начале партии: боты ещё не могут ударить.
    pub grace: f32,
    /// Время с начала партии: вращает монеты и качает траву.
    pub time: f32,
    pub over: bool,
    /// Сила тряски камеры, гаснет сама.
    pub shake: f32,
    /// Красная вспышка при ударе, гаснет сама.
    pub flash: f32,
    /// Таймер надписи «Волна N».
    pub banner: f32,
    rng: Rng,
}

pub struct Coin {
    pub pos: Vec3,
    /// Сдвиг фазы, чтобы монеты не качались синхронно.
    pub phase: f32,
}

pub struct Enemy {
    pub pos: Vec3,
    pub phase: f32,
    /// Индивидуальная скорость: волна задаёт базу, разброс делает ботов живыми.
    pub speed: f32,
    pub facing: f32,
}

pub struct Particle {
    pub pos: Vec3,
    pub velocity: Vec3,
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
    pub color: Color,
}

pub struct Firefly {
    pub anchor: Vec3,
    pub phase: f32,
    pub radius: f32,
    pub speed: f32,
}

impl Game {
    pub fn new(seed: u32) -> Self {
        let mut game = Game {
            player: Vec3::ZERO,
            facing: 0.0,
            moving: false,
            coins: Vec::new(),
            enemies: Vec::new(),
            particles: Vec::new(),
            fireflies: Vec::new(),
            score: 0,
            lives: LIVES,
            wave: 1,
            invulnerable: 0.0,
            grace: 2.0,
            time: 0.0,
            over: false,
            shake: 0.0,
            flash: 0.0,
            banner: 0.0,
            rng: Rng::new(seed),
        };

        game.spawn_wave();
        game.spawn_fireflies();
        game
    }

    /// Раскладывает монеты и ботов подальше от игрока.
    fn spawn_wave(&mut self) {
        self.coins.clear();

        for _ in 0..COINS {
            let pos = self.point_away(1.8);
            let phase = self.rng.range(0.0, TAU);
            self.coins.push(Coin { pos, phase });
        }

        self.enemies.clear();

        // 3 бота на первой волне, дальше до шести.
        let count = 2 + self.wave.min(4) as usize;
        let base_speed = (1.9 + self.wave as f32 * 0.22).min(4.6);

        for _ in 0..count {
            let pos = self.point_away(5.0);
            let phase = self.rng.range(0.0, TAU);
            let speed = base_speed * self.rng.range(0.9, 1.12);
            self.enemies.push(Enemy {
                pos,
                phase,
                speed,
                facing: 0.0,
            });
        }

        self.banner = 2.4;
    }

    /// Светлячки над травой: статичные якоря, движение считается в рендере.
    fn spawn_fireflies(&mut self) {
        self.fireflies.clear();

        for _ in 0..28 {
            let anchor = Vec3::new(
                self.rng.range(-ARENA + 1.0, ARENA - 1.0),
                self.rng.range(0.35, 1.9),
                self.rng.range(-ARENA + 1.0, ARENA - 1.0),
            );
            self.fireflies.push(Firefly {
                anchor,
                phase: self.rng.range(0.0, TAU),
                radius: self.rng.range(0.15, 0.6),
                speed: self.rng.range(0.4, 1.0),
            });
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

        self.point()
    }

    fn point(&mut self) -> Vec3 {
        Vec3::new(
            self.rng.range(-ARENA + 1.2, ARENA - 1.2),
            0.0,
            self.rng.range(-ARENA + 1.2, ARENA - 1.2),
        )
    }

    pub fn update(&mut self, dt: f32, keys: &Keys) {
        self.time += dt;
        self.shake = (self.shake - dt * 2.6).max(0.0);
        self.flash = (self.flash - dt * 2.2).max(0.0);
        self.banner = (self.banner - dt).max(0.0);
        self.grace = (self.grace - dt).max(0.0);

        self.update_particles(dt);

        if self.over {
            self.moving = false;
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

        self.moving = direction.length_squared() > 0.0;

        if self.moving {
            let direction = direction.normalize();
            self.player += direction * (PLAYER_SPEED * dt);
            self.player.x = self.player.x.clamp(-ARENA + 0.7, ARENA - 0.7);
            self.player.z = self.player.z.clamp(-ARENA + 0.7, ARENA - 0.7);

            let target = direction.x.atan2(direction.z);
            self.facing = lerp_angle(self.facing, target, (dt * 12.0).min(1.0));
        }

        self.collect_coins();

        if self.coins.is_empty() {
            self.wave += 1;
            self.score += 50;
            self.spawn_wave();
            self.burst(self.player, Color::rgb(100, 210, 255), 28, 4.0);
            self.shake = self.shake.max(0.55);
        }

        self.update_enemies(dt);
    }

    fn collect_coins(&mut self) {
        let player = self.player;
        let mut gained = 0;
        let mut collected = Vec::new();

        self.coins.retain(|coin| {
            if (coin.pos - player).length() < COIN_RADIUS {
                gained += 10;
                collected.push(coin.pos);
                false
            } else {
                true
            }
        });

        for pos in collected {
            self.burst(pos + Vec3::Y * 0.6, Color::rgb(255, 205, 90), 14, 3.0);
            self.shake = self.shake.max(0.12);
        }

        self.score += gained;
    }

    fn update_enemies(&mut self, dt: f32) {
        let player = self.player;
        let mut hit = false;

        for enemy in &mut self.enemies {
            let to_player = player - enemy.pos;
            let distance = to_player.length();

            if distance > 0.001 {
                // Не перескакиваем игрока на большом шаге.
                let step = (enemy.speed * dt).min(distance);
                let direction = to_player / distance;
                enemy.pos += direction * step;

                let target = direction.x.atan2(direction.z);
                enemy.facing = lerp_angle(enemy.facing, target, (dt * 8.0).min(1.0));
            }

            if (player - enemy.pos).length() < HIT_RADIUS {
                hit = true;
            }
        }

        // Боты не должны слипаться в одну точку: расталкиваем соседей.
        let count = self.enemies.len();
        for i in 0..count {
            for j in (i + 1)..count {
                let delta = self.enemies[j].pos - self.enemies[i].pos;
                let distance = delta.length();

                if distance > 0.001 && distance < 1.1 {
                    let push = delta / distance * ((1.1 - distance) * 0.5);
                    self.enemies[i].pos -= push;
                    self.enemies[j].pos += push;
                }
            }
        }

        for enemy in &mut self.enemies {
            enemy.pos.x = enemy.pos.x.clamp(-ARENA + 0.8, ARENA - 0.8);
            enemy.pos.z = enemy.pos.z.clamp(-ARENA + 0.8, ARENA - 0.8);
        }

        if hit && self.invulnerable <= 0.0 && self.grace <= 0.0 {
            self.lives = self.lives.saturating_sub(1);
            self.invulnerable = INVULNERABLE;
            self.shake = 1.0;
            self.flash = 1.0;
            self.burst(self.player + Vec3::Y * 0.5, Color::rgb(255, 70, 60), 30, 5.0);

            if self.lives == 0 {
                self.over = true;
                self.burst(self.player + Vec3::Y * 0.5, Color::rgb(255, 150, 60), 40, 6.0);
            } else {
                // Разбрасываем ботов, иначе следующий удар придёт сразу же.
                let mut spots = Vec::with_capacity(self.enemies.len());
                for _ in 0..self.enemies.len() {
                    spots.push(self.point_away(6.5));
                }

                for (enemy, spot) in self.enemies.iter_mut().zip(spots) {
                    enemy.pos = spot;
                }
            }
        }
    }

    fn update_particles(&mut self, dt: f32) {
        for particle in &mut self.particles {
            particle.life -= dt;
            particle.velocity.y -= 7.0 * dt;
            particle.velocity *= (1.0 - 2.4 * dt).max(0.0);
            particle.pos += particle.velocity * dt;
        }

        self.particles.retain(|particle| particle.life > 0.0);
    }

    /// Взрыв из маленьких кубиков: золото при сборе, красный при ударе.
    pub fn burst(&mut self, pos: Vec3, color: Color, count: usize, speed: f32) {
        for _ in 0..count {
            let angle = self.rng.range(0.0, TAU);
            let horizontal = self.rng.range(0.15, 1.0);
            let velocity = Vec3::new(
                angle.cos() * horizontal,
                self.rng.range(0.35, 1.4),
                angle.sin() * horizontal,
            ) * speed
                * self.rng.range(0.35, 1.0);

            let life = self.rng.range(0.35, 0.9);
            let size = self.rng.range(0.06, 0.16);

            self.particles.push(Particle {
                pos,
                velocity,
                life,
                max_life: life,
                size,
                color,
            });
        }
    }
}

/// Коротчайший поворот от `from` к `to` на долю `t`.
fn lerp_angle(from: f32, to: f32, t: f32) -> f32 {
    let difference = (to - from + PI).rem_euclid(TAU) - PI;
    from + difference * t
}

/// Какие клавиши сейчас зажаты.
#[derive(Default)]
pub struct Keys {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

impl Keys {
    pub fn press(&mut self, key: Keycode, down: bool) {
        match key {
            Keycode::W | Keycode::Up => self.up = down,
            Keycode::S | Keycode::Down => self.down = down,
            Keycode::A | Keycode::Left => self.left = down,
            Keycode::D | Keycode::Right => self.right = down,
            _ => {}
        }
    }
}

/// Крошечный xorshift32: игра не тянет `rand` ради пары сотен частиц.
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
pub fn seed() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos() as u32 ^ elapsed.subsec_nanos())
        .unwrap_or(0x9E37_79B9)
}
