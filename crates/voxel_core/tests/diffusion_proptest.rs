use proptest::prelude::*;
use voxel_core::{
    chunk::Chunk,
    coordinate::ChunkCoordinate,
    diffusion::{BorderSnapshot, DiffusionContext, DiffusionSolver, heat::HeatDiffusionSolver},
    voxel::VoxelMaterial,
};

const VOLUME: usize = 32 * 32 * 32;

proptest! {
    #[test]
    fn heat_never_produces_nan(
        temps in prop::collection::vec(0.0f32..100.0f32, VOLUME..=VOLUME)
    ) {
        let mut chunk = Chunk::new_filled(VoxelMaterial::Stone);
        chunk.activate_diffusion();

        for (i, &t) in temps.iter().enumerate() {
            chunk.diffusion.as_mut().unwrap()[i].temperature = t;
        }

        let solver = HeatDiffusionSolver::default();
        let ctx = DiffusionContext { dt: 0.1, gravity: -9.81, rainfall_rate: 0.0 };
        solver.step(&mut chunk, ChunkCoordinate::new(0, 0, 0), &BorderSnapshot::new(), &ctx);

        for s in chunk.diffusion.as_ref().unwrap().iter() {
            prop_assert!(!s.temperature.is_nan(), "NaN produced");
            prop_assert!(s.temperature.is_finite(), "non-finite produced");
            prop_assert!(s.temperature >= 0.0, "negative temperature: {}", s.temperature);
            prop_assert!(s.temperature <= 100.0 + f32::EPSILON, "exceeded max: {}", s.temperature);
        }
    }

    #[test]
    fn heat_uniform_field_is_stable(
        temp in 0.0f32..100.0f32
    ) {
        let mut chunk = Chunk::new_filled(VoxelMaterial::Stone);
        chunk.activate_diffusion();

        for s in chunk.diffusion.as_mut().unwrap().iter_mut() {
            s.temperature = temp;
        }

        let solver = HeatDiffusionSolver::default();
        let ctx = DiffusionContext { dt: 0.1, gravity: -9.81, rainfall_rate: 0.0 };
        solver.step(&mut chunk, ChunkCoordinate::new(0, 0, 0), &BorderSnapshot::new(), &ctx);

        for s in chunk.diffusion.as_ref().unwrap().iter() {
            let drift = (s.temperature - temp).abs();
            prop_assert!(drift < 1e-4, "uniform field drifted by {} from {}", drift, temp);
        }
    }
}
