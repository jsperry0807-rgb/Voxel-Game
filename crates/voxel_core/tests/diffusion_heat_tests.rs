use voxel_core::{
    chunk::Chunk,
    coordinate::{ChunkCoordinate, VoxelIndex},
    diffusion::{BorderSnapshot, DiffusionContext, DiffusionSolver, heat::HeatDiffusionSolver},
    voxel::VoxelMaterial,
};

fn default_ctx() -> DiffusionContext {
    DiffusionContext {
        dt: 0.1,
        gravity: -9.81,
        rainfall_rate: 0.0,
    }
}

fn origin() -> ChunkCoordinate {
    ChunkCoordinate::new(0, 0, 0)
}

#[test]
fn test_heat_diffuses_to_colder_neighbors() {
    let mut chunk = Chunk::new_filled(VoxelMaterial::Stone);
    chunk.activate_diffusion();

    let center = VoxelIndex::new(16, 16, 16);
    chunk.diffusion.as_mut().unwrap()[center.linearize()].temperature = 1.0;

    let solver = HeatDiffusionSolver::default();
    solver.step(&mut chunk, origin(), &BorderSnapshot::new(), &default_ctx());

    let center_temp = chunk.diffusion.as_ref().unwrap()[center.linearize()].temperature;
    assert!(
        center_temp < 1.0,
        "center should cool after diffusion, got {}",
        center_temp
    );

    let neighbor = VoxelIndex::new(17, 16, 16);
    let neighbor_temp = chunk.diffusion.as_ref().unwrap()[neighbor.linearize()].temperature;
    assert!(
        neighbor_temp > 0.0,
        "neighbor should warm from diffusion, got {}",
        neighbor_temp
    );
}

#[test]
fn test_uniform_temperature_is_stable() {
    let mut chunk = Chunk::new_filled(VoxelMaterial::Stone);
    chunk.activate_diffusion();

    // Set all voxels to the same temperature
    for s in chunk.diffusion.as_mut().unwrap().iter_mut() {
        s.temperature = 0.5;
    }

    let solver = HeatDiffusionSolver::default();
    solver.step(&mut chunk, origin(), &BorderSnapshot::new(), &default_ctx());

    // Uniform field should not change (net delta = 0 everywhere)
    for s in chunk.diffusion.as_ref().unwrap().iter() {
        let diff = (s.temperature - 0.5).abs();
        assert!(
            diff < 1e-5,
            "uniform field should be stable, got {}",
            s.temperature
        );
    }
}

#[test]
fn test_temperature_never_goes_negative() {
    let mut chunk = Chunk::new_filled(VoxelMaterial::Stone);
    chunk.activate_diffusion();
    // All zeros, one step — should stay at zero
    let solver = HeatDiffusionSolver::default();
    solver.step(&mut chunk, origin(), &BorderSnapshot::new(), &default_ctx());

    for s in chunk.diffusion.as_ref().unwrap().iter() {
        assert!(
            s.temperature >= 0.0,
            "temperature went negative: {}",
            s.temperature
        );
    }
}

#[test]
fn test_temperature_never_exceeds_max() {
    let mut chunk = Chunk::new_filled(VoxelMaterial::Stone);
    chunk.activate_diffusion();

    // Set all to max temperature
    let solver = HeatDiffusionSolver::default();
    for s in chunk.diffusion.as_mut().unwrap().iter_mut() {
        s.temperature = solver.max_temperature;
    }

    solver.step(&mut chunk, origin(), &BorderSnapshot::new(), &default_ctx());

    for s in chunk.diffusion.as_ref().unwrap().iter() {
        assert!(
            s.temperature <= solver.max_temperature + f32::EPSILON,
            "temperature exceeded max: {}",
            s.temperature
        );
    }
}

#[test]
fn test_air_voxels_do_not_diffuse() {
    // Air chunk — diffusion state activates but no voxels should change
    let mut chunk = Chunk::new_filled(VoxelMaterial::Air);
    chunk.activate_diffusion();
    chunk.diffusion.as_mut().unwrap()[VoxelIndex::new(16, 16, 16).linearize()].temperature = 1.0;

    let solver = HeatDiffusionSolver::default();
    solver.step(&mut chunk, origin(), &BorderSnapshot::new(), &default_ctx());

    // Air voxels are skipped — center stays at 1.0, neighbors stay at 0.0
    let center_temp =
        chunk.diffusion.as_ref().unwrap()[VoxelIndex::new(16, 16, 16).linearize()].temperature;
    assert_eq!(
        center_temp, 1.0,
        "air voxel should not participate in diffusion"
    );
}

#[test]
fn test_diffusion_result_cells_modified() {
    let mut chunk = Chunk::new_filled(VoxelMaterial::Stone);
    chunk.activate_diffusion();
    chunk.diffusion.as_mut().unwrap()[VoxelIndex::new(1, 1, 1).linearize()].temperature = 1.0;

    let solver = HeatDiffusionSolver::default();
    let result = solver.step(&mut chunk, origin(), &BorderSnapshot::new(), &default_ctx());

    assert!(
        result.cells_modified > 0,
        "at least some cells should be modified"
    );
    assert!(
        result.execution_time_us > 0,
        "execution time should be recorded"
    );
}

#[test]
fn test_no_nan_produced() {
    let mut chunk = Chunk::new_filled(VoxelMaterial::Stone);
    chunk.activate_diffusion();

    // Seed with values that could cause instability
    chunk.diffusion.as_mut().unwrap()[0].temperature = 99.9;
    chunk.diffusion.as_mut().unwrap()[1].temperature = 0.001;

    let solver = HeatDiffusionSolver::default();
    for _ in 0..10 {
        solver.step(&mut chunk, origin(), &BorderSnapshot::new(), &default_ctx());
    }

    for (i, s) in chunk.diffusion.as_ref().unwrap().iter().enumerate() {
        assert!(!s.temperature.is_nan(), "NaN at index {}", i);
        assert!(s.temperature.is_finite(), "non-finite at index {}", i);
    }
}
