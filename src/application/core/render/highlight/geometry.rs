use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct HighlightVertex {
    pub position: [f32; 3],
}

pub(super) const VERTICES: [HighlightVertex; 24] = [
    vertex(0., 0., 0.),
    vertex(1., 0., 0.),
    vertex(1., 0., 0.),
    vertex(1., 0., 1.),
    vertex(1., 0., 1.),
    vertex(0., 0., 1.),
    vertex(0., 0., 1.),
    vertex(0., 0., 0.),
    vertex(0., 1., 0.),
    vertex(1., 1., 0.),
    vertex(1., 1., 0.),
    vertex(1., 1., 1.),
    vertex(1., 1., 1.),
    vertex(0., 1., 1.),
    vertex(0., 1., 1.),
    vertex(0., 1., 0.),
    vertex(0., 0., 0.),
    vertex(0., 1., 0.),
    vertex(1., 0., 0.),
    vertex(1., 1., 0.),
    vertex(1., 0., 1.),
    vertex(1., 1., 1.),
    vertex(0., 0., 1.),
    vertex(0., 1., 1.),
];

const fn vertex(x: f32, y: f32, z: f32) -> HighlightVertex {
    HighlightVertex {
        position: [x, y, z],
    }
}
