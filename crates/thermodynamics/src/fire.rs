use crate::chunk_heat::{ChunkHeat, HEAT_GRID_SIZE};
use core::events::FuelState;

pub type OxygenLevel = f32;

#[derive(Debug, Clone, Copy)]
pub struct FireCell {
    pub fuel: FuelState,
    pub oxygen: OxygenLevel,
    pub burning: bool,
    pub intensity: f32,
}

impl Default for FireCell {
    fn default() -> Self {
        Self {
            fuel: FuelState::new(600.0, 0.02),
            oxygen: 1.0,
            burning: false,
            intensity: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChunkFireSystem {
    pub cells: Box<[FireCell]>,
}

impl ChunkFireSystem {
    pub fn new() -> Self {
        let n = HEAT_GRID_SIZE.pow(3);
        Self {
            cells: vec![FireCell::default(); n].into_boxed_slice(),
        }
    }

    fn idx(x: usize, y: usize, z: usize) -> usize {
        x + y * HEAT_GRID_SIZE + z * HEAT_GRID_SIZE * HEAT_GRID_SIZE
    }

    pub fn cell(&self, x: usize, y: usize, z: usize) -> &FireCell {
        &self.cells[Self::idx(x, y, z)]
    }
    pub fn cell_mut(&mut self, x: usize, y: usize, z: usize) -> &mut FireCell {
        &mut self.cells[Self::idx(x, y, z)]
    }

    pub fn step(&mut self, heat: &mut ChunkHeat, dt: f32) -> u64 {
        let mut modified = 0u64;
        for z in 0..HEAT_GRID_SIZE {
            for y in 0..HEAT_GRID_SIZE {
                for x in 0..HEAT_GRID_SIZE {
                    let temp = heat.get_temperature(x, y, z);
                    let cell = self.cell_mut(x, y, z);

                    // Ignition
                    if !cell.burning
                        && !cell.fuel.is_exhausted()
                        && temp >= cell.fuel.ignition_temperature
                        && cell.oxygen > 0.1
                    {
                        cell.burning = true;
                        modified += 1;
                    }

                    if cell.burning {
                        if cell.fuel.is_exhausted() || cell.oxygen < 0.05 {
                            cell.burning = false;
                            cell.intensity = 0.0;
                            modified += 1;
                        } else {
                            let burn = cell.fuel.burn_rate * dt * cell.oxygen;
                            cell.fuel.fuel_level = (cell.fuel.fuel_level - burn).max(0.0);
                            cell.oxygen = (cell.oxygen - burn * 0.5).max(0.0);
                            cell.intensity = burn * 1000.0;
                            let new_temp = heat.get_temperature(x, y, z) + cell.intensity * dt;
                            heat.set_temperature(x, y, z, new_temp);
                            modified += 1;
                        }
                    }

                    // O2 diffusion
                    let avg = self.avg_o2(x, y, z);
                    let cell = self.cell_mut(x, y, z);
                    cell.oxygen = (cell.oxygen + (avg - cell.oxygen) * 0.01).clamp(0.0, 1.0);
                }
            }
        }
        modified
    }

    fn avg_o2(&self, x: usize, y: usize, z: usize) -> OxygenLevel {
        let mut sum = 0.0f32;
        let mut cnt = 0u32;
        for dx in [-1i32, 0, 1] {
            for dy in [-1i32, 0, 1] {
                for dz in [-1i32, 0, 1] {
                    if dx == 0 && dy == 0 && dz == 0 {
                        continue;
                    }
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    let nz = z as i32 + dz;
                    let s = HEAT_GRID_SIZE as i32;
                    if (0..s).contains(&nx) && (0..s).contains(&ny) && (0..s).contains(&nz) {
                        sum += self.cell(nx as usize, ny as usize, nz as usize).oxygen;
                        cnt += 1;
                    }
                }
            }
        }
        if cnt == 0 { 1.0 } else { sum / cnt as f32 }
    }
}

impl Default for ChunkFireSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignition() {
        let mut h = ChunkHeat::new();
        h.set_solid(4, 4, 4, true);
        h.set_temperature(4, 4, 4, 700.0);
        let mut f = ChunkFireSystem::new();
        f.cell_mut(4, 4, 4).fuel = FuelState::new(500.0, 0.02);
        f.step(&mut h, 0.1);
        assert!(f.cell(4, 4, 4).burning);
    }

    #[test]
    fn no_ignition_when_cold() {
        let mut h = ChunkHeat::new();
        h.set_solid(4, 4, 4, true);
        h.set_temperature(4, 4, 4, 200.0);
        let mut f = ChunkFireSystem::new();
        f.cell_mut(4, 4, 4).fuel = FuelState::new(500.0, 0.02);
        f.step(&mut h, 0.1);
        assert!(!f.cell(4, 4, 4).burning);
    }

    #[test]
    fn consumes_oxygen() {
        let mut h = ChunkHeat::new();
        h.set_solid(4, 4, 4, true);
        h.set_temperature(4, 4, 4, 600.0);
        let mut f = ChunkFireSystem::new();
        f.cell_mut(4, 4, 4).fuel = FuelState::new(500.0, 0.02);
        f.cell_mut(4, 4, 4).oxygen = 1.0;
        f.step(&mut h, 0.1);
        assert!(f.cell(4, 4, 4).oxygen < 1.0);
    }

    #[test]
    fn extinguished_by_no_oxygen() {
        let mut h = ChunkHeat::new();
        let mut f = ChunkFireSystem::new();
        f.cell_mut(4, 4, 4).burning = true;
        f.cell_mut(4, 4, 4).oxygen = 0.0;
        f.step(&mut h, 0.1);
        assert!(!f.cell(4, 4, 4).burning);
    }
}
