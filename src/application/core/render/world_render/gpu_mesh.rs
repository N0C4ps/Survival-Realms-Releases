use wgpu::{
    Buffer, BufferUsages, Device, IndexFormat, RenderPass,
    util::{BufferInitDescriptor, DeviceExt},
};

use crate::application::core::world::ChunkMesh;

pub(super) struct GpuChunkMesh {
    pub chunk_position: glam::IVec3,
    opaque: Option<GpuGeometry>,
    liquid: Option<GpuGeometry>,
}

struct GpuGeometry {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    index_count: u32,
}

impl GpuChunkMesh {
    pub fn new(device: &Device, mesh: &ChunkMesh) -> Self {
        Self {
            chunk_position: mesh.chunk_position,
            opaque: mesh
                .has_opaque_geometry()
                .then(|| GpuGeometry::new(device, "opaque chunk", &mesh.vertices, &mesh.indices)),
            liquid: mesh.has_liquid_geometry().then(|| {
                GpuGeometry::new(
                    device,
                    "liquid chunk",
                    &mesh.liquid_vertices,
                    &mesh.liquid_indices,
                )
            }),
        }
    }

    pub fn draw_opaque<'pass>(&'pass self, render_pass: &mut RenderPass<'pass>) {
        if let Some(geometry) = self.opaque.as_ref() {
            geometry.draw(render_pass);
        }
    }

    pub fn draw_liquids<'pass>(&'pass self, render_pass: &mut RenderPass<'pass>) {
        if let Some(geometry) = self.liquid.as_ref() {
            geometry.draw(render_pass);
        }
    }

    pub const fn has_liquids(&self) -> bool {
        self.liquid.is_some()
    }
}

impl GpuGeometry {
    fn new(
        device: &Device,
        label: &str,
        vertices: &[crate::application::core::world::Vertex],
        indices: &[u32],
    ) -> Self {
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(vertices),
            usage: BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(indices),
            usage: BufferUsages::INDEX,
        });

        Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        }
    }

    pub fn draw<'pass>(&'pass self, render_pass: &mut RenderPass<'pass>) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}
