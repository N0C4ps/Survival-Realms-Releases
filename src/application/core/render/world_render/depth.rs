use wgpu::{
    Extent3d, LoadOp, Operations, RenderPassDepthStencilAttachment, StoreOp, Texture,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor,
};

use super::super::context::RenderContext;

pub(crate) const FORMAT: TextureFormat = TextureFormat::Depth32Float;

pub(super) struct DepthTarget {
    _texture: Texture,
    view: TextureView,
}

impl DepthTarget {
    pub fn new(context: &RenderContext) -> Self {
        let texture = context.device.create_texture(&TextureDescriptor {
            label: Some("world depth texture"),
            size: Extent3d {
                width: context.config.width,
                height: context.config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        Self {
            _texture: texture,
            view,
        }
    }

    pub fn attachment(&self) -> RenderPassDepthStencilAttachment<'_> {
        RenderPassDepthStencilAttachment {
            view: &self.view,
            depth_ops: Some(Operations {
                load: LoadOp::Clear(1.0),
                store: StoreOp::Store,
            }),
            stencil_ops: None,
        }
    }
}
