use std::{
    collections::VecDeque,
    sync::mpsc::{self, Receiver, Sender},
};

use glam::IVec3;
use rayon::prelude::*;
use rustc_hash::FxHashSet;

use super::super::chunk::Chunk;
use super::{RenderDistance, StreamingUpdate};
use crate::application::core::world::{World, WorldGenerator};

type GeneratedChunk = (IVec3, Option<Chunk>);

pub struct ChunkStreamer {
    render_distance: RenderDistance,
    center: Option<IVec3>,
    pending_loads: VecDeque<IVec3>,
    in_flight: FxHashSet<IVec3>,
    ready_chunks: VecDeque<(IVec3, Chunk)>,
    known_empty: FxHashSet<IVec3>,
    generated_sender: Sender<GeneratedChunk>,
    generated_receiver: Receiver<GeneratedChunk>,
    generation_pool: rayon::ThreadPool,
    generation_threads: usize,
}

impl ChunkStreamer {
    pub fn new(render_distance: RenderDistance) -> Self {
        let (generated_sender, generated_receiver) = mpsc::channel();
        let generation_threads = std::thread::available_parallelism()
            .map_or(1, |threads| usize::from(threads.get() >= 8) + 1);
        let generation_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(generation_threads)
            .thread_name(|index| format!("chunk-generator-{index}"))
            .build()
            .expect("chunk generation thread pool must initialize");
        Self {
            render_distance,
            center: None,
            pending_loads: VecDeque::new(),
            in_flight: FxHashSet::default(),
            ready_chunks: VecDeque::new(),
            known_empty: FxHashSet::default(),
            generated_sender,
            generated_receiver,
            generation_pool,
            generation_threads,
        }
    }

    pub fn update_center(
        &mut self,
        center: IVec3,
        world: &mut World,
        generator: WorldGenerator,
    ) -> StreamingUpdate {
        if self.center == Some(center) {
            return StreamingUpdate::default();
        }

        let desired = desired_chunks(center, self.render_distance, generator);
        self.known_empty
            .retain(|position| desired.contains(position));
        let loaded_positions: Vec<_> = world.chunks().map(|(&position, _)| position).collect();
        let obsolete: Vec<_> = loaded_positions
            .into_iter()
            .filter(|position| !desired.contains(position))
            .collect();
        let unloaded = world.remove_chunks(&obsolete);

        let mut missing: Vec<_> = desired
            .into_iter()
            .filter(|position| {
                world.chunk(*position).is_none()
                    && !self.known_empty.contains(position)
                    && !self.in_flight.contains(position)
                    && !self.ready_chunks.iter().any(|(ready, _)| ready == position)
            })
            .collect();
        missing.sort_unstable_by_key(|position| distance_squared(*position, center));
        self.pending_loads = missing.into();
        self.center = Some(center);

        let update = StreamingUpdate::scheduled(self.pending_loads.len(), unloaded);
        tracing::debug!(
            center = ?center,
            scheduled = update.scheduled_count(),
            unloaded = update.unloaded_count(),
            resident = world.chunks().len(),
            render_distance = self.render_distance.chunks(),
            "chunk streaming center changed"
        );
        update
    }

    pub fn process_pending(
        &mut self,
        world: &mut World,
        generator: WorldGenerator,
        chunk_budget: usize,
    ) -> StreamingUpdate {
        let loaded = self.collect_generated(world, generator, chunk_budget);
        self.schedule_generation(world, generator);
        StreamingUpdate::loaded(loaded)
    }

    pub fn process_pending_blocking(
        &mut self,
        world: &mut World,
        generator: WorldGenerator,
        chunk_budget: usize,
    ) -> StreamingUpdate {
        let positions = self.take_pending_positions(world, chunk_budget);

        let generated: Vec<_> = self.generation_pool.install(|| {
            positions
                .into_par_iter()
                .map(|position| (position, generator.generate_chunk(position)))
                .collect()
        });
        let mut loaded = 0;
        for (position, chunk) in generated {
            if let Some(chunk) = chunk {
                world.insert_chunk(position, chunk);
                loaded += 1;
            } else if world.has_chunk_edits(position) {
                world.insert_chunk(position, Chunk::empty());
                loaded += 1;
            } else {
                self.known_empty.insert(position);
            }
        }

        StreamingUpdate::loaded(loaded)
    }

    pub fn pending_count(&self) -> usize {
        self.pending_loads.len() + self.in_flight.len() + self.ready_chunks.len()
    }

