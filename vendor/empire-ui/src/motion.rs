//! Interruptible, frame-driven transitions. No timers run once a value settles.
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct Tween {
    from: f32,
    target: f32,
    started: Instant,
    duration: Duration,
}
impl Tween {
    pub fn new(value: f32) -> Self {
        Self {
            from: value,
            target: value,
            started: Instant::now(),
            duration: Duration::ZERO,
        }
    }
    pub fn value_at(&self, now: Instant) -> f32 {
        if self.duration.is_zero() {
            return self.target;
        }
        let t = (now.saturating_duration_since(self.started).as_secs_f32()
            / self.duration.as_secs_f32())
        .clamp(0., 1.);
        if t >= 1. {
            return self.target;
        }
        let ease = 1. - (1. - t).powi(3);
        self.from + (self.target - self.from) * ease
    }
    pub fn value(&self) -> f32 {
        self.value_at(Instant::now())
    }
    pub fn target(&self) -> f32 {
        self.target
    }
    pub fn set_at(&mut self, value: f32, duration: Duration, now: Instant) {
        if self.target == value {
            return;
        }
        self.from = self.value_at(now);
        self.target = value;
        self.started = now;
        self.duration = duration;
    }
    pub fn set(&mut self, value: f32, millis: u64) {
        self.set_at(value, Duration::from_millis(millis), Instant::now());
    }
    pub fn moving(&self) -> bool {
        self.from != self.target && self.started.elapsed() < self.duration
    }
}
