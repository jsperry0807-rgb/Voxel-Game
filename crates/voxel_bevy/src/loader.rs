use bevy::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;
use voxel_core::coordinate::ChunkCoordinate;
use voxel_core::io::{load_chunk, save_chunk};
use voxel_core::world::World;

#[derive(Resource)]
pub struct ChunkLoader {
    pub world_dir: PathBuf,
    pub load_radius: u32,
    pub loads_per_frame: usize,
    pub load_queue: Vec<ChunkCoordinate>,
    pub loaded: HashSet<ChunkCoordinate>,
}

impl ChunkLoader {
    pub fn new(world_dir: PathBuf, load_radius: u32) -> Self {
        Self {
            world_dir,
            load_radius,
            loads_per_frame: 2,
            load_queue: Vec::new(),
            loaded: HashSet::new(),
        }
    }

    pub fn chunk_path(&self, coord: ChunkCoordinate) -> PathBuf {
        self.world_dir.join(format!(
            "chunk_{}_{}_{}.bin",
            coord.0.x, coord.0.y, coord.0.z
        ))
    }
}

pub fn spiral_coords(center: ChunkCoordinate, radius: u32) -> Vec<ChunkCoordinate> {
    let mut coords = Vec::new();
    for r in 0..=radius as i32 {
        for x in -r..=r {
            for z in -r..=r {
                if x.abs() == r || z.abs() == r {
                    for y in -2..=4 {
                        coords.push(ChunkCoordinate::new(
                            center.0.x + x,
                            center.0.y + y,
                            center.0.z + z,
                        ))
                    }
                }
            }
        }
    }
    coords.sort_by_key(|c| c.0.distance_squared(center.0));
    coords
}

pub fn chunk_loading_system(
    mut loader: ResMut<ChunkLoader>,
    mut world: ResMut<World>,
    camera: Query<&Transform, With<Camera3d>>,
) {
    let Ok(cam_transform) = camera.single() else {
        return;
    };

    // Convert camera position to chunk coordinate
    let pos = cam_transform.translation;
    let center = ChunkCoordinate::new(
        (pos.x / 32.0).floor() as i32,
        (pos.y / 32.0).floor() as i32,
        (pos.z / 32.0).floor() as i32,
    );

    let desired: HashSet<ChunkCoordinate> = spiral_coords(center, loader.load_radius)
        .into_iter()
        .collect();

    let to_unload: Vec<_> = loader
        .loaded
        .iter()
        .filter(|c| !desired.contains(c))
        .copied()
        .collect();

    for coord in to_unload {
        if let Some(chunk) = world.remove_chunk(coord) {
            let path = loader.chunk_path(coord);
            let _ = save_chunk(&path, &chunk, coord);
        }
        loader.loaded.remove(&coord);
    }

    // Rebuild load queue form desired - loaded
    loader.load_queue = desired
        .into_iter()
        .filter(|c| !loader.loaded.contains(c))
        .collect();
    loader
        .load_queue
        .sort_by_key(|c| c.0.distance_squared(center.0));

    let count = loader.load_queue.len().min(loader.loads_per_frame);
    let to_load: Vec<_> = loader.load_queue.drain(..count).collect();

    for coord in to_load {
        let path = loader.chunk_path(coord);
        if let Ok((chunk, coord)) = load_chunk(&path) {
            world.insert_chunk(coord, chunk);
            loader.loaded.insert(coord);
        }
    }
}
