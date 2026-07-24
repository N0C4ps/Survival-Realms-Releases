use bytemuck::{Pod, Zeroable};
use winit::dpi::PhysicalSize;

use crate::application::core::{
    blocks::{BlockId, TextureFace},
    player::hotbar::SLOTS,
};

pub(super) const VERTICES_PER_SLOT: usize = 24;
pub(super) const VERTEX_COUNT: usize = SLOTS.len() * VERTICES_PER_SLOT;

const SLOT_QUAD: [([f32; 2], [f32; 2]); 6] = [
    ([0.0, 0.0], [0.0, 0.0]),
    ([1.0, 0.0], [1.0, 0.0]),
    ([1.0, 1.0], [1.0, 1.0]),
    ([0.0, 0.0], [0.0, 0.0]),
    ([1.0, 1.0], [1.0, 1.0]),
    ([0.0, 1.0], [0.0, 1.0]),
];

const CUBE: [CubeVertex; 18] = [
    cube([-0.78, 0.52], [0.0, 0.0], 0.72, TextureFace::Side),
    cube([0.0, 0.09], [1.0, 0.0], 0.72, TextureFace::Side),
    cube([0.0, -0.88], [1.0, 1.0], 0.72, TextureFace::Side),
    cube([-0.78, 0.52], [0.0, 0.0], 0.72, TextureFace::Side),
    cube([0.0, -0.88], [1.0, 1.0], 0.72, TextureFace::Side),
    cube([-0.78, -0.45], [0.0, 1.0], 0.72, TextureFace::Side),
    cube([0.0, 0.09], [0.0, 0.0], 0.86, TextureFace::Side),
    cube([0.78, 0.52], [1.0, 0.0], 0.86, TextureFace::Side),
    cube([0.78, -0.45], [1.0, 1.0], 0.86, TextureFace::Side),
    cube([0.0, 0.09], [0.0, 0.0], 0.86, TextureFace::Side),
    cube([0.78, -0.45], [1.0, 1.0], 0.86, TextureFace::Side),
    cube([0.0, -0.88], [0.0, 1.0], 0.86, TextureFace::Side),
    cube([0.0, 0.95], [0.0, 0.0], 1.0, TextureFace::Top),
    cube([0.78, 0.52], [1.0, 0.0], 1.0, TextureFace::Top),
    cube([0.0, 0.09], [1.0, 1.0], 1.0, TextureFace::Top),
    cube([0.0, 0.95], [0.0, 0.0], 1.0, TextureFace::Top),
    cube([0.0, 0.09], [1.0, 1.0], 1.0, TextureFace::Top),
    cube([-0.78, 0.52], [0.0, 1.0], 1.0, TextureFace::Top),
];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct HotbarVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub texture_kind: u32,
    pub texture_layer: u32,
    pub shade: f32,
}

#[derive(Clone, Copy)]
struct CubeVertex {
    position: [f32; 2],
    uv: [f32; 2],
    shade: f32,
    face: TextureFace,
}

pub(super) fn build(size: PhysicalSize<u32>, selected: usize) -> [HotbarVertex; VERTEX_COUNT] {
    let width = size.width.max(1) as f32;
    let height = size.height.max(1) as f32;
    let slot_size = (height * 0.065).clamp(36.0, 96.0);
    let bar_width = slot_size * SLOTS.len() as f32;
    let left = (width - bar_width) * 0.5;
    let top = height - slot_size - (height * 0.022).clamp(10.0, 24.0);
    let mut vertices = [HotbarVertex::zeroed(); VERTEX_COUNT];
    let mut cursor = 0;

    for (index, block) in SLOTS.into_iter().enumerate() {
        let slot_left = left + index as f32 * slot_size;
        let ui_layer = u32::from(index == selected);
        for (position, uv) in SLOT_QUAD {
            vertices[cursor] = HotbarVertex {
                position: to_clip(
                    slot_left + position[0] * slot_size,
                    top + position[1] * slot_size,
                    width,
                    height,
                ),
                uv,
                texture_kind: 1,
                texture_layer: ui_layer,
                shade: 1.0,
            };
            cursor += 1;
        }

        append_block(
            &mut vertices,
            &mut cursor,
            block,
            slot_left + slot_size * 0.5,
            top + slot_size * 0.54,
            slot_size,
            width,
            height,
        );
    }

    vertices
}

#[allow(clippy::too_many_arguments)]
fn append_block(
    vertices: &mut [HotbarVertex; VERTEX_COUNT],
    cursor: &mut usize,
    block: BlockId,
    center_x: f32,
    center_y: f32,
    slot_size: f32,
    width: f32,
    height: f32,
) {
    for vertex in CUBE {
        let x = center_x + vertex.position[0] * slot_size * 0.31;
        let y = center_y - vertex.position[1] * slot_size * 0.29;
        vertices[*cursor] = HotbarVertex {
            position: to_clip(x, y, width, height),
            uv: vertex.uv,
            texture_kind: 0,
            texture_layer: vertex.face.layer(block),
            shade: vertex.shade,
        };
        *cursor += 1;
    }
}

fn to_clip(x: f32, y: f32, width: f32, height: f32) -> [f32; 2] {
    [x / width * 2.0 - 1.0, 1.0 - y / height * 2.0]
}

const fn cube(position: [f32; 2], uv: [f32; 2], shade: f32, face: TextureFace) -> CubeVertex {
    CubeVertex {
        position,
        uv,
        shade,
        face,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_eight_slots_with_one_equipped_layer() {
        let vertices = build(PhysicalSize::new(1280, 720), 4);
        let equipped_slots = vertices
            .chunks_exact(VERTICES_PER_SLOT)
            .filter(|slot| slot[0].texture_layer == 1)
            .count();

        assert_eq!(vertices.len(), 8 * VERTICES_PER_SLOT);
        assert_eq!(equipped_slots, 1);
    }
}
