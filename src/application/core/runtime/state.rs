use std::sync::Arc;

use winit::window::Window;

use super::{
    super::{
        blocks::BlockRegistry,
        debug_overlay::DebugOverlay,
        fluids::FluidSystem,
        lighting::skylight::{SkylightPropagationQueue, SkylightUpdateQueue},
        particles::ParticleSystem,
        paths::GamePaths,
        persistence::PersistenceService,
        physics::FallingBlockSystem,
        player::Player,
        render::Renderer,
        settings::{GameSettings, SettingsStore},
        vegetation::GrassSystem,
        world::{ChunkStreamer, RenderDistance, TerrainDimensions, World, WorldGenerator},
    },
    clock::FrameClock,
};

pub(super) struct Runtime {
    pub paths: GamePaths,
    pub window: Option<Arc<Window>>,
    pub renderer: Option<Renderer>,
    pub player: Option<Player>,
    pub clock: FrameClock,
    pub blocks: BlockRegistry,
    pub world: Option<World>,
    pub terrain_generator: WorldGenerator,
    pub chunk_streamer: ChunkStreamer,
    pub skylight_updates: SkylightUpdateQueue,
    pub skylight_propagation: SkylightPropagationQueue,
    pub paused: bool,
    pub exit_requested: bool,
    pub persistence: PersistenceService,
    pub level_dirty: bool,
    pub particles: ParticleSystem,
    pub debug_overlay: DebugOverlay,
    pub grass: GrassSystem,
    pub fluids: FluidSystem,
    pub falling_blocks: FallingBlockSystem,
    pub settings: GameSettings,
    pub settings_store: SettingsStore,
}

impl Runtime {
    pub fn new(paths: GamePaths) -> Self {
        let persistence = PersistenceService::new(paths.level());
        let (settings_store, settings) = SettingsStore::load(paths.settings());
        Self {
            paths,
            window: None,
            renderer: None,
            player: None,
            clock: FrameClock::default(),
            blocks: BlockRegistry::default(),
            world: None,
            terrain_generator: WorldGenerator::procedural(TerrainDimensions::default(), 0),
            chunk_streamer: ChunkStreamer::new(RenderDistance::new(u32::from(
                settings.render_distance,
            ))),
            skylight_updates: SkylightUpdateQueue::default(),
            skylight_propagation: SkylightPropagationQueue::default(),
            paused: false,
            exit_requested: false,
            persistence,
            level_dirty: false,
            particles: ParticleSystem::default(),
            debug_overlay: DebugOverlay::default(),
            grass: GrassSystem::default(),
            fluids: FluidSystem::default(),
            falling_blocks: FallingBlockSystem::default(),
            settings,
            settings_store,
        }
    }
}
