//! Which cells the camera sees, and drawing them.

use super::map::TileMap;

impl<T> TileMap<T> {
    /// Cells the camera can see, row by row and left to right.
    ///
    /// Everything outside this is off screen, and a map is usually far larger
    /// than the window -- drawing it cell by cell without this is what makes a
    /// big map slow. The range runs one cell past the visible edge on every
    /// side, so a tile half in the frame still gets drawn rather than popping
    /// in as the camera crosses a boundary.
    ///
    /// Needs the camera it is being viewed through: the same map is visible
    /// or not depending on where the camera points and how far it is zoomed.
    pub fn visible(
        &self,
        camera: crate::Camera,
        view: (u32, u32),
    ) -> impl Iterator<Item = (usize, usize, &T)> + '_ {
        let (col0, col1, row0, row1) = self.visible_bounds(camera, view);

        (row0..row1).flat_map(move |row| {
            (col0..col1).filter_map(move |col| self.get(col, row).map(|tile| (col, row, tile)))
        })
    }

    /// The cell range [`visible`](Self::visible) walks, as `start..end` pairs,
    /// already clipped to the map.
    fn visible_bounds(&self, camera: crate::Camera, view: (u32, u32)) -> (usize, usize, usize, usize) {
        // Same guard as `Camera::scale`: a zero or negative zoom would divide
        // by nothing and report the whole map as visible.
        let zoom = if camera.zoom > f32::EPSILON {
            camera.zoom
        } else {
            1.0
        };

        let half_w = view.0 as f32 / 2.0 / zoom;
        let half_h = view.1 as f32 / 2.0 / zoom;

        let (col0, col1) = self.strip(
            camera.position.x - half_w,
            camera.position.x + half_w,
            self.origin.x,
            self.cols,
        );
        let (row0, row1) = self.strip(
            camera.position.y - half_h,
            camera.position.y + half_h,
            self.origin.y,
            self.rows,
        );

        (col0, col1, row0, row1)
    }

    /// Which cells the world span `from..to` covers along one axis, padded by
    /// one cell on each side and clipped to `len`.
    fn strip(&self, from: f32, to: f32, origin: f32, len: usize) -> (usize, usize) {
        let start = ((from - origin) / self.cell).floor() as i64 - 1;
        let end = ((to - origin) / self.cell).ceil() as i64 + 1;

        // Clamping also handles the map being entirely off one side: the two
        // ends land on the same edge and the range comes out empty.
        let len = len as i64;
        let start = start.clamp(0, len) as usize;
        let end = end.clamp(0, len) as usize;

        (start, end.max(start))
    }

    /// Keeps the camera looking at the map, so there is no empty space past
    /// its edges.
    ///
    /// A map smaller than the window on an axis is simply centred; otherwise
    /// the camera stops at the edge. Same rule as seeing the whole world
    /// behind a window: past the map there is nothing, so the view does not go
    /// there.
    pub fn clamp_camera(&self, camera: &mut crate::Camera, view: (u32, u32)) {
        let zoom = if camera.zoom > f32::EPSILON {
            camera.zoom
        } else {
            1.0
        };

        let half_w = view.0 as f32 / 2.0 / zoom;
        let half_h = view.1 as f32 / 2.0 / zoom;

        let (map_w, map_h) = (self.width(), self.height());
        let (left, top) = (self.origin.x, self.origin.y);

        camera.position.x = if map_w <= half_w * 2.0 {
            left + map_w / 2.0
        } else {
            camera.position.x.clamp(left + half_w, left + map_w - half_w)
        };

        camera.position.y = if map_h <= half_h * 2.0 {
            top + map_h / 2.0
        } else {
            camera.position.y.clamp(top + half_h, top + map_h - half_h)
        };
    }

    /// Draws the visible cells.
    ///
    /// `texture_of` maps a cell to the texture to draw it with; [`None`] skips
    /// the cell, which is how an empty cell stays empty without the map having
    /// to know what "empty" means.
    ///
    /// The camera is the one the frame is being drawn through -- pass the same
    /// one given to [`Draw::set_camera`](crate::Draw::set_camera) -- and it
    /// decides which cells are visible.
    ///
    /// Behind `cached-textures` because the alternative is uploading a texture
    /// per cell per frame, which is what made the first version of this
    /// take a minute to start.
    #[cfg(feature = "cached-textures")]
    pub fn draw(
        &self,
        d: &mut crate::Draw<'_>,
        camera: crate::Camera,
        texture_of: impl Fn(&T) -> Option<crate::graphics::Texture>,
    ) -> Result<(), crate::window::Error> {
        for (col, row, tile) in self.visible(camera, d.size()) {
            let Some(texture) = texture_of(tile) else {
                continue;
            };

            let centre = self.cell_center(col, row);
            d.fill_object_texture(
                &crate::Object::from_xywh(centre.x, centre.y, self.cell, self.cell),
                texture,
            )?;
        }

        Ok(())
    }
}
