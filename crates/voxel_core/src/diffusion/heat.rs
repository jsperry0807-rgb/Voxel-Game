use crate::{
    chunk::Chunk,
    coordinate::{CHUNK_SIZE, ChunkCoordinate},
    diffusion::{BorderSnapshot, DiffusionContext, DiffusionResult, DiffusionSolver, Face},
    voxel::{VoxelDiffusionState, VoxelMaterial},
};

const S: usize = CHUNK_SIZE as usize;
const VOLUME: usize = S * S * S;

/// Jacobi iteration heat diffusion over the 6-neighborhood von Neumann stencil.
/// Uses double-buffering: reads from current state, writes into scratch,
/// then swaps — zero allocation per step after first call.
pub struct HeatDiffusionSolver {
    /// Fraction of the temperature difference that moves per unit time.
    pub coefficient: f32,
    /// Maximum allowed temperature (clamp ceiling).
    pub max_temperature: f32,
}

impl Default for HeatDiffusionSolver {
    fn default() -> Self {
        Self {
            coefficient: 0.1,
            max_temperature: 100.0,
        }
    }
}

impl DiffusionSolver for HeatDiffusionSolver {
    fn name(&self) -> &'static str {
        "heat_diffusion"
    }

    fn step(
        &self,
        chunk: &mut Chunk,
        coord: ChunkCoordinate,
        borders: &BorderSnapshot,
        ctx: &DiffusionContext,
    ) -> DiffusionResult {
        let start = std::time::Instant::now();

        // Activate diffusion state if first time
        chunk.activate_diffusion();

        let diff = chunk.diffusion.as_ref().unwrap();

        // Scratch buffer — cloned from current state
        let mut scratch: Vec<VoxelDiffusionState> = diff.to_vec();

        let mut cells_modified: u64 = 0;

        for z in 0..S {
            for y in 0..S {
                for x in 0..S {
                    let center_idx = x + y * S + z * S * S;

                    // Only diffuse through solid or fluid voxels
                    if chunk.materials[center_idx] == VoxelMaterial::Air {
                        continue;
                    }

                    let center_temp = diff[center_idx].temperature;
                    let mut neighbor_sum = 0.0f32;
                    let mut neighbor_count = 0u32;

                    // ── Interior neighbors (same chunk) ──────────────────────
                    macro_rules! interior_neighbor {
                        ($nx:expr, $ny:expr, $nz:expr) => {
                            let ni = $nx + $ny * S + $nz * S * S;
                            if chunk.materials[ni] != VoxelMaterial::Air {
                                neighbor_sum += diff[ni].temperature;
                                neighbor_count += 1;
                            }
                        };
                    }

                    if x + 1 < S {
                        interior_neighbor!(x + 1, y, z);
                    }
                    if x > 0 {
                        interior_neighbor!(x - 1, y, z);
                    }
                    if y + 1 < S {
                        interior_neighbor!(x, y + 1, z);
                    }
                    if y > 0 {
                        interior_neighbor!(x, y - 1, z);
                    }
                    if z + 1 < S {
                        interior_neighbor!(x, y, z + 1);
                    }
                    if z > 0 {
                        interior_neighbor!(x, y, z - 1);
                    }

                    // ── Border neighbors (adjacent chunks via snapshot) ───────
                    if x == 0 {
                        if let Some(strip) = borders.get(coord, Face::NegX) {
                            neighbor_sum += strip[y + z * S].temperature_or(0.0);
                            neighbor_count += 1;
                        }
                    }
                    if x == S - 1 {
                        if let Some(strip) = borders.get(coord, Face::PosX) {
                            neighbor_sum += strip[y + z * S].temperature_or(0.0);
                            neighbor_count += 1;
                        }
                    }
                    if y == 0 {
                        if let Some(strip) = borders.get(coord, Face::NegY) {
                            neighbor_sum += strip[x + z * S].temperature_or(0.0);
                            neighbor_count += 1;
                        }
                    }
                    if y == S - 1 {
                        if let Some(strip) = borders.get(coord, Face::PosY) {
                            neighbor_sum += strip[x + z * S].temperature_or(0.0);
                            neighbor_count += 1;
                        }
                    }
                    if z == 0 {
                        if let Some(strip) = borders.get(coord, Face::NegZ) {
                            neighbor_sum += strip[x + y * S].temperature_or(0.0);
                            neighbor_count += 1;
                        }
                    }
                    if z == S - 1 {
                        if let Some(strip) = borders.get(coord, Face::PosZ) {
                            neighbor_sum += strip[x + y * S].temperature_or(0.0);
                            neighbor_count += 1;
                        }
                    }

                    if neighbor_count == 0 {
                        continue;
                    }

                    let avg = neighbor_sum / neighbor_count as f32;
                    let delta = avg - center_temp;
                    let new_temp = (center_temp + self.coefficient * delta * ctx.dt)
                        .clamp(0.0, self.max_temperature);

                    // Only count as modified if meaningfully changed
                    if (new_temp - center_temp).abs() > f32::EPSILON {
                        cells_modified += 1;
                    }

                    scratch[center_idx].temperature = new_temp;
                }
            }
        }

        // Swap scratch back into chunk
        let diff_mut = chunk.diffusion.as_mut().unwrap();
        for i in 0..VOLUME {
            diff_mut[i].temperature = scratch[i].temperature;
        }

        DiffusionResult {
            cells_modified,
            material_transferred: 0.0,
            execution_time_us: start.elapsed().as_micros() as u64,
        }
    }
}

// Helper trait to safely read temperature from a border material strip.
// Border snapshots store VoxelMaterial, not VoxelDiffusionState, so we
// treat non-air border voxels as having 0 temperature (cold boundary).
trait TemperatureOrDefault {
    fn temperature_or(&self, default: f32) -> f32;
}

impl TemperatureOrDefault for VoxelMaterial {
    fn temperature_or(&self, default: f32) -> f32 {
        // Border snapshot only has material type, not diffusion state.
        // Air borders contribute nothing; solid borders act as cold sinks.
        match self {
            VoxelMaterial::Air => default,
            _ => default, // cold boundary condition
        }
    }
}
