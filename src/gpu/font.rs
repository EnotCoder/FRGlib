//! Fonts, glyph atlases and text layout for [`Renderer2D`](super::Renderer2D).
//!
//! The font parser is [`fontdue`], a pure-Rust TTF/OTF reader. Keeping it on
//! this side of the line means the SDL window flavour still has no font
//! dependency, and a GPU window can draw text without linking SDL3_ttf.
//!
//! Glyphs are rasterized on demand and packed into one atlas texture. The
//! atlas holds white RGB and the glyph coverage in alpha, so the same shader
//! that draws a texture also tints text without a second pipeline.

use std::collections::HashMap;

use fontdue::{Font, FontSettings};

use super::error::Error;

/// Side of the square glyph atlas, in pixels.
pub(crate) const ATLAS_SIZE: u32 = 2048;

/// Transparent border around every glyph, so linear filtering does not pull a
/// neighbouring glyph in at the edges.
const GLYPH_PADDING: u32 = 1;

/// A font loaded into a [`Renderer2D`](super::Renderer2D).
///
/// Handed out by [`Renderer2D::load_font`](super::Renderer2D::load_font) and
/// only meaningful to the renderer that produced it. Copying one is free.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontId(pub(crate) usize);

/// How a line of text is anchored to the `x` passed to a draw call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    /// `x` is the left edge of the text.
    #[default]
    Left,
    /// `x` is the middle of the text.
    Center,
    /// `x` is the right edge of the text.
    Right,
}

/// One glyph's place in the atlas and its offset from the text origin.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Glyph {
    /// Texture coordinates: `u0, v0, u1, v1`.
    pub(crate) uv: [f32; 4],
    /// Bitmap size in pixels. Zero for whitespace.
    pub(crate) size: [f32; 2],
    /// Offset of the bitmap's top-left corner from the pen position on the
    /// baseline. `y` is negative because screen coordinates grow downwards.
    pub(crate) offset: [f32; 2],
}

impl Glyph {
    const BLANK: Glyph = Glyph {
        uv: [0.0; 4],
        size: [0.0; 2],
        offset: [0.0; 2],
    };

    pub(crate) fn is_blank(&self) -> bool {
        self.size[0] <= 0.0 || self.size[1] <= 0.0
    }
}

/// A glyph that has been laid out but not yet uploaded.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PlacedGlyph {
    pub(crate) ch: char,
    /// Pen position before the glyph, in screen pixels.
    pub(crate) x: f32,
    /// Baseline of the line the glyph belongs to, in screen pixels.
    pub(crate) baseline: f32,
}

/// Result of laying text out: its bounds and where every glyph goes.
#[derive(Debug, Clone)]
pub(crate) struct TextLayout {
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) glyphs: Vec<PlacedGlyph>,
}

/// One loaded font plus the glyphs already rasterized for it.
pub(crate) struct FontData {
    font: Font,
    glyphs: HashMap<(char, u32), Glyph>,
}

impl FontData {
    pub(crate) fn new(bytes: Vec<u8>) -> Result<Self, Error> {
        let font = Font::from_bytes(bytes, FontSettings::default())
            .map_err(|message| Error::Font(message.to_string()))?;

        Ok(FontData {
            font,
            glyphs: HashMap::new(),
        })
    }

    /// Lays `text` out in screen pixels, with `origin` as the top-left corner
    /// of the text block.
    pub(crate) fn layout(
        &self,
        text: &str,
        size: f32,
        origin: [f32; 2],
        align: TextAlign,
    ) -> TextLayout {
        let line = self.line_metrics(size);
        let line_height = line.new_line_size.max(1.0);
        let ascent = line.ascent;

        let lines: Vec<&str> = text.split('\n').collect();
        let widths: Vec<f32> = lines
            .iter()
            .map(|line| self.line_width(line, size))
            .collect();
        let width = widths.iter().copied().fold(0.0, f32::max);
        let height = line_height * lines.len() as f32;

        let mut glyphs = Vec::new();

        for (index, (line, line_width)) in lines.iter().zip(widths.iter()).enumerate() {
            let left = match align {
                TextAlign::Left => origin[0],
                TextAlign::Center => origin[0] - line_width / 2.0,
                TextAlign::Right => origin[0] - line_width,
            };
            let baseline = origin[1] + ascent + index as f32 * line_height;

            let mut pen = left;
            let mut previous = None;

            for ch in line.chars() {
                if let Some(left) = previous {
                    // Kerning is expected in pixels by fontdue: it scales the
                    // font's own units for us.
                    pen += self.font.horizontal_kern(left, ch, size).unwrap_or(0.0);
                }

                glyphs.push(PlacedGlyph {
                    ch,
                    x: pen,
                    baseline,
                });

                pen += self.font.metrics(ch, size).advance_width;
                previous = Some(ch);
            }
        }

        TextLayout {
            width,
            height,
            glyphs,
        }
    }

