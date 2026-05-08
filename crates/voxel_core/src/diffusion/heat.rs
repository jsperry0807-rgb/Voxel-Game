use crate::{
    chunk::Chunk,
    coordinate::{CHUNK_SIZE, ChunkCoordinate},
    diffusion::{BorderSnapshot, DiffusionContext, DiffusionResult, DiffusionSolver, Face},
    voxel::{VoxelDiffusionState, VoxelMaterial},
};

const S: usize = CHUNK_SIZE as usize;
const VOLUME: usize = S * S * S;

pub struct HeatDiffusionSolver {
    pub coefficient: f32,
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
        chunk.activate_diffusion();

        let diff = chunk.diffusion.as_ref().unwrap();
        let mut scratch: Vec<VoxelDiffusionState> = diff.to_vec();
        let mut cells_modified: u64 = 0;

        // Pre-fetch border diffusion strips for all 6 faces
        let border_px = borders.get(coord, Face::PosX);
        let border_nx = borders.get(coord, Face::NegX);
        let border_py = borders.get(coord, Face::PosY);
        let border_ny = borders.get(coord, Face::NegY);
        let border_pz = borders.get(coord, Face::PosZ);
        let border_nz = borders.get(coord, Face::NegZ);

        for z in 0..S {
            for y in 0..S {
                for x in 0..S {
                    let center_i = x + y * S + z * S * S;

                    if chunk.materials[center_i] == VoxelMaterial::Air {
                        continue;
                    }

                    let center_temp = diff[center_i].temperature;
                    let mut neighbor_sum = 0.0f32;
                    let mut neighbor_count = 0u32;

                    // ── Interior neighbors ─────────────────────────────────
                    macro_rules! add_interior {
                        ($nx:expr, $ny:expr, $nz:expr) => {
                            let ni = $nx + $ny * S + $nz * S * S;
                            if chunk.materials[ni] != VoxelMaterial::Air {
                                neighbor_sum += diff[ni].temperature;
                                neighbor_count += 1;
                            }
                        };
                    }

                    if x + 1 < S {
                        add_interior!(x + 1, y, z);
                    }
                    if x > 0 {
                        add_interior!(x - 1, y, z);
                    }
                    if y + 1 < S {
                        add_interior!(x, y + 1, z);
                    }
                    if y > 0 {
                        add_interior!(x, y - 1, z);
                    }
                    if z + 1 < S {
                        add_interior!(x, y, z + 1);
                    }
                    if z > 0 {
                        add_interior!(x, y, z - 1);
                    }

                    // ── Border neighbors (cross-chunk) ─────────────────────
                    // Strip layout: PosX/NegX → u=y, v=z
                    //               PosY/NegY → u=x, v=z
                    //               PosZ/NegZ → u=x, v=y
                    macro_rules! add_border {
                        ($strip_opt:expr, $u:expr, $v:expr) => {
                            if let Some(strip) = $strip_opt {
                                let bi = $u + $v * S;
                                if strip.materials[bi] != VoxelMaterial::Air {
                                    let temp = strip
                                        .diffusion
                                        .as_ref()
                                        .map(|d| d[bi].temperature)
                                        .unwrap_or(0.0);
                                    neighbor_sum += temp;
                                    neighbor_count += 1;
                                }
                            }
                        };
                    }

                    if x == S - 1 {
                        add_border!(border_px, y, z);
                    }
                    if x == 0 {
                        add_border!(border_nx, y, z);
                    }
                    if y == S - 1 {
                        add_border!(border_py, x, z);
                    }
                    if y == 0 {
                        add_border!(border_ny, x, z);
                    }
                    if z == S - 1 {
                        add_border!(border_pz, x, y);
                    }
                    if z == 0 {
                        add_border!(border_nz, x, y);
                    }

                    if neighbor_count == 0 {
                        continue;
                    }

                    let avg = neighbor_sum / neighbor_count as f32;
                    let delta = avg - center_temp;
                    let new_temp = (center_temp + self.coefficient * delta * ctx.dt)
                        .clamp(0.0, self.max_temperature);

                    if (new_temp - center_temp).abs() > f32::EPSILON {
                        cells_modified += 1;
                    }

                    scratch[center_i].temperature = new_temp;
                }
            }
        }

        // Write scratch back
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
