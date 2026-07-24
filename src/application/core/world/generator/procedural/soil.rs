const SOIL_SEED: u64 = 0x4528_21E6_38D0_1377;

pub fn dirt_depth(seed: u64, x: i32, z: i32) -> i32 {
    let mut value = seed
        ^ SOIL_SEED
        ^ (x as u32 as u64).wrapping_mul(0x9E37_79B1_85EB_CA87)
        ^ (z as u32 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    3 + (value & 1) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soil_is_deterministic_and_always_three_or_four_blocks_deep() {
        let depths: Vec<_> = (-32..32)
            .flat_map(|x| (-32..32).map(move |z| dirt_depth(42, x, z)))
            .collect();

        assert!(depths.iter().all(|depth| matches!(depth, 3 | 4)));
        assert!(depths.contains(&3));
        assert!(depths.contains(&4));
        assert_eq!(dirt_depth(42, -7, 19), dirt_depth(42, -7, 19));
    }
}
