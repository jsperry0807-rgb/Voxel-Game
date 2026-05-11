use crate::faults::{Fault, FaultType, Fold};
use crate::rock::{RockType, StratigraphicFormation};
use glam::Vec3;
use noise::NoiseGenerator;

#[derive(Clone)]
pub struct GeologyGenerator {
    _seed: u64,
    pub formations: Vec<StratigraphicFormation>,
    folds: Vec<Fold>,
    faults: Vec<Fault>,
}

impl GeologyGenerator {
    pub fn new(seed: u64) -> Self {
        let formations = vec![
            StratigraphicFormation::new(RockType::Sandstone, 30.0, 30.0, 50.0),
            StratigraphicFormation::new(RockType::Limestone, 70.0, 40.0, 150.0),
            StratigraphicFormation::new(RockType::Shale, 120.0, 50.0, 200.0),
            StratigraphicFormation::new(RockType::Granite, 320.0, 200.0, 500.0),
        ];

        let mut noise = NoiseGenerator::new(seed);
        let fold_count = 2 + (seed % 3) as usize;
        let folds = (0..fold_count)
            .map(|i| Fold {
                axis: Vec3::new(
                    noise.fbm_2d(i as f32 * 100.0, 0.0, 3) * 100.0,
                    0.0,
                    noise.fbm_2d(i as f32 * 200.0, 0.0, 3) * 100.0,
                ),
                axis_direction: Vec3::Z,
                wavelength: 80.0 + i as f32 * 20.0,
                amplitude: 10.0 + i as f32 * 5.0,
                plunge: 0.0,
            })
            .collect();

        let fault_count = 1 + (seed % 2) as usize;
        let faults = (0..fault_count)
            .map(|i| Fault {
                plane_origin: Vec3::new(
                    noise.fbm_2d(i as f32 * 300.0, 0.0, 3) * 200.0,
                    0.0,
                    noise.fbm_2d(i as f32 * 400.0, 0.0, 3) * 200.0,
                ),
                plane_normal: Vec3::X,
                dip: std::f32::consts::FRAC_PI_3,
                displacement: 20.0 + i as f32 * 10.0,
                fault_type: if i % 2 == 0 {
                    FaultType::Normal
                } else {
                    FaultType::Reverse
                },
            })
            .collect();

        Self {
            _seed: seed,
            formations,
            folds,
            faults,
        }
    }

    pub fn rock_at(&self, position: Vec3) -> (RockType, f32) {
        let depth = self.deformed_depth(position);
        for f in &self.formations {
            if depth >= f.top_depth() && depth < f.base_depth {
                return (f.rock_type, (depth - f.top_depth()) / f.thinkness);
            }
        }
        (RockType::Granite, 1.0)
    }

    pub fn rock_at_depth(&self, depth: f32) -> RockType {
        for f in &self.formations {
            if depth >= f.top_depth() && depth < f.base_depth {
                return f.rock_type;
            }
        }
        RockType::Granite
    }

    fn deformed_depth(&self, position: Vec3) -> f32 {
        let mut depth = -position.y;
        for fold in &self.folds {
            depth += fold.displacement_at(position);
        }
        for fault in &self.faults {
            depth += fault.displacement_vector(position).y;
        }
        depth.max(0.0)
    }

    pub fn formations_at_column(&self, _x: f32, _z: f32) -> Vec<&StratigraphicFormation> {
        self.formations.iter().collect()
    }

    pub fn formation_count(&self) -> usize {
        self.formations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_formations() {
        assert!(!GeologyGenerator::new(42).formations.is_empty());
    }

    #[test]
    fn surface_is_sandstone() {
        let r#gen = GeologyGenerator::new(42);
        assert_eq!(r#gen.rock_at_depth(5.0), RockType::Sandstone);
    }

    #[test]
    fn deep_is_granite() {
        let r#gen = GeologyGenerator::new(42);
        let (rock, _) = r#gen.rock_at(Vec3::new(0.0, -200.0, 0.0));
        assert_eq!(rock, RockType::Granite);
    }

    #[test]
    fn formations_oldest_last() {
        let r#gen = GeologyGenerator::new(42);
        let ages: Vec<f64> = r#gen
            .formations_at_column(0.0, 0.0)
            .iter()
            .map(|f| f.age_mya)
            .collect();
        for i in 1..ages.len() {
            assert!(ages[i] >= ages[i - 1]);
        }
    }

    #[test]
    fn deterministic() {
        let g1 = GeologyGenerator::new(99);
        let g2 = GeologyGenerator::new(99);
        assert_eq!(
            g1.rock_at(Vec3::new(10.0, -50.0, 10.0)).0,
            g2.rock_at(Vec3::new(10.0, -50.0, 10.0)).0,
        );
    }
}