    pub const fn render_distance(&self) -> RenderDistance {
        self.render_distance
    }

    pub fn set_render_distance(&mut self, render_distance: RenderDistance) {
        if self.render_distance == render_distance {
            return;
        }
        self.render_distance = render_distance;
        self.center = None;
        tracing::info!(chunks = render_distance.chunks(), "render distance changed");
    }

    fn collect_generated(
        &mut self,
        world: &mut World,
        generator: WorldGenerator,
        chunk_budget: usize,
    ) -> usize {
        while let Ok((position, chunk)) = self.generated_receiver.try_recv() {
            self.in_flight.remove(&position);
            if !is_desired(position, self.center, self.render_distance, generator) {
                continue;
            }
            if let Some(chunk) = chunk {
                self.ready_chunks.push_back((position, chunk));
            } else if world.has_chunk_edits(position) {
                self.ready_chunks.push_back((position, Chunk::empty()));
            } else {
                self.known_empty.insert(position);
            }
        }

        let mut loaded = 0;
        while loaded < chunk_budget {
            let Some((position, chunk)) = self.ready_chunks.pop_front() else {
                break;
            };
            if is_desired(position, self.center, self.render_distance, generator)
                && world.chunk(position).is_none()
            {
                world.insert_chunk(position, chunk);
                loaded += 1;
            }
        }
        loaded
    }

    fn schedule_generation(&mut self, world: &World, generator: WorldGenerator) {
        let maximum_in_flight = self.generation_threads * 2;
        while self.in_flight.len() + self.ready_chunks.len() < maximum_in_flight {
            let Some(position) = self.pending_loads.pop_front() else {
                break;
            };
            if world.chunk(position).is_some()
                || self.known_empty.contains(&position)
                || !self.in_flight.insert(position)
            {
                continue;
            }

            let sender = self.generated_sender.clone();
            self.generation_pool.spawn(move || {
                let _ = sender.send((position, generator.generate_chunk(position)));
            });
        }
    }

    fn take_pending_positions(&mut self, world: &World, budget: usize) -> Vec<IVec3> {
        let mut positions = Vec::with_capacity(budget.min(self.pending_loads.len()));
        while positions.len() < budget {
            let Some(position) = self.pending_loads.pop_front() else {
                break;
            };
            if world.chunk(position).is_none()
                && !self.known_empty.contains(&position)
                && !self.in_flight.contains(&position)
            {
                positions.push(position);
            }
        }
        positions
    }
}

impl Default for ChunkStreamer {
    fn default() -> Self {
        Self::new(RenderDistance::DEFAULT)
    }
}

fn desired_chunks(
    center: IVec3,
    render_distance: RenderDistance,
    generator: WorldGenerator,
) -> FxHashSet<IVec3> {
    let radius = render_distance.chunks() as i32;
    let side = (radius * 2 + 1) as usize;
    let mut desired = FxHashSet::with_capacity_and_hasher(side * side * side, Default::default());

    for y in center.y - radius..=center.y + radius {
        for z in center.z - radius..=center.z + radius {
            for x in center.x - radius..=center.x + radius {
                let position = IVec3::new(x, y, z);
                if distance_squared(position, center) <= radius * radius
                    && generator.contains_chunk(position)
                {
                    desired.insert(position);
                }
            }
        }
    }
    desired
}

fn distance_squared(position: IVec3, center: IVec3) -> i32 {
    (position - center).length_squared()
}

