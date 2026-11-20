//! Ландшафт из шума Перлина: сетка вершин с высотами, нормалями и цветами.
//!
//! Каждая перегенерация создаёт новый `Perlin` из нового seed, поэтому старый
//! рельеф полностью исчезает. Высоты хранятся рядом с мешем: камера спрашивает
//! их через [`Terrain::height_at`], чтобы не проваливаться под землю.
//!
//! Цвет задаётся на ячейку, а не на вершину: соседние ячейки не смешиваются,
//! поэтому поверхность выглядит пиксельной, а не размытой.

use frglib::gpu::glam::Vec3;
use frglib::gpu::{GpuMesh, Mesh, Renderer3D, Vertex};
use frglib::Perlin;

/// Размер квадрата ландшафта в мировых единицах.
pub const SIZE: f32 = 240.0;
/// Число ячеек по каждой оси. 192 — компромисс: крупные формы и заметные
/// детали при разумном количестве вершин (около 37 тысяч).
pub const SEGMENTS: usize = 192;

pub struct Terrain {
    /// Меш, загруженный в GPU; пересоздаётся при смене seed.
    pub mesh: GpuMesh,
    /// Высоты вершин в том же порядке, что и вершины меша.
    heights: Vec<f32>,
    pub seed: u64,
}

impl Terrain {
    /// Генерирует рельеф для `seed` и загружает его в GPU.
    pub fn generate(renderer: &Renderer3D, seed: u64) -> Self {
        let noise = Perlin::new(seed);
        let cols = SEGMENTS + 1;
        let step = SIZE / SEGMENTS as f32;
        let half = SIZE * 0.5;

        let mut heights = Vec::with_capacity(cols * cols);
        for j in 0..cols {
            let z = -half + j as f32 * step;
            for i in 0..cols {
                let x = -half + i as f32 * step;
                heights.push(height(&noise, x, z));
            }
        }

        // Нормалы считаем один раз по сетке высот, чтобы освещение оставалось
        // гладким даже там, где цвет даётся отдельной ячейке.
        let mut normals = Vec::with_capacity(cols * cols);
        for j in 0..cols {
            for i in 0..cols {
                let left = heights[j * cols + i.saturating_sub(1)];
                let right = heights[j * cols + (i + 1).min(SEGMENTS)];
                let near = heights[j.saturating_sub(1) * cols + i];
                let far = heights[(j + 1).min(SEGMENTS) * cols + i];
                normals.push(Vec3::new(left - right, 2.0 * step, near - far).normalize());
            }
        }

        // Каждая ячейка получает свои четыре вершины с одинаковым цветом.
        // Так цвет не интерполируется между соседними ячейками — границы
        // остаются резкими, как у пиксельной текстуры с точечной фильтрацией.
        let mut vertices = Vec::with_capacity(SEGMENTS * SEGMENTS * 4);
        let mut indices = Vec::with_capacity(SEGMENTS * SEGMENTS * 6);
        for j in 0..SEGMENTS {
            for i in 0..SEGMENTS {
                let corners = [
                    j * cols + i,
                    j * cols + i + 1,
                    (j + 1) * cols + i,
                    (j + 1) * cols + i + 1,
                ];

                let center_h = corners.iter().map(|&index| heights[index]).sum::<f32>() / 4.0;
                let center_slope = corners.iter().map(|&index| normals[index].y).sum::<f32>() / 4.0;
                let cx = -half + (i as f32 + 0.5) * step;
                let cz = -half + (j as f32 + 0.5) * step;
                let variation = noise.sample_2d(cx * 0.08 + 19.0, cz * 0.08 - 7.0);
                let color = surface_color(center_h, center_slope, variation);

                let base = vertices.len() as u32;
                for &index in &corners {
                    let x = -half + (index % cols) as f32 * step;
                    let z = -half + (index / cols) as f32 * step;
                    vertices.push(Vertex::with_color(
                        [x, heights[index], z],
                        normals[index].to_array(),
                        color,
                    ));
                }

                // Обход против часовой стрелки, если смотреть сверху: иначе
                // back-face culling уберёт всю поверхность.
                indices.extend_from_slice(&[base, base + 2, base + 1, base + 1, base + 2, base + 3]);
            }
        }

        Terrain {
            mesh: renderer.create_mesh(&Mesh::from_vertices(vertices, indices)),
            heights,
            seed,
        }
    }

