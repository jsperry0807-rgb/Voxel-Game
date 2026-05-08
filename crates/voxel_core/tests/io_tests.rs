use std::fs;
use voxel_core::{
    chunk::Chunk,
    coordinate::{ChunkCoordinate, VoxelIndex},
    io::{chunk_path, load_chunk, save_chunk},
    voxel::VoxelMaterial,
};

fn temp_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("voxel_io_tests");
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn test_coord() -> ChunkCoordinate {
    ChunkCoordinate::new(3, -1, 7)
}

#[test]
fn test_save_and_load_roundtrip() {
    let mut chunk = Chunk::new_filled(VoxelMaterial::Stone);
    chunk.set(VoxelIndex::new(0, 0, 0), VoxelMaterial::Grass);
    chunk.set(VoxelIndex::new(31, 31, 31), VoxelMaterial::Water);
    chunk.set(VoxelIndex::new(15, 8, 20), VoxelMaterial::Sand);
    chunk.dirty = false;

    let coord = test_coord();
    let dir = temp_dir();
    let path = chunk_path(&dir, coord);

    save_chunk(&path, &chunk, coord).expect("save should succeed");
    assert!(path.exists(), "chunk file should exist after save");

    let (loaded, loaded_coord) = load_chunk(&path).expect("load should succeed");

    assert_eq!(loaded_coord, coord);
    assert_eq!(loaded.get(VoxelIndex::new(0, 0, 0)), VoxelMaterial::Grass);
    assert_eq!(
        loaded.get(VoxelIndex::new(31, 31, 31)),
        VoxelMaterial::Water
    );
    assert_eq!(loaded.get(VoxelIndex::new(15, 8, 20)), VoxelMaterial::Sand);
    assert_eq!(loaded.get(VoxelIndex::new(1, 1, 1)), VoxelMaterial::Stone);
    assert!(!loaded.dirty, "loaded chunk should not be dirty");
}

#[test]
fn test_save_and_load_with_diffusion() {
    let mut chunk = Chunk::new_filled(VoxelMaterial::Stone);
    chunk.activate_diffusion();

    let idx = VoxelIndex::new(10, 10, 10);
    chunk.diffusion.as_mut().unwrap()[idx.linearize()].temperature = 42.5;
    chunk.diffusion.as_mut().unwrap()[idx.linearize()].water_level = 0.75;

    let coord = ChunkCoordinate::new(-5, 2, 0);
    let dir = temp_dir();
    let path = chunk_path(&dir, coord);

    save_chunk(&path, &chunk, coord).expect("save with diffusion should succeed");

    let (loaded, _) = load_chunk(&path).expect("load with diffusion should succeed");

    let diff = loaded
        .diffusion
        .as_ref()
        .expect("diffusion should be present");
    let state = diff[idx.linearize()];
    assert!(
        (state.temperature - 42.5).abs() < 1e-4,
        "temperature should roundtrip, got {}",
        state.temperature
    );
    assert!(
        (state.water_level - 0.75).abs() < 1e-4,
        "water_level should roundtrip, got {}",
        state.water_level
    );
}

#[test]
fn test_save_and_load_all_air() {
    let chunk = Chunk::new_filled(VoxelMaterial::Air);
    let coord = ChunkCoordinate::new(0, 0, 0);
    let dir = temp_dir();
    let path = chunk_path(&dir, coord);

    save_chunk(&path, &chunk, coord).expect("save air chunk should succeed");
    let (loaded, _) = load_chunk(&path).expect("load air chunk should succeed");

    for idx in voxel_core::coordinate::VoxelIndex::iter_all() {
        assert_eq!(loaded.get(idx), VoxelMaterial::Air);
    }
}

#[test]
fn test_no_diffusion_roundtrip() {
    // Chunk without diffusion — diffusion field should stay None after load
    let chunk = Chunk::new_filled(VoxelMaterial::Dirt);
    let coord = ChunkCoordinate::new(1, 2, 3);
    let dir = temp_dir();
    let path = chunk_path(&dir, coord);

    save_chunk(&path, &chunk, coord).unwrap();
    let (loaded, _) = load_chunk(&path).unwrap();

    assert!(
        loaded.diffusion.is_none(),
        "diffusion should be None when not activated"
    );
}

#[test]
fn test_chunk_path_format() {
    let dir = std::path::Path::new("/world");
    let coord = ChunkCoordinate::new(-3, 0, 12);
    let path = chunk_path(dir, coord);
    assert_eq!(
        path.to_str().unwrap().replace('\\', "/"),
        "/world/chunks/r_-3_0_12.chunk"
    );
}

#[test]
fn test_atomic_write_no_partial_file_on_success() {
    let chunk = Chunk::new_filled(VoxelMaterial::Stone);
    let coord = ChunkCoordinate::new(99, 99, 99);
    let dir = temp_dir();
    let path = chunk_path(&dir, coord);

    save_chunk(&path, &chunk, coord).unwrap();

    // The .tmp file should have been cleaned up (renamed to final path)
    let tmp = path.with_extension("chunk.tmp");
    assert!(
        !tmp.exists(),
        ".tmp file should not exist after successful save"
    );
    assert!(path.exists(), "final file should exist");
}

#[test]
fn test_load_nonexistent_file_errors() {
    let path = temp_dir().join("nonexistent_chunk.chunk");
    let result = load_chunk(&path);
    assert!(result.is_err(), "loading nonexistent file should error");
}

#[test]
fn test_coordinate_roundtrip_negative() {
    let chunk = Chunk::new_filled(VoxelMaterial::Stone);
    let coord = ChunkCoordinate::new(-100, -50, -200);
    let dir = temp_dir();
    let path = chunk_path(&dir, coord);

    save_chunk(&path, &chunk, coord).unwrap();
    let (_, loaded_coord) = load_chunk(&path).unwrap();

    assert_eq!(loaded_coord, coord, "negative coordinates should roundtrip");
}
