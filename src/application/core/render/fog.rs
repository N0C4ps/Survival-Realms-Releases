use crate::application::core::{
    blocks::BlockId,
    world::{CHUNK_SIZE, RenderDistance},
};

const FADE_WIDTH_CHUNKS: f32 = 2.0;

#[derive(Clone, Copy)]
pub(super) struct FogSettings {
    pub distance: [f32; 4],
    pub color: [f32; 4],
}

pub(super) fn settings(
    render_distance: RenderDistance,
    camera_liquid: Option<BlockId>,
) -> FogSettings {
    if camera_liquid == Some(BlockId::WATER) {
        return FogSettings {
            distance: [3.0, 24.0, 0.0, 0.0],
            color: [0.012, 0.065, 0.20, 0.22],
        };
    }
    if camera_liquid == Some(BlockId::LAVA) {
        return FogSettings {
            distance: [0.5, 7.0, 0.0, 0.0],
            color: [0.72, 0.16, 0.018, 0.30],
        };
    }

    let end = render_distance.chunks() as f32 * CHUNK_SIZE as f32;
    let start = (end - FADE_WIDTH_CHUNKS * CHUNK_SIZE as f32).max(end * 0.5);
    FogSettings {
        distance: [start, end, 0.0, 0.0],
        color: [0.0; 4],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_fog_occupies_the_last_two_chunks() {
        assert_eq!(
            settings(RenderDistance::DEFAULT, None).distance,
            [96.0, 128.0, 0.0, 0.0]
        );
    }

    #[test]
    fn water_and_lava_use_short_colored_fog() {
        let water = settings(RenderDistance::DEFAULT, Some(BlockId::WATER));
        let lava = settings(RenderDistance::DEFAULT, Some(BlockId::LAVA));
        assert!(water.distance[1] < 128.0);
        assert!(lava.distance[1] < water.distance[1]);
        assert!(water.color[3] > 0.0);
        assert!(lava.color[3] > water.color[3]);
    }
}
