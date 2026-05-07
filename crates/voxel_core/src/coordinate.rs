use glam::IVec3;
use serde::{Deserialize, Serialize};

pub const CHUNK_SIZE: u32 = 32;
const CHUNK_SIZE_U: usize = CHUNK_SIZE as usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkCoordinate(pub IVec3);

impl ChunkCoordinate {
    #[inline]
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self(IVec3::new(x, y, z))
    }

    pub fn face_neighbors(self) -> [Self; 6] {
        [
            Self(self.0 + IVec3::X),
            Self(self.0 - IVec3::X),
            Self(self.0 + IVec3::Y),
            Self(self.0 - IVec3::Y),
            Self(self.0 + IVec3::Z),
            Self(self.0 - IVec3::Z),
        ]
    }

    #[inline]
    pub fn world_min(self) -> glam::Vec3 {
        self.0.as_vec3() * CHUNK_SIZE as f32
    }

    #[inline]
    pub fn voxel_world_position(self, voxel: VoxelIndex) -> glam::Vec3 {
        self.world_min() + glam::vec3(voxel.x as f32, voxel.y as f32, voxel.z as f32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoxelIndex {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl VoxelIndex {
    #[inline]
    pub fn new(x: u32, y: u32, z: u32) -> Self {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE);
        Self { x, y, z }
    }

    #[inline]
    pub fn linearize(self) -> usize {
        self.x as usize
            + self.y as usize * CHUNK_SIZE_U
            + self.z as usize * CHUNK_SIZE_U * CHUNK_SIZE_U
    }

    #[inline]
    pub fn from_linear(idx: usize) -> Self {
        let x = (idx % CHUNK_SIZE_U) as u32;
        let y = ((idx / CHUNK_SIZE_U) % CHUNK_SIZE_U) as u32;
        let z = (idx / (CHUNK_SIZE_U * CHUNK_SIZE_U)) as u32;
        Self { x, y, z }
    }

    pub fn iter_all() -> impl Iterator<Item = Self> {
        (0..CHUNK_SIZE_U.pow(3)).map(Self::from_linear)
    }
}