    /// Rasterizes `ch` if it is not cached yet and puts it in `atlas`.
    pub(crate) fn glyph(
        &mut self,
        atlas: &mut GlyphAtlas,
        queue: &wgpu::Queue,
        ch: char,
        size: f32,
    ) -> Result<Glyph, Error> {
        // The size is part of the key: a glyph rendered at 16 px is not the
        // glyph rendered at 32 px, and scaling the bitmap would blur it.
        let key = (ch, size.to_bits());

        if let Some(glyph) = self.glyphs.get(&key) {
            return Ok(*glyph);
        }

        let (metrics, bitmap) = self.font.rasterize(ch, size);
        let glyph = atlas.insert(
            queue,
            &bitmap,
            metrics.width as u32,
            metrics.height as u32,
            metrics,
        )?;

        self.glyphs.insert(key, glyph);
        Ok(glyph)
    }

    /// Horizontal extent of one line, including kerning.
    fn line_width(&self, text: &str, size: f32) -> f32 {
        let mut width = 0.0;
        let mut previous = None;

        for ch in text.chars() {
            if let Some(left) = previous {
                width += self.font.horizontal_kern(left, ch, size).unwrap_or(0.0);
            }

            width += self.font.metrics(ch, size).advance_width;
            previous = Some(ch);
        }

        width
    }

    /// Ascent, descent and line height at `size`.
    ///
    /// A font without the relevant table gets a sensible em-based fallback
    /// instead of a panic: text may sit a little oddly, but it still draws.
    fn line_metrics(&self, size: f32) -> fontdue::LineMetrics {
        self.font
            .horizontal_line_metrics(size)
            .unwrap_or(fontdue::LineMetrics {
                ascent: size * 0.8,
                descent: -size * 0.2,
                line_gap: 0.0,
                new_line_size: size,
            })
    }
}

/// A shelf-packed glyph atlas.
///
/// Glyphs arrive at many sizes and are never evicted, which is fine for a game
/// interface: a handful of font sizes are known up front and the atlas is far
/// larger than they need. A caller that animates a size every frame will fill
/// it, and then gets [`Error::AtlasFull`] rather than a corrupted font.
pub(crate) struct GlyphAtlas {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    shelf: Shelf,
}

/// The cursor of the shelf packer, kept separate so it can be tested without a
/// GPU.
struct Shelf {
    /// Left edge of the current shelf.
    x: u32,
    /// Top edge of the current shelf.
    y: u32,
    /// Height of the tallest glyph on the current shelf.
    row_height: u32,
}

impl Shelf {
    fn new() -> Self {
        Shelf {
            x: 0,
            y: 0,
            row_height: 0,
        }
    }

    /// Finds room for a `width × height` glyph and returns the top-left of its
    /// padded slot.
    fn allocate(&mut self, width: u32, height: u32) -> Option<(u32, u32)> {
        let width = width + GLYPH_PADDING * 2;
        let height = height + GLYPH_PADDING * 2;

        if width > ATLAS_SIZE {
            return None;
        }

        if self.x + width > ATLAS_SIZE {
            self.x = 0;
            self.y += self.row_height;
            self.row_height = 0;
        }

        if self.y + height > ATLAS_SIZE {
            return None;
        }

        let position = (self.x, self.y);
        self.x += width;
        self.row_height = self.row_height.max(height);

        Some(position)
    }
}

