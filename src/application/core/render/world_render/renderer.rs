use std::collections::VecDeque;

use rustc_hash::{FxHashMap, FxHashSet};
use wgpu::{BindGroup, Device, RenderPass, RenderPipeline};

use crate::application::core::{
    blocks::BlockRegistry,
    paths::GamePaths,
    player::Camera,
    world::{CHUNK_SIZE, ChunkFlags, RenderDistance, World, build_chunk_mesh},
};

use super::{
    super::{camera_buffer::CameraBuffer, context::RenderContext},
    depth::DepthTarget,
    frustum::Frustum,
    gpu_mesh::GpuChunkMesh,
    pipeline,
    texture_array::BlockTextureArray,
};

pub(crate) struct WorldRenderer {
    opaque_pipeline: RenderPipeline,
    liquid_pipeline: RenderPipeline,
    textures: BlockTextureArray,
    chunk_meshes: FxHashMap<glam::IVec3, GpuChunkMesh>,
    pending_meshes: VecDeque<glam::IVec3>,
    queued_meshes: FxHashSet<glam::IVec3>,
    depth_target: DepthTarget,
    render_distance: RenderDistance,
}

impl WorldRenderer {
    pub fn new(
        context: &RenderContext,
        camera: &CameraBuffer,
        registry: &BlockRegistry,
        world: &mut World,
        render_distance: RenderDistance,
        paths: &GamePaths,
    ) -> Self {
        let textures = BlockTextureArray::new(&context.device, &context.queue, registry, paths);
        let pipelines = pipeline::create(
            &context.device,
            context.config.format,
            &camera.layout,
            &textures.layout,
        );
        let mut renderer = Self {
            opaque_pipeline: pipelines.opaque,
            liquid_pipeline: pipelines.liquid,
            textures,
            chunk_meshes: FxHashMap::default(),
            pending_meshes: VecDeque::new(),
            queued_meshes: FxHashSet::default(),
            depth_target: DepthTarget::new(context),
            render_distance,
        };
        renderer.schedule_synchronization(world);
        renderer.process_pending(&context.device, registry, world, usize::MAX);
        world.take_dirty_chunks();
        world.take_removed_chunks();
        tracing::info!(
            non_empty_chunk_meshes = renderer.chunk_meshes.len(),
            "initial world meshes built"
        );
        renderer
    }

    pub fn resize(&mut self, context: &RenderContext) {
        self.depth_target = DepthTarget::new(context);
    }

    pub fn set_render_distance(&mut self, render_distance: RenderDistance) {
        self.render_distance = render_distance;
    }

    pub fn schedule_synchronization(&mut self, world: &World) {
        self.chunk_meshes
            .retain(|position, _| world.chunk(*position).is_some());

        for (&position, chunk) in world.chunks() {
            if chunk.flags().contains(ChunkFlags::DIRTY) && self.queued_meshes.insert(position) {
                self.pending_meshes.push_back(position);
            }
        }
    }

    pub fn schedule_changes(
        &mut self,
        world: &World,
        dirty: &[glam::IVec3],
        removed: &[glam::IVec3],
    ) {
        if !removed.is_empty() {
            let removed: FxHashSet<_> = removed.iter().copied().collect();
            self.chunk_meshes
                .retain(|position, _| !removed.contains(position));
            self.queued_meshes
                .retain(|position| !removed.contains(position));
            self.pending_meshes
                .retain(|position| !removed.contains(position));
        }

        for &position in dirty {
            if world.chunk(position).is_some() && self.queued_meshes.insert(position) {
                self.pending_meshes.push_back(position);
            }
        }
    }

    pub fn prioritize(&mut self, world: &World, positions: &[glam::IVec3]) {
        if positions.is_empty() {
            return;
        }

        let mut priorities: FxHashSet<_> = positions
            .iter()
            .copied()
            .filter(|position| world.chunk(*position).is_some())
            .collect();
        if priorities.is_empty() {
            return;
        }

        // Remove every promoted chunk in one pass. The old implementation
        // retained the whole queue once per chunk, becoming quadratic during
        // large lighting/fluid updates.
        self.pending_meshes
            .retain(|position| !priorities.contains(position));

        for &position in positions.iter().rev() {
            if priorities.remove(&position) {
                self.queued_meshes.insert(position);
                self.pending_meshes.push_front(position);
            }
        }
    }

