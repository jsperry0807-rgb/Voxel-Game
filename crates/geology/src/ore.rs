use crate::rock::OreDeposit;
use glam::Vec3;
use noise::NoiseGenerator;
use rand::SeedableRng;
use rand_xoshiro::Xoshiro256Plus;

#[derive(Clone)]
pub struct OreGenerator {
    seed: u64,
    pub ore_density: f32,
}

impl OreGenerator {
    pub fn new(seed: u64, ore_density: f32) -> Self {
        Self { seed, ore_density }
    }

    pub fn generate_deposits(&self, center: Vec3, radius: f32) -> Vec<(Vec3, OreDeposit)> {
        let mut deposits = Vec::new();
        let mut noise = NoiseGenerator::new(self.seed);
        let mut rng = Xoshiro256Plus::seed_from_u64(self.seed);
        let step = 8.0f32;
        let half = (radius / step) as i32;

        for ix in -half..=half {
            for iy in -half..=half {
                for iz in -half..=half {
                    let pos =
                        center + Vec3::new(ix as f32 * step, iy as f32 * step, iz as f32 * step);
                    if pos.y > 0.0 {
                        continue;
                    }
                    let conc = noise.fbm(pos.x * 0.05, pos.y * 0.05, pos.z * 0.05, 3);
                    if conc > 0.3 {
                        let mut dep = OreDeposit::new(&mut rng);
                        dep.concentration *= (conc - 0.3) * 2.0;
                        if dep.concentration > 0.05 {
                            deposits.push((pos, dep));
                        }
                    }
                }
            }
        }
        deposits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ore_only_below_surface() {
        let r#gen = OreGenerator::new(42, 1.0);
        for (pos, _) in r#gen.generate_deposits(Vec3::ZERO, 20.0) {
            assert!(pos.y <= 0.0);
        }
    }

    #[test]
    fn deterministic() {
        let g1 = OreGenerator::new(42, 1.0);
        let g2 = OreGenerator::new(42, 1.0);
        assert_eq!(
            g1.generate_deposits(Vec3::ZERO, 16.0).len(),
            g2.generate_deposits(Vec3::ZERO, 16.0).len(),
        );
    }
}
