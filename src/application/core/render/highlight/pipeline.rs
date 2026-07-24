use wgpu::{
    BindGroupLayout, ColorTargetState, ColorWrites, CompareFunction, DepthStencilState, Device,
    FragmentState, MultisampleState, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology,
    RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource, StencilState,
    TextureFormat, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};

use super::{super::world_render, geometry::HighlightVertex};

pub(super) fn create(
    device: &Device,
    surface_format: TextureFormat,
    camera_layout: &BindGroupLayout,
    highlight_layout: &BindGroupLayout,
) -> RenderPipeline {
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("block highlight shader"),
        source: ShaderSource::Wgsl(include_str!("../shaders/block_highlight.wgsl").into()),
    });
    let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("block highlight pipeline layout"),
        bind_group_layouts: &[Some(camera_layout), Some(highlight_layout)],
        immediate_size: 0,
    });
    let attributes = [VertexAttribute {
        format: VertexFormat::Float32x3,
        offset: 0,
        shader_location: 0,
    }];

    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("block highlight pipeline"),
        layout: Some(&layout),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vertex_main"),
            buffers: &[VertexBufferLayout {
                array_stride: std::mem::size_of::<HighlightVertex>() as u64,
                step_mode: VertexStepMode::Vertex,
                attributes: &attributes,
            }],
            compilation_options: Default::default(),
        },
        primitive: PrimitiveState {
            topology: PrimitiveTopology::LineList,
            ..Default::default()
        },
        depth_stencil: Some(DepthStencilState {
            format: world_render::DEPTH_FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(CompareFunction::LessEqual),
            stencil: StencilState::default(),
            bias: Default::default(),
        }),
        multisample: MultisampleState::default(),
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: Some("fragment_main"),
            targets: &[Some(ColorTargetState {
                format: surface_format,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        multiview_mask: None,
        cache: None,
    })
}
