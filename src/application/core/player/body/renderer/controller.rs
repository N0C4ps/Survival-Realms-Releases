use wgpu::{
    BindGroup, BindGroupLayout, Buffer, BufferDescriptor, BufferUsages, Device, Queue, RenderPass,
    RenderPipeline, TextureFormat,
};

use super::{super::mesh, pipeline};
use crate::application::core::player::body::{BodyPose, mesh::BodyVertex};

const MAX_RENDERED_BODIES: usize = 16;
const MAX_VERTICES_PER_BODY: usize = 768;
const MAX_BODY_VERTICES: usize = MAX_RENDERED_BODIES * MAX_VERTICES_PER_BODY;

pub(crate) struct BodyRenderer {
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    vertices: Vec<BodyVertex>,
    vertex_count: u32,
}

impl BodyRenderer {
    pub(crate) fn new(
        device: &Device,
        surface_format: TextureFormat,
        depth_format: TextureFormat,
        camera_layout: &BindGroupLayout,
    ) -> Self {
        Self {
            pipeline: pipeline::create(device, surface_format, depth_format, camera_layout),
            vertex_buffer: device.create_buffer(&BufferDescriptor {
                label: Some("player body vertex buffer"),
                size: (MAX_BODY_VERTICES * std::mem::size_of::<BodyVertex>()) as u64,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            vertices: Vec::with_capacity(MAX_BODY_VERTICES),
            vertex_count: 0,
        }
    }

    pub(crate) fn prepare_remote_players(&mut self, queue: &Queue, poses: &[BodyPose]) {
        self.vertices.clear();
        for &pose in poses.iter().take(MAX_RENDERED_BODIES) {
            mesh::append(pose, true, &mut self.vertices);
        }
        debug_assert!(self.vertices.len() <= MAX_BODY_VERTICES);
        self.vertex_count = self.vertices.len() as u32;
        if !self.vertices.is_empty() {
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
        }
    }

    pub(crate) fn draw<'pass>(&'pass self, pass: &mut RenderPass<'pass>, camera: &'pass BindGroup) {
        if self.vertex_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
    }
}
