use wgpu::{
    BindGroup, BindGroupLayout, Buffer, BufferUsages, Device, Queue, RenderPass, TextureFormat,
    util::{BufferInitDescriptor, DeviceExt},
};
use winit::dpi::PhysicalSize;

use crate::application::core::paths::GamePaths;

use super::{geometry, pipeline, texture::HotbarTextures};

pub(crate) struct HotbarRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: Buffer,
    textures: HotbarTextures,
    selected: usize,
    size: PhysicalSize<u32>,
}

impl HotbarRenderer {
    pub fn new(
        device: &Device,
        queue: &Queue,
        surface_format: TextureFormat,
        depth_format: TextureFormat,
        block_texture_layout: &BindGroupLayout,
        paths: &GamePaths,
        size: PhysicalSize<u32>,
    ) -> Self {
        let textures = HotbarTextures::new(device, queue, paths);
        let selected = 0;
        let vertices = geometry::build(size, selected);
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("hotbar vertex buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        let pipeline = pipeline::create(
            device,
            surface_format,
            depth_format,
            block_texture_layout,
            &textures.layout,
        );

        Self {
            pipeline,
            vertex_buffer,
            textures,
            selected,
            size,
        }
    }

    pub fn prepare(&mut self, queue: &Queue, selected: usize, size: PhysicalSize<u32>) {
        if self.selected == selected && self.size == size {
            return;
        }
        self.selected = selected;
        self.size = size;
        queue.write_buffer(
            &self.vertex_buffer,
            0,
            bytemuck::cast_slice(&geometry::build(size, selected)),
        );
    }

    pub fn draw<'pass>(
        &'pass self,
        pass: &mut RenderPass<'pass>,
        block_textures: &'pass BindGroup,
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, block_textures, &[]);
        pass.set_bind_group(1, &self.textures.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..geometry::VERTEX_COUNT as u32, 0..1);
    }
}
