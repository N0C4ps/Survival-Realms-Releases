use super::noise::value_noise;

const BROAD_SEED: u64 = 0x243F_6A88_85A3_08D3;
const DETAIL_SEED: u64 = 0x1319_8A2E_0370_7344;
const CLIFF_SEED: u64 = 0xA409_3822_299F_31D0;
const CLIFF_EDGE_SEED: u64 = 0x082E_FA98_EC4E_6C89;

pub fn surface_height(seed: u64, x: i32, z: i32) -> i32 {
    let broad = value_noise(seed ^ BROAD_SEED, x, z, 52) * 2.0;
    let detail = value_noise(seed ^ DETAIL_SEED, x, z, 18) * 1.15;
    let rolling_height = (broad + detail).round().clamp(-3.0, 3.0) as i32;

    let cliff_region = value_noise(seed ^ CLIFF_SEED, x, z, 88);
    let cliff_edge = value_noise(seed ^ CLIFF_EDGE_SEED, x, z, 22);
    let cliff_offset = if cliff_region > 0.63 && cliff_edge > 0.28 {
        4
    } else if cliff_region < -0.63 && cliff_edge < -0.28 {
        -4
    } else {
        0
    };

    rolling_height + cliff_offset
}
