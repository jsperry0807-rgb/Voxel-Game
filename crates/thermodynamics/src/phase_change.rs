use crate::chunk_heat::{ChunkHeat, HEAT_GRID_SIZE, HeatResource};
use core::events::{MatterPhase, PhaseChangeEvent};
use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellPhase {
    Solid,
    Liquid,
    Gas,
}

impl CellPhase {
    pub fn from_temp(temp: f32, res: &HeatResource) -> Self {
        if temp < res.melting_point {
            CellPhase::Solid
        } else if res.boiling_point > 0.0 && temp < res.boiling_point {
            CellPhase::Liquid
        } else if res.boiling_point > 0.0 {
            CellPhase::Gas
        } else {
            CellPhase::Solid
        }
    }
}

pub struct PhaseChangeTracker {
    phases: Box<[CellPhase]>,
}

impl PhaseChangeTracker {
    pub fn new() -> Self {
        let n = HEAT_GRID_SIZE.pow(3);
        Self {
            phases: vec![CellPhase::Solid; n].into_boxed_slice(),
        }
    }

    pub fn check_changes(
        &mut self,
        heat: &ChunkHeat,
        resources: &[[HeatResource; 6]],
        offset: Vec3,
        cell_size: f32,
    ) -> Vec<PhaseChangeEvent> {
        let mut events = Vec::new();
        let s = HEAT_GRID_SIZE;
        for z in 0..s {
            for y in 0..s {
                for x in 0..s {
                    let idx = x + y * s + z * s * s;
                    let temp = heat.get_temperature(x, y, z);
                    let res = &resources[0][idx % 6];
                    let new_p = CellPhase::from_temp(temp, res);
                    let old_p = self.phases[idx];

                    if new_p != old_p && heat.is_solid(x, y, z) {
                        let wp = offset
                            + Vec3::new(
                                x as f32 * cell_size,
                                y as f32 * cell_size,
                                z as f32 * cell_size,
                            );
                        events.push(PhaseChangeEvent {
                            world_position: wp,
                            from_phase: match old_p {
                                CellPhase::Solid => MatterPhase::Solid,
                                CellPhase::Liquid => MatterPhase::Liquid,
                                CellPhase::Gas => MatterPhase::Gas,
                            },
                            to_phase: match new_p {
                                CellPhase::Solid => MatterPhase::Solid,
                                CellPhase::Liquid => MatterPhase::Liquid,
                                CellPhase::Gas => MatterPhase::Gas,
                            },
                            temperature: temp,
                        });
                        self.phases[idx] = new_p;
                    }
                }
            }
        }
        events
    }
}

impl Default for PhaseChangeTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stone_phases() {
        let s = HeatResource::stone();
        assert_eq!(CellPhase::from_temp(300.0, &s), CellPhase::Solid);
        assert_eq!(CellPhase::from_temp(1500.0, &s), CellPhase::Liquid);
        assert_eq!(CellPhase::from_temp(3100.0, &s), CellPhase::Gas);
    }

    #[test]
    fn water_phases() {
        let w = HeatResource::water();
        assert_eq!(CellPhase::from_temp(260.0, &w), CellPhase::Solid);
        assert_eq!(CellPhase::from_temp(300.0, &w), CellPhase::Liquid);
        assert_eq!(CellPhase::from_temp(400.0, &w), CellPhase::Gas);
    }

    #[test]
    fn event_generated_on_transition() {
        let mut t = PhaseChangeTracker::new();
        let mut h = ChunkHeat::new();
        h.set_solid(0, 0, 0, true);
        h.set_temperature(0, 0, 0, 300.0);
        let r = [[HeatResource::stone(); 6]];
        assert!(t.check_changes(&h, &r, Vec3::ZERO, 4.0).is_empty());
        h.set_temperature(0, 0, 0, 1500.0);
        let ev = t.check_changes(&h, &r, Vec3::ZERO, 4.0);
        assert!(!ev.is_empty());
        assert_eq!(ev[0].from_phase, MatterPhase::Solid);
        assert_eq!(ev[0].to_phase, MatterPhase::Liquid);
    }
}
