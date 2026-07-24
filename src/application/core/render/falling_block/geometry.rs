use glam::Vec3;

use crate::application::core::{
    blocks::{BlockId, TextureFace},
    world::Vertex,
};

const FACE_INDICES: [usize; 6] = [0, 1, 2, 0, 2, 3];
const UVS: [[f32; 2]; 4] = [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];

struct CubeFace {
    corners: [[f32; 3]; 4],
    normal: [f32; 3],
    texture_face: TextureFace,
}

const FACES: [CubeFace; 6] = [
    CubeFace {
        corners: [[1., 0., 0.], [1., 1., 0.], [1., 1., 1.], [1., 0., 1.]],
        normal: [1., 0., 0.],
        texture_face: TextureFace::Side,
    },
    CubeFace {
        corners: [[0., 0., 1.], [0., 1., 1.], [0., 1., 0.], [0., 0., 0.]],
        normal: [-1., 0., 0.],
        texture_face: TextureFace::Side,
    },
    CubeFace {
        corners: [[0., 1., 1.], [1., 1., 1.], [1., 1., 0.], [0., 1., 0.]],
        normal: [0., 1., 0.],
        texture_face: TextureFace::Top,
    },
    CubeFace {
        corners: [[0., 0., 0.], [1., 0., 0.], [1., 0., 1.], [0., 0., 1.]],
        normal: [0., -1., 0.],
        texture_face: TextureFace::Bottom,
    },
    CubeFace {
        corners: [[1., 0., 1.], [1., 1., 1.], [0., 1., 1.], [0., 0., 1.]],
        normal: [0., 0., 1.],
        texture_face: TextureFace::Side,
    },
    CubeFace {
        corners: [[0., 0., 0.], [0., 1., 0.], [1., 1., 0.], [1., 0., 0.]],
        normal: [0., 0., -1.],
        texture_face: TextureFace::Side,
    },
];

/// Appends a single textured unit cube (36 unindexed vertices) at `position`,
/// matching the world mesh's vertex format so it can share its pipeline.
pub(super) fn append_cube(
    vertices: &mut Vec<Vertex>,
    position: Vec3,
    block: BlockId,
    skylight: u8,
) {
    for face in &FACES {
        let texture_layer = face.texture_face.layer(block);
        let corners = face.corners.map(|corner| Vertex {
            position: [
                position.x + corner[0],
                position.y + corner[1],
                position.z + corner[2],
            ],
            normal: face.normal,
            uv: [0.0, 0.0],
            texture_layer,
            skylight: u32::from(skylight),
        });
        for &index in &FACE_INDICES {
            let mut vertex = corners[index];
            vertex.uv = UVS[index];
            vertices.push(vertex);
        }
    }
}
