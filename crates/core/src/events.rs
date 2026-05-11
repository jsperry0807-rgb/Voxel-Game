use glam::Vec3;

pub type Temperature = f32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MatterPhase {
    Solid,
    Liquid,
    Gas,
}

#[derive(Debug, Clone, Copy)]
pub struct PhaseChangeEvent {
    pub world_position: Vec3,
    pub from_phase: MatterPhase,
    pub to_phase: MatterPhase,
    pub temperature: Temperature,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FuelState {
    pub fuel_level: f32, // 0.0 to 1.0
    pub ignition_temperature: Temperature,
    pub burn_rate: f32, // fuel consumed per second when burning
}

impl FuelState {
    pub fn new(ignition_temperature: Temperature, burn_rate: f32) -> Self {
        Self {
            fuel_level: 1.0,
            ignition_temperature,
            burn_rate,
        }
    }

    pub fn is_exhausted(&self) -> bool {
        self.fuel_level <= 0.0
    }
}

#[derive(Debug, Clone)]
pub struct FireEvent {
    pub chunk_position: glam::Vec3,
    pub started: bool,  // true if fire started, false if fire extinguished
    pub intensity: f32, // 0.0 to 1.0, only relevant if started is true
}

#[derive(Debug, Clone, Copy)]
pub struct GroundwaterChangeEvent {
    pub chunk_position: glam::IVec3,
    pub water_table_height: f32,
    pub old_height: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct SurfaceWaterChangeEvent {
    pub world_position: glam::IVec3,
    pub old_depth: f32,
    pub old_height: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct WeatherChangeEvent {
    pub world_position: glam::IVec3,
    pub pressure_change: f32,
    pub humidity_change: f32,
    pub wind_direction: Vec3,
}

#[cfg_attr(feature = "bevy", derive(bevy::prelude::Event))]
#[derive(Debug, Clone)]
pub struct EarthquakeEvent {
    pub epicenter: Vec3,
    pub magnitude: f32,
    pub depth: f32,
    pub affected_radius: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct OreExposureEvent {
    pub world_position: glam::IVec3,
    pub ore_type: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuel_exhaustion() {
        let mut fuel: FuelState = FuelState::new(300.0, 0.01);
        assert!(!fuel.is_exhausted());

        fuel.fuel_level = 0.0;
        assert!(fuel.is_exhausted());
    }

    #[test]
    fn phase_event_fields() {
        let ev: PhaseChangeEvent = PhaseChangeEvent {
            world_position: Vec3::new(1.0, 2.0, 3.0),
            from_phase: MatterPhase::Solid,
            to_phase: MatterPhase::Liquid,
            temperature: 500.0,
        };
        assert_eq!(ev.from_phase, MatterPhase::Solid);
        assert_eq!(ev.to_phase, MatterPhase::Liquid);
    }
}
