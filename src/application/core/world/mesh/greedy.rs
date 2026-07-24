use glam::{IVec3, UVec3};

use crate::application::core::blocks::{BlockId, BlockRegistry, TextureFace};

use super::{
    super::{
        World,
        chunk::{CHUNK_SIZE, Chunk, ChunkPos},
        coordinates,
    },
    ChunkMesh, Vertex,
    face::Face,
};

const FACE_INDICES: [u32; 6] = [0, 1, 2, 0, 2, 3];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MaskEntry {
    block: BlockId,
    texture_layer: u32,
    skylight: u8,
    liquid: bool,
}

pub(super) fn append_direction(
    mesh: &mut ChunkMesh,
    world: &World,
    chunk: &Chunk,
    registry: &BlockRegistry,
    chunk_position: ChunkPos,
    direction: usize,
    face: &Face,
) {
    let mut mask = [None; CHUNK_SIZE * CHUNK_SIZE];
    for slice in 0..CHUNK_SIZE as u32 {
        fill_mask(
            &mut mask,
            world,
            chunk,
            registry,
            chunk_position,
            direction,
            slice,
            face,
        );
        merge_mask(mesh, world, chunk_position, direction, slice, &mut mask);
    }
}

#[allow(clippy::too_many_arguments)]
fn fill_mask(
    mask: &mut [Option<MaskEntry>; CHUNK_SIZE * CHUNK_SIZE],
    world: &World,
    chunk: &Chunk,
    registry: &BlockRegistry,
    chunk_position: ChunkPos,
    direction: usize,
    slice: u32,
    face: &Face,
) {
    mask.fill(None);
    for v in 0..CHUNK_SIZE as u32 {
        for u in 0..CHUNK_SIZE as u32 {
            let local = local_position(direction, slice, u, v);
            let block = chunk.block(local);
            if !registry.get(block).properties().is_renderable() {
                continue;
            }

            let global = coordinates::global_from_local(chunk_position, local);
            let neighbour_position = global + face.neighbour;
            let neighbour = neighbour_block(world, chunk, global, local, face.neighbour);
            let liquid = registry.get(block).properties().is_liquid();
            let partial_liquid_top = liquid
                && face.neighbour == IVec3::Y
                && liquid_cell_height(world, global, block) < 1.0;
            if (registry.get(neighbour).properties().is_opaque() && !partial_liquid_top)
                || neighbour == block
            {
                continue;
            }

            mask[index(u as usize, v as usize)] = Some(MaskEntry {
                block,
                texture_layer: texture_face(face.neighbour).layer(block),
                skylight: face_skylight(world, global, neighbour_position),
                liquid,
            });
        }
    }
}

