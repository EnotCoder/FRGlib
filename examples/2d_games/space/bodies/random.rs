/// Простой линейный генератор случайных чисел.
///
/// Пояса и звёзды должны выглядеть одинаково при каждом запуске, поэтому
/// общий генератор живёт здесь, а не дублируется в каждом модуле.
pub struct Random {
    seed: u32,
}

impl Random {
    pub fn new(seed: u32) -> Self {
        Self { seed }
    }

    /// Следующее число в диапазоне `0.0..1.0`.
    pub fn next(&mut self) -> f32 {
        self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);

        (self.seed >> 8) as f32 / 16_777_216.0
    }

    pub fn range(&mut self, min: f32, max: f32) -> f32 {
        min + self.next() * (max - min)
    }
}
