//! Colour, kept separate from SDL's own type.

/// An 8-bit-per-channel RGBA colour.
///
/// Deliberately not `sdl3::pixels::Color`: exposing that would tie this crate's
/// public API to a particular version of `sdl3`, so every dependency bump would
/// be a breaking change for whoever depends on us.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Color = Color::rgba(0, 0, 0, 255);
    pub const WHITE: Color = Color::rgba(255, 255, 255, 255);

    pub const RED: Color = Color::rgba(200, 0, 0, 255);
    pub const GREEN: Color = Color::rgba(0, 200, 0, 255);
    pub const BLUE: Color = Color::rgba(0, 0, 200, 255);

    /// Opaque colour, `a = 255`.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color { r, g, b, a }
    }

    pub const fn to_array(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl From<(u8, u8, u8)> for Color {
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Color::rgb(r, g, b)
    }
}

impl From<(u8, u8, u8, u8)> for Color {
    fn from((r, g, b, a): (u8, u8, u8, u8)) -> Self {
        Color::rgba(r, g, b, a)
    }
}

impl From<[u8; 4]> for Color {
    fn from([r, g, b, a]: [u8; 4]) -> Self {
        Color::rgba(r, g, b, a)
    }
}

impl From<Color> for sdl3::pixels::Color {
    fn from(color: Color) -> Self {
        sdl3::pixels::Color {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three named colours are the one thing in this file a reader reaches
    /// for by reflex, and they were all the same green at one point -- shipping
    /// `Color::RED` that renders green. Worth pinning down.
    #[test]
    fn named_colours_point_where_they_say() {
        assert_eq!(Color::RED, Color::rgb(200, 0, 0));
        assert_eq!(Color::GREEN, Color::rgb(0, 200, 0));
        assert_eq!(Color::BLUE, Color::rgb(0, 0, 200));
    }

    #[test]
    fn black_and_white_are_the_extremes() {
        assert_eq!(Color::BLACK, Color::rgb(0, 0, 0));
        assert_eq!(Color::WHITE, Color::rgb(255, 255, 255));
    }

    #[test]
    fn the_named_colours_are_all_opaque() {
        for colour in [
            Color::RED,
            Color::GREEN,
            Color::BLUE,
            Color::BLACK,
            Color::WHITE,
        ] {
            assert_eq!(colour.a, 255, "{colour:?} should be opaque");
        }
    }

    #[test]
    fn the_named_colours_are_all_different() {
        let all = [
            Color::RED,
            Color::GREEN,
            Color::BLUE,
            Color::BLACK,
            Color::WHITE,
        ];

        for (i, one) in all.iter().enumerate() {
            for other in &all[i + 1..] {
                assert_ne!(one, other, "{one:?} and {other:?} are the same");
            }
        }
    }

    #[test]
    fn rgb_is_opaque_and_rgba_is_not() {
        assert_eq!(Color::rgb(1, 2, 3).a, 255);
        assert_eq!(Color::rgba(1, 2, 3, 4).a, 4);
    }

    #[test]
    fn channels_survive_a_round_trip_through_an_array() {
        let colour = Color::rgba(10, 20, 30, 40);
        assert_eq!(colour.to_array(), [10, 20, 30, 40]);
        assert_eq!(Color::from(colour.to_array()), colour);
    }

    #[test]
    fn tuples_and_triples_convert() {
        assert_eq!(Color::from((1, 2, 3)), Color::rgb(1, 2, 3));
        assert_eq!(Color::from((1, 2, 3, 4)), Color::rgba(1, 2, 3, 4));
        // The three-channel form cannot carry alpha, so it comes out opaque.
        assert_eq!(Color::from((1, 2, 3)).a, 255);
    }

    #[test]
    fn converting_to_the_sdl_type_keeps_every_channel() {
        let sdl: sdl3::pixels::Color = Color::rgba(10, 20, 30, 40).into();
        let (r, g, b, a) = sdl.rgba();
        assert_eq!((r, g, b, a), (10, 20, 30, 40));
    }
}
