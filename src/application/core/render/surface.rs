use wgpu::{CurrentSurfaceTexture, SurfaceTexture};
use winit::dpi::PhysicalSize;

use super::context::RenderContext;

pub(super) fn resize(context: &mut RenderContext, size: PhysicalSize<u32>) {
    if size.width == 0 || size.height == 0 {
        return;
    }

    context.config.width = size.width;
    context.config.height = size.height;
    configure(context);
}

pub(super) fn acquire_frame(context: &RenderContext) -> Option<SurfaceTexture> {
    match context.surface.get_current_texture() {
        CurrentSurfaceTexture::Success(frame) => Some(frame),
        CurrentSurfaceTexture::Suboptimal(frame) => {
            configure(context);
            Some(frame)
        }
        CurrentSurfaceTexture::Outdated => {
            configure(context);
            None
        }
        CurrentSurfaceTexture::Timeout
        | CurrentSurfaceTexture::Occluded
        | CurrentSurfaceTexture::Lost
        | CurrentSurfaceTexture::Validation => None,
    }
}

fn configure(context: &RenderContext) {
    context.surface.configure(&context.device, &context.config);
}
