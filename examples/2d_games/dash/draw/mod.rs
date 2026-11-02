//! Рисование мира и куба.
//!
//! Всё, что знает про внешний вид уровня -- небо, земля, тайлы и спрайт
//! игрока, -- живёт здесь. `main` только решает, что и когда рисовать.

use frglib::{Camera, Color, Draw, Object, Texture};

use crate::level::{
    top_of, Tile, BODY, COLS, GROUND, LEVEL_TOP, ROWS, TILE,
};
use crate::player::{Player, State};

/// Высота каменной шапки на экране.
const CAP_H: f32 = 36.0;

/// Высота одного куска грунта на экране.
const DIRT_H: f32 = 60.0;

/// Небо.
pub const SKY_TOP: Color = Color::rgb(58, 152, 233);
const SKY_LOW: Color = Color::rgb(191, 233, 252);
const SKY_BANDS: usize = 8;

/// Смешивает два цвета: `0` — `a`, `1` — `b`.
fn mix(a: Color, b: Color, t: f32) -> Color {
    let ch = |x: u8, y: u8| {
        (x as f32 + (y as f32 - x as f32) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color::rgba(ch(a.r, b.r), ch(a.g, b.g), ch(a.b, b.b), 255)
}

/// Текстуры мира, которыми заливается уровень.
///
/// Отдельная структура, а не три отдельных аргумента функции: их иначе
/// пришлось бы перечислять в обоих местах, где рисуется мир, и легко
/// перепутать местами.
#[derive(Clone, Copy)]
pub struct Tiles {
    top: Texture,
    body: Texture,
    spike: Texture,
}

impl Tiles {
    pub fn new(top: Texture, body: Texture, spike: Texture) -> Self {
        Tiles { top, body, spike }
    }

    /// Рисует небо, землю и уровень при камере `camera`.
    ///
    /// Куб сюда не входит: у меню мира без игрока, и это позволяет ему ехать
    /// под панелью.
    pub fn draw(
        &self,
        d: &mut Draw<'_>,
        grid: &[Vec<Tile>],
        camera: Camera,
        view: (u32, u32),
    ) -> std::result::Result<(), frglib::Error> {
        let (vw, vh) = (view.0 as f32, view.1 as f32);

        // Видимый кусок мира: камера стоит в центре окна, поэтому в кадр
        // попадает ровно пол-окна в каждую сторону. Рисовать надо в этих
        // мировых координатах -- `Draw` сам спроецирует их на экран.
        let left = camera.position.x - vw / 2.0;
        let top = camera.position.y - vh / 2.0;
        let bottom = camera.position.y + vh / 2.0;

        // Видимые столбцы -- со знаком: камера спокойно выезжает и за левый
        // край уровня. Земля отсюда не кончается никогда, а шипы и платформы
        // есть только внутри уровня, и их обрезаем отдельно.
        let vis0 = (left / TILE).floor() as isize;
        let vis1 = ((camera.position.x + vw / 2.0) / TILE).ceil() as isize;

        let col0 = vis0.max(0) as usize;
        let col1 = (vis1.min(COLS as isize)).max(0) as usize;
        let row0 = ((top - LEVEL_TOP) / TILE).floor().max(0.0) as usize;
        let row1 = (((bottom - LEVEL_TOP) / TILE).ceil().max(0.0)).min(ROWS as f32) as usize;

        d.set_camera(camera);

        // Небо полосами: дешёвый градиент из одного цвета не выйдет.
        let band = vh / SKY_BANDS as f32;
        for i in 0..SKY_BANDS {
            let t = i as f32 / (SKY_BANDS - 1) as f32;
            d.fill_object(
                &Object::from_xywh(
                    camera.position.x,
                    top + band * (i as f32 + 0.5),
                    vw + TILE,
                    band + 1.0,
                ),
                mix(SKY_TOP, SKY_LOW, t),
            )?;
        }

        // Земля: каменная шапка по верхнему краю, грунт вниз до низа окна.
        for col in vis0..vis1 {
            let x = col as f32 * TILE + TILE / 2.0;
            d.fill_object_texture(
                &Object::from_xywh(x, GROUND + CAP_H / 2.0, TILE, CAP_H),
                self.top,
            )?;
        }

        let mut y = GROUND + CAP_H;
        while y < bottom + DIRT_H {
            for col in vis0..vis1 {
                let x = col as f32 * TILE + TILE / 2.0;
                d.fill_object_texture(
                    &Object::from_xywh(x, y + DIRT_H / 2.0, TILE, DIRT_H),
                    self.body,
                )?;
            }
            y += DIRT_H;
        }

        for (row, cells) in grid.iter().enumerate().take(row1).skip(row0) {
            let y = top_of(row) + TILE / 2.0;

            for (col, tile) in cells.iter().enumerate().take(col1).skip(col0) {
                let cell = Object::from_xywh(col as f32 * TILE + TILE / 2.0, y, TILE, TILE);

                match tile {
                    Tile::Spike => d.fill_object_texture(&cell, self.spike)?,
                    // Платформа -- тот же камень, что и верх земли, но земля
                    // под ней, так что читается как выступ.
                    Tile::Platform => d.fill_object_texture(&cell, self.top)?,
                    Tile::Empty => {}
                }
            }
        }

        Ok(())
    }
}

/// Рисует куб игрока: живым или разбитым.
pub fn draw_player(
    d: &mut Draw<'_>,
    player: &Player,
    cube: Texture,
    cube_dead: Texture,
) -> std::result::Result<(), frglib::Error> {
    let mut body = Object::from_xywh(player.x, player.y, BODY, BODY);
    body.rotation = player.spin;

    if player.state == State::Dead {
        d.fill_object_texture(&body, cube_dead)?;
    } else {
        d.fill_object_texture(&body, cube)?;
    }

    Ok(())
}
