use ndarray::Array2;

pub const SW_CELL_SIZE: f32 = 4.0;

#[derive(Debug, Clone, Copy)]
pub struct SurfaceWaterCell {
    pub depth: f32,
    pub velocity: (f32, f32),
    pub height: f32,
    pub sediment: f32,
}

impl Default for SurfaceWaterCell {
    fn default() -> Self {
        Self {
            depth: 0.0,
            velocity: (0.0, 0.0),
            height: 0.0,
            sediment: 0.0,
        }
    }
}

#[derive(Clone)]
pub struct SurfaceWaterGrid {
    pub cells: Array2<SurfaceWaterCell>,
    pub gravity: f32,
    pub friction: f32,
}

impl SurfaceWaterGrid {
    pub fn new(width: usize, height: usize, terrain: fn(usize, usize) -> f32) -> Self {
        let cells = Array2::from_shape_fn((width, height), |(x, y)| SurfaceWaterCell {
            height: terrain(x, y),
            ..Default::default()
        });
        Self {
            cells,
            gravity: 9.81,
            friction: 0.01,
        }
    }

    pub fn step(&mut self, dt: f32) -> u64 {
        let (w, h) = self.cells.dim();
        let mut new_depth = Array2::<f32>::zeros((w, h));
        let mut new_vel_x = Array2::<f32>::zeros((w, h));
        let mut new_vel_y = Array2::<f32>::zeros((w, h));
        let mut modified = 0u64;

        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let c = &self.cells[[x, y]];
                let grad_u = self.surface_gradient(x, y, 1, 0);
                let grad_v = self.surface_gradient(x, y, 0, 1);

                let flux_u = self.gravity * c.depth * grad_u;
                let flux_v = self.gravity * c.depth * grad_v;

                let speed = c.velocity.0.hypot(c.velocity.1);
                let friction_u = if speed > 0.0 {
                    -self.friction * c.velocity.0 * speed / (c.depth + 0.01)
                } else {
                    0.0
                };
                let friction_v = if speed > 0.0 {
                    -self.friction * c.velocity.1 * speed / (c.depth + 0.01)
                } else {
                    0.0
                };

                new_depth[[x, y]] = (c.depth + div_flux(x, y, &self.cells) * dt).max(0.0);
                new_vel_x[[x, y]] = (c.velocity.0 + (flux_u + friction_u) * dt).clamp(-50.0, 50.0);
                new_vel_y[[x, y]] = (c.velocity.1 + (flux_v + friction_v) * dt).clamp(-50.0, 50.0);
            }
        }

        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let cell = &mut self.cells[[x, y]];
                if (cell.depth - new_depth[[x, y]]).abs() > f32::EPSILON {
                    modified += 1;
                }
                if (cell.velocity.0 - new_vel_x[[x, y]]).abs() > f32::EPSILON {
                    modified += 1;
                }
                cell.depth = new_depth[[x, y]];
                cell.velocity = (new_vel_x[[x, y]], new_vel_y[[x, y]]);
            }
        }
        modified
    }

    fn surface_gradient(&self, x: usize, y: usize, dx: usize, dy: usize) -> f32 {
        let (w, h) = self.cells.dim();
        let nx = (x + dx).min(w - 1);
        let ny = (y + dy).min(h - 1);
        let px = if dx > 0 && x > 0 { x - dx } else { x };
        let py = if dy > 0 && y > 0 { y - dy } else { y };
        let left = self.cells[[px, py]].height + self.cells[[px, py]].depth;
        let right = self.cells[[nx, ny]].height + self.cells[[nx, ny]].depth;
        (left - right) / (2.0 * SW_CELL_SIZE)
    }

    pub fn add_rainfall(&mut self, rate: f32, dt: f32) {
        for cell in self.cells.iter_mut() {
            cell.depth += rate * dt;
        }
    }

    pub fn total_water_volume(&self) -> f32 {
        self.cells
            .iter()
            .map(|c| c.depth * SW_CELL_SIZE * SW_CELL_SIZE)
            .sum()
    }
}

fn div_flux(x: usize, y: usize, cells: &Array2<SurfaceWaterCell>) -> f32 {
    let left = cells[[x - 1, y]].depth;
    let right = cells[[x + 1, y]].depth;
    let down = cells[[x, y - 1]].depth;
    let up = cells[[x, y + 1]].depth;
    let center = cells[[x, y]].depth;
    (left + right + down + up - 4.0 * center) / (SW_CELL_SIZE * SW_CELL_SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_terrain_stable() {
        let mut grid = SurfaceWaterGrid::new(10, 10, |_, _| 10.0);
        for cell in grid.cells.iter_mut() {
            cell.depth = 1.0;
        }
        grid.step(0.1);
        for x in 2..8 {
            for y in 2..8 {
                let d = grid.cells[[x, y]].depth;
                assert!((d - 1.0).abs() < 0.5, "depth diverged at {x},{y}: {d}");
            }
        }
    }

    #[test]
    fn water_flows_downhill() {
        let mut grid = SurfaceWaterGrid::new(10, 10, |x, _| x as f32 * 0.5);
        grid.cells[[1, 5]].depth = 5.0;
        grid.step(0.5);
        assert!(grid.cells[[2, 5]].depth > 0.0);
    }

    #[test]
    fn volume_conserved() {
        let mut grid = SurfaceWaterGrid::new(10, 10, |_, _| 10.0);
        for cell in grid.cells.iter_mut() {
            cell.depth = 1.0;
        }
        let initial = grid.total_water_volume();
        grid.step(0.1);
        let after = grid.total_water_volume();
        assert!(
            (initial - after).abs() < 5.0,
            "volume: {initial} -> {after}"
        );
    }

    #[test]
    fn rainfall_adds_water() {
        let mut grid = SurfaceWaterGrid::new(5, 5, |_, _| 0.0);
        let before = grid.total_water_volume();
        grid.add_rainfall(0.01, 1.0);
        assert!(grid.total_water_volume() > before);
    }
}
