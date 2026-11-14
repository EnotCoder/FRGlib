mod belt;
mod moon;
mod planet;
mod pose;
mod random;
mod stars;

pub use belt::*;
pub use moon::*;
pub use planet::*;
pub use pose::*;
pub use random::Random;
pub use stars::*;

/// Насколько орбиты сплющены по вертикали.
///
/// Круги в космосе выглядят плоской схемой; эллипс с наклоном читается
/// как орбита, которую мы видим чуть сбоку, и сразу даёт сцене глубину.
pub const ORBIT_TILT: f32 = 0.38;
