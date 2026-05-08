use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;
use std::path::PathBuf;
use voxel_core::world::World;

use crate::{
    assets::{MaterialDefinitions, on_material_defs_changed},
    loader::{ChunkLoader, chunk_loading_system},
    render::{MeshCache, process_dirty_chunks, upload_generated_meshes},
};

pub struct VoxelBevyPlugin;

impl Plugin for VoxelBevyPlugin {
    fn build(&self, app: &mut App) {
        let world = World::new();
        let load_radius = world.settings.load_radius;

        app.add_plugins(RonAssetPlugin::<MaterialDefinitions>::new(&[
            "materials.ron",
        ]))
        .insert_resource(world)
        .insert_resource(MeshCache::default())
        .insert_resource(ChunkLoader::new(PathBuf::from("world"), load_radius))
        .add_systems(Update, chunk_loading_system)
        .add_systems(Update, process_dirty_chunks)
        .add_systems(Update, upload_generated_meshes.after(process_dirty_chunks))
        .add_systems(Update, on_material_defs_changed);
    }
}
