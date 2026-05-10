use voxel_core::chunk_pos::ChunkPos;
use voxel_core::game_tick::GameTick;

#[derive(Clone, Debug)]
pub struct GenInput {
    pub seed: u64,
    pub chunk_pos: ChunkPos,
    pub global_time: GameTick,
    pub climate: ClimateConfig,
}

#[derive(Clone, Debug)]
pub struct ClimateConfig {
    pub sea_level: u8,
    pub terrain_scale: f64,
}

impl Default for ClimateConfig {
    fn default() -> Self {
        Self {
            sea_level: 64,
            terrain_scale: 0.005,
        }
    }
}
