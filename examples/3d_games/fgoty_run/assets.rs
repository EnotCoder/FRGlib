//! Ассеты сцены: модели, вспомогательные меши и небо.
//!
//! Модели лежат рядом с примером, в `examples/3d_games/assets/fgoty_run`.
//! Путь строится от корня крейта, поэтому `cargo run --example` работает из
//! любого каталога. `map.glb` — трава, `player.glb` — Феррис, `bot.glb` — бот,
//! которого игра использует как врага, а `coin.glb` — золотая монета,
//! которую игрок собирает.

use std::f32::consts::TAU;
use std::path::PathBuf;

use frglib::gpu::{
    GltfModel, Gpu, GpuGltfModel, GpuMesh, Mesh, Renderer3D, Skybox, Texture, Vertex,
};

use crate::game::ARENA;

/// Ширина и глубина `map.glb` в локальных единицах модели.
pub const MAP_SIZE: f32 = 12.1875;
/// Во сколько раз растянуть карту, чтобы она закрыла всю арену.
pub const MAP_SCALE: f32 = ARENA * 2.0 / MAP_SIZE;

/// Всё, что загружается один раз при старте.
pub struct Assets {
    /// Земля с травой и веточками.
    pub map: GpuGltfModel,
    /// Красный краб Феррис.
    pub player: GpuGltfModel,
    /// Бот-враг.
    pub bot: GpuGltfModel,
    /// Золотая монета, которую собирает игрок.
    pub coin: GpuGltfModel,
    /// Единичный квадрат в XZ: тени и световые пятна на земле.
    pub plane: GpuMesh,
    /// Единичный квадрат в XY: светящиеся билборды.
    pub billboard: GpuMesh,
    /// Плоское кольцо для щита игрока.
    pub ring: GpuMesh,
    /// Единичный куб для частиц.
    pub cube: GpuMesh,
    /// Мягкое радиальное свечение.
    pub glow: Texture,
    /// Ровное светло-голубое небо.
    pub skybox: Skybox,
}

impl Assets {
    /// Читает модели, готовит процедурные меши и загружает небо.
    pub fn load(
        renderer: &Renderer3D,
        gpu: &Gpu,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Трава в модели очень яркая, а ветки — тёмные. Лёгкое притушивание
        // задаёт вечерний тон, но оставляет и траву, и ветки различимыми:
        // модель рисуется через `Material::Unlit`, поэтому свет её больше
        // не подсвечивает.
        let map = load_model(renderer, "map.glb", [0.5, 0.7, 0.5])?;
        let player = load_model(renderer, "player.glb", [1.12, 1.08, 1.08])?;
        let bot = load_model(renderer, "bot.glb", [1.0, 1.0, 1.03])?;
        // Материалы монеты тёмно-золотые; лёгкое усиление возвращает им
        // солнечный блеск, не теряя контраст между гранями.
        let coin = load_model(renderer, "coin.glb", [1.28, 1.2, 1.08])?;

        let plane = renderer.create_mesh(&Mesh::plane(1.0, 1.0));
        let billboard = renderer.create_mesh(&billboard_mesh());
        let ring = renderer.create_mesh(&ring_mesh());
        let cube = renderer.create_mesh(&Mesh::cube(1.0));

        let glow = renderer.create_texture_rgba8(128, 128, &glow_pixels(128))?;
        let skybox = Skybox::from_equirectangular_rgba8(gpu, 1024, 512, &sky_pixels(1024, 512))?;

        Ok(Assets {
            map,
            player,
            bot,
            coin,
            plane,
            billboard,
            ring,
            cube,
            glow,
            skybox,
        })
    }
}

/// Путь к модели рядом с примером.
fn model_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/3d_games/assets/fgoty_run")
        .join(name)
}

fn load_model(
    renderer: &Renderer3D,
    name: &str,
    tint: [f32; 3],
) -> Result<GpuGltfModel, Box<dyn std::error::Error>> {
    let mut model = GltfModel::from_file(model_path(name))?;
    tint_materials(&mut model, tint);
    Ok(renderer.create_gltf(&model)?)
}

