use crate::air_cell::AirCell;
use glam::Vec3;
use ndarray::Array3;

pub const ATM_GRID_SIZE: usize = 16;

#[derive(Clone)]
pub struct AtmosphereGrid {
    pub cells: Array3<AirCell>,
    pub world_offset: Vec3,
    pub cell_size: f32,
}

impl AtmosphereGrid {
    pub fn new() -> Self {
        let mut cells = Array3::<AirCell>::default((ATM_GRID_SIZE, ATM_GRID_SIZE, ATM_GRID_SIZE));
        // Initialize pressure profile decreasing with height
        for z in 0..ATM_GRID_SIZE {
            for y in 0..ATM_GRID_SIZE {
                for x in 0..ATM_GRID_SIZE {
                    cells[[x, y, z]].pressure = 101325.0 * (-(z as f32) / 8434.0).exp();
                }
            }
        }
        Self {
            cells,
            world_offset: Vec3::ZERO,
            cell_size: 8.0,
        }
    }

    pub fn step(&mut self, dt: f32) -> u64 {
        let mut modified = 0u64;
        let s = ATM_GRID_SIZE;

        // Pressure + temperature update
        for z in 1..s - 1 {
            for y in 1..s - 1 {
                for x in 1..s - 1 {
                    let cell = &self.cells[[x, y, z]];
                    let div = self.wind_divergence(x, y, z);
                    let dp_dt = -1.4 * cell.pressure * div;
                    let new_p = (cell.pressure + dp_dt * dt).max(1000.0);

                    let wind = cell.wind;
                    let t_adv = -(wind.x * self.temp_gradient(x, y, z, 1, 0, 0)
                        + wind.y * self.temp_gradient(x, y, z, 0, 1, 0)
                        + wind.z * self.temp_gradient(x, y, z, 0, 0, 1));
                    let new_t = (cell.temperature + t_adv * dt).clamp(200.0, 600.0);

                    let cell = &mut self.cells[[x, y, z]];
                    if (cell.pressure - new_p).abs() > f32::EPSILON {
                        modified += 1;
                    }
                    if (cell.temperature - new_t).abs() > f32::EPSILON {
                        modified += 1;
                    }
                    cell.pressure = new_p;
                    cell.temperature = new_t;
                    cell.process_humidity(dt);
                }
            }
        }

        // Wind update from pressure gradient
        for z in 1..s - 1 {
            for y in 1..s - 1 {
                for x in 1..s - 1 {
                    let rho = self.density_at(x, y, z);
                    let dpdx = -(self.cells[[x + 1, y, z]].pressure
                        - self.cells[[x - 1, y, z]].pressure)
                        / (2.0 * self.cell_size);
                    let dpdy = -(self.cells[[x, y + 1, z]].pressure
                        - self.cells[[x, y - 1, z]].pressure)
                        / (2.0 * self.cell_size);
                    let dpdz = -(self.cells[[x, y, z + 1]].pressure
                        - self.cells[[x, y, z - 1]].pressure)
                        / (2.0 * self.cell_size);
                    let cell = &mut self.cells[[x, y, z]];
                    cell.wind.x += dpdx / rho * dt;
                    cell.wind.y += dpdy / rho * dt;
                    cell.wind.z += dpdz / rho * dt;
                    cell.wind *= 0.99;
                }
            }
        }

        modified
    }

    fn wind_divergence(&self, x: usize, y: usize, z: usize) -> f32 {
        let dx = (self.cells[[x + 1, y, z]].wind.x - self.cells[[x - 1, y, z]].wind.x)
            / (2.0 * self.cell_size);
        let dy = (self.cells[[x, y + 1, z]].wind.y - self.cells[[x, y - 1, z]].wind.y)
            / (2.0 * self.cell_size);
        let dz = (self.cells[[x, y, z + 1]].wind.z - self.cells[[x, y, z - 1]].wind.z)
            / (2.0 * self.cell_size);
        dx + dy + dz
    }

    fn temp_gradient(&self, x: usize, y: usize, z: usize, dx: usize, dy: usize, dz: usize) -> f32 {
        let s = ATM_GRID_SIZE - 1;
        let nx = (x + dx).min(s);
        let ny = (y + dy).min(s);
        let nz = (z + dz).min(s);
        let px = if dx > 0 && x > 0 { x - dx } else { x };
        let py = if dy > 0 && y > 0 { y - dy } else { y };
        let pz = if dz > 0 && z > 0 { z - dz } else { z };
        (self.cells[[nx, ny, nz]].temperature - self.cells[[px, py, pz]].temperature)
            / (2.0 * self.cell_size)
    }

    fn density_at(&self, x: usize, y: usize, z: usize) -> f32 {
        let cell = &self.cells[[x, y, z]];
        cell.pressure / (287.058 * cell.temperature.max(1.0))
    }
}

impl Default for AtmosphereGrid {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pressure_decreases_with_height() {
        let grid = AtmosphereGrid::new();
        assert!(grid.cells[[8, 8, 0]].pressure > grid.cells[[8, 8, 8]].pressure);
    }

    #[test]
    fn step_doesnt_crash() {
        let mut grid = AtmosphereGrid::new();
        for z in 0..ATM_GRID_SIZE {
            for y in 0..ATM_GRID_SIZE {
                grid.cells[[0, y, z]].temperature = 320.0;
                grid.cells[[15, y, z]].temperature = 270.0;
                grid.cells[[0, y, z]].pressure = 102000.0;
                grid.cells[[15, y, z]].pressure = 100000.0;
            }
        }
        grid.step(0.5);
        assert!(grid.cells[[7, 7, 7]].wind.length() >= 0.0);
    }

    #[test]
    fn condensation_increases_rain() {
        let mut cell = AirCell::default();
        cell.humidity = 1.5;
        cell.process_humidity(1.0);
        assert!(cell.rainfall_rate > 0.0);
        assert!(cell.cloud_cover > 0.0);
    }

    #[test]
    fn uniform_field_stable() {
        let mut grid = AtmosphereGrid::new();
        for z in 0..ATM_GRID_SIZE {
            for y in 0..ATM_GRID_SIZE {
                for x in 0..ATM_GRID_SIZE {
                    grid.cells[[x, y, z]].temperature = 293.15;
                    grid.cells[[x, y, z]].wind = Vec3::ZERO;
                }
            }
        }
        let before = grid.cells[[8, 8, 4]].pressure;
        grid.step(0.1);
        let after = grid.cells[[8, 8, 4]].pressure;
        assert!((before - after).abs() < 100.0);
    }
}
