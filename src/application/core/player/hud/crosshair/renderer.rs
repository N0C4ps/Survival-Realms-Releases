use wgpu::{
    Buffer, BufferUsages, Device, RenderPass, RenderPipeline, TextureFormat,
    util::{BufferInitDescriptor, DeviceExt},
};
use winit::dpi::PhysicalSize;

use super::{geometry, pipeline};

pub(crate) struct CrosshairRenderer {
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
}

impl CrosshairRenderer {
    pub fn new(
        device: &Device,
        surface_format: TextureFormat,
        depth_format: TextureFormat,
        size: PhysicalSize<u32>,
    ) -> Self {
        Self {
            pipeline: pipeline::create(device, surface_format, depth_format),
            vertex_buffer: create_vertex_buffer(device, size),
        }
    }

    pub fn resize(&mut self, device: &Device, size: PhysicalSize<u32>) {
        self.vertex_buffer = create_vertex_buffer(device, size);
    }

    pub fn draw<'pass>(&'pass self, render_pass: &mut RenderPass<'pass>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..12, 0..1);
    }
}

fn create_vertex_buffer(device: &Device, size: PhysicalSize<u32>) -> Buffer {
    device.create_buffer_init(&BufferInitDescriptor {
        label: Some("crosshair vertex buffer"),
        contents: bytemuck::cast_slice(&geometry::vertices(size.width, size.height)),
        usage: BufferUsages::VERTEX,
    })
}
