# Voxel Game Engine — Architecture Design

**Date:** 2026-05-07
**Approach:** Engine Core + Bevy Integration Layer (Approach B)
**Status:** Validated

## Overview

A Rust voxel game engine with terrain diffusion (hydraulic erosion, thermal erosion, material/heat transfer) as a core feature. Uses a standalone engine library with a Bevy ECS integration layer for rendering, input, and asset management.

## Crate Architecture

```
voxel-engine/
├── Cargo.toml              # workspace root
├── crates/
│   ├── voxel_core/         # engine-agnostic data structures + algorithms
│   │   └── deps: glam, serde, rayon, ndshape, thiserror
│   ├── voxel_bevy/         # Bevy integration layer
│   │   └── deps: bevy, voxel_core, wgpu, block-mesh
│   └── voxel_game/         # game binary
│       └── deps: voxel_bevy, tracing, puffin
```

## Core Data Structures

### Voxel (1-2 bytes)
- `VoxelMaterial`: `#[repr(u16)]` enum — Air, Stone, Dirt, Sand, Grass, Water, etc.
- `VoxelDiffusionState`: water_level, sediment, temperature, velocity — separate SoA array per chunk, Option-wrapped (None for cold chunks)

### Chunk (32³ voxels)
- `materials`: `Box<[VoxelMaterial; 32768]>` — constant size, no heap churn
- `diffusion`: `Option<Box<[VoxelDiffusionState; 32768]>>` — ~300 KB when active
- `dirty`: bool for mesh invalidation
- Indexing: `ndshape::ConstShape3u32<32, 32, 32>`

### World
- `HashMap<ChunkCoordinate, Chunk>` — all loaded chunks
- Dirty queue, active chunk list, settings

## Diffusion System

Pluggable solvers via `DiffusionSolver` trait. Three implementations, phased:

1. **Heat/Material Diffusion** — 3D Jacobi iteration, 6-neighborhood von Neumann stencil, double-buffered per chunk
2. **Thermal Erosion** — Talus angle threshold on surface voxels only
3. **Hydraulic Erosion** — Shallow water model (rainfall → flux → velocity → erosion/deposition → evaporation), runs at lower frequency (4 Hz)

Threading: Separate rayon pools for mesh generation and diffusion. Diffusion writes dirty flags; mesh pool consumes them.

## Meshing

- `block-mesh-rs` for `greedy_quads` algorithm
- Per-chunk mesh generation, uploaded to GPU via staging belt
- LOD: 3 levels based on camera distance, coarsest level merges 2×2×2 voxels

## Bevy Integration

- `VoxelBevyPlugin` registers core types as Bevy resources
- Thin system wrappers call `voxel_core` methods
- Core library has zero Bevy dependencies

## Rendering (wgpu)

- Single vertex format: position, normal, uv, material_id
- Texture array for block textures (256 materials max)
- Optional DX12 backend via wgpu settings
- Wireframe/AABB debug rendering for development

## Serialization

- Save format: bincode + zstd compressed chunk files
- Config: RON for material definitions, TOML for world settings
- Versioned chunk format for forward compatibility

## Audio & Input

- Audio: rodio (high-level playback) on cpal (low-level device access)
- Input: Bevy's winit-backed `Input<T>` with remappable keybinds via TOML

## Profiling Toolchain

- `tracing` — structured logging and span instrumentation
- `puffin` / `puffin_egui` — frame-level profiler overlay
- `wgpu-profiler` — GPU timing queries
- `bevy-inspector-egui` — dev entity inspection
- `bevy_mod_debugdump` — schedule graph export

## Testing Strategy

- Unit tests: diffusion solvers on hand-crafted 3×3×3 grids
- Integration tests: multi-chunk World, N diffusion steps, mass conservation
- Snapshot tests: chunk serialize/deserialize roundtrip
- Property tests: random grids via `proptest`, assert no NaN/Inf, bounds checking
- Benchmarks: `criterion` for greedy meshing and diffusion step throughput

## Potential Pitfalls

| Pitfall | Mitigation |
|---------|------------|
| Cross-chunk diffusion contention | Read-only border snapshots before step |
| Greedy meshing texture seams | Texture array + material_id per face |
| Hydraulic instability with large dt | Clamp dt, sub-step, double-buffer |
| Per-chunk draw call overhead | Batch into single buffer, indirect draw |
| Bevy version churn | Thin integration layer minimizes breakage surface |
