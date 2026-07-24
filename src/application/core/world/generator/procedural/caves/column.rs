use super::config::{RAVINE_TOP_WIDTH, WORLD_FLOOR_MARGIN};

#[derive(Clone, Copy, Debug)]
pub struct CaveColumn {
    surface: i32,
    world_floor: i32,
    ravine_line_distance: f64,
    ravine_depth: i32,
}

impl CaveColumn {
    pub(super) const fn new(
        surface: i32,
        world_floor: i32,
        ravine_line_distance: f64,
        ravine_depth: i32,
    ) -> Self {
        Self {
            surface,
            world_floor,
            ravine_line_distance,
            ravine_depth,
        }
    }

    pub const fn surface(self) -> i32 {
        self.surface
    }

    pub fn is_ravine(self, y: i32) -> bool {
        if self.ravine_depth == 0 || y > self.surface || y <= self.world_floor + WORLD_FLOOR_MARGIN
        {
            return false;
        }

        let depth = self.surface - y;
        if depth > self.ravine_depth {
            return false;
        }

        // A V-shaped profile guarantees that every carved cell has a clear vertical
        // path to the sky. It also keeps the ravine from becoming a floating cavity.
        let remaining = 1.0 - depth as f64 / (self.ravine_depth + 1) as f64;
        let width = RAVINE_TOP_WIDTH * remaining.max(0.12);
        self.ravine_line_distance < width
    }

    pub fn ravine_skylight(self, y: i32) -> Option<u8> {
        self.is_ravine(y)
            .then(|| (15 - (self.surface - y)).clamp(0, 15) as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ravine_skylight_descends_through_all_sixteen_levels() {
        let column = CaveColumn::new(3, -128, 0.0, 22);

        for depth in 0..=15 {
            assert_eq!(column.ravine_skylight(3 - depth), Some((15 - depth) as u8));
        }
        assert_eq!(column.ravine_skylight(3 - 16), Some(0));
    }
}
