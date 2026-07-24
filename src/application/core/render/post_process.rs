use bytemuck::{Pod, Zeroable};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferUsages, ColorTargetState,
    ColorWrites, CommandEncoder, Device, Extent3d, FilterMode, FragmentState, LoadOp,
    MultisampleState, Operations, PipelineLayoutDescriptor, PrimitiveState,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    Sampler, SamplerBindingType, SamplerDescriptor, ShaderModuleDescriptor, ShaderSource,
    ShaderStages, StoreOp, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension,
    VertexState,
    util::{BufferInitDescriptor, DeviceExt},
};

use super::context::RenderContext;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PauseEffectUniform {
    texel_size: [f32; 2],
    blur_enabled: f32,
    darkness: f32,
}

pub(super) struct PausePostProcess {
    _scene_texture: Texture,
    scene_view: TextureView,
    sampler: Sampler,
    layout: BindGroupLayout,
    bind_group: BindGroup,
    uniform: Buffer,
    pipeline: RenderPipeline,
}

impl PausePostProcess {
    pub fn new(context: &RenderContext) -> Self {
        let sampler = context.device.create_sampler(&SamplerDescriptor {
            label: Some("pause scene sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });
        let layout = create_layout(&context.device);
        let uniform = context.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("pause effect uniform"),
            contents: bytemuck::bytes_of(&effect_uniform(context, true)),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let (scene_texture, scene_view) = create_scene_target(context);
        let bind_group =
            create_bind_group(&context.device, &layout, &scene_view, &sampler, &uniform);
        let pipeline = create_pipeline(&context.device, context.config.format, &layout);
        Self {
            _scene_texture: scene_texture,
            scene_view,
            sampler,
            layout,
            bind_group,
            uniform,
            pipeline,
        }
    }

    pub fn resize(&mut self, context: &RenderContext) {
        let (texture, view) = create_scene_target(context);
        self._scene_texture = texture;
        self.scene_view = view;
        self.bind_group = create_bind_group(
            &context.device,
            &self.layout,
            &self.scene_view,
            &self.sampler,
            &self.uniform,
        );
    }

    pub fn scene_view(&self) -> &TextureView {
        &self.scene_view
    }

    pub fn composite(
        &self,
        context: &RenderContext,
        encoder: &mut CommandEncoder,
        target: &TextureView,
    ) {
        context.queue.write_buffer(
            &self.uniform,
            0,
            bytemuck::bytes_of(&effect_uniform(context, true)),
        );
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("pause blur and dim pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(wgpu::Color::BLACK),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

fn effect_uniform(context: &RenderContext, paused: bool) -> PauseEffectUniform {
    PauseEffectUniform {
        texel_size: [
            1.0 / context.config.width.max(1) as f32,
            1.0 / context.config.height.max(1) as f32,
        ],
        blur_enabled: if paused { 1.0 } else { 0.0 },
        darkness: if paused { 0.68 } else { 0.0 },
    }
}

fn create_scene_target(context: &RenderContext) -> (Texture, TextureView) {
    let texture = context.device.create_texture(&TextureDescriptor {
        label: Some("pause scene texture"),
        size: Extent3d {
            width: context.config.width.max(1),
            height: context.config.height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: context.config.format,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&TextureViewDescriptor::default());
    (texture, view)
}

fn create_layout(device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("pause effect bind group layout"),
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

fn create_bind_group(
    device: &Device,
    layout: &BindGroupLayout,
    scene: &TextureView,
    sampler: &Sampler,
    uniform: &Buffer,
) -> BindGroup {
    device.create_bind_group(&BindGroupDescriptor {
        label: Some("pause effect bind group"),
        layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(scene),
            },
            BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            BindGroupEntry {
                binding: 2,
                resource: uniform.as_entire_binding(),
            },
        ],
    })
}

fn create_pipeline(
    device: &Device,
    format: TextureFormat,
    layout: &BindGroupLayout,
) -> RenderPipeline {
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("pause blur shader"),
        source: ShaderSource::Wgsl(include_str!("shaders/pause_blur.wgsl").into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("pause effect pipeline layout"),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("pause effect pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vertex_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        primitive: PrimitiveState::default(),
        depth_stencil: None,
        multisample: MultisampleState::default(),
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: Some("fragment_main"),
            targets: &[Some(ColorTargetState {
                format,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        multiview_mask: None,
        cache: None,
    })
}
