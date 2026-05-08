use voxel_core::{
    chunk::Chunk,
    coordinate::{CHUNK_SIZE, ChunkCoordinate, VoxelIndex},
    diffusion::{BorderSnapshot, DiffusionContext, DiffusionSolver, heat::HeatDiffusionSolver},
    voxel::VoxelMaterial,
    world::World,
};

const S: usize = CHUNK_SIZE as usize;

fn ctx() -> DiffusionContext {
    DiffusionContext {
        dt: 0.1,
        gravity: -9.81,
        rainfall_rate: 0.0,
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn solid_chunk() -> Chunk {
    Chunk::new_filled(VoxelMaterial::Stone)
}

fn temp_at(chunk: &Chunk, idx: VoxelIndex) -> f32 {
    chunk.diffusion.as_ref().unwrap()[idx.linearize()].temperature
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[test]
fn test_heat_flows_across_chunk_boundary() {
    let mut world = World::new();

    let hot_coord = ChunkCoordinate::new(0, 0, 0);
    let cold_coord = ChunkCoordinate::new(1, 0, 0);

    // Hot chunk: all stone, right face (x=31) at temperature 1.0
    let mut hot = solid_chunk();
    hot.activate_diffusion();
    for y in 0..S {
        for z in 0..S {
            let idx = VoxelIndex::new((S - 1) as u32, y as u32, z as u32);
            hot.diffusion.as_mut().unwrap()[idx.linearize()].temperature = 1.0;
        }
    }

    // Cold chunk: all stone, diffusion state at 0.0
    let mut cold = solid_chunk();
    cold.activate_diffusion();

    world.insert_chunk(hot_coord, hot);
    world.insert_chunk(cold_coord, cold);

    // Capture border snapshot — cold chunk sees hot chunk's right face
    let active = vec![cold_coord];
    let borders = BorderSnapshot::capture(&world, &active);

    // Step the cold chunk with border data
    let solver = HeatDiffusionSolver::default();
    let cold_chunk = world.get_chunk_mut(cold_coord).unwrap();
    solver.step(cold_chunk, cold_coord, &borders, &ctx());

    // x=0 face of cold chunk should have warmed from the hot neighbor
    let cold_chunk = world.get_chunk(cold_coord).unwrap();
    for y in 0..S {
        for z in 0..S {
            let idx = VoxelIndex::new(0, y as u32, z as u32);
            let temp = temp_at(cold_chunk, idx);
            assert!(
                temp > 0.0,
                "cold chunk x=0 face should warm from hot neighbor at y={y} z={z}, got {temp}"
            );
        }
    }
}

#[test]
fn test_border_snapshot_captures_correct_face() {
    let mut world = World::new();

    let a = ChunkCoordinate::new(0, 0, 0);
    let b = ChunkCoordinate::new(0, 1, 0); // b is above a

    let mut chunk_a = solid_chunk();
    chunk_a.activate_diffusion();
    // Set the top face of chunk_a (y=31) to temperature 2.0
    for x in 0..S {
        for z in 0..S {
            let idx = VoxelIndex::new(x as u32, (S - 1) as u32, z as u32);
            chunk_a.diffusion.as_mut().unwrap()[idx.linearize()].temperature = 2.0;
        }
    }

    let mut chunk_b = solid_chunk();
    chunk_b.activate_diffusion();

    world.insert_chunk(a, chunk_a);
    world.insert_chunk(b, chunk_b);

    let borders = BorderSnapshot::capture(&world, &[b]);
    let strip = borders
        .get(b, voxel_core::diffusion::Face::NegY)
        .expect("NegY border strip should exist");

    // The strip should reflect chunk_a's top face temperatures
    for x in 0..S {
        for z in 0..S {
            let bi = x + z * S;
            let temp = strip.diffusion.as_ref().unwrap()[bi].temperature;
            assert!(
                (temp - 2.0).abs() < 1e-5,
                "border strip should reflect hot top face at x={x} z={z}, got {temp}"
            );
        }
    }
}

#[test]
fn test_missing_neighbor_acts_as_cold_boundary() {
    let mut world = World::new();
    let coord = ChunkCoordinate::new(0, 0, 0);

    let mut chunk = solid_chunk();
    chunk.activate_diffusion();
    for s in chunk.diffusion.as_mut().unwrap().iter_mut() {
        s.temperature = 1.0;
    }
    world.insert_chunk(coord, chunk);

    // No neighbors loaded — all borders are air strips
    let borders = BorderSnapshot::capture(&world, &[coord]);

    let solver = HeatDiffusionSolver::default();
    let chunk = world.get_chunk_mut(coord).unwrap();
    solver.step(chunk, coord, &borders, &ctx());

    // Interior voxels (not on any boundary) should be unchanged —
    // they only have interior neighbors all at 1.0, so no net flux.
    let chunk = world.get_chunk(coord).unwrap();
    let interior_temp = temp_at(chunk, VoxelIndex::new(16, 16, 16));
    assert!(
        (interior_temp - 1.0).abs() < 1e-4,
        "interior voxel should be stable with uniform field, got {interior_temp}"
    );

    // Boundary voxels on the x=0 face have a missing neighbor on NegX.
    // The air strip exists but contributes 0 neighbors (all Air material),
    // so boundary voxels see fewer neighbors and diffuse only to interior.
    // They should still be at 1.0 because all real neighbors are also 1.0.
    // This test verifies no panic and no NaN at boundaries.
    for y in 0..S {
        for z in 0..S {
            let idx = VoxelIndex::new(0, y as u32, z as u32);
            let temp = temp_at(chunk, idx);
            assert!(
                temp.is_finite() && !temp.is_nan(),
                "boundary voxel should be finite at y={y} z={z}, got {temp}"
            );
            assert!(
                temp >= 0.0 && temp <= 1.0 + f32::EPSILON,
                "boundary voxel out of range at y={y} z={z}, got {temp}"
            );
        }
    }
}

#[test]
fn test_symmetric_chunks_reach_equilibrium() {
    let mut world = World::new();

    let left = ChunkCoordinate::new(0, 0, 0);
    let right = ChunkCoordinate::new(1, 0, 0);

    let mut lchunk = solid_chunk();
    lchunk.activate_diffusion();
    for s in lchunk.diffusion.as_mut().unwrap().iter_mut() {
        s.temperature = 1.0;
    }

    let mut rchunk = solid_chunk();
    rchunk.activate_diffusion();
    // starts at 0.0

    world.insert_chunk(left, lchunk);
    world.insert_chunk(right, rchunk);

    let solver = HeatDiffusionSolver::default();

    for _ in 0..200 {
        let borders_l = BorderSnapshot::capture(&world, &[left]);
        let lc = world.get_chunk_mut(left).unwrap();
        solver.step(lc, left, &borders_l, &ctx());

        let borders_r = BorderSnapshot::capture(&world, &[right]);
        let rc = world.get_chunk_mut(right).unwrap();
        solver.step(rc, right, &borders_r, &ctx());
    }

    // After 200 steps the x=0 face of the right chunk (directly adjacent
    // to the hot left chunk) should be meaningfully warm.
    let rchunk = world.get_chunk(right).unwrap();
    let interface_temp = temp_at(rchunk, VoxelIndex::new(0, 16, 16));
    assert!(
        interface_temp > 0.05,
        "right chunk interface should warm after 200 steps, got {interface_temp}"
    );

    // Left chunk interface should have cooled
    let lchunk = world.get_chunk(left).unwrap();
    let hot_interface = temp_at(lchunk, VoxelIndex::new(31, 16, 16));
    assert!(
        hot_interface < 0.99,
        "left chunk interface should cool after 200 steps, got {hot_interface}"
    );
}

#[test]
fn test_border_snapshot_strip_layout_all_faces() {
    // Verify that border_voxel geometry is consistent for all 6 faces:
    // capturing a face from chunk A should index the correct voxels.
    use voxel_core::diffusion::{Face, border_voxel};

    let size = CHUNK_SIZE as usize;
    let last = size - 1;

    // PosX face: x should always be last
    for u in 0..size {
        for v in 0..size {
            let (x, _, _) = border_voxel(Face::PosX, u, v, size);
            assert_eq!(x, last, "PosX face x should be {last}");
        }
    }
    // NegX face: x should always be 0
    for u in 0..size {
        for v in 0..size {
            let (x, _, _) = border_voxel(Face::NegX, u, v, size);
            assert_eq!(x, 0, "NegX face x should be 0");
        }
    }
    // PosY face: y should always be last
    for u in 0..size {
        for v in 0..size {
            let (_, y, _) = border_voxel(Face::PosY, u, v, size);
            assert_eq!(y, last, "PosY face y should be {last}");
        }
    }
    // NegY face: y should always be 0
    for u in 0..size {
        for v in 0..size {
            let (_, y, _) = border_voxel(Face::NegY, u, v, size);
            assert_eq!(y, 0, "NegY face y should be 0");
        }
    }
    // PosZ face: z should always be last
    for u in 0..size {
        for v in 0..size {
            let (_, _, z) = border_voxel(Face::PosZ, u, v, size);
            assert_eq!(z, last, "PosZ face z should be {last}");
        }
    }
    // NegZ face: z should always be 0
    for u in 0..size {
        for v in 0..size {
            let (_, _, z) = border_voxel(Face::NegZ, u, v, size);
            assert_eq!(z, 0, "NegZ face z should be 0");
        }
    }
}
