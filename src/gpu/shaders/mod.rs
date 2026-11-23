//! Ready-made WGSL looks for [`Renderer3D`](super::Renderer3D).
//!
//! Each preset is a pair of fragment shaders written for the same bindings as
//! the built-in `lit.wgsl`, so it can be handed to
//! [`Renderer3D::create_material`](super::Renderer3D::create_material) and then
//! used like any other [`Material`](super::Material).

use std::fmt;

/// A built-in stylised shader.
///
/// Presets use the scene's lights, camera and per-draw colour, and work with
/// both plain meshes and textured ones. They are compiled on demand: call
/// [`Renderer3D::create_preset_material`](super::Renderer3D::create_preset_material)
/// once and reuse the returned [`Material`](super::Material) for every frame.
///
/// ```no_run
/// # use frglib::gpu::{Renderer3D, ShaderPreset};
/// # let mut renderer: Renderer3D = unimplemented!();
/// let toon = renderer.create_preset_material(ShaderPreset::Toon);
/// let comic = renderer.create_preset_material(ShaderPreset::Comic);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ShaderPreset {
    /// Three or four hard light bands, a sharp highlight and a dark ink rim.
    #[default]
    Toon,
    /// Posterised colours, printed halftone dots and derivative-based contours.
    Comic,
    /// Palette banding, chunky texture sampling, scanlines and colour fringes.
    Retro,
    /// Tiny palette with ordered dithering; textures snap to a coarse grid.
    Pixel,
    /// Fresnel glow, flickering scanlines and partial alpha.
    Hologram,
    /// Dark tube core with a saturated halo and energy rings.
    Neon,
    /// Sepia image with grain, a scratch and a vignette.
    Film,
    /// Debug view: world-space normals encoded as RGB.
    Normals,
}

impl ShaderPreset {
    /// Every preset, in the order used by the example and the docs.
    pub const ALL: &'static [Self] = &[
        Self::Toon,
        Self::Comic,
        Self::Retro,
        Self::Pixel,
        Self::Hologram,
        Self::Neon,
        Self::Film,
        Self::Normals,
    ];

    /// Short human-readable name, useful for menus and debug overlays.
    pub fn name(self) -> &'static str {
        match self {
            Self::Toon => "toon",
            Self::Comic => "comic",
            Self::Retro => "retro",
            Self::Pixel => "pixel",
            Self::Hologram => "hologram",
            Self::Neon => "neon",
            Self::Film => "film",
            Self::Normals => "normals",
        }
    }

    /// WGSL body to pass to [`Renderer3D::create_material`](super::Renderer3D::create_material).
    ///
    /// The renderer prepends the `lit.wgsl` header with the camera, draw and
    /// vertex structs; this returns the shared helpers plus the two fragment
    /// entry points (`fs_main` and `fs_main_textured`).
    pub fn source(self) -> String {
        let body = match self {
            Self::Toon => include_str!("presets/toon.wgsl"),
            Self::Comic => include_str!("presets/comic.wgsl"),
            Self::Retro => include_str!("presets/retro.wgsl"),
            Self::Pixel => include_str!("presets/pixel.wgsl"),
            Self::Hologram => include_str!("presets/hologram.wgsl"),
            Self::Neon => include_str!("presets/neon.wgsl"),
            Self::Film => include_str!("presets/film.wgsl"),
            Self::Normals => include_str!("presets/normals.wgsl"),
        };
        format!("{}\n{}", include_str!("presets/common.wgsl"), body)
    }
}

impl fmt::Display for ShaderPreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The same header `create_material` adds before compiling a preset.
    fn full_source(preset: ShaderPreset) -> String {
        let header = include_str!("lit.wgsl").split("@vertex").next().unwrap();
        format!("{}\n{}", header, preset.source())
    }

    #[test]
    fn every_preset_is_valid_wgsl() {
        for preset in ShaderPreset::ALL {
            let source = full_source(*preset);
            let module = wgpu::naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
                panic!("{}: {}", preset.name(), error.emit_to_string(&source))
            });
            wgpu::naga::valid::Validator::new(
                wgpu::naga::valid::ValidationFlags::all(),
                wgpu::naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .unwrap_or_else(|error| panic!("{}: {}", preset.name(), error));
        }
    }

    #[test]
    fn every_preset_has_both_fragment_entry_points() {
        for preset in ShaderPreset::ALL {
            let source = full_source(*preset);
            let module = wgpu::naga::front::wgsl::parse_str(&source).unwrap();
            for entry in ["fs_main", "fs_main_textured"] {
                assert!(
                    module
                        .entry_points
                        .iter()
                        .any(|function| function.name == entry),
                    "{} is missing {}",
                    preset.name(),
                    entry
                );
            }
        }
    }
}
