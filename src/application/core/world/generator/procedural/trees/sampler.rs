use glam::{IVec2, IVec3};

use super::{config::CLEARANCE_HEIGHT, shape::TreeShape, tree::GeneratedTree};
use crate::application::core::world::generator::procedural::biomes::{BiomeSampler, hash};
use crate::application::core::{
    blocks::BlockId,
    world::{CHUNK_SIZE, coordinates},
};

const POSITION_SEED: u64 = 0x61D7_8BD4_A985_315E;
const SHAPE_SEED: u64 = 0x7C39_56BA_20D4_F18B;

#[derive(Clone, Copy)]
pub(crate) struct TreeSampler {
    seed: u64,
    biomes: BiomeSampler,
}

impl TreeSampler {
    pub(crate) const fn new(seed: u64) -> Self {
        Self {
            seed,
            biomes: BiomeSampler::new(seed),
        }
    }

    pub(crate) fn trees_affecting_chunk(
        self,
        chunk: IVec3,
        base_block: impl Fn(IVec3) -> Option<BlockId> + Copy,
    ) -> Vec<GeneratedTree> {
        let reach = (TreeShape::maximum_reach() + CHUNK_SIZE as i32 - 1) / CHUNK_SIZE as i32;
        let chunk_min = chunk * CHUNK_SIZE as i32;
        let chunk_max = chunk_min + IVec3::splat(CHUNK_SIZE as i32 - 1);
        let mut trees = Vec::new();

        for owner_x in chunk.x - reach..=chunk.x + reach {
            for owner_z in chunk.z - reach..=chunk.z + reach {
                self.for_each_candidate(IVec3::new(owner_x, 0, owner_z), |candidate| {
                    if candidate.maximum().cmplt(chunk_min).any()
                        || candidate.minimum().cmpgt(chunk_max).any()
                        || !self.is_valid(candidate, base_block)
                    {
                        return;
                    }
                    trees.push(candidate);
                });
            }
        }
        trees
    }

    pub(crate) fn block(
        self,
        position: IVec3,
        base_block: impl Fn(IVec3) -> Option<BlockId> + Copy,
    ) -> Option<BlockId> {
        let owner = coordinates::split_global(position).0;
        let reach = (TreeShape::maximum_reach() + CHUNK_SIZE as i32 - 1) / CHUNK_SIZE as i32;
        let mut leaf = false;
        for owner_x in owner.x - reach..=owner.x + reach {
            for owner_z in owner.z - reach..=owner.z + reach {
                let mut log = false;
                self.for_each_candidate(IVec3::new(owner_x, 0, owner_z), |candidate| {
                    if candidate.minimum().cmple(position).all()
                        && candidate.maximum().cmpge(position).all()
                        && self.is_valid(candidate, base_block)
                    {
                        match candidate.block_at(position) {
                            Some(BlockId::WOOD_LOG) => log = true,
                            Some(BlockId::LEAVES) => leaf = true,
                            _ => {}
                        }
                    }
                });
                if log {
                    return Some(BlockId::WOOD_LOG);
                }
            }
        }
        leaf.then_some(BlockId::LEAVES)
    }

    fn for_each_candidate(self, owner: IVec3, mut visit: impl FnMut(GeneratedTree)) {
        let plan = self.biomes.tree_plan_for_chunk(owner);
        if let Some(anchor) = plan.dreamcore_tree() {
            visit(self.candidate(anchor, owner, 0));
            return;
        }

        let origin = IVec2::new(owner.x, owner.z) * CHUNK_SIZE as i32;
        let mut occupied = [IVec2::ZERO; 10];
        let mut occupied_len = 0usize;
        for attempt in 0..plan.attempts() {
            let identity = hash(
                self.seed ^ POSITION_SEED ^ u64::from(attempt),
                owner.x,
                owner.z,
            );
            let horizontal = origin
                + IVec2::new(
                    (identity % CHUNK_SIZE as u64) as i32,
                    ((identity >> 8) % CHUNK_SIZE as u64) as i32,
                );
            if self.biomes.zone_at(horizontal.x, horizontal.y) != plan.zone() {
                continue;
            }
            if occupied[..occupied_len].contains(&horizontal) {
                continue;
            }
            occupied[occupied_len] = horizontal;
            occupied_len += 1;
            visit(self.candidate(horizontal, owner, attempt));
        }
    }

