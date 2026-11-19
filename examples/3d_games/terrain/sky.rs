//! Процедурное небо для сцены: градиент, тёплый горизонт и солнце.

use std::f32::consts::{PI, TAU};

use frglib::gpu::glam::Vec3;
use frglib::gpu::{Gpu, Skybox};

/// Строит кубическое небо из панорамы 2:1 без единого файла с текстурой.
pub fn create(gpu: &Gpu) -> Result<Skybox, frglib::gpu::Error> {
    let (width, height) = (1024, 512);
    let pixels = panorama(width, height);
    Skybox::from_equirectangular_rgba8(gpu, width, height, &pixels)
}

fn panorama(width: u32, height: u32) -> Vec<u8> {
    let sun = Vec3::new(0.55, 0.28, -0.78).normalize();
    let horizon = Vec3::new(0.88, 0.82, 0.72);
    let zenith = Vec3::new(0.16, 0.36, 0.68);
    let ground = Vec3::new(0.10, 0.14, 0.18);
    let sun_color = Vec3::new(1.0, 0.90, 0.72);

    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
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

            let mut color = if elevation >= 0.0 {
                let t = (elevation / (PI * 0.5)).clamp(0.0, 1.0);
                horizon.lerp(zenith, t.powf(0.75))
            } else {
                let t = (-elevation / (PI * 0.5)).clamp(0.0, 1.0);
                horizon.lerp(ground, t.powf(0.5))
            };

            let sun_dot = direction.dot(sun).max(0.0);
            color += sun_color * (sun_dot.powf(320.0) * 1.6 + sun_dot.powf(14.0) * 0.16);

            pixels.extend_from_slice(&[
                (color.x.clamp(0.0, 1.0) * 255.0).round() as u8,
                (color.y.clamp(0.0, 1.0) * 255.0).round() as u8,
                (color.z.clamp(0.0, 1.0) * 255.0).round() as u8,
                255,
            ]);
        }
    }

    pixels
}