    pub fn process_pending(
        &mut self,
        device: &Device,
        registry: &BlockRegistry,
        world: &mut World,
        mesh_budget: usize,
    ) {
        let mut processed = 0;

        while processed < mesh_budget {
            let Some(position) = self.pending_meshes.pop_front() else {
                break;
            };
            self.queued_meshes.remove(&position);
            if world.chunk(position).is_none() {
                continue;
            }

            let mesh = build_chunk_mesh(world, registry, position);
            if mesh.is_empty() {
                self.chunk_meshes.remove(&position);
            } else {
                self.chunk_meshes
                    .insert(position, GpuChunkMesh::new(device, &mesh));
            }

            let chunk = world
                .chunk_mut(position)
                .expect("a meshed chunk must remain loaded");
            chunk.flags_mut().remove(ChunkFlags::DIRTY);
            chunk.flags_mut().insert(ChunkFlags::MESHED);
            processed += 1;
        }
    }

    pub fn depth_attachment(&self) -> wgpu::RenderPassDepthStencilAttachment<'_> {
        self.depth_target.attachment()
    }

    pub fn texture_layout(&self) -> &wgpu::BindGroupLayout {
        &self.textures.layout
    }

    pub fn texture_bind_group(&self) -> &BindGroup {
        &self.textures.bind_group
    }

    /// Binds the opaque voxel pipeline and its texture array so a caller can
    /// draw its own vertex buffer of `world::Vertex`s in the same style as
    /// the chunk meshes (used by the falling block renderer).
    pub fn bind_opaque<'pass>(
        &'pass self,
        render_pass: &mut RenderPass<'pass>,
        camera_bind_group: &'pass BindGroup,
    ) {
        render_pass.set_pipeline(&self.opaque_pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.textures.bind_group, &[]);
    }

    pub fn draw<'pass>(
        &'pass self,
        render_pass: &mut RenderPass<'pass>,
        camera_bind_group: &'pass BindGroup,
        camera: &Camera,
    ) {
        let frustum = Frustum::new(camera.view_projection_matrix());
        render_pass.set_pipeline(&self.opaque_pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.textures.bind_group, &[]);

        for mesh in self.chunk_meshes.values() {
            if chunk_inside_fog_distance(mesh.chunk_position, camera, self.render_distance)
                && frustum.contains_chunk(mesh.chunk_position)
            {
                mesh.draw_opaque(render_pass);
            }
        }

        let mut visible_liquids: Vec<_> = self
            .chunk_meshes
            .values()
            .filter(|mesh| {
                mesh.has_liquids()
                    && chunk_inside_fog_distance(mesh.chunk_position, camera, self.render_distance)
                    && frustum.contains_chunk(mesh.chunk_position)
            })
            .collect();
        visible_liquids.sort_unstable_by(|left, right| {
            let left_distance = chunk_distance_squared(left.chunk_position, camera);
            let right_distance = chunk_distance_squared(right.chunk_position, camera);
            right_distance.total_cmp(&left_distance)
        });

        render_pass.set_pipeline(&self.liquid_pipeline);
        for mesh in visible_liquids {
            mesh.draw_liquids(render_pass);
        }
    }
}

fn chunk_distance_squared(chunk_position: glam::IVec3, camera: &Camera) -> f32 {
    let center =
        (chunk_position * CHUNK_SIZE as i32).as_vec3() + glam::Vec3::splat(CHUNK_SIZE as f32 * 0.5);
    camera.position().distance_squared(center)
}

fn chunk_inside_fog_distance(
    chunk_position: glam::IVec3,
    camera: &Camera,
    render_distance: RenderDistance,
) -> bool {
    let minimum = (chunk_position * CHUNK_SIZE as i32).as_vec3();
    let maximum = minimum + glam::Vec3::splat(CHUNK_SIZE as f32);
    let closest = camera.position().clamp(minimum, maximum);
    let maximum_distance = render_distance.chunks() as f32 * CHUNK_SIZE as f32;
    camera.position().distance_squared(closest) <= maximum_distance * maximum_distance
}
