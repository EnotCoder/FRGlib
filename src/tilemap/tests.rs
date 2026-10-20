use super::*;
use crate::math::{Rect, Vec2};
use crate::Camera;

/// What a test map holds. `None` in the text is not a cell: this is the
/// parsed form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cell {
    Floor,
    Water,
    Wall,
}

/// Parses the characters a test map uses.
fn parse(ch: char) -> Option<Cell> {
    Some(match ch {
        '.' => Cell::Floor,
        '~' => Cell::Water,
        '#' => Cell::Wall,
        _ => return None,
    })
}

/// Four rows of eight cells, all floors except the ones that are not.
fn map() -> TileMap<Cell> {
    TileMap::from_ascii(
        &[
            "........",
            "..~~~~..",
            "..#...#.",
            "........",
        ],
        32.0,
        parse,
    )
    .expect("map should parse")
}

#[test]
fn a_parsed_map_has_the_shape_of_its_text() {
    let map = map();

    assert_eq!(map.cols(), 8);
    assert_eq!(map.rows(), 4);
    assert_eq!(map.cell_size(), 32.0);
    assert_eq!(map.width(), 8.0 * 32.0);
    assert_eq!(map.height(), 4.0 * 32.0);
}

#[test]
fn cells_land_where_their_text_says() {
    let map = map();

    assert_eq!(map.get(0, 0), Some(&Cell::Floor));
    assert_eq!(map.get(2, 1), Some(&Cell::Water));
    assert_eq!(map.get(2, 2), Some(&Cell::Wall));
    // Off every edge.
    assert_eq!(map.get(8, 0), None);
    assert_eq!(map.get(0, 4), None);
    assert_eq!(map.get(9, 9), None);
}

#[test]
fn a_world_point_finds_the_cell_it_is_in() {
    let map = map();

    // Cell (0, 0) covers 0..32 on both axes.
    assert_eq!(map.cell_at(Vec2::new(0.0, 0.0)), Some((0, 0)));
    assert_eq!(map.cell_at(Vec2::new(31.9, 31.9)), Some((0, 0)));

    // Just over the border belongs to the next cell, not the previous one.
    assert_eq!(map.cell_at(Vec2::new(32.0, 0.0)), Some((1, 0)));
    assert_eq!(map.cell_at(Vec2::new(64.0, 64.0)), Some((2, 2)));

    // Off the map, in every direction.
    assert_eq!(map.cell_at(Vec2::new(-1.0, 5.0)), None);
    assert_eq!(map.cell_at(Vec2::new(5.0, -1.0)), None);
    assert_eq!(map.cell_at(Vec2::new(256.0, 0.0)), None);
    assert_eq!(map.cell_at(Vec2::new(0.0, 128.0)), None);
}

#[test]
fn a_broken_world_point_finds_nothing() {
    let map = map();

    // NaN and infinity must not be cast into a plausible cell index.
    assert_eq!(map.cell_at(Vec2::new(f32::NAN, 0.0)), None);
    assert_eq!(map.cell_at(Vec2::new(0.0, f32::INFINITY)), None);
    assert_eq!(map.cell_at(Vec2::new(f32::NEG_INFINITY, 5.0)), None);
}

#[test]
fn get_at_combines_the_two_lookups() {
    let map = map();

    assert_eq!(map.get_at(Vec2::new(70.0, 70.0)), Some(&Cell::Wall));
    assert_eq!(map.get_at(Vec2::new(70.0, 70.0)), map.get(2, 2));
    assert_eq!(map.get_at(Vec2::new(-100.0, -100.0)), None);
}

#[test]
fn an_origin_moves_every_cell_with_the_map() {
    let plain = map();
    let moved = map().origin(100.0, 200.0);

    // The same cell, asked for in world coordinates offset by the origin.
    assert_eq!(
        moved.cell_at(Vec2::new(100.0, 200.0)),
        Some((0, 0))
    );
    assert_eq!(
        moved.cell_at(Vec2::new(170.0, 270.0)),
        Some((2, 2))
    );

    // The origin is where the map starts, so cell rects move too.
    assert_eq!(plain.cell_rect(1, 1), Rect::new(32.0, 32.0, 32.0, 32.0));
    assert_eq!(
        moved.cell_rect(1, 1),
        Rect::new(132.0, 232.0, 32.0, 32.0)
    );
}

#[test]
fn cell_rects_tile_up_without_gaps_or_overlaps() {
    let map = map();

    let first = map.cell_rect(0, 0);
    let second = map.cell_rect(1, 0);

    assert_eq!(first.x + first.w, second.x);
    assert_eq!(first.y, second.y);
    assert_eq!(first.w, second.w);

    let centre = map.cell_center(0, 0);
    assert_eq!(centre, Vec2::new(16.0, 16.0));
}

#[test]
fn only_the_cells_on_screen_are_visible() {
    let map = map();

    // Centred on cell (1, 1), showing 400x400 world units.
    let camera = Camera::new(Vec2::new(48.0, 48.0));
    let visible: Vec<_> = map
        .visible(camera, (400, 400))
        .map(|(col, row, _)| (col, row))
        .collect();

    // The whole 8x4 map fits inside a 400-wide view, so all of it shows.
    assert_eq!(visible.len(), map.cols() * map.rows());

    // Far away, nothing shows.
    let off = Camera::new(Vec2::new(5000.0, 5000.0));
    assert_eq!(map.visible(off, (400, 400)).count(), 0);
}

