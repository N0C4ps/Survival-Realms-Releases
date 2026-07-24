use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct GameSettings {
    pub fov: u16,
    pub mouse_sensitivity: u8,
    pub render_distance: u8,
    pub brightness: u8,
}

impl GameSettings {
    pub const DEFAULT_FOV: u16 = 100;
    pub const DEFAULT_MOUSE_SENSITIVITY: u8 = 60;
    pub const DEFAULT_RENDER_DISTANCE: u8 = 8;
    pub const DEFAULT_BRIGHTNESS: u8 = 75;

    pub fn clamp(&mut self) {
        self.fov = self.fov.min(200);
        self.mouse_sensitivity = self.mouse_sensitivity.min(100);
        self.render_distance = self.render_distance.clamp(2, 12);
        self.brightness = self.brightness.clamp(1, 100);
    }

    pub fn fov_degrees(self) -> f32 {
        30.0 + self.fov as f32 * 0.4
    }

    pub fn mouse_radians_per_pixel(self) -> f32 {
        0.002 * self.mouse_sensitivity as f32 / Self::DEFAULT_MOUSE_SENSITIVITY as f32
    }

    pub fn gamma(self) -> f32 {
        0.25 + self.brightness as f32 / 100.0
    }
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            fov: Self::DEFAULT_FOV,
            mouse_sensitivity: Self::DEFAULT_MOUSE_SENSITIVITY,
            render_distance: Self::DEFAULT_RENDER_DISTANCE,
            brightness: Self::DEFAULT_BRIGHTNESS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_preserve_the_existing_game_values() {
        let settings = GameSettings::default();
        assert_eq!(settings.fov_degrees(), 70.0);
        assert!((settings.mouse_radians_per_pixel() - 0.002).abs() < f32::EPSILON);
        assert_eq!(settings.render_distance, 8);
        assert_eq!(settings.gamma(), 1.0);
    }

    #[test]
    fn invalid_values_are_clamped_to_supported_ranges() {
        let mut settings = GameSettings {
            fov: u16::MAX,
            mouse_sensitivity: u8::MAX,
            render_distance: 0,
            brightness: 0,
        };
        settings.clamp();
        assert_eq!(settings.fov, 200);
        assert_eq!(settings.mouse_sensitivity, 100);
        assert_eq!(settings.render_distance, 2);
        assert_eq!(settings.brightness, 1);
    }
}
