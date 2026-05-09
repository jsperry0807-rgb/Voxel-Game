use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy_mesh::{Indices, PrimitiveTopology};
use std::collections::HashMap;
use voxel_core::{
    coordinate::ChunkCoordinate,
    mesh::{ChunkMesh, generate_meshes_per_material},
    voxel::VoxelMaterial,
    world::World,
};
use wgpu_types::{Extent3d, TextureDimension, TextureFormat};

// ── Components & Resources ────────────────────────────────────────────────────

#[derive(Component)]
pub struct ChunkMaterialEntity(pub ChunkCoordinate, pub VoxelMaterial);

#[derive(Resource, Default)]
pub struct MeshCache {
    pub pending: Vec<(ChunkCoordinate, VoxelMaterial, ChunkMesh)>,
}

#[derive(Resource)]
pub struct MaterialTextures {
    pub handles: HashMap<VoxelMaterial, Handle<Image>>,
    pub top_handles: HashMap<VoxelMaterial, Handle<Image>>,
    pub side_handles: HashMap<VoxelMaterial, Handle<Image>>,
    pub bottom_handles: HashMap<VoxelMaterial, Handle<Image>>,
}

// ── Face index constants (RIGHT_HANDED_Y_UP_CONFIG order) ────────────────────
// 0: -X  1: +X  2: -Y (bottom)  3: +Y (top)  4: -Z  5: +Z

const FACE_TOP: u8 = 3;
const FACE_BOTTOM: u8 = 2;

// ── Startup system ────────────────────────────────────────────────────────────

pub fn load_material_textures(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let nearest = ImageSampler::nearest();

    let mut load = |path: &str| -> Handle<Image> {
        let bytes = std::fs::read(format!("crates/voxel_game/assets/{}", path))
            .unwrap_or_else(|_| panic!("Failed to read texture: {path}"));
        let img = image::load_from_memory(&bytes)
            .unwrap_or_else(|_| panic!("Failed to decode texture: {path}"));
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let mut image = Image::new(
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            rgba.into_raw(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        image.sampler = nearest.clone();
        images.add(image)
    };

    let mut handles = HashMap::new();
    handles.insert(VoxelMaterial::Stone, load("textures/stone.png"));
    handles.insert(VoxelMaterial::Dirt, load("textures/dirt.png"));
    handles.insert(VoxelMaterial::Sand, load("textures/sand.png"));
    handles.insert(VoxelMaterial::Water, load("textures/water.png"));

    let mut top_handles = HashMap::new();
    let mut side_handles = HashMap::new();
    let mut bottom_handles = HashMap::new();
    top_handles.insert(VoxelMaterial::Grass, load("textures/dirt.png"));
    side_handles.insert(VoxelMaterial::Grass, load("textures/grass_side.png"));
    bottom_handles.insert(VoxelMaterial::Grass, load("textures/dirt.png"));

    commands.insert_resource(MaterialTextures {
        handles,
        top_handles,
        side_handles,
        bottom_handles,
    });
}

// ── Per-frame systems ─────────────────────────────────────────────────────────

pub fn process_dirty_chunks(mut world: ResMut<World>, mut cache: ResMut<MeshCache>) {
    for coord in world.drain_dirty() {
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

pub fn upload_generated_meshes(
    mut commands: Commands,
    mut cache: ResMut<MeshCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    textures: Res<MaterialTextures>,
    existing: Query<(Entity, &ChunkMaterialEntity)>,
) {
    for (coord, material, chunk_mesh) in cache.pending.drain(..) {
        // Despawn stale entities for this chunk+material before re-spawning.
        for entity in existing
            .iter()
            .filter(|(_, ce)| ce.0 == coord && ce.1 == material)
            .map(|(e, _)| e)
            .collect::<Vec<_>>()
        {
            commands.entity(entity).despawn();
        }

        let p = coord.world_min();
        let transform = Transform::from_xyz(p.x, p.y, p.z);

        if has_per_face_textures(material) {
            spawn_face_filtered(
                &mut commands,
                &mut meshes,
                &mut materials,
                &textures,
                &chunk_mesh,
                coord,
                material,
                transform,
            );
        } else {
            let bevy_mesh = build_bevy_mesh(&chunk_mesh, |_| true);
            if bevy_mesh.count_vertices() == 0 {
                continue;
            }
            let mesh_handle = meshes.add(bevy_mesh);
            eprintln!("Looking up material: {:?}", material);
            let mat_handle = make_material(&mut materials, textures.handles.get(&material));
            commands.spawn((
                ChunkMaterialEntity(coord, material),
                Mesh3d(mesh_handle),
                MeshMaterial3d(mat_handle),
                transform,
            ));
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn has_per_face_textures(mat: VoxelMaterial) -> bool {
    matches!(mat, VoxelMaterial::Grass)
}

/// Spawn up to 3 entities (top / sides / bottom) with different textures.
fn spawn_face_filtered(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    textures: &MaterialTextures,
    chunk_mesh: &ChunkMesh,
    coord: ChunkCoordinate,
    material: VoxelMaterial,
    transform: Transform,
) {
    let groups: &[(&dyn Fn(u8) -> bool, Option<&Handle<Image>>)] = &[
        (&|fi| fi == FACE_TOP, textures.top_handles.get(&material)),
        (
            &|fi| fi == FACE_BOTTOM,
            textures.bottom_handles.get(&material),
        ),
        (
            &|fi| fi != FACE_TOP && fi != FACE_BOTTOM,
            textures.side_handles.get(&material),
        ),
    ];

    for (filter, tex) in groups {
        let mesh = build_bevy_mesh(chunk_mesh, filter);
        if mesh.count_vertices() == 0 {
            continue;
        }
        commands.spawn((
            ChunkMaterialEntity(coord, material),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(make_material(materials, *tex)),
            transform,
        ));
    }
}

fn make_material(
    materials: &mut Assets<StandardMaterial>,
    texture: Option<&Handle<Image>>,
) -> Handle<StandardMaterial> {
    if texture.is_none() {
        eprintln!("make_material: texture is None!");
    }
    materials.add(if let Some(tex) = texture {
        StandardMaterial {
            base_color_texture: Some(tex.clone()),
            perceptual_roughness: 0.9,
            ..default()
        }
    } else {
        StandardMaterial {
            base_color: Color::srgb(0.5, 0.5, 0.5),
            ..default()
        }
    })
}

/// Build a filtered Bevy `Mesh` from a `ChunkMesh`.
///
/// Only quads whose face index passes `filter` are included.
/// Indices use block_mesh's corner layout: CCW triangles (0,1,2) and (1,3,2).
fn build_bevy_mesh(chunk_mesh: &ChunkMesh, filter: impl Fn(u8) -> bool) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for q in 0..chunk_mesh.vertices.len() / 4 {
        let v = q * 4;
        let face_idx = chunk_mesh.face_indices[v];
        if !filter(face_idx) {
            continue;
        }

        let base = positions.len() as u32;
        positions.extend_from_slice(&chunk_mesh.vertices[v..v + 4]);
        normals.extend_from_slice(&chunk_mesh.normals[v..v + 4]);
        uvs.extend_from_slice(&chunk_mesh.uvs[v..v + 4]);
        indices.extend([base, base + 1, base + 2, base + 1, base + 3, base + 2]);
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
