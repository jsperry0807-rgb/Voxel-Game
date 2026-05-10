use serde::{Deserialize, Serialize};
use voxel_core::block_id::BlockId;

pub const GEN_CHUNK_W: usize = 16;
pub const GEN_CHUNK_H: usize = 256;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct GenOutput {
    pub blocks: Vec<BlockId>, // [y][z][x] order
    pub heightmap: Vec<u8>,   // [z][x] surface y
}

impl GenOutput {
    pub fn new() -> Self {
        Self {
            blocks: vec![0; GEN_CHUNK_W * GEN_CHUNK_H * GEN_CHUNK_W],
            heightmap: vec![0; GEN_CHUNK_W * GEN_CHUNK_W],
        }
    }

    pub fn block_idx(x: usize, y: usize, z: usize) -> usize {
        y * GEN_CHUNK_W * GEN_CHUNK_W + z * GEN_CHUNK_W + x
    }
}
