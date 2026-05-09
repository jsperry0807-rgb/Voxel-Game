use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
use bevy::window::WindowResolution;
use voxel_bevy::{VoxelBevyPlugin, assets::MaterialDefinitions};
use voxel_core::{
    chunk::Chunk,
    coordinate::{ChunkCoordinate, VoxelIndex},
    voxel::VoxelMaterial,
    world::World,
};

use crate::{
    audio::VoxelAudioPlugin,
    render::{MeshCache, process_dirty_chunks, upload_generated_meshes},
};

#[derive(Resource)]
#[allow(dead_code)]
struct MaterialsHandle(Handle<MaterialDefinitions>);

pub struct VoxelBevyPlugin;

fn main() {
    // Optional: keep panic hook for debugging; remove if you want.
    std::panic::set_hook(Box::new(|info| {
        eprintln!("==================== PANIC ====================");
        eprintln!("{}", info);
        if let Some(location) = info.location() {
            eprintln!(
                "  at {}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            );
        }
        eprintln!("===============================================");
    }));

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Voxel Engine".into(),
                        resolution: WindowResolution::new(1280, 720),
                        ..default()
                    }),
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        backends: Some(Backends::DX12),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: "assets".to_string(),
                    ..default()
                }),
        )
        .add_plugins(VoxelBevyPlugin)
        .add_plugins(VoxelAudioPlugin)
        .add_systems(Startup, (setup_camera, seed_test_world, load_materials))
        .run();
}

fn load_materials(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load("materials.ron");
    commands.insert_resource(MaterialsHandle(handle));
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(80.0, 80.0, 140.0).looking_at(Vec3::new(48.0, 0.0, 48.0), Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::FRAC_PI_4,
            std::f32::consts::FRAC_PI_4,
            0.0,
        )),
    ));
}

fn seed_test_world(mut world: ResMut<World>) {
    for cx in -1..=1_i32 {
        for cz in -1..=1_i32 {
            let mut ground = Chunk::new_filled(VoxelMaterial::Stone);
            for x in 0..32_u32 {
                for z in 0..32_u32 {
                    ground.set(VoxelIndex::new(x, 30, z), VoxelMaterial::Dirt);
                    ground.set(VoxelIndex::new(x, 31, z), VoxelMaterial::Grass);
                }
            }
            world.insert_chunk(ChunkCoordinate::new(cx, 0, cz), ground);

            world.insert_chunk(
                ChunkCoordinate::new(cx, 1, cz),
                Chunk::new_filled(VoxelMaterial::Air),
            );
        }
    }
}
