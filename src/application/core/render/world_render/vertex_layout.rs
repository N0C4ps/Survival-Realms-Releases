use wgpu::{BufferAddress, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};

use crate::application::core::world::Vertex;

pub(super) fn layout() -> VertexBufferLayout<'static> {
    const ATTRIBUTES: [VertexAttribute; 5] = [
        VertexAttribute {
            format: VertexFormat::Float32x3,
            offset: 0,
            shader_location: 0,
        },
        VertexAttribute {
            format: VertexFormat::Float32x3,
            offset: 12,
            shader_location: 1,
        },
        VertexAttribute {
            format: VertexFormat::Float32x2,
            offset: 24,
            shader_location: 2,
        },
        VertexAttribute {
            format: VertexFormat::Uint32,
            offset: 32,
            shader_location: 3,
        },
        VertexAttribute {
            format: VertexFormat::Uint32,
            offset: 36,
            shader_location: 4,
        },
    ];

    VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
        step_mode: VertexStepMode::Vertex,
        attributes: &ATTRIBUTES,
    }
}
