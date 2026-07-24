use glam::{IVec2, IVec3, UVec3};
use rustc_hash::FxHashMap;

use crate::application::core::{
    blocks::BlockRegistry,
    world::{CHUNK_SIZE, World},
};

pub(super) type Heightmap = FxHashMap<IVec2, i32>;

pub(super) fn build(world: &World, registry: &BlockRegistry) -> Heightmap {
    let mut heightmap = Heightmap::default();

    for (&chunk_position, chunk) in world.chunks() {
        let origin = chunk_position * CHUNK_SIZE as i32;
        for x in 0..CHUNK_SIZE as u32 {
            for z in 0..CHUNK_SIZE as u32 {
                let column = IVec2::new(origin.x + x as i32, origin.z + z as i32);
                for y in (0..CHUNK_SIZE as u32).rev() {
                    let block = chunk.block(UVec3::new(x, y, z));
                    if registry.get(block).properties().is_opaque() {
                        let global_y = origin.y + y as i32;
                        heightmap
                            .entry(column)
                            .and_modify(|height| *height = (*height).max(global_y))
                            .or_insert(global_y);
                        break;
                    }
                }
            }
        }
    }

    heightmap
}

pub(super) fn surface_at(heightmap: &Heightmap, position: IVec3) -> Option<i32> {
    heightmap.get(&IVec2::new(position.x, position.z)).copied()
}
