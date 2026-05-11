use ndarray::Array3;

pub const HEAT_GRID_SIZE: usize = 8;

#[derive(Debug, Clone)]
pub struct ChunkHeat {
    pub temperature: Array3<f32>,
    pub solid_mask: Array3<bool>,
    pub heat_capacity: Array3<f32>,
}

impl ChunkHeat {
    pub fn new() -> Self {
        Self {
            temperature: Array3::zeros((HEAT_GRID_SIZE, HEAT_GRID_SIZE, HEAT_GRID_SIZE)),
            solid_mask: Array3::default((HEAT_GRID_SIZE, HEAT_GRID_SIZE, HEAT_GRID_SIZE)),
            heat_capacity: Array3::from_elem((HEAT_GRID_SIZE, HEAT_GRID_SIZE, HEAT_GRID_SIZE), 1.0),
        }
    }

    #[inline]
    pub fn get_temperature(&self, x: usize, y: usize, z: usize) -> f32 {
        self.temperature[[x, y, z]]
    }

    #[inline]
    pub fn set_temperature(&mut self, x: usize, y: usize, z: usize, temperature: f32) {
        self.temperature[[x, y, z]] = temperature.clamp(0.0, 5000.0);
    }

    #[inline]
    pub fn is_solid(&self, x: usize, y: usize, z: usize) -> bool {
        self.solid_mask[[x, y, z]]
    }

    #[inline]
    pub fn set_solid(&mut self, x: usize, y: usize, z: usize, solid: bool) {
        self.solid_mask[[x, y, z]] = solid;
    }

    #[inline]
    pub fn set_capacity(&mut self, x: usize, y: usize, z: usize, capacity: f32) {
        self.heat_capacity[[x, y, z]] = capacity;
    }

    pub fn average_temperature(&self) -> f32 {
        let mut sum = 0.0f32;
        let mut count = 0u32;
        for x in 0..HEAT_GRID_SIZE {
            for y in 0..HEAT_GRID_SIZE {
                for z in 0..HEAT_GRID_SIZE {
                    if self.solid_mask[[x, y, z]] {
                        sum += self.temperature[[x, y, z]];
                        count += 1;
                    }
                }
            }
        }
        if count == 0 {
            return 293.15;
        }
        sum / count as f32
    }

    #[inline(always)]
    pub fn voxel_to_heat(idx: usize) -> usize {
        idx / 4
    }
}

impl Default for ChunkHeat {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HeatResource {
    pub conductivity: f32,
    pub heat_capacity: f32,
    pub density: f32,
    pub melting_point: f32,
    pub boiling_point: f32,
}

impl HeatResource {
    pub fn air() -> Self {
        Self {
            conductivity: 0.025,
            heat_capacity: 1.005,
            density: 1.225,
            melting_point: 0.0,
            boiling_point: 0.0,
        }
    }

    pub fn stone() -> Self {
        Self {
            conductivity: 2.5,
            heat_capacity: 840.0,
            density: 2650.0,
            melting_point: 1400.0,
            boiling_point: 3000.0,
        }
    }

    pub fn water() -> Self {
        Self {
            conductivity: 0.6,
            heat_capacity: 4186.0,
            density: 1000.0,
            melting_point: 273.15,
            boiling_point: 373.15,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shape() {
        let heat = ChunkHeat::new();
        assert_eq!(heat.temperature.shape(), &[8, 8, 8]);
    }

    #[test]
    fn set_get() {
        let mut heat = ChunkHeat::new();
        heat.set_temperature(3, 3, 3, 500.0);
        assert_eq!(heat.get_temperature(3, 3, 3), 500.0);
    }

    #[test]
    fn clamped() {
        let mut heat = ChunkHeat::new();
        heat.set_temperature(0, 0, 0, -100.0);
        assert_eq!(heat.get_temperature(0, 0, 0), 0.0);
        heat.set_temperature(0, 0, 0, 10000.0);
        assert_eq!(heat.get_temperature(0, 0, 0), 5000.0);
    }

    #[test]
    fn voxel_mapping() {
        assert_eq!(ChunkHeat::voxel_to_heat(0), 0);
        assert_eq!(ChunkHeat::voxel_to_heat(3), 0);
        assert_eq!(ChunkHeat::voxel_to_heat(4), 1);
        assert_eq!(ChunkHeat::voxel_to_heat(31), 7);
    }

    #[test]
    fn average() {
        let mut heat = ChunkHeat::new();
        heat.set_solid(0, 0, 0, true);
        heat.set_temperature(0, 0, 0, 400.0);
        heat.set_solid(1, 0, 0, true);
        heat.set_temperature(1, 0, 0, 600.0);
        assert!((heat.average_temperature() - 500.0).abs() < 0.001);
    }
}
