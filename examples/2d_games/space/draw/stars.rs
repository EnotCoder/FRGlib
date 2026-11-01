use frglib::{
    Object, Vec2,
};

use crate::bodies::*;

pub fn draw_stars(
    d: &mut frglib::Draw, 
    stars: &[Star],
    window_size: &Vec2,
) -> Result<(), Box<dyn std::error::Error>> {
    for star in stars {
        let position = Vec2::new(
            star.position.x * window_size.x,
            star.position.y * window_size.y,
        );

        let star_obj = Object::new(
            position,
            Vec2::new(star.size, star.size),
        );

        d.fill_object(&star_obj, star.color)?;
    }

    Ok(())
}