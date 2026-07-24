use bytemuck::{Pod, Zeroable};

const HALF_LENGTH_PIXELS: f32 = 9.0;
const HALF_THICKNESS_PIXELS: f32 = 1.5;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct CrosshairVertex {
    position: [f32; 2],
}

pub(super) fn vertices(width: u32, height: u32) -> [CrosshairVertex; 12] {
    let horizontal_length = HALF_LENGTH_PIXELS * 2.0 / width.max(1) as f32;
    let horizontal_thickness = HALF_THICKNESS_PIXELS * 2.0 / height.max(1) as f32;
    let vertical_thickness = HALF_THICKNESS_PIXELS * 2.0 / width.max(1) as f32;
    let vertical_length = HALF_LENGTH_PIXELS * 2.0 / height.max(1) as f32;

    let horizontal = rectangle(
        -horizontal_length,
        horizontal_length,
        -horizontal_thickness,
        horizontal_thickness,
    );
    let vertical = rectangle(
        -vertical_thickness,
        vertical_thickness,
        -vertical_length,
        vertical_length,
    );

    [
        horizontal[0],
        horizontal[1],
        horizontal[2],
        horizontal[3],
        horizontal[4],
        horizontal[5],
        vertical[0],
        vertical[1],
        vertical[2],
        vertical[3],
        vertical[4],
        vertical[5],
    ]
}

fn rectangle(left: f32, right: f32, bottom: f32, top: f32) -> [CrosshairVertex; 6] {
    [
        vertex(left, bottom),
        vertex(right, bottom),
        vertex(right, top),
        vertex(left, bottom),
        vertex(right, top),
        vertex(left, top),
    ]
}

const fn vertex(x: f32, y: f32) -> CrosshairVertex {
    CrosshairVertex { position: [x, y] }
}