fn is_desired(
    position: IVec3,
    center: Option<IVec3>,
    render_distance: RenderDistance,
    generator: WorldGenerator,
) -> bool {
    let Some(center) = center else {
        return false;
    };
    let radius = render_distance.chunks() as i32;
    distance_squared(position, center) <= radius * radius && generator.contains_chunk(position)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;
    use crate::application::core::{blocks::BlockId, world::TerrainDimensions};

    #[test]
    fn spreads_chunk_loading_across_multiple_budgets() {
        let generator = WorldGenerator::procedural(TerrainDimensions::new(32, 32, 8, 8), 0);
        let mut world = World::default();
        let mut streamer = ChunkStreamer::new(RenderDistance::new(1));

        let center = IVec3::new(0, -2, 0);
        let scheduled = streamer.update_center(center, &mut world, generator);
        assert_eq!(scheduled.scheduled_count(), 7);
        assert_eq!(world.chunks().len(), 0);

        let first_batch = streamer.process_pending_blocking(&mut world, generator, 4);
        assert_eq!(first_batch.loaded_count(), 4);
        assert_eq!(world.chunks().len(), 4);
        assert_eq!(streamer.pending_count(), 3);
    }

    #[test]
    fn unloads_old_chunks_and_prioritizes_new_nearby_chunks() {
        let generator = WorldGenerator::procedural(TerrainDimensions::new(32, 32, 8, 8), 0);
        let mut world = World::default();
        let mut streamer = ChunkStreamer::new(RenderDistance::new(1));

        let center = IVec3::new(0, -2, 0);
        streamer.update_center(center, &mut world, generator);
        streamer.process_pending_blocking(&mut world, generator, usize::MAX);
        let moved = streamer.update_center(center + IVec3::X, &mut world, generator);

        assert_eq!(moved.scheduled_count(), 5);
        assert_eq!(moved.unloaded_count(), 5);
        assert_eq!(world.chunks().len(), 2);

        streamer.process_pending_blocking(&mut world, generator, usize::MAX);
        assert_eq!(world.chunks().len(), 7);
    }

    #[test]
    fn does_no_work_until_the_player_changes_chunks() {
        let generator = WorldGenerator::default();
        let mut world = World::default();
        let mut streamer = ChunkStreamer::default();

        let center = IVec3::new(0, -2, 0);
        streamer.update_center(center, &mut world, generator);
        let unchanged = streamer.update_center(center, &mut world, generator);

        assert_eq!(unchanged, StreamingUpdate::default());
    }

    #[test]
    fn changing_render_distance_rebuilds_the_desired_radius() {
        let generator = WorldGenerator::default();
        let mut world = World::default();
        let mut streamer = ChunkStreamer::new(RenderDistance::new(1));
        streamer.update_center(IVec3::ZERO, &mut world, generator);
        streamer.process_pending_blocking(&mut world, generator, usize::MAX);

        streamer.set_render_distance(RenderDistance::new(2));
        let expanded = streamer.update_center(IVec3::ZERO, &mut world, generator);

        assert_eq!(streamer.render_distance(), RenderDistance::new(2));
        assert!(expanded.scheduled_count() > 0);
        assert_eq!(expanded.unloaded_count(), 0);
    }

    #[test]
    fn background_generation_eventually_delivers_every_scheduled_chunk() {
        let generator = WorldGenerator::procedural(TerrainDimensions::new(32, 32, 8, 8), 17);
        let mut world = World::default();
        let mut streamer = ChunkStreamer::new(RenderDistance::new(1));
        streamer.update_center(IVec3::new(0, -2, 0), &mut world, generator);
        let deadline = Instant::now() + Duration::from_secs(3);

        while world.chunks().len() < 7 && Instant::now() < deadline {
            streamer.process_pending(&mut world, generator, 4);
            std::thread::yield_now();
        }

        assert_eq!(world.chunks().len(), 7);
        assert_eq!(streamer.pending_count(), 0);
    }

    #[test]
    fn caches_empty_sky_chunks_without_making_them_resident() {
        let generator = WorldGenerator::procedural(TerrainDimensions::new(32, 32, 8, 8), 17);
        let mut world = World::default();
        let mut streamer = ChunkStreamer::new(RenderDistance::new(1));

        let scheduled = streamer.update_center(IVec3::new(0, 3, 0), &mut world, generator);
        let loaded = streamer.process_pending_blocking(&mut world, generator, usize::MAX);

        assert_eq!(scheduled.scheduled_count(), 7);
        assert_eq!(loaded.loaded_count(), 0);
        assert_eq!(world.chunks().len(), 0);
        assert_eq!(streamer.known_empty.len(), 7);
        assert_eq!(streamer.pending_count(), 0);
    }

    #[test]
    fn saved_builds_restore_inside_otherwise_empty_sky_chunks() {
        let generator = WorldGenerator::procedural(TerrainDimensions::new(32, 32, 8, 8), 17);
        let mut world = World::default();
        let mut streamer = ChunkStreamer::new(RenderDistance::new(1));
        let block_position = IVec3::new(1, 49, 1);
        world.restore_edit(block_position, BlockId::WOOD);

        streamer.update_center(IVec3::new(0, 3, 0), &mut world, generator);
        let loaded = streamer.process_pending_blocking(&mut world, generator, usize::MAX);

        assert_eq!(loaded.loaded_count(), 1);
        assert_eq!(world.block(block_position), BlockId::WOOD);
        assert_eq!(world.chunks().len(), 1);
        assert_eq!(streamer.known_empty.len(), 6);
    }
}
