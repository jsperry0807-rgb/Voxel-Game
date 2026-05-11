// === Tier 1: core game types ===
pub mod acoustic_props;
pub mod block_definition;
pub mod block_id;
pub mod block_pos;
pub mod chunk_pos;
pub mod game_tick;
pub mod mechanical_props;
pub mod region_pos;
pub mod thermal_props;

pub use acoustic_props::*;
pub use block_definition::*;
pub use block_id::*;
pub use block_pos::*;
pub use chunk_pos::*;
pub use game_tick::*;
pub use mechanical_props::*;
pub use region_pos::*;
pub use thermal_props::*;

// === Tier 2: physics simulation shared types ===
pub mod events;
pub mod sim_scale;
pub mod sim_tick;

pub use events::*;
pub use sim_scale::{SimScale, SimScaleQuery};
pub use sim_tick::SimTick;