#[test]
fn a_narrow_view_only_reaches_some_of_the_columns() {
    let map = map();

    // A 100-wide window centred on x=64 covers 14..114, plus the cell of
    // padding on each side: columns 0..4, so four of the eight.
    let camera = Camera::new(Vec2::new(64.0, 64.0));
    let cols: std::collections::HashSet<_> = map
        .visible(camera, (100, 100))
        .map(|(col, _, _)| col)
        .collect();

    assert!(cols.len() < map.cols(), "should not show every column");
    assert!(cols.contains(&0), "the padding cell at the left shows");
}

#[test]
fn zooming_out_reaches_further_across_the_map() {
    let map = map();

    let near = Camera::new(Vec2::new(64.0, 64.0));
    let far = Camera::new(Vec2::new(64.0, 64.0)).with_zoom(0.25);

    assert!(map.visible(far, (400, 400)).count() >= map.visible(near, (400, 400)).count());
}

#[test]
fn a_map_smaller_than_the_window_is_centred() {
    let mut camera = Camera::new(Vec2::new(9999.0, 9999.0));
    let map = map(); // 256x128 world units.

    map.clamp_camera(&mut camera, (800, 600));

    assert_eq!(camera.position, Vec2::new(128.0, 64.0));
}

#[test]
fn a_larger_map_stops_the_camera_at_its_edges() {
    let mut camera = Camera::new(Vec2::new(0.0, 0.0));
    let map = TileMap::from_rows(vec![vec![Cell::Floor; 40]; 40], 32.0).unwrap();

    map.clamp_camera(&mut camera, (400, 400));

    // Half the view is 200, the map is 1280 wide: the camera may not go
    // below 200 or above 1080, and it was asked to sit at 0.
    assert_eq!(camera.position.x, 200.0);
    assert_eq!(camera.position.y, 200.0);

    // And from the other side, it stops at the far edge.
    camera.position = Vec2::new(99999.0, 99999.0);
    map.clamp_camera(&mut camera, (400, 400));
    assert_eq!(camera.position.x, 1080.0);
    assert_eq!(camera.position.y, 1080.0);
}

#[test]
fn an_origin_shifts_where_the_camera_may_look() {
    let mut camera = Camera::new(Vec2::ZERO);
    let map = TileMap::from_rows(vec![vec![Cell::Floor; 40]; 40], 32.0)
        .unwrap()
        .origin(500.0, 0.0);

    map.clamp_camera(&mut camera, (400, 400));

    // Left edge is now x=500, so the camera cannot sit at 200.
    assert_eq!(camera.position.x, 700.0);
}

#[test]
fn rows_are_flattened_row_major() {
    let rows = vec![
        vec![Cell::Floor, Cell::Wall],
        vec![Cell::Water, Cell::Floor],
    ];
    let map = TileMap::from_rows(rows, 16.0).unwrap();

    assert_eq!(map.cols(), 2);
    assert_eq!(map.rows(), 2);
    assert_eq!(map.get(1, 0), Some(&Cell::Wall));
    assert_eq!(map.get(0, 1), Some(&Cell::Water));
    assert_eq!(map.get(1, 1), Some(&Cell::Floor));
}

// --- failures ---

#[test]
fn no_rows_is_an_error() {
    assert_eq!(
        TileMap::<Cell>::from_ascii(&[], 32.0, parse),
        Err(MapError::NoRows)
    );
    assert_eq!(
        TileMap::<Cell>::from_ascii(&["", ""], 32.0, parse),
        Err(MapError::NoRows)
    );
    assert_eq!(TileMap::<Cell>::from_rows(vec![], 32.0), Err(MapError::NoRows));
}

#[test]
fn rows_of_different_lengths_are_rejected() {
    let got = TileMap::from_ascii(&["....", "...", "...."], 32.0, parse);

    assert_eq!(
        got,
        Err(MapError::RaggedRow {
            row: 1,
            expected: 4,
            got: 3
        })
    );
}

#[test]
fn rows_of_different_lengths_from_code_are_rejected_too() {
    let got = TileMap::from_rows(
        vec![
            vec![Cell::Floor, Cell::Floor],
            vec![Cell::Floor],
        ],
        32.0,
    );

    assert_eq!(
        got,
        Err(MapError::RaggedRow {
            row: 1,
            expected: 2,
            got: 1
        })
    );
}

#[test]
fn a_character_the_parser_rejects_stops_the_map() {
    let got = TileMap::from_ascii(&["....", "..X."], 32.0, parse);

    assert_eq!(
        got,
        Err(MapError::UnknownChar {
            row: 1,
            col: 2,
            ch: 'X'
        })
    );
}

#[test]
fn a_cell_size_that_is_not_positive_is_rejected() {
    // Compared without `assert_eq` because `NaN == NaN` is false, and
    // `MapError` derives `PartialEq` on a payload that can hold one.
    for cell in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        let got = TileMap::<Cell>::from_ascii(&[".."], cell, parse);

        assert!(
            matches!(got, Err(MapError::BadCell { cell: got }) if got.is_nan() || !got.is_finite() || got <= 0.0),
            "cell size {cell} should be rejected, got {got:?}"
        );
    }
}

#[test]
fn errors_read_as_sentences() {
    assert!(!MapError::NoRows.to_string().is_empty());
    assert!(MapError::RaggedRow {
        row: 2,
        expected: 5,
        got: 3
    }
    .to_string()
    .contains("row 2"));
    assert!(MapError::UnknownChar {
        row: 1,
        col: 0,
        ch: 'x'
    }
    .to_string()
    .contains('x'));
}
