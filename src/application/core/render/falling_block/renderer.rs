use wgpu::{BindGroup, Buffer, BufferDescriptor, BufferUsages, Device, Queue, RenderPass};

use crate::application::core::{physics::FallingBlockSystem, world::Vertex};

use super::{super::world_render::WorldRenderer, geometry};

const INITIAL_CAPACITY: usize = 36 * 32;

/// Draws currently falling sand/gravel blocks as small textured cubes at
/// their fractional (mid-air) position, reusing the world renderer's opaque
/// voxel pipeline so they're lit and textured exactly like placed blocks.
pub(crate) struct FallingBlockRenderer {
    vertex_buffer: Buffer,
    capacity: usize,
    vertex_count: u32,
}

impl FallingBlockRenderer {
    pub fn new(device: &Device) -> Self {
        Self {
            vertex_buffer: create_buffer(device, INITIAL_CAPACITY),
            capacity: INITIAL_CAPACITY,
            vertex_count: 0,
        }
    }

    pub fn prepare(&mut self, device: &Device, queue: &Queue, falling_blocks: &FallingBlockSystem) {
        let mut vertices = Vec::new();
        for instance in falling_blocks.instances() {
            geometry::append_cube(
                &mut vertices,
                instance.position,
                instance.block,
                instance.skylight,
            );
        }

        if vertices.len() > self.capacity {
            self.capacity = vertices.len().next_power_of_two();
            self.vertex_buffer = create_buffer(device, self.capacity);
        }

        self.vertex_count = vertices.len() as u32;
        if !vertices.is_empty() {
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        }
    }

    pub fn draw<'pass>(
        &'pass self,
        render_pass: &mut RenderPass<'pass>,
        world: &'pass WorldRenderer,
        camera_bind_group: &'pass BindGroup,
    ) {
        if self.vertex_count == 0 {
            return;
        }
        world.bind_opaque(render_pass, camera_bind_group);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..self.vertex_count, 0..1);
    }
}

fn create_buffer(device: &Device, capacity: usize) -> Buffer {
    device.create_buffer(&BufferDescriptor {
        label: Some("falling block vertex buffer"),
        size: (capacity * std::mem::size_of::<Vertex>()) as u64,
        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
