use serde::{Deserialize, Serialize};

pub const CHUNK_SIZE: i32 = 32;
pub const CHUNK_HEIGHT: i32 = 256;
pub const REGION_CHUNKS: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct ChunkPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    // Returns this chunk's (x, y) offset within its region, in the range [0, REGION_CHUNKS).
    pub fn region_local(&self) -> (i32, i32) {
        let r: i32 = REGION_CHUNKS as i32;
        let rx: i32 = ((self.x % r) + r) % r;
        let rz: i32 = ((self.z % r) + r) % r;
        (rx, rz)
    }
}
