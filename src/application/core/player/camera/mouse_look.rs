use super::Camera;

pub struct MouseLook {
    pending_delta: (f64, f64),
    radians_per_pixel: f32,
}

impl Default for MouseLook {
    fn default() -> Self {
        Self {
            pending_delta: (0.0, 0.0),
            radians_per_pixel: 0.002,
        }
    }
}

impl MouseLook {
    pub fn process_motion(&mut self, delta: (f64, f64)) {
        self.pending_delta.0 += delta.0;
        self.pending_delta.1 += delta.1;
    }

    pub fn update(&mut self, camera: &mut Camera) {
        let yaw_delta = self.pending_delta.0 as f32 * self.radians_per_pixel;
        let pitch_delta = -(self.pending_delta.1 as f32) * self.radians_per_pixel;
        camera.rotate(yaw_delta, pitch_delta);
        self.pending_delta = (0.0, 0.0);
    }

    pub fn reset(&mut self) {
        self.pending_delta = (0.0, 0.0);
    }

    pub fn set_radians_per_pixel(&mut self, radians_per_pixel: f32) {
        self.radians_per_pixel = radians_per_pixel.max(0.0);
    }
}
