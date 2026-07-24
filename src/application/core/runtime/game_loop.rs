use super::state::Runtime;
use crate::application::core::{debug_overlay::DebugSnapshot, render::RenderFrame, world};

const CHUNK_LOAD_BUDGET_PER_FRAME: usize = 1;
const CHUNK_MESH_BUDGET_PER_FRAME: usize = 1;
const PRIORITY_MESH_BUDGET_PER_FRAME: usize = 2;
const SKYLIGHT_UPDATE_BUDGET_PER_FRAME: usize = 4;
const SKYLIGHT_PROPAGATION_BUDGET_PER_FRAME: usize = 1_024;

impl Runtime {
    pub(super) fn render_next_frame(&mut self) {
        if let Err(error) = self.settings_store.flush_if_due() {
            tracing::warn!(%error, "settings change could not be saved");
        }
        if self.paused {
            self.clock.reset();
            let debug_snapshot = self.debug_snapshot();
            let pause_action = if let (Some(renderer), Some(player)) =
                (self.renderer.as_mut(), self.player.as_ref())
            {
                renderer.render(RenderFrame {
                    camera: player.camera(),
                    camera_liquid: player.camera_liquid(),
                    targeted_block: player.targeted_block(),
                    selected_hotbar_slot: player.selected_hotbar_slot(),
                    particles: &self.particles,
                    falling_blocks: &self.falling_blocks,
                    remote_player_poses: &[],
                    debug_snapshot,
                    paused: true,
                    settings: self.settings,
                })
            } else {
                None
            };
            self.handle_pause_menu_action(pause_action);
            return;
        }

        let delta_time = self.clock.tick();
        let (player_chunk, block_action) = {
            let (Some(player), Some(world)) = (self.player.as_mut(), self.world.as_ref()) else {
                return;
            };

            player.update(delta_time, world, &self.blocks);
            (
                world::chunk_from_position(player.position()),
                player.take_block_action(),
            )
        };
        if let Some(action) = block_action {
            self.apply_block_action(action);
        }

        let (mesh_priorities, dirty_chunks, removed_chunks) =
            if let Some(world) = self.world.as_mut() {
                self.chunk_streamer
                    .update_center(player_chunk, world, self.terrain_generator);
                self.chunk_streamer.process_pending(
                    world,
                    self.terrain_generator,
                    CHUNK_LOAD_BUDGET_PER_FRAME,
                );
                for change in world.take_pending_fluid_updates() {
                    self.fluids.on_block_changed(world, change);
                }
                for chunk in world.take_pending_fluid_chunks() {
                    self.fluids.register_chunk(world, chunk);
                }
                self.fluids.update(delta_time, world);
                for chunk in world.take_pending_physics_chunks() {
                    self.falling_blocks
                        .register_chunk(world, &self.blocks, chunk);
                }
                self.falling_blocks.update(delta_time, world, &self.blocks);
                for change in world.take_pending_light_updates() {
                    self.skylight_updates.schedule(change);
                }
                for chunk in world.take_pending_skylight_chunks() {
                    self.skylight_propagation
                        .schedule_chunk(world, &self.blocks, chunk);
                }
                self.skylight_updates.process(
                    world,
                    &self.blocks,
                    self.terrain_generator,
                    SKYLIGHT_UPDATE_BUDGET_PER_FRAME,
                );
                self.skylight_propagation.process(
                    world,
                    &self.blocks,
                    SKYLIGHT_PROPAGATION_BUDGET_PER_FRAME,
                );
                let mut grass_mutations = 0;
                for chunk in world.take_pending_grass_chunks() {
                    grass_mutations += self.grass.register_chunk(world, chunk);
                }
                for chunk in world.take_grass_light_chunks() {
                    grass_mutations += self.grass.on_lighting_changed(world, chunk);
                }
                grass_mutations += self.grass.update(delta_time, world);
                if grass_mutations > 0 {
                    self.level_dirty = true;
                }
                self.particles.update(delta_time, world, &self.blocks);
                let removed_chunks = world.take_removed_chunks();
                for &chunk in &removed_chunks {
                    self.grass.unload_chunk(chunk);
                }
                self.fluids.unload_chunks(&removed_chunks);
                self.falling_blocks.unload_chunks(&removed_chunks);
                (
                    world.take_mesh_priorities(),
                    world.take_dirty_chunks(),
                    removed_chunks,
                )
            } else {
                return;
            };

        let debug_snapshot = self.debug_snapshot();
        let (Some(renderer), Some(world)) = (self.renderer.as_mut(), self.world.as_mut()) else {
            return;
        };
        renderer.schedule_world_updates(world, &dirty_chunks, &removed_chunks);
        renderer.prioritize_world_updates(world, &mesh_priorities);
        renderer.process_world_updates(
            &self.blocks,
            world,
            CHUNK_MESH_BUDGET_PER_FRAME + mesh_priorities.len().min(PRIORITY_MESH_BUDGET_PER_FRAME),
        );

        if let Some(player) = self.player.as_ref() {
            let _ = renderer.render(RenderFrame {
                camera: player.camera(),
                camera_liquid: player.camera_liquid(),
                targeted_block: player.targeted_block(),
                selected_hotbar_slot: player.selected_hotbar_slot(),
                particles: &self.particles,
                falling_blocks: &self.falling_blocks,
                remote_player_poses: &[],
                debug_snapshot,
                paused: false,
                settings: self.settings,
            });
        }
    }

    pub(super) fn request_next_frame(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn debug_snapshot(&self) -> Option<DebugSnapshot> {
        if !self.debug_overlay.is_visible() {
            return None;
        }
        let player = self.player.as_ref()?;
        let world = self.world.as_ref()?;
        Some(DebugSnapshot {
            fps: self.clock.frames_per_second(),
            player_position: player.position(),
            player_chunk: world::chunk_from_position(player.position()),
            render_distance: self.chunk_streamer.render_distance().chunks(),
            loaded_chunks: world.chunks().len(),
        })
    }
}
