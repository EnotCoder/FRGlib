//! The shape an interpolation takes.

/// How far along its curve a weight lands, before anything is interpolated.
///
/// [`lerp`](crate::lerp) takes this as its fourth argument, the way Godot's
/// tweens take a transition: [`Ease::Linear`](Ease::Linear) covers the distance
/// at a constant rate, and the easings added later will instead start slow and
/// finish fast, or the other way round.
///
/// ```
/// use frglib::{lerp, Ease};
///
/// // Same weight, same endpoints: only the shape of the walk differs.
/// assert_eq!(lerp(0.0f32, 10.0, 0.5, Ease::Linear), 5.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Ease {
    /// A straight line: the weight is used exactly as it came.
    #[default]
    Linear,
}

impl Ease {
    /// Bends `t` the way this easing says, ready to be used as a weight.
    ///
    /// [`Ease::Linear`](Ease::Linear) hands `t` back untouched, so today this
    /// costs nothing and every later easing has one place to hook into.
    pub fn apply(self, t: f32) -> f32 {
        match self {
            Ease::Linear => t,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_passes_the_weight_through_unchanged() {
        assert_eq!(Ease::Linear.apply(0.0), 0.0);
        assert_eq!(Ease::Linear.apply(0.25), 0.25);
        assert_eq!(Ease::Linear.apply(1.0), 1.0);
    }

    /// A weight outside `0..=1` is not a special case: linear means linear
    /// there too, so extrapolation reaches the same far side of the endpoints.
    #[test]
    fn linear_hands_back_weights_outside_the_range_as_well() {
        assert_eq!(Ease::Linear.apply(-1.0), -1.0);
        assert_eq!(Ease::Linear.apply(2.0), 2.0);
    }

    #[test]
    fn linear_is_what_you_get_by_default() {
        assert_eq!(Ease::default(), Ease::Linear);
    }
}