/// Умножает базовый цвет всех материалов модели.
///
/// glTF хранит цвет в линейном пространстве, поэтому множитель можно
/// применять напрямую, не трогая гамму.
fn tint_materials(model: &mut GltfModel, tint: [f32; 3]) {
    for material in &mut model.materials {
        material.base_color[0] *= tint[0];
        material.base_color[1] *= tint[1];
        material.base_color[2] *= tint[2];
    }
}

/// Плоское кольцо в XZ для щита вокруг игрока.
fn ring_mesh() -> Mesh {
    const SEGMENTS: usize = 64;
    const INNER: f32 = 0.88;
    const OUTER: f32 = 1.0;

    let mut vertices = Vec::with_capacity(SEGMENTS * 4);
    let mut indices = Vec::with_capacity(SEGMENTS * 6);

    for i in 0..SEGMENTS {
        let a = i as f32 / SEGMENTS as f32 * TAU;
        let b = (i + 1) as f32 / SEGMENTS as f32 * TAU;
        let (sin_a, cos_a) = a.sin_cos();
        let (sin_b, cos_b) = b.sin_cos();

        let base = vertices.len() as u32;
        vertices.push(Vertex::new(
            [cos_a * INNER, 0.0, sin_a * INNER],
            [0.0, 1.0, 0.0],
        ));
        vertices.push(Vertex::new(
            [cos_a * OUTER, 0.0, sin_a * OUTER],
            [0.0, 1.0, 0.0],
        ));
        vertices.push(Vertex::new(
            [cos_b * OUTER, 0.0, sin_b * OUTER],
            [0.0, 1.0, 0.0],
        ));
        vertices.push(Vertex::new(
            [cos_b * INNER, 0.0, sin_b * INNER],
            [0.0, 1.0, 0.0],
        ));

        // Обход против часовой стрелки, если смотреть сверху: иначе кольцо
        // отсечёт back-face culling.
        indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
    }

    Mesh::from_vertices(vertices, indices)
}

/// Вертикальный квадрат в XY, повёрнутый лицевой стороной к +Z.
fn billboard_mesh() -> Mesh {
    let normal = [0.0, 0.0, 1.0];
    Mesh::from_vertices(
        vec![
            Vertex::with_uv([-0.5, -0.5, 0.0], normal, [0.0, 1.0]),
            Vertex::with_uv([0.5, -0.5, 0.0], normal, [1.0, 1.0]),
            Vertex::with_uv([0.5, 0.5, 0.0], normal, [1.0, 0.0]),
            Vertex::with_uv([-0.5, 0.5, 0.0], normal, [0.0, 0.0]),
        ],
        vec![0, 1, 2, 0, 2, 3],
    )
}

/// Белая текстура с мягким затуханием к краям.
fn glow_pixels(size: u32) -> Vec<u8> {
    let mut pixels = vec![0u8; (size * size * 4) as usize];
    let center = (size as f32 - 1.0) * 0.5;
    let radius = center.max(1.0);

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let distance = (dx * dx + dy * dy).sqrt() / radius;
            let alpha = (1.0 - distance).clamp(0.0, 1.0).powf(2.4);
            let index = ((y * size + x) * 4) as usize;
            pixels[index] = 255;
            pixels[index + 1] = 255;
            pixels[index + 2] = 255;
            pixels[index + 3] = (alpha * 255.0) as u8;
        }
    }

    pixels
}

/// Цвет неба: светло-голубой `#d3e6f7`.
const SKY_COLOR: [u8; 3] = [0xd3, 0xe6, 0xf7];

/// Ровная панорама неба без градиента, солнца и звёзд.
fn sky_pixels(width: u32, height: u32) -> Vec<u8> {
    let [r, g, b] = SKY_COLOR;
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..width * height {
        pixels.extend_from_slice(&[r, g, b, 255]);
    }
    pixels
}