    /// Высота в точке `(x, z)` с билинейной интерполяцией между вершинами.
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        let max = SEGMENTS as f32 - 0.001;
        let fx = ((x / SIZE + 0.5) * SEGMENTS as f32).clamp(0.0, max);
        let fz = ((z / SIZE + 0.5) * SEGMENTS as f32).clamp(0.0, max);

        let i = fx as usize;
        let j = fz as usize;
        let tx = fx - i as f32;
        let tz = fz - j as f32;
        let cols = SEGMENTS + 1;

        let h00 = self.heights[j * cols + i];
        let h10 = self.heights[j * cols + i + 1];
        let h01 = self.heights[(j + 1) * cols + i];
        let h11 = self.heights[(j + 1) * cols + i + 1];

        let lower = h00 + (h10 - h00) * tx;
        let upper = h01 + (h11 - h01) * tx;
        lower + (upper - lower) * tz
    }
}

/// Высота рельефа в мировой точке: материки + хребты + мелкие детали.
///
/// Края квадрата плавно уходят под воду, так что ландшафт выглядит островом,
/// а не обрезанным краем бесконечной карты.
fn height(noise: &Perlin, x: f32, z: f32) -> f32 {
    // Невысокие плавные холмы.
    let hills = noise.fbm_2d(
        x, z,
        3, 0.011, 2.0, 0.4,
    );

    // Отдельный шум определяет расположение горных районов.
    let region = noise.fbm_2d(
        x + 731.0, z - 419.0,
        2, 0.018, 2.0, 0.5,
    );

    // Ниже 0.25 гор нет, от 0.25 до 0.50 они плавно появляются.
    let mountain_mask = smoothstep(0.25, 0.50, region);

    // Форма горных хребтов.
    let ridge_noise = noise.fbm_2d(
        x + 317.0, z - 211.0,
        4, 0.026, 2.0, 0.5,
    );
    let ridges = (1.0 - ridge_noise.abs()).clamp(0.0, 1.0);

    // Мелкие неровности.
    let detail = noise.fbm_2d(
        x - 91.0, z + 43.0,
        3, 0.09, 2.0, 0.5,
    );

    // Основа острова: небольшой подъём над водой и пологие холмы.
    let base = 3.0 + hills * 4.0;

    // Горы добавляются только там, где разрешает маска.
    let mountains = ridges.powi(3) * 24.0 * mountain_mask;

    // На равнинах деталей меньше, в горах — больше.
    let roughness = detail * (0.25 + mountain_mask * 1.0);

    let land = base + mountains + roughness;

    // Прежнее плавное опускание краёв острова под воду.
    let radius = (x * x + z * z).sqrt() / (SIZE * 0.5);
    let shore = 1.0 - smoothstep(0.55, 1.05, radius);

    land * shore - 9.0 * (1.0 - shore)
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Цвет поверхности по высоте, крутизне и мелкому шуму.
fn surface_color(h: f32, slope: f32, variation: f32) -> [f32; 4] {
    let v = variation * 0.5 + 0.5;

    let (r, g, b) = if h < 0.0 {
        // Подводный грунт: сквозь полупрозрачную воду он выглядит темнее.
        (52.0 + v * 16.0, 96.0 + v * 18.0, 104.0 + v * 20.0)
    } else if h < 1.6 {
        // Пляж.
        (214.0 + v * 16.0, 196.0 + v * 14.0, 148.0 + v * 16.0)
    } else if h < 12.0 {
        // Трава: оттенок зависит от шума, чтобы не было ровной заливки.
        (70.0 + v * 28.0, 126.0 + v * 34.0, 66.0 + v * 22.0)
    } else if h < 22.0 || slope < 0.72 {
        // Скалы и крутые склоны выше линии деревьев.
        (108.0 + v * 22.0, 104.0 + v * 18.0, 98.0 + v * 16.0)
    } else {
        // Снежные вершины.
        (232.0 + v * 16.0, 238.0 + v * 12.0, 244.0 + v * 11.0)
    };

    linear(r, g, b)
}

/// Шейдеры работают в линейном пространстве, а палитра задана в sRGB.
fn linear(r: f32, g: f32, b: f32) -> [f32; 4] {
    fn channel(value: f32) -> f32 {
        let c = (value / 255.0).clamp(0.0, 1.0);
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    [channel(r), channel(g), channel(b), 1.0]
}
