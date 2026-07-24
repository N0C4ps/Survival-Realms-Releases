use std::sync::Arc;

use winit::{dpi::PhysicalSize, window::Window};

use super::{
    camera_buffer::CameraBuffer, context::RenderContext, falling_block::FallingBlockRenderer,
    frame, highlight::HighlightRenderer, post_process::PausePostProcess, sky::SkyRenderer, surface,
    world_render::WorldRenderer,
};
use crate::application::core::{
    blocks::BlockId,
    blocks::BlockRegistry,
    debug_overlay::{DebugOverlayRenderer, DebugSnapshot},
    particles::{ParticleRenderer, ParticleSystem},
    paths::GamePaths,
    pause_menu::PauseMenuAction,
    physics::FallingBlockSystem,
    player::{BodyPose, BodyRenderer, Camera, CrosshairRenderer, HotbarRenderer},
    settings::GameSettings,
    world::{RenderDistance, World},
};
use glam::IVec3;

pub struct Renderer {
    context: RenderContext,
    camera_buffer: CameraBuffer,
    sky_renderer: SkyRenderer,
    world_renderer: WorldRenderer,
    highlight_renderer: HighlightRenderer,
    crosshair_renderer: CrosshairRenderer,
    hotbar_renderer: HotbarRenderer,
    particle_renderer: ParticleRenderer,
    falling_block_renderer: FallingBlockRenderer,
    body_renderer: BodyRenderer,
    debug_overlay_renderer: DebugOverlayRenderer,
    pause_post_process: PausePostProcess,
}

pub(crate) struct RenderFrame<'a> {
    pub camera: &'a Camera,
    pub camera_liquid: Option<BlockId>,
    pub targeted_block: Option<IVec3>,
    pub selected_hotbar_slot: usize,
    pub particles: &'a ParticleSystem,
    pub falling_blocks: &'a FallingBlockSystem,
    pub remote_player_poses: &'a [BodyPose],
    pub debug_snapshot: Option<DebugSnapshot>,
    pub paused: bool,
    pub settings: GameSettings,
}

impl Renderer {
    pub async fn new(
        window: Arc<Window>,
        registry: &BlockRegistry,
        world: &mut World,
        render_distance: RenderDistance,
        paths: &GamePaths,
    ) -> Self {
        let context = RenderContext::new(Arc::clone(&window)).await;
        let camera_buffer = CameraBuffer::new(&context.device, render_distance);
        let sky_renderer = SkyRenderer::new(&context, &camera_buffer);
        let world_renderer = WorldRenderer::new(
            &context,
            &camera_buffer,
            registry,
            world,
            render_distance,
            paths,
        );
        let highlight_renderer = HighlightRenderer::new(
            &context.device,
            context.config.format,
            &camera_buffer.layout,
        );
        let crosshair_renderer = CrosshairRenderer::new(
            &context.device,
            context.config.format,
            super::world_render::DEPTH_FORMAT,
            PhysicalSize::new(context.config.width, context.config.height),
        );
        let hotbar_renderer = HotbarRenderer::new(
            &context.device,
            &context.queue,
            context.config.format,
            super::world_render::DEPTH_FORMAT,
            world_renderer.texture_layout(),
            paths,
            PhysicalSize::new(context.config.width, context.config.height),
        );
        let particle_renderer = ParticleRenderer::new(
            &context.device,
            &context.queue,
            registry,
            context.config.format,
            super::world_render::DEPTH_FORMAT,
            &camera_buffer.layout,
            paths,
        );
        let falling_block_renderer = FallingBlockRenderer::new(&context.device);
        let body_renderer = BodyRenderer::new(
            &context.device,
            context.config.format,
            super::world_render::DEPTH_FORMAT,
            &camera_buffer.layout,
        );
        let debug_overlay_renderer =
            DebugOverlayRenderer::new(window, &context.device, context.config.format);
        let pause_post_process = PausePostProcess::new(&context);

        Self {
            context,
            camera_buffer,
            sky_renderer,
            world_renderer,
            highlight_renderer,
            crosshair_renderer,
            hotbar_renderer,
            particle_renderer,
            falling_block_renderer,
            body_renderer,
            debug_overlay_renderer,
            pause_post_process,
        }
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        surface::resize(&mut self.context, size);
        if size.width > 0 && size.height > 0 {
            self.world_renderer.resize(&self.context);
            self.pause_post_process.resize(&self.context);
            self.crosshair_renderer.resize(&self.context.device, size);
        }
    }

    pub fn render(&mut self, frame_input: RenderFrame<'_>) -> Option<PauseMenuAction> {
        let RenderFrame {
            camera,
            camera_liquid,
            targeted_block,
            selected_hotbar_slot,
            particles,
            falling_blocks,
            remote_player_poses,
            debug_snapshot,
            paused,
            settings,
        } = frame_input;
        self.camera_buffer
            .update(&self.context.queue, camera, camera_liquid);
        self.highlight_renderer
            .prepare(&self.context.queue, targeted_block);
        self.hotbar_renderer.prepare(
            &self.context.queue,
            selected_hotbar_slot,
            PhysicalSize::new(self.context.config.width, self.context.config.height),
        );
        self.particle_renderer
            .prepare(&self.context.queue, particles);
        self.falling_block_renderer.prepare(
            &self.context.device,
            &self.context.queue,
            falling_blocks,
        );
        self.body_renderer
            .prepare_remote_players(&self.context.queue, remote_player_poses);
        frame::draw(
            &mut self.context,
            frame::FrameRenderers {
                sky: &self.sky_renderer,
                world: &self.world_renderer,
                particles: &self.particle_renderer,
                falling_blocks: &self.falling_block_renderer,
                body: &self.body_renderer,
                highlight: &self.highlight_renderer,
                crosshair: &self.crosshair_renderer,
                hotbar: &self.hotbar_renderer,
                debug_overlay: &mut self.debug_overlay_renderer,
                pause_post_process: &self.pause_post_process,
            },
            &self.camera_buffer.bind_group,
            camera,
            debug_snapshot,
            paused,
            settings,
        )
    }

    pub fn handle_window_event(&mut self, event: &winit::event::WindowEvent) {
        self.debug_overlay_renderer.handle_window_event(event);
    }

    pub fn reset_pause_menu(&mut self) {
        self.debug_overlay_renderer.reset_pause_menu();
    }

    pub fn apply_settings(&mut self, settings: GameSettings) {
        let render_distance = RenderDistance::new(u32::from(settings.render_distance));
        self.camera_buffer.set_render_distance(render_distance);
        self.camera_buffer.set_gamma(settings.gamma());
        self.world_renderer.set_render_distance(render_distance);
    }

    pub fn schedule_world_updates(&mut self, world: &World, dirty: &[IVec3], removed: &[IVec3]) {
        self.world_renderer.schedule_changes(world, dirty, removed);
    }

    pub fn prioritize_world_updates(&mut self, world: &World, positions: &[IVec3]) {
        self.world_renderer.prioritize(world, positions);
    }

    pub fn process_world_updates(
        &mut self,
        registry: &BlockRegistry,
        world: &mut World,
        mesh_budget: usize,
    ) {
        self.world_renderer
            .process_pending(&self.context.device, registry, world, mesh_budget);
    }
}
