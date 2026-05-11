pub mod adi_solver;
pub mod chunk_heat;
pub mod fire;
pub mod phase_change;

pub use adi_solver::{ADISolver, ThermalDiffusivity};
pub use chunk_heat::{ChunkHeat, HEAT_GRID_SIZE, HeatResource};
pub use fire::{ChunkFireSystem, FireCell};
pub use phase_change::{CellPhase, PhaseChangeTracker};

use bevy::prelude::*;
use core::chunk_pos::ChunkPos;
use core::sim_tick::SimTick;
use std::collections::HashMap;

#[derive(Resource, Default)]
pub struct ThermodynamicsWorld {
    pub heat_states: HashMap<ChunkPos, ChunkHeat>,
    pub fire_states: HashMap<ChunkPos, ChunkFireSystem>,
}

impl ThermodynamicsWorld {
    pub fn get_or_create_heat(&mut self, c: ChunkPos) -> &mut ChunkHeat {
        self.heat_states.entry(c).or_default()
    }
    pub fn get_or_create_fire(&mut self, c: ChunkPos) -> &mut ChunkFireSystem {
        self.fire_states.entry(c).or_default()
    }
}

pub struct ThermodynamicsPlugin;

impl Plugin for ThermodynamicsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ThermodynamicsWorld::default())
            .add_systems(Last, run_thermodynamics);
    }
}

fn run_thermodynamics(tick: Res<SimTick>, mut tw: ResMut<ThermodynamicsWorld>) {
    let dt = tick.delta();
    let solver = ADISolver::new(ThermalDiffusivity::new(2.5, 2650.0, 840.0));
    for c in tw.heat_states.keys().copied().collect::<Vec<_>>() {
        let heat = tw.get_or_create_heat(c);
        solver.step(heat, dt);
    }
}
