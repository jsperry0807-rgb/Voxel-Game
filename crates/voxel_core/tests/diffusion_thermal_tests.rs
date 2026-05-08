use voxel_core::{
    chunk::Chunk,
    coordinate::{CHUNK_SIZE, ChunkCoordinate},
    diffusion::{BorderSnapshot, DiffusionContext, DiffusionSolver, thermal::ThermalErosionSolver},
    voxel::VoxelMaterial,
};

const S: usize = CHUNK_SIZE as usize;

fn origin() -> ChunkCoordinate {
    ChunkCoordinate::new(0, 0, 0)
}

fn default_ctx() -> DiffusionContext {
    DiffusionContext {
        dt: 1.0,
        gravity: -9.81,
        rainfall_rate: 0.0,
    }
}

/// Count total solid (non-air) voxels in a chunk.
fn count_solid(chunk: &Chunk) -> usize {
    chunk
        .materials
        .iter()
        .filter(|&&m| m != VoxelMaterial::Air)
        .count()
}

/// Find the topmost solid voxel in column (x, z).
fn column_height(chunk: &Chunk, x: usize, z: usize) -> Option<usize> {
    for y in (0..S).rev() {
        if chunk.materials[x + y * S + z * S * S] != VoxelMaterial::Air {
            return Some(y);
        }
    }
    None
}

// ── Basic behaviour ────────────────────────────────────────────────────────────

#[test]
fn test_steep_column_erodes_downhill() {
    let mut chunk = Chunk::new_filled(VoxelMaterial::Air);

    // Build a tall sand column at (16, 16) and a short one at (17, 16)
    // Height diff = 5, talus_threshold default = 1.5 → should erode
    for y in 0..10_usize {
        chunk.materials[16 + y * S + 16 * S * S] = VoxelMaterial::Sand;
    }
    for y in 0..5_usize {
        chunk.materials[17 + y * S + 16 * S * S] = VoxelMaterial::Sand;
    }

    let before_src = column_height(&chunk, 16, 16).unwrap();
    let before_dst = column_height(&chunk, 17, 16).unwrap();
    assert_eq!(before_src, 9);
    assert_eq!(before_dst, 4);

    let solver = ThermalErosionSolver::default();
    solver.step(&mut chunk, origin(), &BorderSnapshot::new(), &default_ctx());

    let after_src = column_height(&chunk, 16, 16);
    let after_dst = column_height(&chunk, 17, 16);

    // Source should have lost at least one voxel
    assert!(
        after_src.unwrap_or(0) < before_src,
        "steep column should erode: before={} after={:?}",
        before_src,
        after_src
    );
    // Destination should have gained at least one voxel
    assert!(
        after_dst.unwrap_or(0) > before_dst,
        "downhill column should gain material: before={} after={:?}",
        before_dst,
        after_dst
    );
}

#[test]
fn test_flat_surface_is_stable() {
    let mut chunk = Chunk::new_filled(VoxelMaterial::Air);

    // Completely flat sand surface at y=8
    for x in 0..S {
        for z in 0..S {
            for y in 0..=8 {
                chunk.materials[x + y * S + z * S * S] = VoxelMaterial::Sand;
            }
        }
    }

    let before = count_solid(&chunk);

    let solver = ThermalErosionSolver::default();
    solver.step(&mut chunk, origin(), &BorderSnapshot::new(), &default_ctx());

    let after = count_solid(&chunk);

    assert_eq!(
        before, after,
        "flat surface should be stable: before={} after={}",
        before, after
    );
}

#[test]
fn test_material_is_conserved() {
    let mut chunk = Chunk::new_filled(VoxelMaterial::Air);

    // Irregular sand terrain
    let heights = [5, 10, 3, 8, 6, 12, 4, 9, 7, 11];
    for (x, &h) in heights.iter().enumerate() {
        for y in 0..h {
            chunk.materials[x + y * S + 8 * S * S] = VoxelMaterial::Sand;
        }
    }

    let before = count_solid(&chunk);

    let solver = ThermalErosionSolver::default();
    // Run multiple steps
    for _ in 0..5 {
        solver.step(&mut chunk, origin(), &BorderSnapshot::new(), &default_ctx());
    }

    let after = count_solid(&chunk);

    // Material conservation: voxels that fall out the top of the chunk are
    // the only permitted loss. In this test all columns are well below S so
    // nothing should escape.
    assert_eq!(
        before, after,
        "material should be conserved: before={} after={}",
        before, after
    );
}

