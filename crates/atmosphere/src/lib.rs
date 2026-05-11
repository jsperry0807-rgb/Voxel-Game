
pub mod air_cell;
pub mod pressure_solver;

pub use air_cell::{AirCell, AtmosphereResource, GasConcentration, WindVelocity};
pub use pressure_solver::{AtmosphereGrid, ATM_GRID_SIZE};

use bevy::prelude::*;
use core::sim_tick::SimTick;

#[derive(Resource)]
pub struct AtmosphereWorld {
    pub grid: AtmosphereGrid,
}

impl Default for AtmosphereWorld {
    fn default() -> Self {
        Self { grid: AtmosphereGrid::new() }
    }
}

impl AtmosphereWorld {
    pub fn rainfall_at(&self, world_pos: glam::Vec3) -> f32 {
        let offset = world_pos - self.grid.world_offset;
        let cx = (offset.x / self.grid.cell_size) as usize;
        let cy = (offset.z / self.grid.cell_size) as usize;
        let cz = (offset.y / self.grid.cell_size) as usize;
        if cx < ATM_GRID_SIZE && cy < ATM_GRID_SIZE && cz < ATM_GRID_SIZE {
            self.grid.cells[[cx, cy, cz]].rainfall_rate
        } else {
            0.0
        }
    }
}

pub struct AtmospherePlugin;

impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AtmosphereWorld::default())
           .add_systems(Last, run_atmosphere);
    }
}

fn run_atmosphere(
    tick: Res<SimTick>,
    mut atmos: ResMut<AtmosphereWorld>,
) {
    let dt = tick.delta();
    atmos.grid.step(dt);
}