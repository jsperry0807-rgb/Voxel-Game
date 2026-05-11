
pub mod groundwater;
pub mod resource;
pub mod surface_water;

pub use groundwater::{GroundwaterCell, GroundwaterGrid};
pub use resource::{HydroEventType, HydrologyEvent, HydrologyResource};
pub use surface_water::{SurfaceWaterCell, SurfaceWaterGrid, SW_CELL_SIZE};

use bevy::prelude::*;
use core::sim_tick::SimTick;
use core::chunk_pos::ChunkPos;
use std::collections::HashMap;

pub struct ChunkHydrology {
    pub surface:     SurfaceWaterGrid,
    pub groundwater: GroundwaterGrid,
}

#[derive(Resource, Default)]
pub struct HydrologyWorld {
    pub chunks: HashMap<ChunkPos, ChunkHydrology>,
}

impl HydrologyWorld {
    pub fn ensure_chunk(&mut self, coord: ChunkPos) {
        self.chunks.entry(coord).or_insert_with(|| ChunkHydrology {
            surface:     SurfaceWaterGrid::new(16, 16, |_, _| 10.0),
            groundwater: GroundwaterGrid::new(),
        });
    }
}

pub struct HydrologyPlugin;

impl Plugin for HydrologyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(HydrologyWorld::default())
           .add_systems(Last, run_hydrology);
    }
}

fn run_hydrology(
    tick: Res<SimTick>,
    mut hydro: ResMut<HydrologyWorld>,
) {
    let dt = tick.delta();
    for chunk in hydro.chunks.values_mut() {
        chunk.surface.step(dt);
        chunk.groundwater.step(dt);
    }
}