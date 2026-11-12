//! Отрисовка 3D-сцены: арена, модели и эффекты.
//!
//! `Renderer3D` рисует всё в один проход. Модели берутся из `Assets`, а
//! процедурные меши и радиальная текстура дают тени, свечение и частицы без
//! единого лишнего файла. Непрозрачные объекты намеренно рисуются через
//! `Material::Unlit`: сцена не зависит от источников света, а цвета моделей
//! остаются ровно такими, как в glTF.

use frglib::gpu::glam::{Mat4, Vec3};
use frglib::gpu::{Camera3D, Material, Renderer3D};
use frglib::Color;

use crate::assets::Assets;
use crate::game::Game;

const PLAYER_SCALE: f32 = 1.7;
/// Насколько поднять Ферриса, чтобы лапы стояли на траве.
const PLAYER_LIFT: f32 = 0.20 * PLAYER_SCALE;
const BOT_SCALE: f32 = 1.3;
/// Бот ниже краба, но заметно выше него; подъём ставит его на землю.
const BOT_LIFT: f32 = 0.365 * BOT_SCALE;
/// Монета заметного размера: её видно с высоты камеры.
const COIN_SCALE: f32 = 0.9;

/// Мягкая камера, которая догоняет игрока и трясётся от ударов.
pub struct CameraRig {
    focus: Vec3,
}

impl CameraRig {
    pub fn new() -> Self {
        CameraRig { focus: Vec3::ZERO }
    }

    pub fn update(&mut self, game: &Game, dt: f32) -> Camera3D {
        // Камера смотрит чуть вперёд по направлению движения, а не строго
        // в центр краба: так видно, куда бежать.
        let lead = Vec3::new(game.facing.sin(), 0.0, game.facing.cos()) * 0.8;
        let target = game.player + lead;
        let follow = 1.0 - (-dt * 6.0).exp();
        self.focus = self.focus.lerp(target, follow);

        let shake = game.shake * 0.42;
        let shake_offset = Vec3::new(
            (game.time * 53.0).sin(),
            (game.time * 67.0).cos() * 0.5,
            (game.time * 61.0).sin(),
        ) * shake;

        Camera3D::perspective(55.0, 0.1, 200.0)
            .position(self.focus + Vec3::new(0.0, 10.0, 11.5) + shake_offset)
            .target(self.focus + Vec3::new(0.0, 0.5, 0.0))
    }
}

/// Рисует всю сцену: небо, арену, модели и эффекты.
pub fn draw_scene(game: &Game, scene: &mut Renderer3D, assets: &Assets, camera: &Camera3D) {
    // Карта — это земля. Растягиваем её ровно под размер арены.
    scene.draw_gltf_with(
        &assets.map,
        Mat4::from_scale(Vec3::splat(crate::assets::MAP_SCALE)),
        Material::Unlit,
    );

    draw_ground_glows(game, scene, assets);

    // Сначала свечение, потом сами объекты: билборды остаются позади модели
    // и дают мягкий ореол вокруг силуэта.
    for enemy in &game.enemies {
        draw_enemy_glow(scene, assets, camera, game.time, enemy);
    }

    for coin in &game.coins {
        draw_coin_glow(scene, assets, camera, game.time, coin);
    }

    for coin in &game.coins {
        draw_coin(scene, assets, game.time, coin);
    }

    for enemy in &game.enemies {
        draw_enemy(scene, assets, game.time, enemy);
    }

    draw_player(game, scene, assets);

    draw_particles(game, scene, assets);
    draw_fireflies(game, scene, assets, camera);
}

