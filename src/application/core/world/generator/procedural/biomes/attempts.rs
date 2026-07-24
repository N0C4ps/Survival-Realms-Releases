use glam::{IVec2, IVec3};

use super::{BiomeSampler, BiomeZone, hash};
use crate::application::core::world::CHUNK_SIZE;

const ATTEMPT_SEED: u64 = 0xC76C_51A3_0654_BE30;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TreeZonePlan {
    zone: BiomeZone,
    attempts: u8,
    dreamcore_tree: Option<IVec2>,
}

impl TreeZonePlan {
    pub(crate) const fn zone(self) -> BiomeZone {
        self.zone
    }

    pub(crate) const fn attempts(self) -> u8 {
        self.attempts
    }

    pub(crate) const fn dreamcore_tree(self) -> Option<IVec2> {
        self.dreamcore_tree
    }
}

impl BiomeSampler {
    pub(crate) fn tree_plan_for_chunk(self, chunk: IVec3) -> TreeZonePlan {
        let origin = IVec2::new(chunk.x, chunk.z) * CHUNK_SIZE as i32;
        let center = origin + IVec2::splat(CHUNK_SIZE as i32 / 2);
        let zone = self.zone_at(center.x, center.y);
        let identity = hash(self.seed() ^ ATTEMPT_SEED, chunk.x, chunk.z);

        match zone {
            BiomeZone::Plains => TreeZonePlan {
                zone,
                attempts: u8::from(identity.is_multiple_of(10)),
                dreamcore_tree: None,
            },
            BiomeZone::Forest => TreeZonePlan {
                zone,
                attempts: 3 + (identity % 8) as u8,
                dreamcore_tree: None,
            },
            BiomeZone::VeryDreamcoreOneTree => {
                let anchor = self
                    .dreamcore_zone_at(center.x, center.y)
                    .expect("dreamcore biome must belong to a dreamcore zone")
                    .center();
                let anchor_chunk = anchor.map(|axis| axis.div_euclid(CHUNK_SIZE as i32));
                let owns_tree = anchor_chunk == IVec2::new(chunk.x, chunk.z);
                TreeZonePlan {
                    zone,
                    attempts: u8::from(owns_tree),
                    dreamcore_tree: owns_tree.then_some(anchor),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plains_average_one_attempt_per_ten_chunks() {
        let sampler = BiomeSampler::new(42);
        let mut plains = 0;
        let mut attempts = 0;
        for x in -100..100 {
            for z in -100..100 {
                let plan = sampler.tree_plan_for_chunk(IVec3::new(x, 0, z));
                if plan.zone() == BiomeZone::Plains {
                    plains += 1;
                    attempts += usize::from(plan.attempts());
                }
            }
        }
        let rate = attempts as f32 / plains as f32;
        assert!((0.08..=0.12).contains(&rate), "plains rate was {rate}");
    }

    #[test]
    fn forests_roll_between_three_and_ten_attempts() {
        let sampler = BiomeSampler::new(91);
        let attempts = (-80..80)
            .flat_map(|x| {
                (-80..80).filter_map(move |z| {
                    let plan = sampler.tree_plan_for_chunk(IVec3::new(x, 0, z));
                    (plan.zone() == BiomeZone::Forest).then_some(plan.attempts())
                })
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(attempts, (3..=10).collect());
    }
}
