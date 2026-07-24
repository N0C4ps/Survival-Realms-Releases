use std::sync::Arc;

use wgpu::{
    Adapter, Device, DeviceDescriptor, Instance, PowerPreference, PresentMode, Queue,
    RequestAdapterOptions, Surface, SurfaceConfiguration,
};
use winit::window::Window;

pub(super) struct RenderContext {
    _instance: Instance,
    pub surface: Surface<'static>,
    _adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
    pub config: SurfaceConfiguration,
}

impl RenderContext {
    pub async fn new(window: Arc<Window>) -> Self {
        let instance = Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .expect("failed to create the rendering surface");
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .expect("failed to find a compatible graphics adapter");
        tracing::info!(adapter = ?adapter.get_info(), "graphics adapter selected");
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor::default())
            .await
            .expect("failed to create the graphics device");

        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .expect("surface is not supported by the graphics adapter");
        config.present_mode = PresentMode::Fifo;
        surface.configure(&device, &config);
        tracing::debug!(
            width = config.width,
            height = config.height,
            format = ?config.format,
            present_mode = ?config.present_mode,
            "render surface configured"
        );

        Self {
            _instance: instance,
            surface,
            _adapter: adapter,
            device,
            queue,
            config,
        }
    }
}
