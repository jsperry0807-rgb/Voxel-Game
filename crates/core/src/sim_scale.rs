use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SimScale {
    Fine,     // 0-4 chunks: full physics every tick
    Medium,   // 5-11 chunks: simplified physics every 4 ticks
    Coarse,   // 13-32 chunks: no physics, just basic state updates 16 ticks
    Unloaded, // >33 chunks: precomputed only
}

impl SimScale {
    pub fn from_distance(distance: f32) -> Self {
        match (distance / 16.0) as u32 {
            0..=4 => SimScale::Fine,
            5..=11 => SimScale::Medium,
            12..=32 => SimScale::Coarse,
            _ => SimScale::Unloaded,
        }
    }

    pub fn tick_stride(&self) -> u32 {
        match self {
            SimScale::Fine => 1,
            SimScale::Medium => 4,
            SimScale::Coarse => 16,
            SimScale::Unloaded => u32::MAX, // effectively never
        }
    }

    pub fn should_run_at_tick(&self, tick: u64) -> bool {
        let s = self.tick_stride() as u64;
        s == u32::MAX as u64 || tick.is_multiple_of(s)
    }

    pub fn coarsening_factor(&self) -> u32 {
        match self {
            SimScale::Fine => 1,
            SimScale::Medium => 2,
            SimScale::Coarse => 4,
            SimScale::Unloaded => 8,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SimScaleQuery {
    pub player_position: Vec3,
}

impl SimScaleQuery {
    pub fn new(player_position: Vec3) -> Self {
        Self { player_position }
    }

    pub fn scale_for(&self, world_position: Vec3) -> SimScale {
        SimScale::from_distance((world_position - self.player_position).length())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn near_is_fine() {
        assert_eq!(SimScale::from_distance(0.0), SimScale::Fine);
    }
    #[test]
    fn medium_range() {
        assert_eq!(SimScale::from_distance(120.0), SimScale::Medium);
    }
    #[test]
    fn coarse_range() {
        assert_eq!(SimScale::from_distance(200.0), SimScale::Coarse);
    }
    #[test]
    fn far_is_unloaded() {
        assert_eq!(SimScale::from_distance(600.0), SimScale::Unloaded);
    }
    #[test]
    fn fine_stride_is_1() {
        assert_eq!(SimScale::Fine.tick_stride(), 1);
    }
    #[test]
    fn medium_stride() {
        assert_eq!(SimScale::Medium.tick_stride(), 4);
    }
    #[test]
    fn fine_always_runs() {
        assert!(SimScale::Fine.should_run_at_tick(7));
    }
    #[test]
    fn medium_runs_on_4() {
        assert!(SimScale::Medium.should_run_at_tick(8));
    }
    #[test]
    fn medium_skips_odd() {
        assert!(!SimScale::Medium.should_run_at_tick(7));
    }
}
