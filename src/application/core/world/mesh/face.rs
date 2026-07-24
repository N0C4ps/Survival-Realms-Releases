use glam::IVec3;

pub(super) struct Face {
    pub neighbour: IVec3,
    pub normal: [f32; 3],
}

pub(super) const FACES: [Face; 6] = [
    Face {
        neighbour: IVec3::X,
        normal: [1.0, 0.0, 0.0],
    },
    Face {
        neighbour: IVec3::NEG_X,
        normal: [-1.0, 0.0, 0.0],
    },
    Face {
        neighbour: IVec3::Y,
        normal: [0.0, 1.0, 0.0],
    },
    Face {
        neighbour: IVec3::NEG_Y,
        normal: [0.0, -1.0, 0.0],
    },
    Face {
        neighbour: IVec3::Z,
        normal: [0.0, 0.0, 1.0],
    },
    Face {
        neighbour: IVec3::NEG_Z,
        normal: [0.0, 0.0, -1.0],
    },
];
