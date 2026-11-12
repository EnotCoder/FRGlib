//! Ассеты сцены: модели, вспомогательные меши и небо.
//!
//! Модели лежат рядом с примером, в `examples/3d_games/assets/fgoty_run`.
//! Путь строится от корня крейта, поэтому `cargo run --example` работает из
//! любого каталога. `map.glb` — трава, `player.glb` — Феррис, `bot.glb` — бот,
//! которого игра использует как врага.

use std::f32::consts::{PI, TAU};
use std::path::PathBuf;

use frglib::gpu::glam::Vec3;
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
    /// Золотой кристалл, который собирает игрок.
    pub crystal: GpuMesh,
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
    /// Градиентное небо с солнцем и звёздами.
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

        let crystal = renderer.create_mesh(&crystal_mesh());
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
            crystal,
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

/// Октаэдр: дешёвый кристалл, который красиво ловит свет гранями.
fn crystal_mesh() -> Mesh {
    let top = Vec3::new(0.0, 0.62, 0.0);
    let bottom = Vec3::new(0.0, -0.62, 0.0);
    let ring = [
        Vec3::new(0.34, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 0.34),
        Vec3::new(-0.34, 0.0, 0.0),
        Vec3::new(0.0, 0.0, -0.34),
    ];

    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(24);

    for i in 0..4 {
        let a = ring[i];
        let b = ring[(i + 1) % 4];
        push_triangle(&mut vertices, &mut indices, top, a, b);
        push_triangle(&mut vertices, &mut indices, bottom, b, a);
    }

    Mesh::from_vertices(vertices, indices)
}

/// Добавляет треугольник с плоской нормалью, разворачивая его наружу.
fn push_triangle(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    a: Vec3,
    mut b: Vec3,
    mut c: Vec3,
) {
    let mut normal = (b - a).cross(c - a).normalize_or_zero();

    // Нормаль должна смотреть от центра фигуры, иначе грань вывернута.
    if normal.dot(a + b + c) < 0.0 {
        std::mem::swap(&mut b, &mut c);
        normal = -normal;
    }

    let base = vertices.len() as u32;
    for point in [a, b, c] {
        vertices.push(Vertex::new(point.to_array(), normal.to_array()));
    }
    indices.extend_from_slice(&[base, base + 1, base + 2]);
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

/// Экваториальная панорама вечернего неба.
///
/// Верх — глубокий индиго, горизонт — тёплый закат, у солнца яркое пятно,
/// а выше горизонта разбросаны редкие звёзды.
fn sky_pixels(width: u32, height: u32) -> Vec<u8> {
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    // Солнце садится за дальним краем арены: камера по умолчанию смотрит
    // туда же, поэтому закат виден прямо над ботами.
    let sun = Vec3::new(0.35, 0.20, -0.92).normalize();

    for y in 0..height {
        let v = (y as f32 + 0.5) / height as f32;
        let elevation = (0.5 - v) * PI;

        for x in 0..width {
            let u = (x as f32 + 0.5) / width as f32;
            let azimuth = (u - 0.5) * TAU;
            let direction = Vec3::new(
                elevation.cos() * azimuth.sin(),
                elevation.sin(),
                elevation.cos() * azimuth.cos(),
            );

            let mut color = sky_color(elevation, direction, sun);

            // Звёзды только над горизонтом и подальше от солнца.
            let sun_dot = direction.dot(sun).max(0.0);
            if elevation > 0.18 && sun_dot < 0.92 {
                let hash = hash2(x, y);
                if hash > 0.9976 {
                    let brightness = ((hash - 0.9976) / 0.0024).powf(2.0) * 220.0;
                    color[0] += brightness;
                    color[1] += brightness * 0.96;
                    color[2] += brightness * 0.86;
                }
            }

            let index = ((y * width + x) * 4) as usize;
            pixels[index] = color[0].clamp(0.0, 255.0) as u8;
            pixels[index + 1] = color[1].clamp(0.0, 255.0) as u8;
            pixels[index + 2] = color[2].clamp(0.0, 255.0) as u8;
            pixels[index + 3] = 255;
        }
    }

    pixels
}

fn sky_color(elevation: f32, direction: Vec3, sun: Vec3) -> [f32; 3] {
    let horizon = [232.0, 118.0, 72.0];
    let zenith = [7.0, 11.0, 30.0];
    let below = [7.0, 10.0, 26.0];

    let mut color = if elevation >= 0.0 {
        let t = (elevation / (PI * 0.5)).powf(0.55);
        lerp3(horizon, zenith, t)
    } else {
        // Ниже горизонта небо быстро уходит в тёмно-синий: камера смотрит
        // вниз, и коричневая полоса у края экрана только мешала бы.
        let t = (-elevation / (PI * 0.5)).powf(0.02);
        lerp3(horizon, below, t)
    };

    let sun_dot = direction.dot(sun).max(0.0);
    let halo = sun_dot.powf(6.0) * 0.35 + sun_dot.powf(64.0) * 0.9;
    color[0] += 255.0 * halo;
    color[1] += 168.0 * halo;
    color[2] += 86.0 * halo;

    // Тёплая полоса прямо над горизонтом, сильнее всего у солнца.
    let band = (-elevation.abs() * 7.0).exp() * (0.22 + 0.78 * sun_dot);
    color[0] += 56.0 * band;
    color[1] += 20.0 * band;
    color[2] += 6.0 * band;

    color
}

fn lerp3(from: [f32; 3], to: [f32; 3], t: f32) -> [f32; 3] {
    [
        from[0] + (to[0] - from[0]) * t,
        from[1] + (to[1] - from[1]) * t,
        from[2] + (to[2] - from[2]) * t,
    ]
}

/// Детерминированный шум для звёзд: один и тот же кадр при каждом запуске.
fn hash2(x: u32, y: u32) -> f32 {
    let mut h = x
        .wrapping_mul(374_761_393)
        .wrapping_add(y.wrapping_mul(668_265_263));
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    ((h ^ (h >> 16)) & 0xffff) as f32 / 65_535.0
}
