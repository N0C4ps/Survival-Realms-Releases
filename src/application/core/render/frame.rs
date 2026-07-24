use wgpu::{
    BindGroup, Color, CommandEncoderDescriptor, LoadOp, Operations, RenderPassColorAttachment,
    RenderPassDescriptor, StoreOp, TextureViewDescriptor,
};

use super::{
    context::RenderContext, falling_block::FallingBlockRenderer, highlight::HighlightRenderer,
    post_process::PausePostProcess, sky::SkyRenderer, surface, world_render::WorldRenderer,
};
use crate::application::core::{
    debug_overlay::{DebugOverlayRenderer, DebugSnapshot, OverlayFrame},
    particles::ParticleRenderer,
    pause_menu::PauseMenuAction,
    player::{BodyRenderer, Camera, CrosshairRenderer, HotbarRenderer},
};

const SKY_COLOR: Color = Color {
    r: 0.45,
    g: 0.70,
    b: 1.0,
    a: 1.0,
};

pub(super) struct FrameRenderers<'a> {
    pub sky: &'a SkyRenderer,
    pub world: &'a WorldRenderer,
    pub particles: &'a ParticleRenderer,
    pub falling_blocks: &'a FallingBlockRenderer,
    pub body: &'a BodyRenderer,
    pub highlight: &'a HighlightRenderer,
    pub crosshair: &'a CrosshairRenderer,
    pub hotbar: &'a HotbarRenderer,
    pub debug_overlay: &'a mut DebugOverlayRenderer,
    pub pause_post_process: &'a PausePostProcess,
}

pub(super) fn draw(
    context: &mut RenderContext,
    renderers: FrameRenderers<'_>,
    camera_bind_group: &BindGroup,
    camera: &Camera,
    debug_snapshot: Option<DebugSnapshot>,
    paused: bool,
    settings: crate::application::core::settings::GameSettings,
) -> Option<PauseMenuAction> {
    let frame = surface::acquire_frame(context)?;

    let view = frame.texture.create_view(&TextureViewDescriptor::default());
    let mut encoder = context
        .device
        .create_command_encoder(&CommandEncoderDescriptor {
            label: Some("main render encoder"),
        });

    {
        let world_target = if paused {
            renderers.pause_post_process.scene_view()
        } else {
            &view
        };
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("world render pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: world_target,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(SKY_COLOR),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(renderers.world.depth_attachment()),
            ..Default::default()
        });
        renderers.sky.draw(&mut render_pass, camera_bind_group);
        renderers
            .world
            .draw(&mut render_pass, camera_bind_group, camera);
        renderers
            .falling_blocks
            .draw(&mut render_pass, renderers.world, camera_bind_group);
        renderers.body.draw(&mut render_pass, camera_bind_group);
        renderers
            .particles
            .draw(&mut render_pass, camera_bind_group);
        renderers
            .highlight
            .draw(&mut render_pass, camera_bind_group);
        renderers
            .hotbar
            .draw(&mut render_pass, renderers.world.texture_bind_group());
        renderers.crosshair.draw(&mut render_pass);
    }

    if paused {
        renderers
            .pause_post_process
            .composite(context, &mut encoder, &view);
    }

    let pause_action = renderers.debug_overlay.draw(
        &context.device,
        &context.queue,
        &mut encoder,
        OverlayFrame {
            target: &view,
            snapshot: debug_snapshot,
            paused,
            settings,
        },
    );

    context.queue.submit([encoder.finish()]);
    frame.present();
    pause_action
}
