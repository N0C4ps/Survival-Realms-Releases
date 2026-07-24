pub fn value_noise(seed: u64, x: i32, z: i32, scale: i32) -> f32 {
    let cell_x = x.div_euclid(scale);
    let cell_z = z.div_euclid(scale);
    let local_x = x.rem_euclid(scale) as f32 / scale as f32;
    let local_z = z.rem_euclid(scale) as f32 / scale as f32;
    let tx = smoothstep(local_x);
    let tz = smoothstep(local_z);

    let top = lerp(
        lattice(seed, cell_x, cell_z),
        lattice(seed, cell_x + 1, cell_z),
        tx,
    );
    let bottom = lerp(
        lattice(seed, cell_x, cell_z + 1),
        lattice(seed, cell_x + 1, cell_z + 1),
        tx,
    );
    lerp(top, bottom, tz)
}

fn lattice(seed: u64, x: i32, z: i32) -> f32 {
    let mut value = seed
        ^ (x as u32 as u64).wrapping_mul(0x9E37_79B1_85EB_CA87)
        ^ (z as u32 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    (value as u32 as f32 / u32::MAX as f32) * 2.0 - 1.0
}

fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

fn lerp(start: f32, end: f32, amount: f32) -> f32 {
    start + (end - start) * amount
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_deterministic_and_continuous_across_negative_cells() {
        assert_eq!(value_noise(42, -1, -1, 32), value_noise(42, -1, -1, 32));
        assert!((value_noise(42, -1, 0, 32) - value_noise(42, 0, 0, 32)).abs() < 0.2);
    }
}
