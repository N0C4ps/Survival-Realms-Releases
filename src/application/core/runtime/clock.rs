use std::time::{Duration, Instant};

const MAX_FRAME_TIME: Duration = Duration::from_millis(100);
// This affects only the FPS value displayed in F3. A longer window prevents normal
// frame-time noise from making the counter flicker, without capping the game loop.
const FPS_SMOOTHING_FACTOR: f64 = 0.025;

pub(super) struct FrameClock {
    previous_frame: Instant,
    sample_started: Instant,
    sample_frames: u32,
    smoothed_frame_seconds: f64,
}

impl Default for FrameClock {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            previous_frame: now,
            sample_started: now,
            sample_frames: 0,
            smoothed_frame_seconds: 1.0 / 60.0,
        }
    }
}

impl FrameClock {
    pub fn tick(&mut self) -> Duration {
        let now = Instant::now();
        let delta_time = now
            .saturating_duration_since(self.previous_frame)
            .min(MAX_FRAME_TIME);
        self.previous_frame = now;
        self.smoothed_frame_seconds +=
            FPS_SMOOTHING_FACTOR * (delta_time.as_secs_f64() - self.smoothed_frame_seconds);
        self.record_frame(now);
        delta_time
    }

    pub fn frames_per_second(&self) -> f64 {
        self.smoothed_frame_seconds.recip()
    }

    pub fn reset(&mut self) {
        let now = Instant::now();
        self.previous_frame = now;
        self.sample_started = now;
        self.sample_frames = 0;
    }

    fn record_frame(&mut self, now: Instant) {
        self.sample_frames += 1;
        let sample_time = now.saturating_duration_since(self.sample_started);

        if sample_time >= Duration::from_secs(1) {
            let frames_per_second = self.sample_frames as f64 / sample_time.as_secs_f64();
            tracing::debug!(fps = format_args!("{frames_per_second:.1}"), "frame rate");
            self.sample_started = now;
            self.sample_frames = 0;
        }
    }
}