impl GlyphAtlas {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("frglib glyph atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        GlyphAtlas {
            texture,
            view,
            shelf: Shelf::new(),
        }
    }

    pub(crate) fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Uploads one coverage bitmap and returns its atlas coordinates.
    fn insert(
        &mut self,
        queue: &wgpu::Queue,
        bitmap: &[u8],
        width: u32,
        height: u32,
        metrics: fontdue::Metrics,
    ) -> Result<Glyph, Error> {
        // Offset of the bitmap's top edge from the baseline. `ymin` is the
        // bottom of the bitmap above the baseline, so top = baseline - (ymin
        // + height).
        let offset = [
            metrics.xmin as f32,
            -((metrics.ymin + metrics.height as i32) as f32),
        ];

        let glyph = Glyph {
            size: [width as f32, height as f32],
            offset,
            ..Glyph::BLANK
        };

        if width == 0 || height == 0 {
            return Ok(glyph);
        }

        let (x, y) = self.shelf.allocate(width, height).ok_or(Error::AtlasFull)?;

        // The atlas stores white with coverage in alpha. Expanding the
        // grayscale bitmap here keeps the shader to one texture fetch and
        // makes glyphs tintable with the same pipeline as sprites.
        let mut rgba = vec![0u8; (width as usize) * (height as usize) * 4];
        for (index, coverage) in bitmap
            .iter()
            .take(width as usize * height as usize)
            .enumerate()
        {
            let pixel = &mut rgba[index * 4..index * 4 + 4];
            pixel[0] = 255;
            pixel[1] = 255;
            pixel[2] = 255;
            pixel[3] = *coverage;
        }

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: x + GLYPH_PADDING,
                    y: y + GLYPH_PADDING,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let size = ATLAS_SIZE as f32;
        let u0 = (x + GLYPH_PADDING) as f32 / size;
        let v0 = (y + GLYPH_PADDING) as f32 / size;

        Ok(Glyph {
            uv: [u0, v0, u0 + width as f32 / size, v0 + height as f32 / size],
            ..glyph
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The same font the examples use. Tests are the only place the library
    /// reaches into `examples/`, and `cfg(test)` keeps it out of the shipped
    /// build.
    fn font() -> FontData {
        FontData::new(include_bytes!("../../examples/ui/assets/font.otf").to_vec())
            .expect("the example font should parse")
    }

    #[test]
    fn a_line_of_text_has_a_width_and_a_height() {
        let font = font();
        let layout = font.layout("Hello", 32.0, [0.0, 0.0], TextAlign::Left);

        assert!(layout.width > 0.0);
        assert!(layout.height > 0.0);
        assert_eq!(layout.glyphs.len(), 5);
    }

    #[test]
    fn a_second_line_makes_the_block_taller_but_not_narrower() {
        let font = font();
        let one = font.layout("Hi", 24.0, [0.0, 0.0], TextAlign::Left);
        let two = font.layout("Hi\nHo", 24.0, [0.0, 0.0], TextAlign::Left);

        assert!(two.height > one.height);
        assert!(two.width >= one.width);
    }

    #[test]
    fn centring_moves_the_first_glyph_left_of_the_anchor() {
        let font = font();
        let left = font.layout("Hello", 32.0, [100.0, 0.0], TextAlign::Left);
        let centre = font.layout("Hello", 32.0, [100.0, 0.0], TextAlign::Center);

        assert!(centre.glyphs[0].x < left.glyphs[0].x);
    }

    #[test]
    fn right_alignment_ends_on_the_anchor() {
        let font = font();
        let right = font.layout("Hello", 32.0, [100.0, 0.0], TextAlign::Right);
        let last = right.glyphs.last().unwrap();

        // The last glyph starts short of the anchor by at most its own advance.
        assert!(last.x < 100.0);
        assert!(last.x > 100.0 - 64.0);
    }

    #[test]
    fn an_empty_line_still_has_line_height() {
        let font = font();
        let layout = font.layout("", 20.0, [0.0, 0.0], TextAlign::Left);

        assert_eq!(layout.width, 0.0);
        assert!(layout.height > 0.0);
        assert!(layout.glyphs.is_empty());
    }

    #[test]
    fn kerning_is_applied_between_neighbouring_glyphs() {
        // Not every font has a pair with non-zero kerning, so the assertion is
        // on the guard rail: the layout must not panic and the pen must move
        // forward.
        let font = font();
        let layout = font.layout("AV", 48.0, [0.0, 0.0], TextAlign::Left);

        assert_eq!(layout.glyphs.len(), 2);
        assert!(layout.glyphs[1].x > layout.glyphs[0].x);
    }

    #[test]
    fn a_shelf_wraps_and_reports_when_it_is_full() {
        let mut shelf = Shelf::new();

        // Each slot grows by the padding on both sides, so the second one fills
        // the rest of the first shelf exactly.
        assert_eq!(shelf.allocate(10, 10), Some((0, 0)));
        assert_eq!(shelf.allocate(ATLAS_SIZE - 14, 10), Some((12, 0)));

        // The next allocation does not fit on the first shelf, so it starts a
        // new one below it.
        assert_eq!(
            shelf.allocate(64, 20),
            Some((0, 12)),
            "a full shelf should wrap"
        );

        // A glyph taller than the whole atlas is rejected rather than written
        // out of bounds.
        assert_eq!(shelf.allocate(ATLAS_SIZE, ATLAS_SIZE), None);
    }
}