fn merge_mask(
    mesh: &mut ChunkMesh,
    world: &World,
    chunk_position: ChunkPos,
    direction: usize,
    slice: u32,
    mask: &mut [Option<MaskEntry>; CHUNK_SIZE * CHUNK_SIZE],
) {
    for v in 0..CHUNK_SIZE {
        let mut u = 0;
        while u < CHUNK_SIZE {
            let Some(entry) = mask[index(u, v)] else {
                u += 1;
                continue;
            };

            let mut width = 1;
            if !entry.liquid {
                while u + width < CHUNK_SIZE && mask[index(u + width, v)] == Some(entry) {
                    width += 1;
                }
            }

            let mut height = 1;
            if !entry.liquid {
                'height: while v + height < CHUNK_SIZE {
                    for offset in 0..width {
                        if mask[index(u + offset, v + height)] != Some(entry) {
                            break 'height;
                        }
                    }
                    height += 1;
                }
            }

            for row in 0..height {
                for column in 0..width {
                    mask[index(u + column, v + row)] = None;
                }
            }
            append_quad(
                mesh,
                world,
                chunk_position,
                direction,
                slice,
                u as u32,
                v as u32,
                width as u32,
                height as u32,
                entry,
            );
            u += width;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn append_quad(
    mesh: &mut ChunkMesh,
    world: &World,
    chunk_position: ChunkPos,
    direction: usize,
    slice: u32,
    u: u32,
    v: u32,
    width: u32,
    height: u32,
    entry: MaskEntry,
) {
    let local_block = local_position(direction, slice, u, v);
    let global_block = coordinates::global_from_local(chunk_position, local_block);
    let origin = (chunk_position * CHUNK_SIZE as i32).as_vec3();
    let s = slice as f32;
    let u = u as f32;
    let v = v as f32;
    let width = width as f32;
    let height = height as f32;
    let mut local_corners = match direction {
        0 => [
            [s + 1.0, v, u],
            [s + 1.0, v + height, u],
            [s + 1.0, v + height, u + width],
            [s + 1.0, v, u + width],
        ],
        1 => [
            [s, v, u + width],
            [s, v + height, u + width],
            [s, v + height, u],
            [s, v, u],
        ],
        2 => [
            [v, s + 1.0, u + width],
            [v + height, s + 1.0, u + width],
            [v + height, s + 1.0, u],
            [v, s + 1.0, u],
        ],
        3 => [
            [v, s, u],
            [v + height, s, u],
            [v + height, s, u + width],
            [v, s, u + width],
        ],
        4 => [
            [u + width, v, s + 1.0],
            [u + width, v + height, s + 1.0],
            [u, v + height, s + 1.0],
            [u, v, s + 1.0],
        ],
        5 => [
            [u, v, s],
            [u, v + height, s],
            [u + width, v + height, s],
            [u + width, v, s],
        ],
        _ => unreachable!("voxel faces have six directions"),
    };
    if entry.liquid {
        let top = local_block.y as f32 + 1.0;
        for corner in &mut local_corners {
            if (corner[1] - top).abs() > f32::EPSILON {
                continue;
            }
            let positive_x = corner[0] > local_block.x as f32 + 0.5;
            let positive_z = corner[2] > local_block.z as f32 + 0.5;
            corner[1] = local_block.y as f32
                + liquid_corner_height(world, global_block, entry.block, positive_x, positive_z);
        }
    }
    let normal = super::face::FACES[direction].normal;
    let uvs = [[0.0, height], [0.0, 0.0], [width, 0.0], [width, height]];
    let (vertices, indices) = if entry.liquid {
        (&mut mesh.liquid_vertices, &mut mesh.liquid_indices)
    } else {
        (&mut mesh.vertices, &mut mesh.indices)
    };
    let first_vertex = vertices.len() as u32;
    for (corner, uv) in local_corners.into_iter().zip(uvs) {
        vertices.push(Vertex {
            position: [
                origin.x + corner[0],
                origin.y + corner[1],
                origin.z + corner[2],
            ],
            normal,
            uv,
            texture_layer: entry.texture_layer,
            skylight: u32::from(entry.skylight),
        });
    }
    indices.extend(FACE_INDICES.map(|index| first_vertex + index));
}

const fn texture_face(direction: IVec3) -> TextureFace {
    if direction.y > 0 {
        TextureFace::Top
    } else if direction.y < 0 {
        TextureFace::Bottom
    } else {
        TextureFace::Side
    }
}

fn liquid_corner_height(
    world: &World,
    position: IVec3,
    block: BlockId,
    positive_x: bool,
    positive_z: bool,
) -> f32 {
    let x_direction = if positive_x { IVec3::X } else { IVec3::NEG_X };
    let z_direction = if positive_z { IVec3::Z } else { IVec3::NEG_Z };
    let samples = [
        position,
        position + x_direction,
        position + z_direction,
        position + x_direction + z_direction,
    ];

    if samples
        .iter()
        .any(|&sample| world.block(sample + IVec3::Y) == block)
    {
        return 1.0;
    }

    let mut height = 0.0;
    let mut count = 0;
    for sample in samples {
        if world.block(sample) == block {
            height += liquid_cell_height(world, sample, block);
            count += 1;
        }
    }
    if count == 0 {
        liquid_cell_height(world, position, block)
    } else {
        height / count as f32
    }
}

fn liquid_cell_height(world: &World, position: IVec3, block: BlockId) -> f32 {
    debug_assert_eq!(world.block(position), block);
    world
        .liquid_height(position)
        .expect("liquid geometry must point to a liquid block")
}

fn local_position(direction: usize, slice: u32, u: u32, v: u32) -> UVec3 {
    match direction {
        0 | 1 => UVec3::new(slice, v, u),
        2 | 3 => UVec3::new(v, slice, u),
        4 | 5 => UVec3::new(u, v, slice),
        _ => unreachable!("voxel faces have six directions"),
    }
}

fn face_skylight(world: &World, block: IVec3, neighbour: IVec3) -> u8 {
    let neighbour_chunk = coordinates::split_global(neighbour).0;
    if world.chunk(neighbour_chunk).is_some() {
        world.skylight(neighbour)
    } else {
        world.skylight(block)
    }
}

fn neighbour_block(
    world: &World,
    chunk: &Chunk,
    global: IVec3,
    local: UVec3,
    direction: IVec3,
) -> BlockId {
    let neighbour_local = local.as_ivec3() + direction;
    let size = CHUNK_SIZE as i32;
    let inside_chunk = (0..size).contains(&neighbour_local.x)
        && (0..size).contains(&neighbour_local.y)
        && (0..size).contains(&neighbour_local.z);

    if inside_chunk {
        chunk.block(neighbour_local.as_uvec3())
    } else {
        world.block(global + direction)
    }
}

const fn index(u: usize, v: usize) -> usize {
    v * CHUNK_SIZE + u
}
