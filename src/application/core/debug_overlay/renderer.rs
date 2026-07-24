use std::sync::Arc;

use egui_wgpu::{Renderer, RendererOptions, ScreenDescriptor};
use wgpu::{
    CommandEncoder, LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor, StoreOp,
    TextureFormat, TextureView,
};
use winit::{event::WindowEvent, window::Window};

use super::{DebugSnapshot, ui};
use crate::application::core::pause_menu::{self, PauseMenuAction, PauseMenuState};
use crate::application::core::settings::GameSettings;

pub(crate) struct DebugOverlayRenderer {
    window: Arc<Window>,
    context: egui::Context,
    state: egui_winit::State,
    renderer: Renderer,
    pause_menu: PauseMenuState,
}

pub(crate) struct OverlayFrame<'a> {
    pub target: &'a TextureView,
    pub snapshot: Option<DebugSnapshot>,
    pub paused: bool,
    pub settings: GameSettings,
}

impl DebugOverlayRenderer {
    pub fn new(window: Arc<Window>, device: &wgpu::Device, surface_format: TextureFormat) -> Self {
        let context = egui::Context::default();
        let state = egui_winit::State::new(
            context.clone(),
            egui::ViewportId::ROOT,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            window.theme(),
            Some(device.limits().max_texture_dimension_2d as usize),
        );
        let renderer = Renderer::new(device, surface_format, RendererOptions::default());
        Self {
            window,
            context,
            state,
            renderer,
            pause_menu: PauseMenuState::default(),
        }
    }

    pub fn handle_window_event(&mut self, event: &WindowEvent) {
        let _ = self.state.on_window_event(&self.window, event);
    }

    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut CommandEncoder,
        frame: OverlayFrame<'_>,
    ) -> Option<PauseMenuAction> {
        let OverlayFrame {
            target,
            snapshot,
            paused,
            mut settings,
        } = frame;
        let input = self.state.take_egui_input(&self.window);
        if snapshot.is_none() && !paused {
            return None;
        }
        let context = self.context.clone();
        let mut pause_action = None;
        let output = self.context.run_ui(input, |_| {
            if let Some(snapshot) = snapshot
                && !paused
            {
                ui::draw(&context, snapshot);
            }
            if paused {
                pause_action = pause_menu::draw(&context, &mut self.pause_menu, &mut settings);
            }
        });
        self.state
            .handle_platform_output(&self.window, output.platform_output);

        for (id, delta) in &output.textures_delta.set {
            self.renderer.update_texture(device, queue, *id, delta);
        }

        let pixels_per_point = self.context.pixels_per_point();
        let paint_jobs = self.context.tessellate(output.shapes, pixels_per_point);
        let size = self.window.inner_size();
        let screen = ScreenDescriptor {
            size_in_pixels: [size.width, size.height],
            pixels_per_point,
        };
        let command_buffers =
            self.renderer
                .update_buffers(device, queue, encoder, &paint_jobs, &screen);
        debug_assert!(
            command_buffers.is_empty(),
            "debug overlay has no paint callbacks"
        );

        {
            let render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("egui debug overlay pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            self.renderer
                .render(&mut render_pass.forget_lifetime(), &paint_jobs, &screen);
        }

        for id in &output.textures_delta.free {
            self.renderer.free_texture(id);
        }
        pause_action
    }

    pub fn reset_pause_menu(&mut self) {
        self.pause_menu.reset();
    }
}
