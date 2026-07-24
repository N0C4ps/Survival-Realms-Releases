use glam::IVec3;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages,
    Device, Queue, RenderPass, RenderPipeline, ShaderStages, TextureFormat,
    util::{BufferInitDescriptor, DeviceExt},
};

use super::{geometry, pipeline, uniform::HighlightUniform};

pub(crate) struct HighlightRenderer {
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    uniform_buffer: Buffer,
    bind_group: BindGroup,
    selected: Option<IVec3>,
}

impl HighlightRenderer {
    pub fn new(device: &Device, format: TextureFormat, camera_layout: &BindGroupLayout) -> Self {
        let uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("block highlight uniform buffer"),
            size: std::mem::size_of::<HighlightUniform>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("block highlight bind group layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("block highlight bind group"),
            layout: &uniform_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("block highlight vertex buffer"),
            contents: bytemuck::cast_slice(&geometry::VERTICES),
            usage: BufferUsages::VERTEX,
        });

        Self {
            pipeline: pipeline::create(device, format, camera_layout, &uniform_layout),
            vertex_buffer,
            uniform_buffer,
            bind_group,
            selected: None,
        }
    }

    pub fn prepare(&mut self, queue: &Queue, selected: Option<IVec3>) {
        if selected == self.selected {
            return;
        }
        self.selected = selected;
        if let Some(block) = selected {
            queue.write_buffer(
                &self.uniform_buffer,
                0,
                bytemuck::bytes_of(&HighlightUniform::from(block)),
            );
        }
    }

    pub fn draw<'pass>(&'pass self, pass: &mut RenderPass<'pass>, camera: &'pass BindGroup) {
        if self.selected.is_none() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera, &[]);
        pass.set_bind_group(1, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..geometry::VERTICES.len() as u32, 0..1);
    }
}