/// Мягкие тени и световые пятна под игроком, монетами и ботами.
fn draw_ground_glows(game: &Game, scene: &mut Renderer3D, assets: &Assets) {
    // Сначала тени: они прижимают модели к траве и дают сцене глубину.
    ground_glow(
        scene,
        assets,
        game.player,
        Color::rgba(0, 0, 0, 120),
        1.45,
    );

    for coin in &game.coins {
        ground_glow(scene, assets, coin.pos, Color::rgba(0, 0, 0, 70), 0.8);
    }

    for enemy in &game.enemies {
        ground_glow(scene, assets, enemy.pos, Color::rgba(0, 0, 0, 105), 1.55);
    }

    let player_pulse = 0.6 + 0.4 * (game.time * 3.0).sin();
    ground_glow(
        scene,
        assets,
        game.player,
        Color::rgba(70, 180, 255, (55.0 + 25.0 * player_pulse) as u8),
        1.75,
    );

    for coin in &game.coins {
        let pulse = 0.5 + 0.5 * (game.time * 3.0 + coin.phase).sin();
        ground_glow(
            scene,
            assets,
            coin.pos,
            Color::rgba(255, 190, 80, (50.0 + 30.0 * pulse) as u8),
            1.15,
        );
    }

    for enemy in &game.enemies {
        let pulse = 0.5 + 0.5 * (game.time * 4.0 + enemy.phase).sin();
        ground_glow(
            scene,
            assets,
            enemy.pos,
            Color::rgba(255, 52, 42, (44.0 + 30.0 * pulse) as u8),
            1.65,
        );
    }
}

fn draw_coin(scene: &mut Renderer3D, assets: &Assets, time: f32, coin: &crate::game::Coin) {
    let bob = (time * 2.4 + coin.phase).sin() * 0.12;
    let spin = time * 2.2 + coin.phase;
    let position = coin.pos + Vec3::Y * (0.65 + bob);

    // Монета стоит вертикально и вращается вокруг Y. Небольшой наклон к
    // камере не даёт ей превратиться в тонкую полоску на ребре.
    let model = Mat4::from_translation(position)
        * Mat4::from_rotation_x(-0.25)
        * Mat4::from_rotation_y(spin)
        * Mat4::from_scale(Vec3::splat(COIN_SCALE));

    scene.draw_gltf_with(&assets.coin, model, Material::Unlit);
}

fn draw_coin_glow(
    scene: &mut Renderer3D,
    assets: &Assets,
    camera: &Camera3D,
    time: f32,
    coin: &crate::game::Coin,
) {
    let bob = (time * 2.4 + coin.phase).sin() * 0.12;
    let pulse = 0.5 + 0.5 * (time * 3.0 + coin.phase).sin();

    billboard_glow(
        scene,
        assets,
        camera,
        coin.pos + Vec3::Y * (0.65 + bob),
        Color::rgba(255, 200, 90, (45.0 + 35.0 * pulse) as u8),
        1.25,
    );
}

fn draw_enemy(
    scene: &mut Renderer3D,
    assets: &Assets,
    time: f32,
    enemy: &crate::game::Enemy,
) {
    let bob = (time * 9.0 + enemy.phase).sin() * 0.04;
    let model = Mat4::from_translation(enemy.pos + Vec3::Y * (BOT_LIFT + bob))
        * Mat4::from_rotation_y(enemy.facing)
        * Mat4::from_scale(Vec3::splat(BOT_SCALE));

    scene.draw_gltf_with(&assets.bot, model, Material::Unlit);
}

fn draw_enemy_glow(
    scene: &mut Renderer3D,
    assets: &Assets,
    camera: &Camera3D,
    time: f32,
    enemy: &crate::game::Enemy,
) {
    let pulse = 0.5 + 0.5 * (time * 4.0 + enemy.phase).sin();

    billboard_glow(
        scene,
        assets,
        camera,
        enemy.pos + Vec3::Y * 0.9,
        Color::rgba(255, 44, 34, (22.0 + 20.0 * pulse) as u8),
        1.65,
    );

    // Красный «глаз» на груди бота: он смотрит туда же, куда идёт враг.
    let forward = Vec3::new(enemy.facing.sin(), 0.0, enemy.facing.cos());
    billboard_glow(
        scene,
        assets,
        camera,
        enemy.pos + forward * 0.38 + Vec3::Y * (BOT_LIFT + 0.55),
        Color::rgba(255, 36, 28, (130.0 + 70.0 * pulse) as u8),
        0.72,
    );
}

