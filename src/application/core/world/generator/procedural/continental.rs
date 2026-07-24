use super::noise::value_noise;

pub const SEA_LEVEL: i32 = 0;

const CONTINENT_SEED: u64 = 0x510E_527F_ADE6_82D1;
const REGION_SEED: u64 = 0x9B05_688C_2B3E_6C1F;
const COAST_SEED: u64 = 0x1F83_D9AB_FB41_BD6B;
const ISLAND_SEED: u64 = 0x5BE0_CD19_137E_2179;
const OCEAN_FLOOR_SEED: u64 = 0xCBBB_9D5D_C105_9ED8;
const LAND_DETAIL_SEED: u64 = 0x629A_292A_367C_D507;

pub fn surface_height(seed: u64, x: i32, z: i32) -> i32 {
    let continentalness = continentalness(seed, x, z);
    let terrain_detail = value_noise(seed ^ LAND_DETAIL_SEED, x, z, 42) * 1.8
        + value_noise(seed ^ LAND_DETAIL_SEED.rotate_left(17), x, z, 17) * 0.8;

    if continentalness >= 0.0 {
        let inland_height = 1.0 + continentalness * 14.0 + terrain_detail;
        inland_height.round().clamp(0.0, 16.0) as i32
    } else {
        let depth = 2.0 + (-continentalness) * 13.0;
        let floor_detail = value_noise(seed ^ OCEAN_FLOOR_SEED, x, z, 54) * 1.5;
        (-(depth + floor_detail).round() as i32).clamp(-16, -1)
    }
}

pub fn continentalness(seed: u64, x: i32, z: i32) -> f32 {
    let broad = value_noise(seed ^ CONTINENT_SEED, x, z, 420) * 0.68;
    let regional = value_noise(seed ^ REGION_SEED, x, z, 175) * 0.25;
    let coast = value_noise(seed ^ COAST_SEED, x, z, 72) * 0.10;
    let island_field = value_noise(seed ^ ISLAND_SEED, x, z, 110);
    let islands = ((island_field - 0.48) / 0.52).clamp(0.0, 1.0) * 0.32;
    broad + regional + coast + islands - 0.015
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continental_height_is_seeded_and_contains_land_and_ocean() {
        let first: Vec<_> = (-768..768)
            .step_by(8)
            .map(|x| surface_height(42, x, 137))
            .collect();
        let repeated: Vec<_> = (-768..768)
            .step_by(8)
            .map(|x| surface_height(42, x, 137))
            .collect();
        let different: Vec<_> = (-768..768)
            .step_by(8)
            .map(|x| surface_height(43, x, 137))
            .collect();

        assert_eq!(first, repeated);
        assert_ne!(first, different);
        assert!(first.iter().any(|height| *height >= SEA_LEVEL));
        assert!(first.iter().any(|height| *height < SEA_LEVEL));
    }

    #[test]
    fn ocean_floor_stays_below_zero_and_land_stays_above_it() {
        for x in (-512..512).step_by(13) {
            for z in (-512..512).step_by(13) {
                let height = surface_height(91, x, z);
                assert!((-16..=16).contains(&height));
                if continentalness(91, x, z) < 0.0 {
                    assert!(height < SEA_LEVEL);
                } else {
                    assert!(height >= SEA_LEVEL);
                }
            }
        }
    }
}
