use crate::coordinate::VoxelIndex;
use crate::voxel::{VoxelDiffusionState, VoxelMaterial};
use ndshape::ConstShape3u32;

pub const CHUNK_SIZE: u32 = 32;

const VOXEL_COUNT: usize = (CHUNK_SIZE as usize).pow(3);

pub type ChunkShape = ConstShape3u32<CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE>;

pub struct Chunk {
    pub materials: Box<[VoxelMaterial]>,
    pub diffusion: Option<Box<[VoxelDiffusionState]>>,
    pub dirty: bool,
}

impl Chunk {
    pub fn new_filled(fill: VoxelMaterial) -> Self {
        Self {
            materials: vec![fill; VOXEL_COUNT].into_boxed_slice(),
            diffusion: None,
            dirty: true,
        }
    }

    #[inline]
    pub fn get(&self, idx: VoxelIndex) -> VoxelMaterial {
        self.materials[idx.linearize()]
    }

    #[inline]
    pub fn set(&mut self, idx: VoxelIndex, material: VoxelMaterial) {
        let i = idx.linearize();
        if self.materials[i] != material {
            self.materials[i] = material;
            self.dirty = true;
        }
    }

    #[inline]
    pub fn clear_dirty(&mut self) {
        self.dirty = false
    }

    pub fn activate_diffusion(&mut self) {
        if self.diffusion.is_none() {
            self.diffusion =
                Some(vec![VoxelDiffusionState::default(); VOXEL_COUNT].into_boxed_slice())
        }
    }
}
