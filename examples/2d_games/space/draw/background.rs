use frglib::{Color, Draw, Ease, Lerp, Object, Vec2};

/// Мягкий градиент вместо плоской заливки: сверху почти чёрный космос,
/// снизу -- чуть более тёплый фиолетовый.
pub fn draw_background(d: &mut Draw, size: Vec2) -> Result<(), frglib::Error> {
    const BANDS: usize = 48;

    let top = Color::rgb(7, 9, 20);
    let bottom = Color::rgb(24, 16, 30);

    let band_height = size.y / BANDS as f32;

    for band in 0..BANDS {
        let t = (band as f32 + 0.5) / BANDS as f32;
        let color = top.lerp(bottom, t, Ease::Linear);

        // +1 пиксель перекрытия, чтобы между полосами не было шва.
        let rect = Object::from_xywh(
            size.x * 0.5,
            band_height * (band as f32 + 0.5),
            size.x,
            band_height + 1.0,
        );

        d.fill_object(&rect, color)?;
    }

    Ok(())
}
