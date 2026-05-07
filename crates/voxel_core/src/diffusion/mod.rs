pub mod heat;

use crate::{
    chunk::Chunk,
    coordinate::{CHUNK_SIZE, ChunkCoordinate},
    voxel::VoxelMaterial,
};
use rustc_hash::FxHashMap;

// ── Context passed to every solver step ───────────────────────────────────────

pub struct DiffusionContext {
    /// Timestep in sec
    pub dt: f32,
    /// Gravitational acceleration (negative = downward).
    pub gravity: f32,
    /// Rainfall rate for hydraulic erosion (units/second).
    pub rainfall_rate: f32,
}

impl Default for DiffusionContext {
    fn default() -> Self {
        Self {
            dt: 0.05,
            gravity: -9.81,
            rainfall_rate: 0.01,
        }
    }
}

// ── Per-step statistics ────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct DiffusionResult {
    pub cells_modified: u64,
    pub material_transferred: f32,
    pub execution_time_us: u64,
}

// ── Border snapshot ────────────────────────────────────────────────────────────

const FACE_VOXEL_COUNT: usize = (CHUNK_SIZE * CHUNK_SIZE) as usize;

/// One face direction on a chunk boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Face {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

impl Face {
    pub const ALL: [Face; 6] = [
        Face::PosX,
        Face::NegX,
        Face::PosY,
        Face::NegY,
        Face::PosZ,
        Face::NegZ,
    ];
}

/// The key into the border map: which chunk, which face of that chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BorderKey {
    pub coord: ChunkCoordinate,
    pub face: Face,
}

/// Read-only snapshot of 1-voxel border strips extracted from neighbor chunks.
/// Prevents cross-chunk read/write data races during parallel diffusion steps.
#[derive(Default)]
pub struct BorderSnapshot {
    /// Maps (chunk coordinate + face) → flat array of CHUNK_SIZE² materials.
    pub data: FxHashMap<BorderKey, Box<[VoxelMaterial; FACE_VOXEL_COUNT]>>,
}

impl BorderSnapshot {
    pub fn new() -> Self {
        Self::default()
    }

    /// Extract border strips from all chunks in `world_chunks` that are
    /// adjacent to any coordinate in `active`.
    pub fn capture(
        world_chunks: &FxHashMap<ChunkCoordinate, Chunk>,
        active: &[ChunkCoordinate],
    ) -> Self {
        let mut snap = Self::new();
        let size = CHUNK_SIZE as usize;

        for &coord in active {
            for face in Face::ALL {
                let neighbor_coord = neighbor_of(coord, face);
                let Some(neighbor) = world_chunks.get(&neighbor_coord) else {
                    continue;
                };

                let key = BorderKey { coord, face };
                if snap.data.contains_key(&key) {
                    continue;
                }

                let mut strip = Box::new([VoxelMaterial::Air; FACE_VOXEL_COUNT]);
                for u in 0..size {
                    for v in 0..size {
                        let (nx, ny, nz) = border_voxel_in_neighbor(face, u, v, size);
                        let src_idx = nx + ny * size + nz * size * size;
                        strip[u + v * size] = neighbor.materials[src_idx];
                    }
                }
                snap.data.insert(key, strip);
            }
        }

        snap
    }

    /// Look up a border strip. Returns None if the neighbor chunk wasn't loaded.
    pub fn get(
        &self,
        coord: ChunkCoordinate,
        face: Face,
    ) -> Option<&[VoxelMaterial; FACE_VOXEL_COUNT]> {
        self.data
            .get(&BorderKey { coord, face })
            .map(|b| b.as_ref())
    }
}

/// Given a face direction, return the neighbor chunk coordinate.
fn neighbor_of(coord: ChunkCoordinate, face: Face) -> ChunkCoordinate {
    use glam::IVec3;
    let offset = match face {
        Face::PosX => IVec3::X,
        Face::NegX => -IVec3::X,
        Face::PosY => IVec3::Y,
        Face::NegY => -IVec3::Y,
        Face::PosZ => IVec3::Z,
        Face::NegZ => -IVec3::Z,
    };
    ChunkCoordinate(coord.0 + offset)
}

/// Given a face and (u, v) coordinates in the strip, return the (x, y, z)
/// index of the border voxel *inside the neighbor chunk*.
fn border_voxel_in_neighbor(face: Face, u: usize, v: usize, size: usize) -> (usize, usize, usize) {
    let last = size - 1;
    match face {
        // Our +X face looks at neighbor's x=0 strip
        Face::PosX => (0, u, v),
        // Our -X face looks at neighbor's x=last strip
        Face::NegX => (last, u, v),
        Face::PosY => (u, 0, v),
        Face::NegY => (u, last, v),
        Face::PosZ => (u, v, 0),
        Face::NegZ => (u, v, last),
    }
}

// ── Solver trait ───────────────────────────────────────────────────────────────

/// A diffusion or erosion algorithm that operates on chunk voxel data.
/// Implementations must be `Send + Sync` to allow parallel execution.
pub trait DiffusionSolver: Send + Sync {
    fn name(&self) -> &'static str;

    fn step(
        &self,
        chunk: &mut Chunk,
        coord: ChunkCoordinate,
        borders: &BorderSnapshot,
        ctx: &DiffusionContext,
    ) -> DiffusionResult;
}
