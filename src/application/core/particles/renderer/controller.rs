use wgpu::{
    BindGroup, BindGroupLayout, Buffer, BufferDescriptor, BufferUsages, Device, Queue, RenderPass,
    RenderPipeline, TextureFormat,
};

use crate::application::core::{
    blocks::BlockRegistry, particles::ParticleSystem, paths::GamePaths,
};

use super::{instance::ParticleInstance, pipeline, texture::ParticleTextures};

const MAX_RENDERED_PARTICLES: usize = 2_048;

pub(crate) struct ParticleRenderer {
    pipeline: RenderPipeline,
    textures: ParticleTextures,
    instance_buffer: Buffer,
    instances: Vec<ParticleInstance>,
    instance_count: u32,
}

impl ParticleRenderer {
    pub fn new(
        device: &Device,
        queue: &Queue,
        registry: &BlockRegistry,
        surface_format: TextureFormat,
        depth_format: TextureFormat,
        camera_layout: &BindGroupLayout,
        paths: &GamePaths,
    ) -> Self {
        let textures = ParticleTextures::new(device, queue, registry, paths);
        let pipeline = pipeline::create(
            device,
            surface_format,
            depth_format,
            camera_layout,
            &textures.layout,
        );
        let instance_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("particle instance buffer"),
            size: (MAX_RENDERED_PARTICLES * std::mem::size_of::<ParticleInstance>()) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            pipeline,
            textures,
            instance_buffer,
            instances: Vec::with_capacity(MAX_RENDERED_PARTICLES),
            instance_count: 0,
        }
    }

    pub fn prepare(&mut self, queue: &Queue, particles: &ParticleSystem) {
        self.instances.clear();
        self.instances.extend(
            particles
                .particles()
                .iter()
                .take(MAX_RENDERED_PARTICLES)
                .map(ParticleInstance::from),
        );
        self.instance_count = self.instances.len() as u32;
        if !self.instances.is_empty() {
            queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&self.instances),
            );
        }
    }

    pub fn draw<'pass>(&'pass self, pass: &mut RenderPass<'pass>, camera: &'pass BindGroup) {
        if self.instance_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera, &[]);
        pass.set_bind_group(1, &self.textures.bind_group, &[]);
        pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
        pass.draw(0..6, 0..self.instance_count);
    }
}
