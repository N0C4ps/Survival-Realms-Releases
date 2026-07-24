use glam::IVec3;

pub const SPREAD_DIRECTIONS: [IVec3; 24] = [
    IVec3::new(-1, -1, -1),
    IVec3::new(-1, -1, 0),
    IVec3::new(-1, -1, 1),
    IVec3::new(0, -1, -1),
    IVec3::new(0, -1, 1),
    IVec3::new(1, -1, -1),
    IVec3::new(1, -1, 0),
    IVec3::new(1, -1, 1),
    IVec3::new(-1, 0, -1),
    IVec3::new(-1, 0, 0),
    IVec3::new(-1, 0, 1),
    IVec3::new(0, 0, -1),
    IVec3::new(0, 0, 1),
    IVec3::new(1, 0, -1),
    IVec3::new(1, 0, 0),
    IVec3::new(1, 0, 1),
    IVec3::new(-1, 1, -1),
    IVec3::new(-1, 1, 0),
    IVec3::new(-1, 1, 1),
    IVec3::new(0, 1, -1),
    IVec3::new(0, 1, 1),
    IVec3::new(1, 1, -1),
    IVec3::new(1, 1, 0),
    IVec3::new(1, 1, 1),
];

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn spread_uses_every_neighbour_except_the_vertical_axis() {
        let unique: HashSet<_> = SPREAD_DIRECTIONS.into_iter().collect();
        assert_eq!(unique.len(), 24);
        assert!(!unique.contains(&IVec3::ZERO));
        assert!(!unique.contains(&IVec3::Y));
        assert!(!unique.contains(&IVec3::NEG_Y));
        assert!(unique.iter().all(|direction| {
            direction
                .to_array()
                .into_iter()
                .all(|axis| (-1..=1).contains(&axis))
        }));
    }
}
