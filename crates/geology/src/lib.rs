pub mod faults;
pub mod generator;
pub mod ore;
pub mod rock;

pub use faults::{Fault, FaultType, Fold};
pub use generator::GeologyGenerator;
pub use ore::OreGenerator;
pub use rock::{OreDeposit, RockType, StratigraphicFormation};

use bevy::prelude::*;
use core::sim_tick::SimTick;
use glam::Vec3;

#[derive(Resource)]
pub struct GeologyWorld {
    pub generator: GeologyGenerator,
    pub ore_generator: OreGenerator,
    pub earthquake_timer: f32,
    pub earthquake_interval: f32,
}

impl Default for GeologyWorld {
    fn default() -> Self {
        Self {
            generator: GeologyGenerator::new(0),
            ore_generator: OreGenerator::new(0, 1.0),
            earthquake_timer: 0.0,
            earthquake_interval: 3600.0,
        }
    }
}

impl GeologyWorld {
    pub fn get_rock(&self, pos: Vec3) -> (RockType, f32) {
        self.generator.rock_at(pos)
    }
}

pub struct GeologyPlugin;

impl Plugin for GeologyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GeologyWorld::default())
            .add_systems(Last, run_geology);
    }
}

fn run_geology(tick: Res<SimTick>, mut geo: ResMut<GeologyWorld>) {
    let dt = tick.delta();
    geo.earthquake_timer += dt;
    if geo.earthquake_timer >= geo.earthquake_interval {
        geo.earthquake_timer = 0.0;
        // EarthquakeEvent emission wired during integration phase
    }
}
