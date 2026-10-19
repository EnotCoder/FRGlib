//! Text placed on screen: where it goes, and how not to render it twice.

use std::collections::HashMap;

use crate::graphics::{Image, Object};
use crate::math::{Rect, Vec2};
use crate::window::{Draw, Error};

/// Which point of a [`Label`] its position refers to.
///
/// Every label is placed by one corner or edge, or by its centre. Centring is
/// the default because that is what a caption on a screen usually does: the
/// caller says where the middle of the text goes and does not have to measure
/// the text first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Anchor {
    /// The centre of the text.
    #[default]
    Center,
    /// The top-left corner.
    TopLeft,
    /// The middle of the top edge.
    Top,
    /// The top-right corner.
    TopRight,
    /// The middle of the left edge.
    Left,
    /// The middle of the right edge.
    Right,
    /// The bottom-left corner.
    BottomLeft,
    /// The middle of the bottom edge.
    Bottom,
    /// The bottom-right corner.
    BottomRight,
}

/// The box `size` covers once `anchor` is pinned to `position`.
///
/// Returns the top-left corner, since that is what a [`Rect`] carries. Kept
/// free of any `Image` so the whole placement rule is testable without a
/// window, an SDL initialisation or a font.
fn place(anchor: Anchor, position: Vec2, size: (f32, f32)) -> Rect {
    let (w, h) = size;

    let (dx, dy) = match anchor {
        Anchor::Center => (-w / 2.0, -h / 2.0),
        Anchor::TopLeft => (0.0, 0.0),
        Anchor::Top => (-w / 2.0, 0.0),
        Anchor::TopRight => (-w, 0.0),
        Anchor::Left => (0.0, -h / 2.0),
        Anchor::Right => (-w, -h / 2.0),
        Anchor::BottomLeft => (0.0, -h),
        Anchor::Bottom => (-w / 2.0, -h),
        Anchor::BottomRight => (-w, -h),
    };

    Rect::new(position.x + dx, position.y + dy, w, h)
}

/// A line of already rendered text, put somewhere on screen.
///
/// The label holds an [`Image`] rather than a font, for the same reason a
/// [`Button`](crate::Button) does: this crate has never made anybody pay for a
/// text parser they did not ask for, so rendering stays on the caller's side
/// and `frglib` only works out where the result goes.
///
/// ```no_run
/// # fn main() -> Result<(), frglib::Error> {
/// # let image = frglib::Image::from_surface(unimplemented!());
/// use frglib::Label;
///
/// let hello = Label::new(&image).at(360.0, 240.0);
///
/// // ...inside the frame closure:
/// // hello.draw(d)?;
/// # Ok(())
/// # }
/// ```
///
/// Coordinates are screen ones. A [`Draw`] carrying a camera would drag the
/// text around with the world, so [`draw`](Self::draw) takes the camera off
/// for the length of the call and puts it back afterwards -- the same thing
/// [`Button`](crate::Button) and [`Slider`](crate::Slider) do.
pub struct Label<'a> {
    image: &'a Image,
    position: Vec2,
    anchor: Anchor,
}

impl<'a> Label<'a> {
    /// A label showing `image`, centred on the origin until it is moved.
    ///
    /// The image is borrowed rather than owned: `sdl3`'s surfaces are not
    /// `Clone`, and a caller that already has the picture should not have to
    /// hand it over to say where it goes.
    pub fn new(image: &'a Image) -> Self {
        Label {
            image,
            position: Vec2::ZERO,
            anchor: Anchor::default(),
        }
    }

    /// Moves the anchored point to `(x, y)`.
    pub fn at(mut self, x: f32, y: f32) -> Self {
        self.position = Vec2::new(x, y);
        self
    }

