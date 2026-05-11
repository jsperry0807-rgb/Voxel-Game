use ndarray::Array3;

const GW_SIZE: usize = 16;

#[derive(Debug, Clone, Copy)]
pub struct GroundwaterCell {
    pub pressure_head: f32,
    pub conductivity: f32,
    pub porosity: f32,
    pub saturation: f32,
}

impl Default for GroundwaterCell {
    fn default() -> Self {
        Self {
            pressure_head: 0.0,
            conductivity: 1e-5,
            porosity: 0.3,
            saturation: 0.0,
        }
    }
}

#[derive(Clone)]
pub struct GroundwaterGrid {
    pub cells: Array3<GroundwaterCell>,
    pub cell_size: f32,
}

impl GroundwaterGrid {
    pub fn new() -> Self {
        Self {
            cells: Array3::default((GW_SIZE, GW_SIZE, GW_SIZE)),
            cell_size: 8.0,
        }
    }

    pub fn step(&mut self, dt: f32) -> u64 {
        let mut new_pressure = Array3::<f32>::zeros((GW_SIZE, GW_SIZE, GW_SIZE));
        let mut modified = 0u64;

        for z in 1..GW_SIZE - 1 {
            for y in 1..GW_SIZE - 1 {
                for x in 1..GW_SIZE - 1 {
                    let c = self.cells[[x, y, z]];
                    let laplacian = (self.cells[[x - 1, y, z]].pressure_head
                        + self.cells[[x + 1, y, z]].pressure_head
                        + self.cells[[x, y - 1, z]].pressure_head
                        + self.cells[[x, y + 1, z]].pressure_head
                        + self.cells[[x, y, z - 1]].pressure_head
                        + self.cells[[x, y, z + 1]].pressure_head
                        - 6.0 * c.pressure_head)
                        / (self.cell_size * self.cell_size);

                    let dh_dt = c.conductivity * laplacian / c.porosity.max(0.01);
                    new_pressure[[x, y, z]] = c.pressure_head + dh_dt * dt;

                    if new_pressure[[x, y, z]] > 0.0 {
                        let new_sat = (new_pressure[[x, y, z]] / 5.0).min(1.0);
                        if (self.cells[[x, y, z]].saturation - new_sat).abs() > f32::EPSILON {
                            modified += 1;
                        }
                        self.cells[[x, y, z]].saturation = new_sat;
                    }
                }
            }
        }

        for z in 1..GW_SIZE - 1 {
            for y in 1..GW_SIZE - 1 {
                for x in 1..GW_SIZE - 1 {
                    let old = self.cells[[x, y, z]].pressure_head;
                    if (old - new_pressure[[x, y, z]]).abs() > f32::EPSILON {
                        modified += 1;
                    }
                    self.cells[[x, y, z]].pressure_head = new_pressure[[x, y, z]];
                }
            }
        }

        modified
    }

    pub fn set_water_table(&mut self, x: usize, z: usize, height: f32) {
        let water_y = ((height / self.cell_size) as usize).min(GW_SIZE - 1);
        for y in 0..water_y {
            self.cells[[x, y, z]].pressure_head = (water_y as f32 - y as f32) * self.cell_size;
            self.cells[[x, y, z]].saturation = 1.0;
        }
    }

    pub fn saturation_at(&self, x: usize, y: usize, z: usize) -> f32 {
        self.cells[[x, y, z]].saturation
    }

    pub fn water_table_height(&self, x: usize, z: usize) -> f32 {
        for y in (0..GW_SIZE).rev() {
            if self.cells[[x, y, z]].saturation > 0.5 {
                return y as f32 * self.cell_size;
            }
        }
        0.0
    }
}

impl Default for GroundwaterGrid {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_grid_shape() {
        let gw = GroundwaterGrid::new();
        assert_eq!(gw.cells.shape(), &[16, 16, 16]);
    }

    #[test]
    fn water_table_sets_saturation() {
        let mut gw = GroundwaterGrid::new();
        gw.set_water_table(8, 8, 40.0);
        assert!(gw.saturation_at(8, 4, 8) > 0.5);
    }

    #[test]
    fn high_pressure_diffuses() {
        let mut gw = GroundwaterGrid::new();
        gw.cells[[8, 8, 8]].pressure_head = 100.0;
        gw.cells[[9, 8, 8]].pressure_head = 10.0;
        let before = gw.cells[[8, 8, 8]].pressure_head;
        gw.step(0.1);
        assert!(gw.cells[[8, 8, 8]].pressure_head < before);
    }

    #[test]
    fn uniform_pressure_stable() {
        let mut gw = GroundwaterGrid::new();
        for z in 0..16 {
            for y in 0..16 {
                for x in 0..16 {
                    gw.cells[[x, y, z]].pressure_head = 50.0;
                }
            }
        }
        gw.step(1.0);
        assert!((gw.cells[[8, 8, 8]].pressure_head - 50.0).abs() < 1.0);
    }
}