fn draw_player(game: &Game, scene: &mut Renderer3D, assets: &Assets) {
    // Пока действует неуязвимость, игрок мигает.
    let blink = game.invulnerable > 0.0 && (game.time * 14.0) as u32 % 2 == 0;

    if !blink {
        let bob = if game.moving {
            (game.time * 10.0).sin() * 0.06
        } else {
            (game.time * 3.0).sin() * 0.02
        };

        let model = Mat4::from_translation(game.player + Vec3::Y * (PLAYER_LIFT + bob))
            * Mat4::from_rotation_y(game.facing)
            * Mat4::from_scale(Vec3::splat(PLAYER_SCALE));

        scene.draw_gltf_with(&assets.player, model, Material::Unlit);
    }

    if game.invulnerable > 0.0 || game.grace > 0.0 {
        let pulse = 0.5 + 0.5 * (game.time * 8.0).sin();
        let model = Mat4::from_translation(game.player + Vec3::Y * 0.06)
            * Mat4::from_rotation_y(game.time * 2.0)
            * Mat4::from_scale(Vec3::splat(1.05 + 0.12 * pulse));

        scene.draw_mesh_with(
            &assets.ring,
            model,
            Material::Transparent,
            Color::rgba(90, 210, 255, (110.0 + 90.0 * pulse) as u8),
        );
    }
}

fn draw_particles(game: &Game, scene: &mut Renderer3D, assets: &Assets) {
    for particle in &game.particles {
        let fade = (particle.life / particle.max_life).clamp(0.0, 1.0);
        let color = Color::rgba(
            particle.color.r,
            particle.color.g,
            particle.color.b,
            (255.0 * fade) as u8,
        );

        let model = Mat4::from_translation(particle.pos)
            * Mat4::from_rotation_y(game.time * 6.0 + particle.pos.x)
            * Mat4::from_scale(Vec3::splat(particle.size));

        scene.draw_mesh_with(&assets.cube, model, Material::Transparent, color);
    }
}

fn draw_fireflies(game: &Game, scene: &mut Renderer3D, assets: &Assets, camera: &Camera3D) {
    for firefly in &game.fireflies {
        let angle = game.time * firefly.speed + firefly.phase;
        let position = firefly.anchor
            + Vec3::new(
                angle.sin() * firefly.radius,
                (angle * 0.7).sin() * 0.18,
                angle.cos() * firefly.radius,
            );

        let pulse = 0.35 + 0.65 * (game.time * 3.0 + firefly.phase).sin().abs();
        billboard_glow(
            scene,
            assets,
            camera,
            position,
            Color::rgba(255, 210, 120, (95.0 * pulse) as u8),
            0.28,
        );
    }
}

/// Мягкое пятно на земле из радиальной текстуры.
fn ground_glow(scene: &mut Renderer3D, assets: &Assets, position: Vec3, color: Color, size: f32) {
    let model = Mat4::from_translation(Vec3::new(position.x, 0.025, position.z))
        * Mat4::from_scale(Vec3::new(size, 1.0, size));

    scene.draw_mesh_textured_with(
        &assets.plane,
        &assets.glow,
        model,
        Material::Transparent,
        color,
    );
}

/// Светящийся билборд, всегда развёрнутый к камере.
fn billboard_glow(
    scene: &mut Renderer3D,
    assets: &Assets,
    camera: &Camera3D,
    position: Vec3,
    color: Color,
    size: f32,
) {
    let to_camera = camera.position - position;
    let yaw = to_camera.x.atan2(to_camera.z);

    let model = Mat4::from_translation(position)
        * Mat4::from_rotation_y(yaw)
        * Mat4::from_scale(Vec3::splat(size));

    scene.draw_mesh_textured_with(
        &assets.billboard,
        &assets.glow,
        model,
        Material::Transparent,
        color,
    );
}
