use bevy::prelude::*;
use voxel_core::world::World;

use crate::render::{MeshCache, process_dirty_chunks, upload_generated_meshes};

pub struct VoxelBevyPlugin;

impl Plugin for VoxelBevyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(World::new())
            .insert_resource(MeshCache::default())
            .add_systems(Update, process_dirty_chunks)
            .add_systems(Update, upload_generated_meshes.after(process_dirty_chunks));
    }
}
