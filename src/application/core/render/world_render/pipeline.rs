use wgpu::{
    BindGroupLayout, BlendState, ColorTargetState, ColorWrites, CompareFunction, DepthStencilState,
    Device, Face, FragmentState, FrontFace, MultisampleState, PipelineLayout,
    PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology, RenderPipeline,
    RenderPipelineDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderSource, StencilState,
    TextureFormat, VertexState,
};

use super::{depth, vertex_layout};

pub(super) struct WorldPipelines {
    pub opaque: RenderPipeline,
    pub liquid: RenderPipeline,
}

pub(super) fn create(
    device: &Device,
    surface_format: TextureFormat,
    camera_layout: &BindGroupLayout,
    texture_layout: &BindGroupLayout,
) -> WorldPipelines {
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("voxel world shader"),
        source: ShaderSource::Wgsl(include_str!("../shaders/voxel_world.wgsl").into()),
    });
    let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("voxel world pipeline layout"),
        bind_group_layouts: &[Some(camera_layout), Some(texture_layout)],
        immediate_size: 0,
    });

    let opaque = create_pipeline(
        device,
        &shader,
        &layout,
        surface_format,
        "opaque voxel world pipeline",
        "fragment_main",
        None,
        true,
    );
    let liquid = create_pipeline(
        device,
        &shader,
        &layout,
        surface_format,
        "liquid voxel world pipeline",
        "liquid_fragment_main",
        Some(BlendState::ALPHA_BLENDING),
        false,
    );
    WorldPipelines { opaque, liquid }
}

#[allow(clippy::too_many_arguments)]
fn create_pipeline(
    device: &Device,
    shader: &ShaderModule,
    layout: &PipelineLayout,
    surface_format: TextureFormat,
    label: &str,
    fragment_entry: &str,
    blend: Option<BlendState>,
    depth_write_enabled: bool,
) -> RenderPipeline {
    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: VertexState {
            module: shader,
            entry_point: Some("vertex_main"),
            buffers: &[vertex_layout::layout()],
            compilation_options: Default::default(),
        },
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: Some(Face::Back),
            unclipped_depth: false,
            polygon_mode: PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(DepthStencilState {
            format: depth::FORMAT,
            depth_write_enabled: Some(depth_write_enabled),
            depth_compare: Some(CompareFunction::Less),
            stencil: StencilState::default(),
            bias: Default::default(),
        }),
        multisample: MultisampleState::default(),
        fragment: Some(FragmentState {
            module: shader,
            entry_point: Some(fragment_entry),
            targets: &[Some(ColorTargetState {
                format: surface_format,
                blend,
                write_mask: ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        multiview_mask: None,
        cache: None,
    })
}
