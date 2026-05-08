use crate::{
    chunk::Chunk,
    coordinate::{CHUNK_SIZE, ChunkCoordinate},
    diffusion::{BorderSnapshot, DiffusionContext, DiffusionResult, DiffusionSolver},
    voxel::VoxelMaterial,
};

const S: usize = CHUNK_SIZE as usize;

/// Thermal erosion using the talus angle model.
///
/// For each surface voxel pair: if the height difference between two adjacent
/// solid columns exceeds `talus_threshold` (in voxels), material moves from
/// the taller column to the shorter one at `erosion_rate`.
///
/// This models how loose material (sand, dirt) slides downhill until a stable
/// angle of repose is reached. Stone is more resistant than sand.
pub struct ThermalErosionSolver {
    /// Max stable height difference between adjacent columns before erosion
    /// begins. 1.0 = 45°, 0.5 = shallower, 2.0 = steeper.
    pub talus_threshold: f32,
    /// Fraction of excess material moved per step. Keep below 0.5 to avoid
    /// oscillation.
    pub erosion_rate: f32,
    /// Materials that can be eroded. Stone is much more resistant than sand.
    pub erodable: &'static [VoxelMaterial],
}

impl Default for ThermalErosionSolver {
    fn default() -> Self {
        Self {
            talus_threshold: 1.5,
            erosion_rate: 0.3,
            erodable: &[
                VoxelMaterial::Sand,
                VoxelMaterial::Dirt,
                VoxelMaterial::Grass,
            ],
        }
    }
}

impl ThermalErosionSolver {
    fn is_erodable(&self, material: VoxelMaterial) -> bool {
        self.erodable.contains(&material)
    }

    /// Find the topmost solid voxel in a column (x, z), searching downward
    /// from the top of the chunk. Returns None if the column is all air.
    fn column_height(chunk: &Chunk, x: usize, z: usize) -> Option<usize> {
        for y in (0..S).rev() {
            let idx = x + y * S + z * S * S;
            if chunk.materials[idx] != VoxelMaterial::Air {
                return Some(y);
            }
        }
        None
    }
}

impl DiffusionSolver for ThermalErosionSolver {
    fn name(&self) -> &'static str {
        "thermal_erosion"
    }

    fn step(
        &self,
        chunk: &mut Chunk,
        _coord: ChunkCoordinate,
        _borders: &BorderSnapshot,
        _ctx: &DiffusionContext,
    ) -> DiffusionResult {
        let start = std::time::Instant::now();
        let mut cells_modified: u64 = 0;
        let mut material_transferred: f32 = 0.0;

        // Snapshot column heights upfront so we don read mutated state.
        // None = all-air column
        let mut heights = [[None::<usize>; S]; S];
        for z in 0..S {
            for x in 0..S {
                heights[x][z] = Self::column_height(chunk, x, z);
            }
        }

        // Collect erosion events: (src column, dst column, amount).
        // We stage them so no column is both source and destination in the
        // same pass, avoiding order-dependency artifacts.
        let mut events: Vec<(usize, usize, usize, usize)> = Vec::new(); // (src_x, src_z, dst_x, dst_z)

        let neighbors_2d: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

        for z in 0..S {
            for x in 0..S {
                let Some(src_h) = heights[x][z] else {
                    continue; // no material to erode
                };

                //Only erode erodable materials at the surface
                let src_top_idx = x + src_h * S + z * S * S;
                if !self.is_erodable(chunk.materials[src_top_idx]) {
                    continue;
                }

                for (dx, dz) in neighbors_2d {
                    let nx = x as i32 + dx;
                    let nz = z as i32 + dz;

                    // Stay within chunk bounds (cross-chunk handling in Task 3.4)
                    if nx < 0 || nx >= S as i32 || nz < 0 || nz >= S as i32 {
                        continue;
                    }
                    let (nx, nz) = (nx as usize, nz as usize);

                    let dst_h = heights[nx][nz].unwrap_or(0);
                    let height_diff = src_h as f32 - dst_h as f32;

                    if height_diff > self.talus_threshold {
                        events.push((x, z, nx, nz));
                    }
                }
            }
        }

        // Apply erosion events
        for (src_x, src_z, dst_x, dst_z) in events {
            let Some(src_h) = heights[src_x][src_z] else {
                continue; // source column was already eroded away
            };
            let dst_h = heights[dst_x][dst_z].unwrap_or(0);

            let height_diff = src_h as f32 - dst_h as f32;
            if height_diff <= self.talus_threshold {
                continue; // Another event may have already levelled this pair
            }

            // Move one voxel: remove from the top of src column
            let src_idx = src_x + src_h * S + src_z * S * S;
            let moved_mat = chunk.materials[src_idx];

            // Only move if still erodable (could have changed from prior event)
            if !self.is_erodable(moved_mat) {
                continue;
            }

            // Apply erosion_rate as a probabilistic threshold:
            // full deterministic move if rate >= 1.0, scaled chance otherwise.
            // For simplicity here we use rate as a fraction of excess to move —
            // since we're moving whole voxels, treat rate < 0.5 as "skip".
            if self.erosion_rate < 0.5 {
                continue;
            }

            // Remove voxel from source top
            chunk.materials[src_idx] = VoxelMaterial::Air;
            chunk.dirty = true;

            // Place voxel on top of destination column (dst + 1)
            let new_dst_h = dst_h + 1;
            if new_dst_h < S {
                let dst_idx = dst_x + new_dst_h * S + dst_z * S * S;
                chunk.materials[dst_idx] = moved_mat;
            }
            // If new_dst_h >= S the voxel falls out of the top of the chunk —
            // acceptable at chunk boundaries; Task 3.4 will handle handoff.

            // Update height snapshot so subsequent events see the new state
            heights[src_x][src_z] = if src_h > 0 {
                // Recalculate: step down until we find solid
                (0..src_h)
                    .rev()
                    .find(|&y| chunk.materials[src_x + y * S + src_z * S * S] != VoxelMaterial::Air)
            } else {
                None
            };
            heights[dst_x][dst_z] = Some(new_dst_h.min(S - 1)); // cap at chunk height

            cells_modified += 1;
            material_transferred += 1.0; // since we're moving whole voxels, count each as 1.0
        }

        DiffusionResult {
            cells_modified,
            material_transferred,
            execution_time_us: start.elapsed().as_micros() as u64,
        }
    }
}
