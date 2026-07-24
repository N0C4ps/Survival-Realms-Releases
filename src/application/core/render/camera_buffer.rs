use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages,
    Device, Queue, ShaderStages,
};

use crate::application::core::blocks::BlockId;
use crate::application::core::player::{Camera, CameraUniform};
use crate::application::core::world::RenderDistance;

use super::fog;

pub(super) struct CameraBuffer {
    buffer: Buffer,
    pub layout: BindGroupLayout,
    pub bind_group: BindGroup,
    render_distance: RenderDistance,
    gamma: f32,
}

impl CameraBuffer {
    pub fn new(device: &Device, render_distance: RenderDistance) -> Self {
        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some("camera uniform buffer"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("camera bind group layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("camera bind group"),
            layout: &layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self {
            buffer,
            layout,
            bind_group,
            render_distance,
            gamma: 1.0,
        }
    }

    pub fn update(&self, queue: &Queue, camera: &Camera, camera_liquid: Option<BlockId>) {
        let fog = fog::settings(self.render_distance, camera_liquid);
        let uniform = CameraUniform::from_camera(camera, fog.distance, fog.color, self.gamma);
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&uniform));
    }

    pub fn set_render_distance(&mut self, render_distance: RenderDistance) {
        self.render_distance = render_distance;
    }

    pub fn set_gamma(&mut self, gamma: f32) {
        self.gamma = gamma.clamp(0.1, 3.0);
    }
}
