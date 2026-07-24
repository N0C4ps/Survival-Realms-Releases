use glam::Mat4;

const NEAR_PLANE: f32 = 0.1;
const FAR_PLANE: f32 = 1_000.0;

const OPENGL_TO_WGPU_MATRIX: Mat4 = Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 0.5, 0.0, //
    0.0, 0.0, 0.5, 1.0,
]);

pub(super) struct Projection {
    aspect_ratio: f32,
    fov_y_radians: f32,
}

impl Projection {
    pub fn new(aspect_ratio: f32) -> Self {
        Self {
            aspect_ratio,
            fov_y_radians: 70.0_f32.to_radians(),
        }
    }

    pub fn matrix(&self) -> Mat4 {
        OPENGL_TO_WGPU_MATRIX
            * Mat4::perspective_rh(self.fov_y_radians, self.aspect_ratio, NEAR_PLANE, FAR_PLANE)
    }

    pub fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.aspect_ratio = aspect_ratio;
    }

    pub fn set_fov_degrees(&mut self, degrees: f32) {
        self.fov_y_radians = degrees.clamp(1.0, 170.0).to_radians();
    }
}
