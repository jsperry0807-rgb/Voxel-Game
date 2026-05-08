pub mod heat;
pub mod thermal;

use crate::{
    chunk::Chunk,
    coordinate::{CHUNK_SIZE, ChunkCoordinate},
    voxel::{VoxelDiffusionState, VoxelMaterial},
    world::World,
};
use rustc_hash::FxHashMap;

// ── Context ────────────────────────────────────────────────────────────────────

pub struct DiffusionContext {
    pub dt: f32,
    pub gravity: f32,
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

// ── Result ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct DiffusionResult {
    pub cells_modified: u64,
    pub material_transferred: f32,
    pub execution_time_us: u64,
}

// ── Face ───────────────────────────────────────────────────────────────────────

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

    /// The face on the neighbor chunk that faces back toward us.
    pub fn opposite(self) -> Self {
        match self {
            Face::PosX => Face::NegX,
            Face::NegX => Face::PosX,
            Face::PosY => Face::NegY,
            Face::NegY => Face::PosY,
            Face::PosZ => Face::NegZ,
            Face::NegZ => Face::PosZ,
        }
    }
}

// ── Border strip types ─────────────────────────────────────────────────────────

const FACE_SIZE: usize = (CHUNK_SIZE * CHUNK_SIZE) as usize;

/// One face-strip's worth of material + diffusion state from a neighbor chunk.
#[derive(Clone)]
pub struct BorderStrip {
    pub materials: Box<[VoxelMaterial; FACE_SIZE]>,
    pub diffusion: Option<Box<[VoxelDiffusionState; FACE_SIZE]>>,
}

impl BorderStrip {
    fn new_air() -> Self {
        Self {
            materials: Box::new([VoxelMaterial::Air; FACE_SIZE]),
            diffusion: None,
        }
    }
}

// ── Border snapshot ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BorderKey {
    pub coord: ChunkCoordinate,
    pub face: Face,
}

/// Read-only snapshot of 1-voxel border strips from all neighbor chunks.
///
/// Captures both `VoxelMaterial` and `VoxelDiffusionState` so solvers can
/// correctly handle cross-chunk heat flow and erosion handoff.
///
/// Layout convention for strip arrays (u, v):
///   PosX / NegX : u = y, v = z
///   PosY / NegY : u = x, v = z
///   PosZ / NegZ : u = x, v = y
#[derive(Default)]
pub struct BorderSnapshot {
    pub strips: FxHashMap<BorderKey, BorderStrip>,
}

impl BorderSnapshot {
    pub fn new() -> Self {
        Self::default()
    }

    /// Capture border strips from all neighbors of each chunk in `active`.
    /// Only chunks present in `world.chunks` contribute — missing neighbors
    /// produce air strips with no diffusion state.
    pub fn capture(world: &World, active: &[ChunkCoordinate]) -> Self {
        let mut snap = Self::new();
        let size = CHUNK_SIZE as usize;

        for &coord in active {
            for face in Face::ALL {
                let key = BorderKey { coord, face };
                if snap.strips.contains_key(&key) {
                    continue;
                }

                let neighbor_coord = neighbor_of(coord, face);
                let Some(neighbor) = world.chunks.get(&neighbor_coord) else {
                    // No neighbor loaded — insert air strip as cold boundary
                    snap.strips.insert(key, BorderStrip::new_air());
                    continue;
                };

                let mut strip = BorderStrip::new_air();

                // Extract the 1-voxel border from the neighbor chunk that
                // faces back toward `coord` (the opposite face).
                let neighbor_face = face.opposite();

                let has_diffusion = neighbor.diffusion.is_some();
                if has_diffusion {
                    strip.diffusion = Some(Box::new([VoxelDiffusionState::default(); FACE_SIZE]));
                }

                for u in 0..size {
                    for v in 0..size {
                        let (nx, ny, nz) = border_voxel(neighbor_face, u, v, size);
                        let src = nx + ny * size + nz * size * size;
                        let dst = u + v * size;

                        strip.materials[dst] = neighbor.materials[src];

                        if let (Some(ndiff), Some(sdiff)) =
                            (&neighbor.diffusion, &mut strip.diffusion)
                        {
                            sdiff[dst] = ndiff[src];
                        }
                    }
                }

                snap.strips.insert(key, strip);
            }
        }

        snap
    }

    pub fn get(&self, coord: ChunkCoordinate, face: Face) -> Option<&BorderStrip> {
        self.strips.get(&BorderKey { coord, face })
    }
}

// ── Solver trait ───────────────────────────────────────────────────────────────

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

// ── Geometry helpers ───────────────────────────────────────────────────────────

pub fn neighbor_of(coord: ChunkCoordinate, face: Face) -> ChunkCoordinate {
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

/// Given a face direction and (u, v) strip coordinates, return the (x, y, z)
/// voxel index on that face of the chunk.
///
///   PosX face: x = S-1, u = y, v = z
///   NegX face: x = 0,   u = y, v = z
///   PosY face: y = S-1, u = x, v = z
///   NegY face: y = 0,   u = x, v = z
///   PosZ face: z = S-1, u = x, v = y
///   NegZ face: z = 0,   u = x, v = y
pub fn border_voxel(face: Face, u: usize, v: usize, size: usize) -> (usize, usize, usize) {
    let last = size - 1;
    match face {
        Face::PosX => (last, u, v),
        Face::NegX => (0, u, v),
        Face::PosY => (u, last, v),
        Face::NegY => (u, 0, v),
        Face::PosZ => (u, v, last),
        Face::NegZ => (u, v, 0),
    }
}
