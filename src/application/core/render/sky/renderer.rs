use wgpu::{BindGroup, RenderPass, RenderPipeline};

use super::pipeline;
use crate::application::core::render::{camera_buffer::CameraBuffer, context::RenderContext};

pub(crate) struct SkyRenderer {
    pipeline: RenderPipeline,
}

impl SkyRenderer {
    pub fn new(context: &RenderContext, camera: &CameraBuffer) -> Self {
        Self {
            pipeline: pipeline::create(&context.device, context.config.format, &camera.layout),
        }
    }

    pub fn draw<'pass>(
        &'pass self,
        render_pass: &mut RenderPass<'pass>,
        camera_bind_group: &'pass BindGroup,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
