//! Рисование карты и фрогти.

use frglib::{Camera, Draw, Object, Texture, TileMap};

use crate::map::Tile;
use crate::player::Facing;

/// Рисует видимые тайлы карты и игрока поверх них.
pub fn draw_scene(
    d: &mut Draw<'_>,
    map: &TileMap<Tile>,
    tile_tex: &[Texture],
    frog_tex: &[Texture],
    camera: Camera,
    player: &Object,
    facing: Facing,
    bob: f32,
) -> Result<(), frglib::Error> {
    d.set_camera(camera);

    // Рисуются только тайлы в кадре; `texture_of` отдаёт текстуру из
    // нарезки атласа по номеру тайла.
    map.draw(d, camera, |tile| tile.texture().map(|i| tile_tex[i]))?;

    let hero = Object::from_xywh(
        player.position.x,
        player.position.y - bob,
        player.size.x,
        player.size.y,
    );
    d.fill_object_texture(&hero, frog_tex[facing.index()])?;

    Ok(())
}