    fn candidate(self, horizontal: IVec2, owner: IVec3, attempt: u8) -> GeneratedTree {
        let identity = hash(
            self.seed ^ SHAPE_SEED ^ u64::from(attempt),
            owner.x ^ horizontal.x,
            owner.z ^ horizontal.y,
        );
        let ground_y =
            crate::application::core::world::generator::procedural::continental::surface_height(
                self.seed,
                horizontal.x,
                horizontal.y,
            );
        GeneratedTree::new(horizontal, ground_y, TreeShape::from_identity(identity))
    }

    fn is_valid(
        self,
        tree: GeneratedTree,
        base_block: impl Fn(IVec3) -> Option<BlockId> + Copy,
    ) -> bool {
        if !matches!(
            base_block(tree.ground()),
            Some(BlockId::GRASS) | Some(BlockId::DIRT)
        ) {
            return false;
        }

        // The complete safety column must be open. Besides guaranteeing room
        // for every trunk variant, this proves that the tree is sky-exposed.
        if (1..=CLEARANCE_HEIGHT)
            .any(|y| base_block(tree.ground() + IVec3::Y * y) != Some(BlockId::AIR))
        {
            return false;
        }

        let minimum = tree.minimum();
        let maximum = tree.maximum();
        for x in minimum.x..=maximum.x {
            for z in minimum.z..=maximum.z {
                let surface = crate::application::core::world::generator::procedural::continental::surface_height(
                    self.seed, x, z,
                );
                for y in minimum.y..=maximum.y {
                    let position = IVec3::new(x, y, z);
                    if tree.shape().is_leaf(position - tree.ground())
                        && !(y > surface
                            && y
                                > crate::application::core::world::generator::procedural::continental::SEA_LEVEL)
                        && base_block(position) != Some(BlockId::AIR)
                    {
                        return false;
                    }
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_block(position: IVec3) -> Option<BlockId> {
        Some(match position.y.cmp(&0) {
            std::cmp::Ordering::Less => BlockId::DIRT,
            std::cmp::Ordering::Equal => BlockId::GRASS,
            std::cmp::Ordering::Greater => BlockId::AIR,
        })
    }

    #[test]
    fn accepted_trees_have_valid_trunks_and_helmet_crowns() {
        let sampler = TreeSampler::new(91);
        let tree = (-64..64)
            .flat_map(|x| {
                (-64..64).flat_map(move |z| {
                    sampler.trees_affecting_chunk(IVec3::new(x, 0, z), flat_block)
                })
            })
            .next()
            .expect("sample must contain a tree");

        assert!((4..=7).contains(&tree.trunk_height()));
        for y in 1..=tree.trunk_height() {
            assert_eq!(
                tree.block_at(tree.ground() + IVec3::Y * y),
                Some(BlockId::WOOD_LOG)
            );
        }
        assert!(
            (tree.trunk_height() + 1..=tree.trunk_height() + 2).any(|y| tree
                .block_at(tree.ground() + IVec3::new(1, y, 0))
                == Some(BlockId::LEAVES))
        );
    }

    #[test]
    fn invalid_ground_obstruction_and_water_each_reject_the_attempt() {
        let sampler = TreeSampler::new(42);
        for blocking in [BlockId::STONE, BlockId::SAND, BlockId::WATER, BlockId::LAVA] {
            let block = |position: IVec3| {
                Some(if position.y == 0 {
                    blocking
                } else {
                    BlockId::AIR
                })
            };
            assert!((-8..=8).all(|x| (-8..=8).all(|z| {
                sampler
                    .trees_affecting_chunk(IVec3::new(x, 0, z), block)
                    .is_empty()
            })));
        }
    }

    #[test]
    fn neighbouring_chunks_receive_the_same_border_tree() {
        let sampler = TreeSampler::new(1337);
        let mut found = false;
        for x in -128..128 {
            for z in -128..128 {
                let chunk = IVec3::new(x, 0, z);
                let here = sampler.trees_affecting_chunk(chunk, flat_block);
                for tree in here {
                    let min_chunk = coordinates::split_global(tree.minimum()).0;
                    let max_chunk = coordinates::split_global(tree.maximum()).0;
                    if min_chunk.x != max_chunk.x || min_chunk.z != max_chunk.z {
                        let neighbour = IVec3::new(max_chunk.x, 0, max_chunk.z);
                        assert!(
                            sampler
                                .trees_affecting_chunk(neighbour, flat_block)
                                .contains(&tree)
                        );
                        found = true;
                        break;
                    }
                }
                if found {
                    break;
                }
            }
            if found {
                break;
            }
        }
        assert!(found, "sample must contain a tree crossing a chunk border");
    }
}
