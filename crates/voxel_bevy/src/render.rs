use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy_mesh::{Indices, PrimitiveTopology};
use voxel_core::{
    coordinate::ChunkCoordinate,
    mesh::{ChunkMesh, generate_mesh},
    world::World,
};

// ── Components ────────────────────────────────────────────────────────────────

/// Marker component: this entity represents a rendered chunk.
#[derive(Component)]
pub struct ChunkEntity(pub ChunkCoordinate);

// ── Resources ─────────────────────────────────────────────────────────────────

/// Holds meshes that have been generated (on worker threads eventually)
/// and are ready to be uploaded to the GPU and spawned into the ECS.
#[derive(Resource, Default)]
pub struct MeshCache {
    pub pending: Vec<(ChunkCoordinate, ChunkMesh)>,
}

// ── Systems ───────────────────────────────────────────────────────────────────

/// Drains dirty chunks from the World, generates CPU-side meshes,
/// and pushes them into MeshCache for the next system to upload.
pub fn process_dirty_chunks(mut world: ResMut<World>, mut cache: ResMut<MeshCache>) {
    let dirty = world.drain_dirty();
    for coord in dirty {
        let Some(chunk) = world.get_chunk_mut(coord) else {
            continue;
        };

        // Skip chunks that have nothing to render
        let mesh = generate_mesh(chunk);
        chunk.clear_dirty();

        cache.pending.push((coord, mesh));
    }
}

/// Takes pending meshes from MeshCache, converts them to Bevy Mesh assets,
/// then either spawns new entities or updates existing ones.
pub fn upload_generated_meshes(
    mut commands: Commands,
    mut cache: ResMut<MeshCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    existing: Query<(Entity, &ChunkEntity)>,
) {
    for (coord, chunk_mesh) in cache.pending.drain(..) {
        // Skip empty meshes (all-air chunk)
        if chunk_mesh.indices.is_empty() {
            // Despawn old entity if it existed
            despawn_chunk(&mut commands, &existing, coord);
            continue;
        }

        let bevy_mesh = build_bevy_mesh(&chunk_mesh);
        let mesh_handle = meshes.add(bevy_mesh);

        // World-space offset for this chunk
        let world_pos = coord.world_min();
        let transform = Transform::from_xyz(world_pos.x, world_pos.y, world_pos.z);

        // Reuse or spawn
        if let Some((entity, _)) = existing.iter().find(|(_, ce)| ce.0 == coord) {
            commands
                .entity(entity)
                .insert((Mesh3d(mesh_handle), transform));
        } else {
            // One shared material for now — swap for atlas later
            let mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.5, 0.5),
                perceptual_roughness: 0.9,
                ..default()
            });

            commands.spawn((
                ChunkEntity(coord),
                Mesh3d(mesh_handle),
                MeshMaterial3d(mat),
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

fn despawn_chunk(
    commands: &mut Commands,
    existing: &Query<(Entity, &ChunkEntity)>,
    coord: ChunkCoordinate,
) {
    if let Some((entity, _)) = existing.iter().find(|(_, ce)| ce.0 == coord) {
        commands.entity(entity).despawn();
    }
}
