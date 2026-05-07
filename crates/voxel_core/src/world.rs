use crate::chunk::Chunk;
use crate::coordinate::ChunkCoordinate;
use rustc_hash::FxHashMap;

pub struct World {
    pub chunks: FxHashMap<ChunkCoordinate, Chunk>,
    pub settings: WorldSettings,
    pub dirty_queue: Vec<ChunkCoordinate>,
}

impl World {
    pub fn new() -> Self {
        Self {
            chunks: FxHashMap::default(),
            settings: WorldSettings::default(),
            dirty_queue: Vec::new(),
        }
    }

    pub fn insert_chunk(&mut self, coord: ChunkCoordinate, mut chunk: Chunk) {
        chunk.dirty = true;
        self.dirty_queue.push(coord);
        self.chunks.insert(coord, chunk);
    }

    pub fn get_chunk(&self, coord: ChunkCoordinate) -> Option<&Chunk> {
        self.chunks.get(&coord)
    }

    pub fn get_chunk_mut(&mut self, coord: ChunkCoordinate) -> Option<&mut Chunk> {
        self.chunks.get_mut(&coord)
    }

    pub fn remove_chunk(&mut self, coord: ChunkCoordinate) -> Option<Chunk> {
        self.dirty_queue.retain(|&c| c != coord);
        self.chunks.remove(&coord)
    }

    pub fn drain_dirty(&mut self) -> Vec<ChunkCoordinate> {
        std::mem::take(&mut self.dirty_queue)
    }
}

#[derive(Debug, Clone)]
pub struct WorldSettings {
    pub seed: u64,
    pub load_radius: u32,
    pub gravity: f32,
}

impl Default for WorldSettings {
    fn default() -> Self {
        Self {
            seed: 0,
            load_radius: 8,
            gravity: -9.81,
        }
    }
}