#[test]
fn test_stone_does_not_erode() {
    let mut chunk = Chunk::new_filled(VoxelMaterial::Air);

    // Steep stone cliff — stone is not in the default erodable list
    for y in 0..15_usize {
        chunk.materials[16 + y * S + 16 * S * S] = VoxelMaterial::Stone;
    }
    for y in 0..3_usize {
        chunk.materials[17 + y * S + 16 * S * S] = VoxelMaterial::Stone;
    }

    let before_src = column_height(&chunk, 16, 16).unwrap();

    let solver = ThermalErosionSolver::default();
    solver.step(&mut chunk, origin(), &BorderSnapshot::new(), &default_ctx());

    let after_src = column_height(&chunk, 16, 16).unwrap();
    assert_eq!(
        before_src, after_src,
        "stone should not erode: before={} after={}",
        before_src, after_src
    );
}

#[test]
fn test_below_threshold_is_stable() {
    let mut chunk = Chunk::new_filled(VoxelMaterial::Air);

    // Height difference of exactly 1 — below default talus_threshold of 1.5
    for y in 0..5_usize {
        chunk.materials[16 + y * S + 16 * S * S] = VoxelMaterial::Sand;
    }
    for y in 0..4_usize {
        chunk.materials[17 + y * S + 16 * S * S] = VoxelMaterial::Sand;
    }

    let before_src = column_height(&chunk, 16, 16).unwrap();
    let before_dst = column_height(&chunk, 17, 16).unwrap();

    let solver = ThermalErosionSolver::default();
    solver.step(&mut chunk, origin(), &BorderSnapshot::new(), &default_ctx());

    assert_eq!(
        column_height(&chunk, 16, 16).unwrap(),
        before_src,
        "height diff below threshold should not erode"
    );
    assert_eq!(
        column_height(&chunk, 17, 16).unwrap(),
        before_dst,
        "destination should not gain material below threshold"
    );
}

#[test]
fn test_result_tracks_cells_modified() {
    let mut chunk = Chunk::new_filled(VoxelMaterial::Air);

    // Steep erodable cliff
    for y in 0..12_usize {
        chunk.materials[8 + y * S + 8 * S * S] = VoxelMaterial::Sand;
    }
    for y in 0..2_usize {
        chunk.materials[9 + y * S + 8 * S * S] = VoxelMaterial::Sand;
    }

    let solver = ThermalErosionSolver::default();
    let result = solver.step(&mut chunk, origin(), &BorderSnapshot::new(), &default_ctx());

    assert!(
        result.cells_modified > 0,
        "should report modified cells, got {}",
        result.cells_modified
    );
    assert!(
        result.material_transferred > 0.0,
        "should report transferred material, got {}",
        result.material_transferred
    );
}

#[test]
fn test_all_air_chunk_no_panic() {
    let mut chunk = Chunk::new_filled(VoxelMaterial::Air);
    let solver = ThermalErosionSolver::default();
    // Should complete without panic or modification
    let result = solver.step(&mut chunk, origin(), &BorderSnapshot::new(), &default_ctx());
    assert_eq!(result.cells_modified, 0);
}

#[test]
fn test_custom_talus_threshold() {
    let mut chunk = Chunk::new_filled(VoxelMaterial::Air);

    // Height diff = 3
    for y in 0..8_usize {
        chunk.materials[16 + y * S + 16 * S * S] = VoxelMaterial::Sand;
    }
    for y in 0..5_usize {
        chunk.materials[17 + y * S + 16 * S * S] = VoxelMaterial::Sand;
    }

    let before_src = column_height(&chunk, 16, 16).unwrap();

    // High threshold — diff of 3 should not trigger erosion
    let stable_solver = ThermalErosionSolver {
        talus_threshold: 5.0,
        erosion_rate: 0.5,
        erodable: &[VoxelMaterial::Sand],
    };
    stable_solver.step(&mut chunk, origin(), &BorderSnapshot::new(), &default_ctx());
    assert_eq!(
        column_height(&chunk, 16, 16).unwrap(),
        before_src,
        "high talus threshold should suppress erosion"
    );

    // Low threshold — same diff should now trigger erosion
    let active_solver = ThermalErosionSolver {
        talus_threshold: 1.5,
        erosion_rate: 0.5,
        erodable: &[VoxelMaterial::Sand],
    };
    active_solver.step(&mut chunk, origin(), &BorderSnapshot::new(), &default_ctx());
    assert!(
        column_height(&chunk, 16, 16).unwrap() < before_src,
        "low talus threshold should allow erosion"
    );
}
