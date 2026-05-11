use std::time::Duration;

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy::prelude::Resource))]
pub struct SimTick {
    pub current: u64,
    pub dt: f32,
    pub accumulated: Duration,
}

impl SimTick {
    pub fn new(dt: f32) -> Self {
        Self {
            current: 0,
            dt,
            accumulated: Duration::ZERO,
        }
    }

    pub fn advance(&mut self) -> f32 {
        self.current += 1;
        self.accumulated += Duration::from_secs_f32(self.dt);
        self.dt
    }

    pub fn delta(&self) -> f32 {
        self.dt
    }
    pub fn total_time(&self) -> Duration {
        self.accumulated
    }
}

impl Default for SimTick {
    fn default() -> Self {
        Self::new(0.05) // Default to 20 ticks per second
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_zero() {
        let tick: SimTick = SimTick::new(0.1);
        assert_eq!(tick.current, 0);
    }

    #[test]
    fn advances_tick() {
        let mut tick: SimTick = SimTick::new(0.1);
        tick.advance();
        assert_eq!(tick.current, 1);
    }

    #[test]
    fn accumulates() {
        let mut t: SimTick = SimTick::new(0.5);
        t.advance();
        t.advance();
        t.advance();
        assert!(t.total_time() >= Duration::from_secs_f32(1.5));
    }
}
