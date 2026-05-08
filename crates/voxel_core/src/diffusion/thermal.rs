use crate::{
    chunk::Chunk,
    coordinate::{CHUNK_SIZE, ChunkCoordinate},
    diffusion::{BorderSnapshot, DiffusionContext, DiffusionResult, DiffusionSolver},
    voxel::VoxelMaterial,
};

const S: usize = CHUNK_SIZE as usize;

pub struct ThermalErosionSolver {
    pub talus_threshold: f32,
    pub erosion_rate: f32,
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
    #[inline]
    fn is_erodable(&self, mat: VoxelMaterial) -> bool {
        self.erodable.contains(&mat)
    }

    /// chunk voxel index: x + y*S + z*S*S
    #[inline]
    fn idx(x: usize, y: usize, z: usize) -> usize {
        x + y * S + z * S * S
    }

    /// heights flat index: x + z*S  (separate from chunk layout)
    #[inline]
    fn hidx(x: usize, z: usize) -> usize {
        x + z * S
    }

    fn column_height(chunk: &Chunk, x: usize, z: usize) -> Option<usize> {
        for y in (0..S).rev() {
            if chunk.materials[Self::idx(x, y, z)] != VoxelMaterial::Air {
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

        // ── Phase 1: snapshot ALL column heights before any mutation ───────
        let mut heights: Vec<Option<usize>> = vec![None; S * S];
        for z in 0..S {
            for x in 0..S {
                heights[Self::hidx(x, z)] = Self::column_height(chunk, x, z);
            }
        }

        let neighbors_2d: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

        // ── Phase 2: collect events using ONLY the frozen snapshot ─────────
        // One source per destination per step — tracked with is_destination.
        // One event per source — a source can only donate once per step.
        let mut is_destination: Vec<bool> = vec![false; S * S];
        let mut is_source: Vec<bool> = vec![false; S * S];
        let mut events: Vec<(usize, usize, usize, usize, VoxelMaterial)> = Vec::new();

        for z in 0..S {
            for x in 0..S {
                // Skip if already targeted as destination or used as source
                if is_source[Self::hidx(x, z)] {
                    continue;
                }

                let Some(src_h) = heights[Self::hidx(x, z)] else {
                    continue;
                };

                let src_top_idx = Self::idx(x, src_h, z);
                let mat = chunk.materials[src_top_idx];
                if !self.is_erodable(mat) {
                    continue;
                }

                // Find steepest eligible downhill neighbor
                let mut steepest_diff = self.talus_threshold;
                let mut best: Option<(usize, usize)> = None;

                for &(dx, dz) in &neighbors_2d {
                    let nx = x as i32 + dx;
                    let nz = z as i32 + dz;
                    if nx < 0 || nx >= S as i32 || nz < 0 || nz >= S as i32 {
                        continue;
                    }
                    let (nx, nz) = (nx as usize, nz as usize);

                    if is_destination[Self::hidx(nx, nz)] {
                        continue;
                    }

                    // Only erode toward columns that have solid ground —
                    // eroding into a fully empty column means material falls
                    // into void, which is unphysical at this stage.
                    let Some(dst_h) = heights[Self::hidx(nx, nz)] else {
                        continue;
                    };

                    let diff = src_h as f32 - dst_h as f32;

                    if diff > steepest_diff {
                        steepest_diff = diff;
                        best = Some((nx, nz));
                    }
                }

                let Some((dst_x, dst_z)) = best else { continue };

                is_source[Self::hidx(x, z)] = true;
                is_destination[Self::hidx(dst_x, dst_z)] = true;
                events.push((x, z, dst_x, dst_z, mat));
            }
        }

        // ── Phase 3: apply events ──────────────────────────────────────────
        for (src_x, src_z, dst_x, dst_z, mat) in events {
            let src_h = match heights[Self::hidx(src_x, src_z)] {
                Some(h) => h,
                None => continue,
            };
            let Some(dst_h) = heights[Self::hidx(dst_x, dst_z)] else {
                continue;
            };

            // Defensive: verify source still has material (should always be true)
            let src_idx = Self::idx(src_x, src_h, src_z);
            if chunk.materials[src_idx] == VoxelMaterial::Air {
                continue;
            }

            // Remove voxel from source surface
            chunk.materials[src_idx] = VoxelMaterial::Air;

            // Place voxel on first air slot above destination surface
            let new_dst_h = dst_h + 1;
            if new_dst_h < S {
                let dst_idx = Self::idx(dst_x, new_dst_h, dst_z);
                chunk.materials[dst_idx] = mat;
            }

            chunk.dirty = true;
            cells_modified += 1;
            material_transferred += 1.0;
        }

        DiffusionResult {
            cells_modified,
            material_transferred,
            execution_time_us: start.elapsed().as_micros() as u64,
        }
    }
}
