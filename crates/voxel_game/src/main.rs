use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
use bevy::window::WindowResolution;
use voxel_bevy::VoxelBevyPlugin;
use voxel_core::{
    chunk::Chunk,
    coordinate::{ChunkCoordinate, VoxelIndex},
    voxel::VoxelMaterial,
    world::World,
};

fn main() {
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
                }),
        )
        .add_plugins(VoxelBevyPlugin)
        .add_systems(Startup, (setup_camera, seed_test_world))
        .run();
}

fn setup_camera(mut commands: Commands) {
    // Orbit-style starting position looking at chunk (0,0,0) centre
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(48.0, 48.0, 80.0).looking_at(Vec3::new(16.0, 16.0, 16.0), Vec3::Y),
    ));

    // Directional light so PBR shading is visible
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

/// Populate the world with a few test chunks so there's something to render.
fn seed_test_world(mut world: ResMut<World>) {
    // 3×3 flat terrain, 2 chunks tall
    for cx in -1..=1_i32 {
        for cz in -1..=1_i32 {
            // Bottom layer: solid stone
            let mut ground = Chunk::new_filled(VoxelMaterial::Stone);
            // Top 2 rows of the ground chunk are grass
            for x in 0..32_u32 {
                for z in 0..32_u32 {
                    ground.set(VoxelIndex::new(x, 30, z), VoxelMaterial::Dirt);
                    ground.set(VoxelIndex::new(x, 31, z), VoxelMaterial::Grass);
                }
            }
            world.insert_chunk(ChunkCoordinate::new(cx, 0, cz), ground);

            // Upper layer: air (still insert so neighbours cull correctly)
            world.insert_chunk(
                ChunkCoordinate::new(cx, 1, cz),
                Chunk::new_filled(VoxelMaterial::Air),
            );
        }
    }
}
