use std::sync::Arc;

use winit::event_loop::ActiveEventLoop;

use super::{
    super::{player::Player, render::Renderer, window, world},
    state::Runtime,
};

const INITIAL_CHUNK_LOAD_BUDGET: usize = 16;

impl Runtime {
    pub(super) fn resume(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = window::create(event_loop);
        window::capture_cursor(&window);
        let size = window.inner_size();
        let (mut world, saved_player_position) = self.load_or_create_world();
        self.grass.reset(self.terrain_generator.seed());
        let aspect_ratio = size.width as f32 / size.height.max(1) as f32;
        let mut player = if let Some(position) = saved_player_position {
            Player::at_position(aspect_ratio, position)
        } else {
            let spawn = self.terrain_generator.spawn_block();
            Player::new(aspect_ratio, spawn)
        };
        player.apply_settings(self.settings);
        let spawn_chunk = world::chunk_from_position(player.position());
        self.chunk_streamer
            .update_center(spawn_chunk, &mut world, self.terrain_generator);
        self.chunk_streamer.process_pending_blocking(
            &mut world,
            self.terrain_generator,
            INITIAL_CHUNK_LOAD_BUDGET,
        );
        for change in world.take_pending_light_updates() {
            self.skylight_updates.schedule(change);
        }
        world.take_mesh_priorities();
        let renderer = pollster::block_on(Renderer::new(
            Arc::clone(&window),
            &self.blocks,
            &mut world,
            self.chunk_streamer.render_distance(),
            &self.paths,
        ));

        self.player = Some(player);
        self.world = Some(world);
        self.renderer = Some(renderer);
        self.window = Some(window);
        self.clock.reset();
        self.blocks.validate();
        tracing::info!(
            registered_blocks = self.blocks.len(),
            resident_chunks = self.world.as_ref().map_or(0, |world| world.chunks().len()),
            pending_chunks = self.chunk_streamer.pending_count(),
            "runtime initialized; remaining chunks will stream in"
        );
    }
}
