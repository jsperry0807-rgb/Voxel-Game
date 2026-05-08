use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use voxel_core::voxel::VoxelMaterial;
use voxel_core::world::World;

#[derive(Asset, TypePath, Serialize, Deserialize)]
pub struct MaterialDefinitions {
    pub materials: Vec<MaterialDef>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MaterialDef {
    pub name: String,
    pub material: VoxelMaterial,
    pub texture_top: u32,
    pub texture_side: u32,
    pub texture_bottom: u32,
    pub hardness: f32,
    pub diffusion_coeff: f32,
}

pub fn on_material_defs_changed(
    mut events: MessageReader<AssetEvent<MaterialDefinitions>>,
    assets: Res<Assets<MaterialDefinitions>>,
    mut world: ResMut<World>,
) {
    for event in events.read() {
        if let AssetEvent::Modified { id } = event {
            if let Some(defs) = assets.get(*id) {
                world.reload_material_definitions(defs.materials.len());
            }
        }
    }
}
