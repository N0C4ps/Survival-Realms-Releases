use glam::{IVec3, UVec3};

use super::chunk::{CHUNK_SIZE, ChunkPos};

pub fn split_global(global: IVec3) -> (ChunkPos, UVec3) {
    let size = CHUNK_SIZE as i32;
    let chunk = IVec3::new(
        global.x.div_euclid(size),
        global.y.div_euclid(size),
        global.z.div_euclid(size),
    );
    let local = UVec3::new(
        global.x.rem_euclid(size) as u32,
        global.y.rem_euclid(size) as u32,
        global.z.rem_euclid(size) as u32,
    );

    (chunk, local)
}

pub fn global_from_local(chunk: ChunkPos, local: UVec3) -> IVec3 {
    chunk * CHUNK_SIZE as i32 + local.as_ivec3()
}

pub fn chunk_from_position(position: glam::Vec3) -> ChunkPos {
    let block = position.floor().as_ivec3();
    split_global(block).0
}
