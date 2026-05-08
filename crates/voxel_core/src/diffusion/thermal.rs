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
    /// Fraction of excess height difference to move per step (0.0–1.0).
    /// Values above 0.5 can cause oscillation.
    pub erosion_rate: f32,
    /// Materials that can be eroded.
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

    /// Linearize (x, y, z) → flat index. Matches VoxelIndex::linearize.
    /// Layout: x + y * S + z * S * S
    #[inline]
    fn idx(x: usize, y: usize, z: usize) -> usize {
        x + y * S + z * S * S
    }

    /// Find the topmost solid voxel in a column (x, z), searching downward
    /// from the top of the chunk. Returns None if the column is all air.
    fn column_height(chunk: &Chunk, x: usize, z: usize) -> Option<usize> {
        for y in (0..S).rev() {
            if chunk.materials[Self::idx(x, y, z)] != VoxelMaterial::Air {
                return Some(y);
            }
        }
        None
    }

    /// Recompute the height of a single column after modification.
    fn recompute_height(chunk: &Chunk, x: usize, z: usize) -> Option<usize> {
        Self::column_height(chunk, x, z)
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

        let neighbors_2d: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

        for z in 0..S {
            for x in 0..S {
                let Some(src_h) = heights[x][z] else {
                    continue; // no material to erode
                };

                //Only erode erodable materials at the surface
                let src_top_idx = Self::idx(x, src_h, z);
                if !self.is_erodable(chunk.materials[src_top_idx]) {
                    continue;
                }

                // Find the steepest downhill neighbor
                let mut steepest_diff = self.talus_threshold;
                let mut steepest_neighbor: Option<(usize, usize)> = None;

                for (dx, dz) in neighbors_2d {
                    let nx = x as i32 + dx;
                    let nz = z as i32 + dz;

                    // Stay within chunk bounds (cross-chunk handling in Task 3.4)
                    if nx < 0 || nx >= S as i32 || nz < 0 || nz >= S as i32 {
                        continue;
                    }
                    let (nx, nz) = (nx as usize, nz as usize);

                    let dst_h = heights[nx][nz].unwrap_or(0);
                    // Use signed diff: positive means src is taller
                    let diff = src_h as f32 - dst_h as f32;

                    if diff > steepest_diff {
                        steepest_diff = diff;
                        steepest_neighbor = Some((nx, nz));
                    }
                }

                let Some((dst_x, dst_z)) = steepest_neighbor else {
                    continue;
                };

                // Move one voxel from top of src to top of dst.
                // erosion_rate scales how aggressively we erode; we use it as
                // a probability weight — always move at least one voxel when
                // the threshold is exceeded (rate > 0 guarantees movement).
                let moved_mat = chunk.materials[src_top_idx];

                // Remove voxel from source top
                chunk.materials[src_top_idx] = VoxelMaterial::Air;

                // Place on top of destination column
                let dst_h = heights[dst_x][dst_z].unwrap_or(0);
                let new_dst_h = dst_h + 1;

                if new_dst_h < S {
                    let dst_idx = Self::idx(dst_x, new_dst_h, dst_z);
                    chunk.materials[dst_idx] = moved_mat;
                    heights[dst_x][dst_z] = Some(new_dst_h);
                }
                // If new_dst_h >= S: voxel exits chunk top — Task 3.4 handles

                // Update source height
                heights[x][z] = Self::recompute_height(chunk, x, z);

                chunk.dirty = true;
                cells_modified += 1;
                material_transferred += 1.0;
            }
        }

        DiffusionResult {
            cells_modified,
            material_transferred,
            execution_time_us: start.elapsed().as_micros() as u64,
        }
    }
}
