use rand::SeedableRng;
use rand_xoshiro::Xoshiro256Plus;
use std::collections::HashMap;

#[derive(Clone)]
pub struct NoiseGenerator {
    rng: Xoshiro256Plus,
    grid: HashMap<(i32, i32, i32), f32>,
    seed: u64,
}

impl NoiseGenerator {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: Xoshiro256Plus::seed_from_u64(seed),
            grid: HashMap::new(),
            seed,
        }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    fn raw_noise(&mut self, x: i32, y: i32, z: i32) -> f32 {
        if let Some(&val) = self.grid.get(&(x, y, z)) {
            return val;
        }
        use rand::Rng;
        let val: f32 = self.rng.r#gen::<f32>() * 2.0 - 1.0;
        self.grid.insert((x, y, z), val);
        val
    }

    fn smooth_noise(&mut self, x: f32, y: f32, z: f32) -> f32 {
        let x0: i32 = x.floor() as i32;
        let y0: i32 = y.floor() as i32;
        let z0: i32 = z.floor() as i32;

        let fx: f32 = {
            let t: f32 = x - x0 as f32;
            t * t * (3.0 - 2.0 * t)
        };
        let fy: f32 = {
            let t: f32 = y - y0 as f32;
            t * t * (3.0 - 2.0 * t)
        };
        let fz: f32 = {
            let t: f32 = z - z0 as f32;
            t * t * (3.0 - 2.0 * t)
        };

        let n000: f32 = self.raw_noise(x0, y0, z0);
        let n001: f32 = self.raw_noise(x0, y0, z0 + 1);
        let n010: f32 = self.raw_noise(x0, y0 + 1, z0);
        let n011: f32 = self.raw_noise(x0, y0 + 1, z0 + 1);
        let n100: f32 = self.raw_noise(x0 + 1, y0, z0);
        let n101: f32 = self.raw_noise(x0 + 1, y0, z0 + 1);
        let n110: f32 = self.raw_noise(x0 + 1, y0 + 1, z0);
        let n111: f32 = self.raw_noise(x0 + 1, y0 + 1, z0 + 1);

        let nx00: f32 = n000 * (1.0 - fx) + n100 * fx;
        let nx01: f32 = n001 * (1.0 - fx) + n101 * fx;
        let nx10: f32 = n010 * (1.0 - fx) + n110 * fx;
        let nx11: f32 = n011 * (1.0 - fx) + n111 * fx;
        let ny0: f32 = nx00 * (1.0 - fy) + nx10 * fy;
        let ny1: f32 = nx01 * (1.0 - fy) + nx11 * fy;

        ny0 * (1.0 - fz) + ny1 * fz
    }

    pub fn fbm(&mut self, x: f32, y: f32, z: f32, octaves: u32) -> f32 {
        let mut total = 0.0f32;
        let mut amplitude = 1.0f32;
        let mut frequency = 1.0f32;
        let mut max_value = 0.0f32;

        for _ in 0..octaves {
            total += self.smooth_noise(x * frequency, y * frequency, z * frequency) * amplitude;
            max_value += amplitude;
            amplitude *= 0.5;
            frequency *= 2.0;
        }

        total / max_value
    }

    pub fn fbm_2d(&mut self, x: f32, z: f32, octaves: u32) -> f32 {
        self.fbm(x, 0.0, z, octaves)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let mut a: NoiseGenerator = NoiseGenerator::new(42);
        let mut b: NoiseGenerator = NoiseGenerator::new(42);
        assert_eq!(a.fbm(1.0, 2.0, 3.0, 3), b.fbm(1.0, 2.0, 3.0, 3));
    }

    #[test]
    fn different_seeds_differ() {
        let mut a: NoiseGenerator = NoiseGenerator::new(1);
        let mut b: NoiseGenerator = NoiseGenerator::new(2);
        assert_ne!(a.fbm(1.0, 2.0, 3.0, 3), b.fbm(1.0, 2.0, 3.0, 3));
    }

    #[test]
    fn output_bounded() {
        let mut r#gen: NoiseGenerator = NoiseGenerator::new(99);
        for x in -5..=5 {
            for z in -5..=5 {
                let v = r#gen.fbm_2d(x as f32, z as f32, 4);
                assert!(v >= -1.0 - 0.001 && v <= 1.0 + 0.001, "out of bounds: {v}");
            }
        }
    }

    #[test]
    fn value_in_range() {
        let mut r#gen: NoiseGenerator = NoiseGenerator::new(42);
        let v = r#gen.fbm(1.0, 2.0, 3.0, 2);
        assert!(v >= -1.0 && v <= 1.0);
    }
}
