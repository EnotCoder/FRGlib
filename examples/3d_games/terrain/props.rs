//! Простые деревья, разбросанные по ландшафту.
//!
//! Позиции зависят только от seed: при перегенерации рельеф и деревья меняются
//! вместе, а при одном и том же seed сцена получается одинаковой. Деревья
//! сажаются на пологих участках выше воды. Геометрия процедурная: конус для
//! кроны, куб для ствола, без единого внешнего файла.

use std::f32::consts::TAU;

use frglib::gpu::glam::{Mat4, Vec3};
use frglib::gpu::{GpuMesh, Material, Mesh, Renderer3D, Vertex};
use frglib::Color;

use crate::terrain::{Terrain, SIZE};

const TREE_ATTEMPTS: usize = 4000;

struct Tree {
    pos: Vec3,
    height: f32,
    yaw: f32,
    foliage: Color,
}

/// Деревья на поверхности плюс меш кроны для отрисовки.
pub struct Props {
    trees: Vec<Tree>,
    cone: GpuMesh,
}

impl Props {
    /// Рассыпает деревья по уже сгенерированному рельефу.
    pub fn generate(renderer: &Renderer3D, terrain: &Terrain, seed: u64) -> Self {
        let mut rng = Rng::new(seed ^ 0xA5A5_5A5A_A5A5_5A5A);
        let half = SIZE * 0.5 - 6.0;
        let mut trees = Vec::new();

        // Деревья: невысокая трава, пологий склон, достаточно далеко от воды.
        for _ in 0..TREE_ATTEMPTS {
            if trees.len() >= 190 {
                break;
            }
            let x = rng.range(-half, half);
            let z = rng.range(-half, half);
            let h = terrain.height_at(x, z);
            let slope = terrain.slope_at(x, z);

            if h < 2.2 || h > 10.5 || slope > 0.55 {
                continue;
            }
            // Немного разреживаем деревья, чтобы не росли сплошным лесом.
            if rng.next_f32() > 0.55 {
                continue;
            }

            trees.push(Tree {
                pos: Vec3::new(x, h - 0.1, z),
                height: rng.range(3.0, 5.8),
                yaw: rng.range(0.0, TAU),
                foliage: Color::rgb(
                    (48.0 + rng.range(0.0, 26.0)) as u8,
                    (104.0 + rng.range(0.0, 32.0)) as u8,
                    (52.0 + rng.range(0.0, 20.0)) as u8,
                ),
            });
        }

        Props {
            trees,
            // Пятигранный конус: грани хорошо видны, сглаживания нет.
            cone: renderer.create_mesh(&cone_mesh(5)),
        }
    }

    /// Отрисовывает все деревья в сцене.
    pub fn draw(&self, scene: &mut Renderer3D) {
        for tree in &self.trees {
            let trunk_h = tree.height * 0.34;
            let trunk_w = tree.height * 0.085;
            scene.draw_cube(
                tree.pos + Vec3::Y * (trunk_h * 0.5),
                Vec3::new(trunk_w, trunk_h, trunk_w),
                Color::rgb(96, 68, 44),
            );

            // Два конуса дают силуэт ели; крона повёрнута случайным образом,
            // чтобы соседние деревья не выглядели копиями.
            let lower = Mat4::from_translation(tree.pos + Vec3::Y * (trunk_h * 0.75))
                * Mat4::from_rotation_y(tree.yaw)
                * Mat4::from_scale(Vec3::new(
                    tree.height * 0.62,
                    tree.height * 0.78,
                    tree.height * 0.62,
                ));
            scene.draw_mesh_with(&self.cone, lower, Material::Lit, tree.foliage);

            let upper = Mat4::from_translation(tree.pos + Vec3::Y * (trunk_h + tree.height * 0.30))
                * Mat4::from_rotation_y(tree.yaw + 0.6)
                * Mat4::from_scale(Vec3::new(
                    tree.height * 0.46,
                    tree.height * 0.62,
                    tree.height * 0.46,
                ));
            let lighter = Color::rgb(
                tree.foliage.r.saturating_add(12),
                tree.foliage.g.saturating_add(14),
                tree.foliage.b.saturating_add(10),
            );
            scene.draw_mesh_with(&self.cone, upper, Material::Lit, lighter);
        }
    }
}

/// Конус с основанием в `y = 0` и вершиной в `y = 1`.
/// У каждой боковой грани свои вершины и своя нормаль — flat shading.
fn cone_mesh(segments: usize) -> Mesh {
    let radius = 0.5;
    let tip = Vec3::new(0.0, 1.0, 0.0);
    let mut vertices = Vec::with_capacity(segments * 3);
    let mut indices = Vec::with_capacity(segments * 3);

    for i in 0..segments {
        let a = i as f32 / segments as f32 * TAU;
        let b = (i + 1) as f32 / segments as f32 * TAU;
        let base_a = Vec3::new(a.cos() * radius, 0.0, a.sin() * radius);
        let base_b = Vec3::new(b.cos() * radius, 0.0, b.sin() * radius);
        let normal = (tip - base_a).cross(base_b - base_a).normalize();
        let base = vertices.len() as u32;

        for point in [base_a, tip, base_b] {
            vertices.push(Vertex::new(point.to_array(), normal.to_array()));
        }
        indices.extend_from_slice(&[base, base + 1, base + 2]);
    }

    Mesh::from_vertices(vertices, indices)
}

/// Детерминированный генератор: тот же seed даёт те же позиции.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.next_f32()
    }
}
