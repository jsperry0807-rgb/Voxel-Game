use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy_mesh::{Indices, PrimitiveTopology};
use std::collections::HashMap;
use voxel_core::{
    coordinate::ChunkCoordinate,
    mesh::{ChunkMesh, generate_meshes_per_material},
    voxel::VoxelMaterial,
    world::World,
};

// ── Components ────────────────────────────────────────────────────────────────

/// Marker component: this entity represents a rendered chunk material pair.
#[derive(Component)]
pub struct ChunkMaterialEntity(pub ChunkCoordinate, pub VoxelMaterial);

// ── Resources ─────────────────────────────────────────────────────────────────

/// Holds meshes that have been generated (on worker threads eventually)
/// and are ready to be uploaded to the GPU and spawned into the ECS.
#[derive(Resource, Default)]
pub struct MeshCache {
    pub pending: Vec<(ChunkCoordinate, VoxelMaterial, ChunkMesh)>,
}

// ── Texture mapping (load once) ──────────────────────────────────────────────

#[derive(Resource)]
pub struct MaterialTextures {
    pub handles: HashMap<VoxelMaterial, Handle<Image>>,
}

pub fn load_material_textures(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut handles = HashMap::new();
    handles.insert(
        VoxelMaterial::Stone,
        asset_server.load("textures/stone.png"),
    );
    handles.insert(VoxelMaterial::Dirt, asset_server.load("textures/dirt.png"));
    handles.insert(
        VoxelMaterial::Grass,
        asset_server.load("textures/grass.png"),
    );
    handles.insert(VoxelMaterial::Sand, asset_server.load("textures/sand.png"));
    handles.insert(
        VoxelMaterial::Water,
        asset_server.load("textures/water.png"),
    );
    // Air has no texture
    commands.insert_resource(MaterialTextures { handles });
}

// ── Systems ───────────────────────────────────────────────────────────────────

/// Drains dirty chunks from the World, generates per‑material CPU‑side meshes,
/// and pushes them into MeshCache for the next system to upload.
pub fn process_dirty_chunks(mut world: ResMut<World>, mut cache: ResMut<MeshCache>) {
    let dirty = world.drain_dirty();
    for coord in dirty {
        let Some(chunk) = world.get_chunk_mut(coord) else {
            continue;
        };

        let per_mat = generate_meshes_per_material(chunk);
        chunk.clear_dirty();

        for (material, mesh) in per_mat {
            if !mesh.indices.is_empty() {
                cache.pending.push((coord, material, mesh));
            }
        }
    }
}

/// Takes pending meshes from MeshCache, converts them to Bevy Mesh assets,
/// then either spawns new entities or updates existing ones.
pub fn upload_generated_meshes(
    mut commands: Commands,
    mut cache: ResMut<MeshCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    textures: Res<MaterialTextures>,
    existing: Query<(Entity, &ChunkMaterialEntity)>,
) {
    for (coord, material, chunk_mesh) in cache.pending.drain(..) {
        let bevy_mesh = build_bevy_mesh(&chunk_mesh);
        let mesh_handle = meshes.add(bevy_mesh);

        let world_pos = coord.world_min();
        let transform = Transform::from_xyz(world_pos.x, world_pos.y, world_pos.z);

        // Find existing entity for this chunk+material pair
        let existing_entity = existing
            .iter()
            .find(|(_, ce)| ce.0 == coord && ce.1 == material)
            .map(|(e, _)| e);

        // Get or create material asset
        let material_handle = if let Some(texture) = textures.handles.get(&material) {
            materials.add(StandardMaterial {
                base_color_texture: Some(texture.clone()),
                perceptual_roughness: 0.9,
                ..default()
            })
        } else {
            // Fallback for Air or missing
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.5, 0.5),
                ..default()
            })
        };

        if let Some(entity) = existing_entity {
            commands.entity(entity).insert((
                Mesh3d(mesh_handle),
                MeshMaterial3d(material_handle),
                transform,
            ));
        } else {
            commands.spawn((
                ChunkMaterialEntity(coord, material),
                Mesh3d(mesh_handle),
                MeshMaterial3d(material_handle),
                transform,
            ));
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn build_bevy_mesh(chunk_mesh: &ChunkMesh) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, chunk_mesh.vertices.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, chunk_mesh.normals.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, chunk_mesh.uvs.clone());
    mesh.insert_indices(Indices::U32(chunk_mesh.indices.clone()));
    mesh
}

/// Helper to despawn a specific chunk+material entity (not used directly here, but can be called elsewhere).
pub fn despawn_chunk_material(
    commands: &mut Commands,
    existing: &Query<(Entity, &ChunkMaterialEntity)>,
    coord: ChunkCoordinate,
    material: VoxelMaterial,
) {
    if let Some((entity, _)) = existing
        .iter()
        .find(|(_, ce)| ce.0 == coord && ce.1 == material)
    {
        commands.entity(entity).despawn();
    }
}
