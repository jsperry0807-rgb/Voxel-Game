use crate::chunk_heat::{ChunkHeat, HEAT_GRID_SIZE};
use ndarray::Array3;

#[derive(Debug, Clone)]
pub struct ThermalDiffusivity {
    pub alpha: f32,
}

impl ThermalDiffusivity {
    pub fn new(k: f32, rho: f32, cp: f32) -> Self {
        Self {
            alpha: k / (rho * cp),
        }
    }
}

pub struct ADISolver {
    pub diffusivity: ThermalDiffusivity,
    pub ambient_temperature: f32,
}

impl ADISolver {
    pub fn new(diffusivity: ThermalDiffusivity) -> Self {
        Self {
            diffusivity,
            ambient_temperature: 293.15,
        }
    }

    pub fn step(&self, heat: &mut ChunkHeat, dt: f32) -> u64 {
        let s = HEAT_GRID_SIZE;
        let r = self.diffusivity.alpha * dt;
        let mut tx = heat.temperature.clone();
        let mut ty = Array3::zeros((s, s, s));

        // X sweep
        for y in 0..s {
            for z in 0..s {
                let vals: Vec<f32> = (0..s).map(|x| heat.temperature[[x, y, z]]).collect();
                let res = self.thomas(&vals, r);
                for x in 0..s {
                    tx[[x, y, z]] = res[x];
                }
            }
        }

        // Y sweep
        for x in 0..s {
            for z in 0..s {
                let vals: Vec<f32> = (0..s).map(|y| tx[[x, y, z]]).collect();
                let res = self.thomas(&vals, r);
                for y in 0..s {
                    ty[[x, y, z]] = res[y];
                }
            }
        }

        // Z sweep + write back
        let mut modified: u64 = 0u64;
        for x in 0..s {
            for y in 0..s {
                let vals: Vec<f32> = (0..s).map(|z| ty[[x, y, z]]).collect();
                let res: Vec<f32> = self.thomas(&vals, r);
                #[allow(clippy::needless_range_loop)]
                for z in 0..s {
                    if heat.solid_mask[[x, y, z]] {
                        let old: f32 = heat.temperature[[x, y, z]];
                        let nv: f32 = res[z].clamp(0.0, 5000.0);
                        heat.temperature[[x, y, z]] = nv;
                        if (nv - old).abs() > f32::EPSILON {
                            modified += 1;
                        }
                    }
                }
            }
        }
        modified
    }

    fn thomas(&self, values: &[f32], r: f32) -> Vec<f32> {
        let n = values.len();
        if n == 0 {
            return vec![];
        }

        let b: Vec<f32> = vec![1.0 + 2.0 * r; n];
        let mut d: Vec<f32> = values.to_vec();
        let mut cp: Vec<f32> = vec![0.0f32; n];
        let mut dp: Vec<f32> = vec![0.0f32; n];

        // Boundary conditions
        d[0] += r * self.ambient_temperature;

        // Forward sweep
        cp[0] = -r / b[0];
        dp[0] = d[0] / b[0];
        for i in 1..n {
            let den: f32 = b[i] + r * cp[i - 1];
            if den.abs() < 1e-10 {
                return values.to_vec();
            } // Avoid division by zero
            cp[i] = -r / den;
            dp[i] = (d[i] + r * dp[i - 1]) / den;
        }

        // Back substitution
        let mut result: Vec<f32> = vec![0.0f32; n];
        result[n - 1] = dp[n - 1];
        for i in (0..n - 1).rev() {
            result[i] = dp[i] - cp[i] * result[i + 1];
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solver() -> ADISolver {
        ADISolver::new(ThermalDiffusivity::new(2.5, 2650.0, 840.0))
    }

    #[test]
    fn hot_center_cools() {
        let mut h: ChunkHeat = ChunkHeat::new();
        for x in 3..=5 {
            for y in 3..=5 {
                for z in 3..=5 {
                    h.set_solid(x, y, z, true);
                    h.set_temperature(
                        x,
                        y,
                        z,
                        if x == 4 && y == 4 && z == 4 {
                            800.0
                        } else {
                            293.15
                        },
                    );
                }
            }
        }
        solver().step(&mut h, 0.1);
        assert!(h.get_temperature(4, 4, 4) < 800.0);
    }

    #[test]
    fn no_nan() {
        let mut h = ChunkHeat::new();
        h.set_solid(4, 4, 4, true);
        h.set_temperature(4, 4, 4, 1e10);
        solver().step(&mut h, 0.1);
        for x in 0..HEAT_GRID_SIZE {
            for y in 0..HEAT_GRID_SIZE {
                for z in 0..HEAT_GRID_SIZE {
                    assert!(h.get_temperature(x, y, z).is_finite());
                }
            }
        }
    }

    #[test]
    fn thomas_smoothing() {
        let s: ADISolver = solver();
        let r: Vec<f32> = s.thomas(&[0.0, 0.0, 100.0, 0.0, 0.0], 0.1);
        assert!(r[2] > r[1] && r[1] > 0.0);
    }
}