    /// Changes which point of the text the position names.
    ///
    /// Defaults to [`Anchor::Center`].
    pub fn anchored(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// Width and height of the text, in pixels.
    pub fn size(&self) -> (u32, u32) {
        self.image.size()
    }

    /// The area the text covers, in screen pixels.
    pub fn rect(&self) -> Rect {
        let (w, h) = self.size();
        place(self.anchor, self.position, (w as f32, h as f32))
    }

    /// Draws the text at its position.
    ///
    /// The image is uploaded to the GPU on every call, so this costs a texture
    /// upload per frame -- the same trade [`Draw::fill_textured_object`] makes.
    pub fn draw(&self, d: &mut Draw<'_>) -> Result<(), Error> {
        let rect = self.rect();

        d.screen_space(|d| {
            d.fill_textured_object(
                &Object::from_xywh(
                    rect.x + rect.w / 2.0,
                    rect.y + rect.h / 2.0,
                    rect.w,
                    rect.h,
                ),
                self.image,
            )
        })
    }
}

/// Rendered strings, remembered by their text.
///
/// A frame loop draws the same captions sixty times a second and re-renders
/// the font for every one of them, which is work SDL3_ttf has already done.
/// This keeps the results in a [`HashMap`] keyed by the string itself, so a
/// caption that did not change is not rendered again.
///
/// The rendering comes in as a closure, which is what keeps fonts out of this
/// crate: the cache stores [`Image`]s and never sees a font, a `Surface` or
/// SDL3_ttf.
///
/// ```no_run
/// # fn main() -> Result<(), frglib::Error> {
/// use frglib::TextCache;
///
/// // However the caller renders text: SDL3_ttf, ab_glyph, anything at all.
/// // The cache only ever sees the resulting `Image`.
/// # fn render(text: &str) -> Result<frglib::Image, frglib::Error> {
/// #     let _ = text;
/// #     frglib::Image::from_file("score.png")
/// # }
/// let mut cache = TextCache::new(render);
///
/// // Before the frame, and all at once: the cache has to be filled while
/// // nothing else holds a reference to it.
/// cache.keep("Очки: 120")?;
///
/// // ...inside the frame closure:
/// // let score = cache.get("Очки: 120");
/// # Ok(())
/// # }
/// ```
pub struct TextCache<R> {
    images: HashMap<String, Image>,
    render: R,
}

impl<R, E> TextCache<R>
where
    R: FnMut(&str) -> Result<Image, E>,
{
    /// A cache that renders through `render`.
    pub fn new(render: R) -> Self {
        TextCache {
            images: HashMap::new(),
            render,
        }
    }

    /// Renders `text` unless it is already in the cache.
    ///
    /// Two calls, because of how borrows work: several strings have to be in
    /// the cache before the frame starts, and holding several references to
    /// them at once is only possible from shared borrows. So this one takes
    /// the cache mutably and returns nothing, and [`get`](Self::get) takes it
    /// shared and hands back the picture.
    pub fn keep(&mut self, text: &str) -> Result<(), E> {
        if self.images.contains_key(text) {
            return Ok(());
        }

        let image = (self.render)(text)?;
        self.images.insert(text.to_string(), image);
        Ok(())
    }

    /// The picture `text` was rendered to.
    ///
    /// # Panics
    ///
    /// Panics if [`keep`](Self::keep) has not been called for this text. The
    /// cache is filled ahead of the frame precisely so that the frame itself
    /// needs no `Result`, and a missing string there is a programming error
    /// rather than something to recover from.
    pub fn get(&self, text: &str) -> &Image {
        self.images
            .get(text)
            .unwrap_or_else(|| panic!("TextCache: {text:?} was never kept"))
    }

    /// Forgets every rendered string.
    ///
    /// A cache of every score a counter has ever shown grows without bound;
    /// this is how a caller that wants a fresh start gets one.
    pub fn clear(&mut self) {
        self.images.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A text of a known size, so placement can be checked against numbers.
    const SIZE: (f32, f32) = (100.0, 40.0);

    /// Where `(x, y)` puts a `SIZE` text under each anchor, worked out by hand.
    #[test]
    fn every_anchor_lands_where_it_says() {
        let at = Vec2::new(200.0, 100.0);

        let cases = [
            (Anchor::Center, Rect::new(150.0, 80.0, 100.0, 40.0)),
            (Anchor::TopLeft, Rect::new(200.0, 100.0, 100.0, 40.0)),
            (Anchor::Top, Rect::new(150.0, 100.0, 100.0, 40.0)),
            (Anchor::TopRight, Rect::new(100.0, 100.0, 100.0, 40.0)),
            (Anchor::Left, Rect::new(200.0, 80.0, 100.0, 40.0)),
            (Anchor::Right, Rect::new(100.0, 80.0, 100.0, 40.0)),
            (Anchor::BottomLeft, Rect::new(200.0, 60.0, 100.0, 40.0)),
            (Anchor::Bottom, Rect::new(150.0, 60.0, 100.0, 40.0)),
            (Anchor::BottomRight, Rect::new(100.0, 60.0, 100.0, 40.0)),
        ];

        for (anchor, want) in cases {
            assert_eq!(place(anchor, at, SIZE), want, "anchor {anchor:?}");
        }
    }

    /// The anchored point really is where the caller put it: reading the box
    /// back through the anchor has to give the position again.
    #[test]
    fn the_anchored_point_is_the_position() {
        let position = Vec2::new(320.0, 180.0);

        for anchor in [
            Anchor::Center,
            Anchor::TopLeft,
            Anchor::Top,
            Anchor::TopRight,
            Anchor::Left,
            Anchor::Right,
            Anchor::BottomLeft,
            Anchor::Bottom,
            Anchor::BottomRight,
        ] {
            let rect = place(anchor, position, SIZE);

            // The point the anchor names, read back out of the box.
            let got = match anchor {
                Anchor::Center => (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0),
                Anchor::TopLeft => (rect.x, rect.y),
                Anchor::Top => (rect.x + rect.w / 2.0, rect.y),
                Anchor::TopRight => (rect.x + rect.w, rect.y),
                Anchor::Left => (rect.x, rect.y + rect.h / 2.0),
                Anchor::Right => (rect.x + rect.w, rect.y + rect.h / 2.0),
                Anchor::BottomLeft => (rect.x, rect.y + rect.h),
                Anchor::Bottom => (rect.x + rect.w / 2.0, rect.y + rect.h),
                Anchor::BottomRight => (rect.x + rect.w, rect.y + rect.h),
            };

            assert_eq!(got, (position.x, position.y), "anchor {anchor:?}");
        }
    }

    /// A label is the size of its image, wherever it is put.
    #[test]
    fn the_box_keeps_the_texts_own_size() {
        for anchor in [Anchor::Center, Anchor::TopLeft, Anchor::BottomRight] {
            let rect = place(anchor, Vec2::ZERO, SIZE);
            assert_eq!((rect.w, rect.h), SIZE);
        }
    }
}
