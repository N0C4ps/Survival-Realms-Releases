use glam::UVec3;

use crate::application::core::blocks::BlockId;

use super::ChunkFlags;

pub const CHUNK_SIZE: usize = 16;

pub struct Chunk {
    blocks: [[[BlockId; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
    skylight: [[[u8; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
    fluid_source_candidates: Vec<u16>,
    flags: ChunkFlags,
}

impl Chunk {
    pub fn empty() -> Self {
        Self {
            blocks: [[[BlockId::AIR; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
            skylight: [[[0; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
            fluid_source_candidates: Vec::new(),
            flags: ChunkFlags::default(),
        }
    }

    pub fn block(&self, local: UVec3) -> BlockId {
        self.blocks[local.x as usize][local.y as usize][local.z as usize]
    }

    pub fn is_empty(&self) -> bool {
        self.blocks
            .iter()
            .flatten()
            .flatten()
            .all(|&block| block == BlockId::AIR)
    }

    pub fn set_block(&mut self, local: UVec3, block: BlockId) -> BlockId {
        let slot = &mut self.blocks[local.x as usize][local.y as usize][local.z as usize];
        let previous = *slot;
        *slot = block;
        self.flags.insert(ChunkFlags::DIRTY);
        self.flags.remove(ChunkFlags::MESHED);
        previous
    }

    pub fn skylight(&self, local: UVec3) -> u8 {
        self.skylight[local.x as usize][local.y as usize][local.z as usize]
    }

    pub fn set_skylight(&mut self, local: UVec3, level: u8) -> u8 {
        assert!(level <= 15, "skylight level must be between 0 and 15");
        let slot = &mut self.skylight[local.x as usize][local.y as usize][local.z as usize];
        let previous = *slot;
        *slot = level;
        previous
    }

    pub fn clear_skylight(&mut self) {
        self.skylight = [[[0; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE];
    }

    pub(crate) fn rebuild_fluid_source_candidates(&mut self) {
        self.fluid_source_candidates.clear();
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let block = self.blocks[x][y][z];
                    if !block.is_liquid() {
                        continue;
                    }
                    let on_border = x == 0
                        || y == 0
                        || z == 0
                        || x + 1 == CHUNK_SIZE
                        || y + 1 == CHUNK_SIZE
                        || z + 1 == CHUNK_SIZE;
                    let exposed_inside = (!on_border)
                        && [
                            (x + 1, y, z),
                            (x - 1, y, z),
                            (x, y + 1, z),
                            (x, y - 1, z),
                            (x, y, z + 1),
                            (x, y, z - 1),
                        ]
                        .into_iter()
                        .any(|(nx, ny, nz)| self.blocks[nx][ny][nz] != block);
                    if on_border || exposed_inside {
                        self.fluid_source_candidates.push(encode_local(x, y, z));
                    }
                }
            }
        }
    }

    pub(crate) fn fluid_source_candidates(&self) -> impl Iterator<Item = UVec3> + '_ {
        self.fluid_source_candidates
            .iter()
            .copied()
            .map(decode_local)
    }

    pub fn flags(&self) -> ChunkFlags {
        self.flags
    }

    pub fn flags_mut(&mut self) -> &mut ChunkFlags {
        &mut self.flags
    }
}

fn encode_local(x: usize, y: usize, z: usize) -> u16 {
    (x * CHUNK_SIZE * CHUNK_SIZE + y * CHUNK_SIZE + z) as u16
}

fn decode_local(index: u16) -> UVec3 {
    let index = index as usize;
    let x = index / (CHUNK_SIZE * CHUNK_SIZE);
    let remainder = index % (CHUNK_SIZE * CHUNK_SIZE);
    let y = remainder / CHUNK_SIZE;
    let z = remainder % CHUNK_SIZE;
    UVec3::new(x as u32, y as u32, z as u32)
}

impl Default for Chunk {
    fn default() -> Self {
        Self::empty()
    }
}
