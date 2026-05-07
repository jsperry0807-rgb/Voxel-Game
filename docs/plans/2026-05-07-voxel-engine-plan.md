# Voxel Game Engine — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use godmode:task-runner to implement this plan task-by-task.

**Goal:** Build a Rust voxel game engine with chunk-based world, terrain diffusion (heat/thermal/hydraulic erosion), wgpu rendering, and Bevy ECS integration.

**Architecture:** Three-crate workspace — `voxel_core` (pure algorithms, no GPU/ECS deps), `voxel_bevy` (Bevy plugin wrapping core types), `voxel_game` (binary). Diffusion runs on a dedicated rayon thread pool, independent of the render loop. Meshing uses `block-mesh-rs` greedy_quads. Audio via rodio, serialization via bincode+zstd.

**Tech Stack:** Rust 1.85+, Bevy 0.15+, wgpu 0.20+, block-mesh 0.5+, glam 0.29+, rodio 0.21+, rayon, serde, bincode, zstd, ndshape, tracing, puffin

**Total Estimated LoC:** ~8,000–10,000 across all crates

---

## Milestone 1: Core ECS and Chunk Storage

**Goal:** Workspace scaffolded, core types defined, chunk map operational, unit tests passing, CI green.

**Recommended Crates:** `glam`, `serde` (with `derive` feature), `ndshape`, `thiserror`, `rayon`, `smallvec`, `rustc-hash` (or `fxhash`)

**Estimated LoC:** ~1,200

**Idiomatic Rust Patterns:**
- `newtype` wrappers for `ChunkCoordinate` and `VoxelIndex` to prevent mixing (i32, i32, i32) with (u32, u32, u32)
- `#[repr(u16)]` enum for VoxelMaterial — niche optimization lets `Option<VoxelMaterial>` fit in 2 bytes
- Indexing via `std::ops::Index<ChunkCoordinate>` and `IndexMut` on `ChunkMap`
- Derive `Copy` for small value types, `Clone` for everything else
- `thiserror` for error enum hierarchy

### Task 1.1: Initialize workspace and CI

**Files:**
- Create: `Cargo.toml` (workspace)
- Create: `crates/voxel_core/Cargo.toml`
- Create: `crates/voxel_bevy/Cargo.toml`
- Create: `crates/voxel_game/Cargo.toml`
- Create: `.github/workflows/ci.yml`
- Create: `.cargo/config.toml`
- Create: `rust-toolchain.toml`

**Step 1: Create workspace root**

```toml
# Cargo.toml (workspace root)
[workspace]
resolver = "2"
members = ["crates/voxel_core", "crates/voxel_bevy", "crates/voxel_game"]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT OR Apache-2.0"
repository = "https://github.com/user/voxel-engine"

[workspace.dependencies]
glam = { version = "0.29", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
ndshape = "1"
thiserror = "2"
rayon = "1"
smallvec = "1"
rustc-hash = "2"
bincode = "2"
zstd = "0.13"
ron = "0.8"
toml = "0.8"
tracing = "0.1"
puffin = "0.19"
bevy = "0.15"
wgpu = "0.20"
block-mesh = "0.5"
rodio = "0.21"
proptest = "1"
criterion = "0.5"
```

**Step 2: Create rust-toolchain.toml**

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy", "rust-analyzer"]
targets = ["wasm32-unknown-unknown"]
```

**Step 3: Create CI workflow**

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]
env:
  CARGO_TERM_COLOR: always
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --workspace --all-features
      - run: cargo clippy --workspace --all-features -- -D warnings
      - run: cargo fmt --check
```

**Step 4: Create crate manifests**

```toml
# crates/voxel_core/Cargo.toml
[package]
name = "voxel_core"
version.workspace = true
edition.workspace = true

[dependencies]
glam.workspace = true
serde.workspace = true
ndshape.workspace = true
thiserror.workspace = true
rayon.workspace = true
smallvec.workspace = true
rustc-hash.workspace = true
```

```toml
# crates/voxel_bevy/Cargo.toml
[package]
name = "voxel_bevy"
version.workspace = true
edition.workspace = true

[dependencies]
voxel_core = { path = "../voxel_core" }
bevy.workspace = true
wgpu.workspace = true
block-mesh.workspace = true
```

```toml
# crates/voxel_game/Cargo.toml
[package]
name = "voxel_game"
version.workspace = true
edition.workspace = true

[dependencies]
voxel_bevy = { path = "../voxel_bevy" }
tracing.workspace = true
puffin.workspace = true
```

**Step 5: Verify and commit**

Run: `cargo check --workspace`
Expected: All crates compile (empty lib.rs files)

```bash
git init && git add -A && git commit -m "feat: initialize workspace with voxel_core, voxel_bevy, voxel_game crates and CI"
```

---

### Task 1.2: Implement VoxelMaterial and VoxelProperties

**Files:**
- Create: `crates/voxel_core/src/lib.rs`
- Create: `crates/voxel_core/src/voxel.rs`
- Create: `crates/voxel_core/src/voxel_test.rs` (or `tests/voxel_tests.rs`)

**Step 1: Write the test**

```rust
// crates/voxel_core/tests/voxel_tests.rs
use voxel_core::voxel::VoxelMaterial;

#[test]
fn test_air_is_default() {
    assert_eq!(VoxelMaterial::default(), VoxelMaterial::Air);
}

#[test]
fn test_option_size() {
    // Niche optimization: Option<VoxelMaterial> should be 2 bytes
    assert_eq!(size_of::<Option<VoxelMaterial>>(), 2);
}

#[test]
fn test_material_is_solid() {
    assert!(!VoxelMaterial::Air.is_solid());
    assert!(VoxelMaterial::Stone.is_solid());
    assert!(!VoxelMaterial::Water.is_solid());
}

#[test]
fn test_serialize_roundtrip() {
    let mat = VoxelMaterial::Grass;
    let json = serde_json::to_string(&mat).unwrap();
    let back: VoxelMaterial = serde_json::from_str(&json).unwrap();
    assert_eq!(mat, back);
}
```

**Step 2: Run test to verify failure**

Run: `cargo test -p voxel_core`
Expected: FAIL — module not found

**Step 3: Implement**

```rust
// crates/voxel_core/src/voxel.rs
use serde::{Deserialize, Serialize};

/// A single voxel's material identity.
/// #[repr(u16)] gives niche optimization for Option<VoxelMaterial>.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[repr(u16)]
pub enum VoxelMaterial {
    #[default]
    Air = 0,
    Stone = 1,
    Dirt = 2,
    Sand = 3,
    Grass = 4,
    Water = 5,
}

impl VoxelMaterial {
    /// Whether entities collide with this material.
    #[inline]
    pub fn is_solid(self) -> bool {
        !matches!(self, Self::Air | Self::Water)
    }

    /// Whether this material is a fluid (affected by flow simulation).
    #[inline]
    pub fn is_fluid(self) -> bool {
        matches!(self, Self::Water)
    }

    /// Whether this material is opaque (blocks light / neighbor faces).
    #[inline]
    pub fn is_opaque(self) -> bool {
        matches!(self, Self::Stone | Self::Dirt | Self::Sand | Self::Grass)
    }
}
```

```rust
// crates/voxel_core/src/lib.rs
pub mod voxel;
```

**Step 4: Verify**

Run: `cargo test -p voxel_core`
Expected: All 4 tests PASS

Run: `cargo clippy -p voxel_core -- -D warnings`
Expected: No warnings

**Step 5: Commit**

```bash
git add crates/voxel_core/
git commit -m "feat(voxel_core): add VoxelMaterial enum with solid/fluid/opaque queries"
```

---

### Task 1.3: Implement ChunkCoordinate and VoxelIndex

**Files:**
- Create: `crates/voxel_core/src/coordinate.rs`
- Create: `crates/voxel_core/tests/coordinate_tests.rs`

**Step 1: Write tests**

```rust
// crates/voxel_core/tests/coordinate_tests.rs
use voxel_core::coordinate::{ChunkCoordinate, VoxelIndex, CHUNK_SIZE};

#[test]
fn test_voxel_index_linearize() {
    let idx = VoxelIndex::new(1, 2, 3);
    assert_eq!(idx.linearize(), 1 + 2 * CHUNK_SIZE + 3 * CHUNK_SIZE * CHUNK_SIZE);
}

#[test]
fn test_chunk_coordinate_neighbors() {
    let center = ChunkCoordinate::new(0, 0, 0);
    let neighbors = center.face_neighbors();
    assert_eq!(neighbors.len(), 6);
    assert!(neighbors.contains(&ChunkCoordinate::new(1, 0, 0)));
    assert!(neighbors.contains(&ChunkCoordinate::new(-1, 0, 0)));
}

#[test]
fn test_voxel_to_world_position() {
    let chunk = ChunkCoordinate::new(1, 0, -1);
    let voxel = VoxelIndex::new(15, 0, 31);
    let pos = chunk.voxel_world_position(voxel);
    assert_eq!(pos.x, (1 * CHUNK_SIZE as i32 + 15) as f32);
    assert_eq!(pos.y, 0.0);
    assert_eq!(pos.z, (-1 * CHUNK_SIZE as i32 + 31) as f32);
}
```

**Step 2: Implement**

```rust
// crates/voxel_core/src/coordinate.rs
use glam::IVec3;

pub const CHUNK_SIZE: u32 = 32;
const CHUNK_SIZE_I: i32 = CHUNK_SIZE as i32;
const CHUNK_SIZE_U: usize = CHUNK_SIZE as usize;

/// Identifies a single chunk in world space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkCoordinate(pub IVec3);

impl ChunkCoordinate {
    #[inline]
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self(IVec3::new(x, y, z))
    }

    /// The 6 face-adjacent chunk coordinates.
    pub fn face_neighbors(self) -> [Self; 6] {
        [
            Self(self.0 + IVec3::X),
            Self(self.0 - IVec3::X),
            Self(self.0 + IVec3::Y),
            Self(self.0 - IVec3::Y),
            Self(self.0 + IVec3::Z),
            Self(self.0 - IVec3::Z),
        ]
    }

    /// World-space position of the chunk's minimum corner.
    #[inline]
    pub fn world_min(self) -> glam::Vec3 {
        self.0.as_vec3() * CHUNK_SIZE as f32
    }

    /// World-space position of a voxel within this chunk.
    #[inline]
    pub fn voxel_world_position(self, voxel: VoxelIndex) -> glam::Vec3 {
        self.world_min() + glam::vec3(voxel.x as f32, voxel.y as f32, voxel.z as f32)
    }
}

/// A voxel's position within a single chunk (0..CHUNK_SIZE).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoxelIndex {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl VoxelIndex {
    #[inline]
    pub fn new(x: u32, y: u32, z: u32) -> Self {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE);
        Self { x, y, z }
    }

    /// Flatten 3D index to 1D for array storage.
    #[inline]
    pub fn linearize(self) -> usize {
        self.x as usize
            + self.y as usize * CHUNK_SIZE_U
            + self.z as usize * CHUNK_SIZE_U * CHUNK_SIZE_U
    }

    /// Recover 3D index from linear index.
    #[inline]
    pub fn from_linear(idx: usize) -> Self {
        let x = (idx % CHUNK_SIZE_U) as u32;
        let y = ((idx / CHUNK_SIZE_U) % CHUNK_SIZE_U) as u32;
        let z = (idx / (CHUNK_SIZE_U * CHUNK_SIZE_U)) as u32;
        Self { x, y, z }
    }

    /// Iterate over all voxel positions in a chunk, row-major.
    pub fn iter_all() -> impl Iterator<Item = Self> {
        (0..CHUNK_SIZE_U * CHUNK_SIZE_U * CHUNK_SIZE_U).map(Self::from_linear)
    }
}
```

**Step 3: Verify and commit**

Run: `cargo test -p voxel_core`
Expected: All tests PASS

```bash
git add crates/voxel_core/
git commit -m "feat(voxel_core): add ChunkCoordinate, VoxelIndex, and world-space math"
```

---

### Task 1.4: Implement Chunk and ChunkMap

**Files:**
- Create: `crates/voxel_core/src/chunk.rs`
- Create: `crates/voxel_core/src/world.rs`
- Create: `crates/voxel_core/tests/chunk_tests.rs`

**Step 1: Write tests**

```rust
// crates/voxel_core/tests/chunk_tests.rs
use voxel_core::chunk::Chunk;
use voxel_core::coordinate::{ChunkCoordinate, VoxelIndex};
use voxel_core::voxel::VoxelMaterial;
use voxel_core::world::World;

#[test]
fn test_new_chunk_is_all_air() {
    let chunk = Chunk::new_filled(VoxelMaterial::Air);
    for idx in VoxelIndex::iter_all() {
        assert_eq!(chunk.get(idx), VoxelMaterial::Air);
    }
}

#[test]
fn test_chunk_set_and_get() {
    let mut chunk = Chunk::new_filled(VoxelMaterial::Air);
    let idx = VoxelIndex::new(10, 5, 20);
    chunk.set(idx, VoxelMaterial::Stone);
    assert_eq!(chunk.get(idx), VoxelMaterial::Stone);
    assert!(chunk.dirty);
}

#[test]
fn test_world_insert_and_retrieve_chunk() {
    let mut world = World::new();
    let coord = ChunkCoordinate::new(0, 0, 0);
    world.insert_chunk(coord, Chunk::new_filled(VoxelMaterial::Stone));
    assert!(world.get_chunk(coord).is_some());
}

#[test]
fn test_chunk_capacity() {
    let chunk = Chunk::new_filled(VoxelMaterial::Air);
    assert_eq!(chunk.materials.len(), 32 * 32 * 32);
}

#[test]
fn test_dirty_flag_cleared() {
    let mut chunk = Chunk::new_filled(VoxelMaterial::Air);
    chunk.set(VoxelIndex::new(0, 0, 0), VoxelMaterial::Stone);
    chunk.clear_dirty();
    assert!(!chunk.dirty);
}
```

**Step 2: Implement**

```rust
// crates/voxel_core/src/chunk.rs
use crate::coordinate::{VoxelIndex, CHUNK_SIZE};
use crate::voxel::VoxelMaterial;

const VOXEL_COUNT: usize = (CHUNK_SIZE as usize).pow(3);

/// A single chunk in the voxel world — 32³ voxels.
pub struct Chunk {
    /// Linear array of material IDs, row-major (x → y → z).
    pub materials: Box<[VoxelMaterial]>,
    /// Diffusion state is None for cold/inactive chunks.
    pub diffusion: Option<Box<[VoxelDiffusionState]>>,
    /// Set when any voxel changes; cleared after mesh is regenerated.
    pub dirty: bool,
}

impl Chunk {
    pub fn new_filled(fill: VoxelMaterial) -> Self {
        Self {
            materials: vec![fill; VOXEL_COUNT].into_boxed_slice(),
            diffusion: None,
            dirty: true,
        }
    }

    #[inline]
    pub fn get(&self, idx: VoxelIndex) -> VoxelMaterial {
        self.materials[idx.linearize()]
    }

    #[inline]
    pub fn set(&mut self, idx: VoxelIndex, material: VoxelMaterial) {
        let i = idx.linearize();
        if self.materials[i] != material {
            self.materials[i] = material;
            self.dirty = true;
        }
    }

    #[inline]
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Activate diffusion state for this chunk.
    pub fn activate_diffusion(&mut self) {
        if self.diffusion.is_none() {
            self.diffusion = Some(vec![VoxelDiffusionState::default(); VOXEL_COUNT].into_boxed_slice());
        }
    }

    /// Deactivate diffusion, freeing ~300 KB per chunk.
    pub fn deactivate_diffusion(&mut self) {
        self.diffusion = None;
    }
}
```

```rust
// Add VoxelDiffusionState to voxel.rs
#[derive(Debug, Clone, Copy, Default)]
pub struct VoxelDiffusionState {
    pub water_level: f32,
    pub sediment: f32,
    pub temperature: f32,
    pub velocity: [f32; 2],
}
```

```rust
// crates/voxel_core/src/world.rs
use rustc_hash::FxHashMap;
use crate::chunk::Chunk;
use crate::coordinate::ChunkCoordinate;

pub struct World {
    pub chunks: FxHashMap<ChunkCoordinate, Chunk>,
    pub settings: WorldSettings,
    pub dirty_queue: Vec<ChunkCoordinate>,
}

impl World {
    pub fn new() -> Self {
        Self {
            chunks: FxHashMap::default(),
            settings: WorldSettings::default(),
            dirty_queue: Vec::new(),
        }
    }

    pub fn insert_chunk(&mut self, coord: ChunkCoordinate, mut chunk: Chunk) {
        chunk.dirty = true;
        self.dirty_queue.push(coord);
        self.chunks.insert(coord, chunk);
    }

    pub fn get_chunk(&self, coord: ChunkCoordinate) -> Option<&Chunk> {
        self.chunks.get(&coord)
    }

    pub fn get_chunk_mut(&mut self, coord: ChunkCoordinate) -> Option<&mut Chunk> {
        self.chunks.get_mut(&coord)
    }

    pub fn remove_chunk(&mut self, coord: ChunkCoordinate) -> Option<Chunk> {
        self.dirty_queue.retain(|&c| c != coord);
        self.chunks.remove(&coord)
    }

    pub fn drain_dirty(&mut self) -> Vec<ChunkCoordinate> {
        std::mem::take(&mut self.dirty_queue)
    }
}

#[derive(Debug, Clone)]
pub struct WorldSettings {
    pub seed: u64,
    pub load_radius: u32,
    pub gravity: f32,
}

impl Default for WorldSettings {
    fn default() -> Self {
        Self { seed: 0, load_radius: 8, gravity: -9.81 }
    }
}
```

**Step 3: Verify and commit**

Run: `cargo test -p voxel_core`
Expected: All 13 tests PASS (8 prior + 5 new)

Run: `cargo bench -p voxel_core --no-run 2>&1` (benchmarks don't exist yet, just verify no linker errors)

```bash
git add crates/voxel_core/
git commit -m "feat(voxel_core): add Chunk, World, ChunkMap with FxHashMap storage"
```

---

## Milestone 2: Basic Meshing

**Goal:** Voxel world renders on screen. Chunks produce optimized triangle meshes. Bevy plugin scaffolding exists.

**Recommended Crates:** `block-mesh`, `bevy` (full), `wgpu`

**Estimated LoC:** ~1,500

**Idiomatic Rust Patterns:**
- `From<&Chunk>` impl for mesh generation — clear ownership boundary
- Builder pattern for `MeshConfig` (LOD level, include_boundary_faces)
- RAII for GPU resources via Bevy's `RenderAsset` / `Extracted` pattern
- Use const generics for CHUNK_SIZE to enable compiler optimizations

### Task 2.1: Implement mesh generation in voxel_core

**Files:**
- Create: `crates/voxel_core/src/mesh.rs`
- Create: `crates/voxel_core/tests/mesh_tests.rs`

**Step 1: Write test**

```rust
// crates/voxel_core/tests/mesh_tests.rs
use voxel_core::chunk::Chunk;
use voxel_core::coordinate::VoxelIndex;
use voxel_core::mesh::generate_mesh;
use voxel_core::voxel::VoxelMaterial;

#[test]
fn test_air_chunk_has_no_faces() {
    let chunk = Chunk::new_filled(VoxelMaterial::Air);
    let mesh = generate_mesh(&chunk, None);
    assert!(mesh.indices.is_empty());
    assert!(mesh.vertices.is_empty());
}

#[test]
fn test_solid_block_has_visible_faces() {
    let mut chunk = Chunk::new_filled(VoxelMaterial::Air);
    chunk.set(VoxelIndex::new(16, 16, 16), VoxelMaterial::Stone);
    let mesh = generate_mesh(&chunk, None);
    // One isolated block surrounded by air → all 6 faces visible
    assert_eq!(mesh.indices.len(), 36); // 6 faces × 6 indices per face
}

#[test]
fn test_adjacent_blocks_cull_faces() {
    let mut chunk = Chunk::new_filled(VoxelMaterial::Stone);
    // All faces between blocks are culled, only chunk boundary faces remain
    let mesh = generate_mesh(&chunk, None);
    // Boundary-only: 32×32 faces × 6 sides (but greedy meshing merges)
    assert!(mesh.indices.len() < 36 * 32 * 32 * 32); // sanity: much less than naive
}
```

**Step 2: Implement mesh type and generation**

```rust
// crates/voxel_core/src/mesh.rs
use glam::Vec3;
use crate::chunk::Chunk;
use crate::coordinate::{VoxelIndex, CHUNK_SIZE};
use crate::voxel::VoxelMaterial;

/// An indexed triangle mesh for a single chunk.
pub struct ChunkMesh {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub material_ids: Vec<u32>,
    pub indices: Vec<u32>,
    /// Number of quads (for debugging / stats)
    pub quad_count: usize,
}

/// Generate an optimized mesh for a chunk.
/// `neighbor_borders` provides 1-voxel border strips from face-adjacent chunks
/// for correct face culling across chunk boundaries. None = treat boundaries as visible.
pub fn generate_mesh(chunk: &Chunk, neighbor_borders: Option<&NeighborBorders>) -> ChunkMesh {
    let mut mesh = ChunkMesh {
        vertices: Vec::new(),
        normals: Vec::new(),
        uvs: Vec::new(),
        material_ids: Vec::new(),
        indices: Vec::new(),
        quad_count: 0,
    };

    let voxels_with_borders = VoxelGridWithBorders {
        chunk,
        borders: neighbor_borders,
    };

    // Use block-mesh greedy_quads algorithm
    let quads = block_mesh::greedy_quads(
        &voxels_with_borders,
        &block_mesh::GreedyQuadsConfig::default(),
    );

    for quad in &quads {
        push_quad(&mut mesh, quad);
    }

    mesh.quad_count = quads.len();
    mesh
}

fn push_quad(mesh: &mut ChunkMesh, quad: &block_mesh::Quad) {
    // Convert block-mesh Quad to triangles
    // ... vertex pushing logic ...
}
```

**Step 3: Verify and commit**

```bash
git add crates/voxel_core/
git commit -m "feat(voxel_core): add ChunkMesh and greedy_quads mesh generation"
```

---

### Task 2.2: Bevy plugin scaffolding

**Files:**
- Create: `crates/voxel_bevy/src/lib.rs`
- Create: `crates/voxel_bevy/src/render.rs`
- Create: `crates/voxel_bevy/src/plugin.rs`

**Implement:**

```rust
// crates/voxel_bevy/src/lib.rs
mod plugin;
mod render;
mod input;
mod audio;
mod assets;

pub use plugin::VoxelBevyPlugin;
```

```rust
// crates/voxel_bevy/src/plugin.rs
use bevy::prelude::*;
use voxel_core::world::World;

pub struct VoxelBevyPlugin;

impl Plugin for VoxelBevyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(World::new())
           .insert_resource(MeshCache::default())
           .add_systems(Update, (
               process_dirty_chunks,
               upload_generated_meshes,
           ));
    }
}

#[derive(Resource, Default)]
pub struct MeshCache {
    pending: Vec<(ChunkCoordinate, ChunkMesh)>,
}
```

**Verify:** `cargo check -p voxel_bevy`
Expected: Compiles (warnings OK about unused functions at this stage)

---

### Task 2.3: First render — camera + a test chunk on screen

**Files:**
- Create: `crates/voxel_game/src/main.rs`
- Modify: `crates/voxel_bevy/src/render.rs`

Implement a minimal Bevy app that:
1. Spawns a `Camera3d` looking at a 3×3×3 solid stone chunk
2. Generates the mesh via `voxel_core::mesh::generate_mesh`
3. Uploads vertex/index buffers to wgpu via Bevy's `Mesh` asset
4. Renders with a basic PBR material

**Step 1: Write the game entry point**

```rust
// crates/voxel_game/src/main.rs
use bevy::prelude::*;
use voxel_bevy::VoxelBevyPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(VoxelBevyPlugin)
        .add_systems(Startup, setup_camera)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera3d::default());
}
```

**Step 2: Verify visually**

Run: `cargo run -p voxel_game`
Expected: Window opens with a 3D view. Chunk renders as a solid cube (if implemented) or empty scene (minimal case).

**Step 3: Commit**

```bash
git add -A && git commit -m "feat(voxel_bevy): add plugin scaffolding and first render pipeline"
```

---

## Milestone 3: Terrain Diffusion

**Goal:** Diffusion solvers run on chunk data, verified by unit and property tests.

**Recommended Crates:** `rayon`, `proptest` (dev), `criterion` (dev)

**Estimated LoC:** ~2,000

**Idiomatic Rust Patterns:**
- Trait-based solver dispatch (`DiffusionSolver` trait) — enables compile-time composition
- Double buffering with `std::mem::swap` — zero-allocation buffer rotation
- `rayon::scope` for cross-chunk parallelism with shared read-only border snapshots
- Associated types for solver-specific configuration
- `#[cfg(test)]` module per solver with exhaustive edge cases

### Task 3.1: Implement DiffusionSolver trait

**Files:**
- Create: `crates/voxel_core/src/diffusion/mod.rs`
- Create: `crates/voxel_core/src/diffusion/config.rs`

```rust
// crates/voxel_core/src/diffusion/mod.rs
use crate::chunk::Chunk;

pub mod heat;
pub mod thermal;
pub mod hydraulic;
mod config;

pub use config::DiffusionConfig;

/// Shared read-only resources available to all solvers.
pub struct DiffusionContext {
    pub dt: f32,
    pub gravity: f32,
    pub rainfall_rate: f32,
}

/// Result of one solver step — used for profiling and validation.
#[derive(Debug, Default)]
pub struct DiffusionResult {
    pub cells_modified: u64,
    pub material_transferred: f32,
    pub execution_time_us: u64,
}

/// A diffusion/erosion algorithm that operates on chunk arrays.
pub trait DiffusionSolver: Send + Sync {
    fn name(&self) -> &'static str;

    fn step(
        &self,
        chunks: &mut [&mut Chunk],
        borders: &BorderSnapshot,
        ctx: &DiffusionContext,
    ) -> DiffusionResult;
}

/// Read-only snapshot of 1-voxel border strips from neighbor chunks.
/// Prevents cross-chunk read/write contention during diffusion steps.
pub struct BorderSnapshot {
    pub data: FxHashMap<ChunkCoordinate, [VoxelMaterial; CHUNK_SIZE * CHUNK_SIZE * 6]>,
}
```

---

### Task 3.2: Implement HeatDiffusion solver (Jacobi iteration)

**Files:**
- Create: `crates/voxel_core/src/diffusion/heat.rs`
- Create: `crates/voxel_core/tests/diffusion_heat_tests.rs`

**Step 1: Write tests**

```rust
// crates/voxel_core/tests/diffusion_heat_tests.rs
use voxel_core::chunk::Chunk;
use voxel_core::coordinate::VoxelIndex;
use voxel_core::diffusion::heat::HeatDiffusionSolver;
use voxel_core::diffusion::{DiffusionContext, DiffusionSolver};
use voxel_core::voxel::{VoxelDiffusionState, VoxelMaterial};

#[test]
fn test_heat_diffuses_to_colder_neighbors() {
    let mut chunk = Chunk::new_filled(VoxelMaterial::Stone);
    chunk.activate_diffusion();

    // Set center voxel hot
    let center = VoxelIndex::new(16, 16, 16);
    chunk.diffusion.as_mut().unwrap()[center.linearize()].temperature = 1.0;

    let solver = HeatDiffusionSolver::default();
    let ctx = DiffusionContext { dt: 0.1, gravity: -9.81, rainfall_rate: 0.0 };
    let borders = BorderSnapshot::default();

    solver.step(&mut [&mut chunk], &borders, &ctx);

    // Center should cool, neighbors should warm
    let center_temp = chunk.diffusion.as_ref().unwrap()[center.linearize()].temperature;
    assert!(center_temp < 1.0, "center should cool (was {}, now {})", 1.0, center_temp);

    let neighbor = voxel_core::coordinate::VoxelIndex::new(17, 16, 16);
    let neighbor_temp = chunk.diffusion.as_ref().unwrap()[neighbor.linearize()].temperature;
    assert!(neighbor_temp > 0.0, "neighbor should warm from 0 (now {})", neighbor_temp);
}

#[test]
fn test_total_energy_conserved() {
    let mut chunk = Chunk::new_filled(VoxelMaterial::Stone);
    chunk.activate_diffusion();

    for i in 0..32 {
        for j in 0..32 {
            chunk.diffusion.as_mut().unwrap()[VoxelIndex::new(i, j, 0).linearize()].temperature = 1.0;
        }
    }
    let initial_energy: f32 = chunk.diffusion.as_ref().unwrap().iter().map(|d| d.temperature).sum();

    let solver = HeatDiffusionSolver::default();
    let ctx = DiffusionContext { dt: 0.01, gravity: -9.81, rainfall_rate: 0.0 };
    let borders = BorderSnapshot::default();

    // After many small steps, energy should be conserved (within boundary loss)
    solver.step(&mut [&mut chunk], &borders, &ctx);
    let final_energy: f32 = chunk.diffusion.as_ref().unwrap().iter().map(|d| d.temperature).sum();

    let ratio = final_energy / initial_energy;
    assert!(ratio > 0.99, "energy should be conserved, ratio = {}", ratio);
}

#[test]
fn test_diffusion_no_nan() {
    let mut chunk = Chunk::new_filled(VoxelMaterial::Stone);
    chunk.activate_diffusion();
    chunk.diffusion.as_mut().unwrap()[VoxelIndex::new(0, 0, 0).linearize()].temperature = f32::INFINITY;

    let solver = HeatDiffusionSolver::default();
    let ctx = DiffusionContext { dt: 0.1, gravity: -9.81, rainfall_rate: 0.0 };
    let borders = BorderSnapshot::default();

    solver.step(&mut [&mut chunk], &borders, &ctx);

    for s in chunk.diffusion.as_ref().unwrap().iter() {
        assert!(!s.temperature.is_nan(), "temperature should never be NaN");
    }
}
```

**Step 2: Implement solver**

```rust
// crates/voxel_core/src/diffusion/heat.rs
use rayon::prelude::*;
use crate::chunk::Chunk;
use crate::coordinate::{VoxelIndex, CHUNK_SIZE};
use crate::diffusion::{BorderSnapshot, DiffusionContext, DiffusionResult, DiffusionSolver};
use crate::voxel::{VoxelDiffusionState, VoxelMaterial};

const CHUNK_SIZE_U: usize = CHUNK_SIZE as usize;
const CHUNK_SIZE_I: i32 = CHUNK_SIZE as i32;

/// Default diffusion coefficient for stone→stone heat transfer.
const DEFAULT_COEFFICIENT: f32 = 0.1;

/// Jacobi iteration heat diffusion over the 6-neighborhood von Neumann stencil.
pub struct HeatDiffusionSolver {
    coefficient: f32,
    /// Scratch buffer for double-buffered iteration.
    scratch: Vec<VoxelDiffusionState>,
}

impl Default for HeatDiffusionSolver {
    fn default() -> Self {
        Self {
            coefficient: DEFAULT_COEFFICIENT,
            scratch: Vec::new(),
        }
    }
}

impl DiffusionSolver for HeatDiffusionSolver {
    fn name(&self) -> &'static str { "heat_diffusion" }

    fn step(
        &self,
        chunks: &mut [&mut Chunk],
        borders: &BorderSnapshot,
        ctx: &DiffusionContext,
    ) -> DiffusionResult {
        let start = std::time::Instant::now();
        let mut result = DiffusionResult::default();

        chunks.par_iter_mut().for_each(|chunk| {
            if let Some(ref diffusion) = chunk.diffusion {
                let mut scratch = diffusion.clone(); // copy current state
                let voxel_count = CHUNK_SIZE_U * CHUNK_SIZE_U * CHUNK_SIZE_U;

                for z in 1..CHUNK_SIZE_I - 1 {
                    for y in 1..CHUNK_SIZE_I - 1 {
                        for x in 1..CHUNK_SIZE_I - 1 {
                            let idx = VoxelIndex::new(x as u32, y as u32, z as u32);
                            let i = idx.linearize();

                            let center_temp = diffusion[i].temperature;

                            // 6-neighbor average (von Neumann)
                            let neighbor_sum: f32 = [
                                VoxelIndex::new(x as u32 + 1, y as u32, z as u32),
                                VoxelIndex::new(x as u32 - 1, y as u32, z as u32),
                                VoxelIndex::new(x as u32, y as u32 + 1, z as u32),
                                VoxelIndex::new(x as u32, y as u32 - 1, z as u32),
                                VoxelIndex::new(x as u32, y as u32, z as u32 + 1),
                                VoxelIndex::new(x as u32, y as u32, z as u32 - 1),
                            ]
                            .iter()
                            .map(|&n| diffusion[n.linearize()].temperature)
                            .sum();

                            let avg_neighbor_temp = neighbor_sum / 6.0;
                            let delta = avg_neighbor_temp - center_temp;
                            let new_temp = center_temp + self.coefficient * delta * ctx.dt;

                            scratch[i].temperature = new_temp.clamp(0.0, 10.0);
                        }
                    }
                }

                // Swap scratch into chunk
                if let Some(ref mut chunk_diff) = chunk.diffusion {
                    chunk_diff.clone_from(&scratch);
                }
            }
        });

        result.execution_time_us = start.elapsed().as_micros() as u64;
        result
    }
}
```

**Step 3: Property tests via proptest**

```rust
// crates/voxel_core/tests/diffusion_proptest.rs
use proptest::prelude::*;
use voxel_core::chunk::Chunk;
use voxel_core::coordinate::VoxelIndex;
use voxel_core::diffusion::heat::HeatDiffusionSolver;
use voxel_core::diffusion::{DiffusionContext, DiffusionSolver, BorderSnapshot};
use voxel_core::voxel::VoxelMaterial;

proptest! {
    #[test]
    fn heat_diffusion_never_produces_nan(
        temps in prop::collection::vec(0.0f32..1.0f32, 0..100)
    ) {
        let mut chunk = Chunk::new_filled(VoxelMaterial::Stone);
        chunk.activate_diffusion();
        for (i, &t) in temps.iter().enumerate() {
            if i < 32 * 32 * 32 {
                chunk.diffusion.as_mut().unwrap()[i].temperature = t;
            }
        }

        let solver = HeatDiffusionSolver::default();
        let ctx = DiffusionContext { dt: 0.1, gravity: -9.81, rainfall_rate: 0.0 };
        let borders = BorderSnapshot::default();
        solver.step(&mut [&mut chunk], &borders, &ctx);

        for s in chunk.diffusion.as_ref().unwrap().iter() {
            prop_assert!(!s.temperature.is_nan());
            prop_assert!(s.temperature.is_finite());
            prop_assert!(s.temperature >= 0.0);
        }
    }
}
```

**Step 4: Verify and commit**

Run: `cargo test -p voxel_core`
Expected: All tests PASS including proptest

```bash
git add crates/voxel_core/
git commit -m "feat(voxel_core): add DiffusionSolver trait and HeatDiffusion Jacobi solver"
```

---

### Task 3.3: Implement ThermalErosion solver

**Files:**
- Create: `crates/voxel_core/src/diffusion/thermal.rs`
- Create: `crates/voxel_core/tests/diffusion_thermal_tests.rs`

```rust
// crates/voxel_core/src/diffusion/thermal.rs
/// Thermal erosion using the talus angle model.
/// For each surface voxel: if slope exceeds talus_threshold,
/// material from the upper cell moves to the lower cell.
pub struct ThermalErosionSolver {
    /// Maximum stable slope angle in radians. Default: ~45 degrees.
    pub talus_threshold: f32,
    /// Rate of material movement once threshold is exceeded.
    pub erosion_rate: f32,
    scratch: Vec<VoxelMaterial>,
}

impl Default for ThermalErosionSolver {
    fn default() -> Self {
        Self {
            talus_threshold: 0.785, // ~45 degrees
            erosion_rate: 0.05,
            scratch: Vec::new(),
        }
    }
}
```

Tests verify:
- Steep cliff erodes (material moves down)
- Flat surface is stable (no movement)
- Material is conserved

---

### Task 3.4: Cross-chunk border snapshot system

**Files:**
- Modify: `crates/voxel_core/src/diffusion/mod.rs`
- Create: `crates/voxel_core/tests/cross_chunk_diffusion_tests.rs`

Implement `BorderSnapshot::capture(world: &World, active: &[ChunkCoordinate])` that extracts 1-voxel border strips. Test that a 2-chunk setup allows heat to flow across the boundary.

---

## Milestone 4: I/O and Save Formats

**Goal:** Chunks save/load to disk, streaming loading by camera distance, config hot-reloads.

**Recommended Crates:** `bincode` (serde), `zstd`, `ron`, `toml`, `walkdir`

**Estimated LoC:** ~1,200

**Idiomatic Rust Patterns:**
- `BufWriter<File>` for chunk writes — batches small structs into OS pages
- `bincode` serialization with `zstd::stream::Encoder` — streaming compression avoids double allocation
- `bevy::tasks::AsyncComputeTaskPool` for background chunk I/O — non-blocking main thread
- `thiserror` for I/O error hierarchy with `#[from]` for transparent `?` usage
- Channel-based async loading: `crossbeam_channel` or Bevy's internal channels

### Task 4.1: Chunk serialization format

```rust
// crates/voxel_core/src/io.rs
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct SavedChunk {
    pub version: u16,
    pub coordinate: (i32, i32, i32),
    pub materials_compressed: Vec<u8>,
    pub diffusion_compressed: Option<Vec<u8>>,
}

pub fn save_chunk(path: &std::path::Path, chunk: &Chunk, coord: ChunkCoordinate) -> io::Result<()> {
    let materials_raw = bytemuck::cast_slice(&chunk.materials);
    let materials_compressed = zstd::encode_all(materials_raw, 3)?;

    let diffusion_compressed = chunk.diffusion.as_ref().map(|d| {
        zstd::encode_all(bytemuck::cast_slice(d), 3).unwrap()
    });

    let saved = SavedChunk {
        version: 1,
        coordinate: (coord.0.x, coord.0.y, coord.0.z),
        materials_compressed,
        diffusion_compressed,
    };

    let mut file = std::fs::File::create(path)?;
    bincode::serialize_into(&mut file, &saved)?;
    Ok(())
}

pub fn load_chunk(path: &std::path::Path) -> io::Result<(Chunk, ChunkCoordinate)> {
    let file = std::fs::File::open(path)?;
    let saved: SavedChunk = bincode::deserialize_from(file)?;

    let materials = zstd::decode_all(&saved.materials_compressed[..])?;
    let materials: Box<[VoxelMaterial]> = bytemuck::cast_vec(materials).into_boxed_slice();

    let diffusion = saved.diffusion_compressed.map(|d| {
        let decompressed = zstd::decode_all(&d[..]).unwrap();
        bytemuck::cast_vec(decompressed).into_boxed_slice()
    });

    let chunk = Chunk { materials, diffusion, dirty: false };
    let coord = ChunkCoordinate::new(saved.coordinate.0, saved.coordinate.1, saved.coordinate.2);

    Ok((chunk, coord))
}
```

---

### Task 4.2: Spiral chunk loader

A system in `voxel_bevy` that:
1. Tracks camera chunk coordinate
2. Maintains a sorted list of desired chunk coordinates (spiral outward, closest first)
3. Compares against loaded set → unload far chunks, enqueue loads for near ones
4. Loads N chunks per frame (cap at 2-3 to avoid frame spikes)

```rust
// crates/voxel_bevy/src/loader.rs
pub fn spiral_loading_radius(center: ChunkCoordinate, radius: u32) -> Vec<ChunkCoordinate> {
    let mut coords = Vec::new();
    for r in 0..=radius as i32 {
        for x in -r..=r {
            for z in -r..=r {
                if x.abs() == r || z.abs() == r {
                    for y in -2..=4 { // vertical range
                        coords.push(ChunkCoordinate::new(
                            center.0.x + x, center.0.y + y, center.0.z + z
                        ));
                    }
                }
            }
        }
    }
    // Sort by distance from center (closest first)
    coords.sort_by_key(|c| c.0.distance_squared(center.0));
    coords
}
```

---

### Task 4.3: Material definition hot-reloading via RON

```rust
// crates/voxel_bevy/src/assets.rs
#[derive(Asset, TypePath, Serialize, Deserialize)]
pub struct MaterialDefinitions {
    pub materials: Vec<MaterialDef>,
}

#[derive(Serialize, Deserialize)]
pub struct MaterialDef {
    pub name: String,
    pub material: VoxelMaterial,
    pub texture_top: u32,
    pub texture_side: u32,
    pub texture_bottom: u32,
    pub hardness: f32,
    pub diffusion_coeff: f32,
}

fn on_material_defs_changed(
    mut events: EventReader<AssetEvent<MaterialDefinitions>>,
    assets: Res<Assets<MaterialDefinitions>>,
    mut world: ResMut<World>,
) {
    for event in events.read() {
        if let AssetEvent::Modified { id } = event {
            if let Some(defs) = assets.get(*id) {
                world.reload_material_definitions(defs);
            }
        }
    }
}
```

---

## Milestone 5: Audio and Input Integration

**Goal:** Sound effects, ambient audio, footstep modulation, and input remapping.

**Recommended Crates:** `rodio`, `cpal` (transitive via rodio)

**Estimated LoC:** ~800

**Idiomatic Rust Patterns:**
- `rodio::OutputStream` held as a Bevy resource — RAII, dropped on app exit
- `rodio::Sink` for looping ambient, `rodio::buffer::SamplesBuffer` for one-shot SFX
- Observer pattern via Bevy events: `BlockBreakEvent` triggers sound
- `HashMap<KeyCode, GameAction>` for rebindable inputs, loaded from TOML

### Task 5.1: Audio engine Bevy resource

```rust
// crates/voxel_bevy/src/audio.rs
use bevy::prelude::*;
use rodio::{OutputStream, OutputStreamHandle, Sink, source::Source};
use std::collections::HashMap;

#[derive(Resource)]
pub struct AudioEngine {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    ambient: Option<Sink>,
    sfx_cache: HashMap<SfxKind, Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SfxKind {
    BlockBreak,
    BlockPlace,
    Footstep(VoxelMaterial),
}

impl AudioEngine {
    pub fn play_sfx(&self, kind: SfxKind) {
        if let Some(data) = self.sfx_cache.get(&kind) {
            let source = rodio::buffer::SamplesBuffer::new(1, 44100, data.clone());
            let _ = self.handle.play_raw(source);
        }
    }
}
```

---

### Task 5.2: Input remapping

```rust
// crates/voxel_bevy/src/input.rs
use bevy::prelude::*;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameAction {
    MoveForward, MoveBack, MoveLeft, MoveRight,
    Jump, Crouch, Sprint,
    BreakBlock, PlaceBlock,
    OpenInventory, Pause,
}

#[derive(Resource, Deserialize)]
pub struct InputBindings {
    pub bindings: HashMap<String, Vec<KeyCode>>,
}
// Loaded from assets/bindings.toml at startup
```

---

## Milestone 6: Optimization and Cross-Platform

**Goal:** LOD rendering, mesh batching, profiler overlay, wasm build, release shipping.

**Recommended Crates:** `puffin`, `puffin_egui`, `wgpu-profiler`, `bevy-inspector-egui`, `wasm-bindgen`

**Estimated LoC:** ~1,500

**Idiomatic Rust Patterns:**
- Feature-gated profiler integration (`#[cfg(feature = "profiling")]`)
- `LazyLock` / `OnceLock` for global profiler state
- Thin wrapper types for platform-specific code behind `#[cfg(target_arch = "wasm32")]`
- `Cargo.toml` profile overrides for `release` (lto = "fat", codegen-units = 1, opt-level = 3)

### Task 6.1: LOD system

Three mesh detail levels:
- **LOD 0** (0-4 chunks): full 32³ greedy mesh
- **LOD 1** (4-8 chunks): 16³ voxels (2×2×2 merged), 1/8 the triangles
- **LOD 2** (8+ chunks): 8³ voxels (4×4×4 merged), 1/64 the triangles

LOD selection based on camera-to-chunk-center distance, recomputed when camera crosses a chunk boundary.

### Task 6.2: Mesh batching

Instead of one draw call per chunk, merge LOD 1 and LOD 2 chunk meshes into "superchunk" buffers (4×4×4 chunks = one draw call). Uses wgpu indirect indexed drawing with per-chunk draw commands in a compute-filled buffer.

### Task 6.3: Profiling integration

```rust
// In voxel_bevy render system, gated behind feature:
#[cfg(feature = "profiling")]
puffin::profile_function!();

// In voxel_game main.rs:
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};

app.add_plugins(LogDiagnosticsPlugin::default())
   .add_plugins(FrameTimeDiagnosticsPlugin);
```

### Task 6.4: Cross-platform packaging

```toml
# .cargo/config.toml
[target.wasm32-unknown-unknown]
runner = "wasm-bindgen-test-runner"

# Release profile
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
strip = true
panic = "abort"

# wasm release
[profile.wasm-release]
inherits = "release"
opt-level = "s"  # optimize for size on web
```

Add `index.html` and wasm build script. Target Windows (native wgpu/DX12), Linux (Vulkan), macOS (Metal via wgpu), and Web (WebGPU/WebGL2 via wasm-bindgen).

---

## Dependency Graph

```
M1 (Core ECS + Storage)
 └── M2 (Meshing) ── requires M1
      └── M3 (Diffusion) ── requires M1 (not M2, parallel development possible)
      └── M4 (I/O) ── requires M1
           └── M5 (Audio + Input) ── depends on M2 (needs rendering for feedback)
                └── M6 (Optimization) ── requires all prior milestones
```

**Parallelizable work:** M3 (diffusion) can begin immediately after M1 is done — it doesn't need meshing to work. The solver test suite validates against raw chunk data. M4 (I/O) is also independent of M2.

---

## Crate Dependency Summary

| Crate | Version | Purpose |
|-------|---------|---------|
| `bevy` | 0.15 | ECS, windowing, asset system, render graph |
| `wgpu` | 0.20 | GPU abstraction, compute shaders, DX12/Vulkan/Metal |
| `block-mesh` | 0.5 | Greedy quads / visible faces voxel meshing |
| `glam` | 0.29 | Vec3, IVec3, Mat4, quaternions |
| `ndshape` | 1 | ConstShape3u32 for chunk array indexing |
| `rayon` | 1 | Parallel iteration over chunks |
| `rodio` | 0.21 | Audio playback (WAV, MP3, FLAC, Vorbis) |
| `serde` | 1 | Serialize/Deserialize derive |
| `bincode` | 2 | Compact binary serialization |
| `zstd` | 0.13 | Compression for chunk saves |
| `ron` | 0.8 | Human-readable asset definitions |
| `toml` | 0.8 | Config files |
| `tracing` | 0.1 | Structured logging |
| `puffin` | 0.19 | Frame profiler |
| `thiserror` | 2 | Error type derivation |
| `rustc-hash` | 2 | FxHashMap for chunk storage |
| `proptest` | 1 (dev) | Property-based testing for diffusion |
| `criterion` | 0.5 (dev) | Microbenchmarks |
| `bytemuck` | 1 | Safe casting for serialization |

---

## Potential Pitfalls (Milestone-Specific)

| Milestone | Pitfall | Mitigation |
|-----------|---------|------------|
| M1 | `FxHashMap` with `ChunkCoordinate` key hashing collisions | `ChunkCoordinate(IVec3)` has good entropy; verify with real-world distribution |
| M2 | Greedy meshing is O(n³) with large constant | Keep CHUNK_SIZE at 32 (not 64); benchmark before changing |
| M2 | First render: wgpu surface configuration mismatch on some GPUs | Use `bevy`'s default surface setup; test on integrated + discrete GPUs early |
| M3 | Diffusion scratch buffer allocation per-frame causes jitter | Pre-allocate scratch in solver struct; reuse across steps |
| M3 | Hydraulic erosion numerical instability at chunk boundaries | Sub-step dt, clamp velocity, use border snapshots |
| M4 | Compressed chunk files corrupt on crash mid-write | Atomic write pattern: write to `.tmp`, then rename |
| M5 | rodio OutputStream drop causes audio pop | Hold stream in resource for app lifetime |
| M6 | wasm build: wgpu WebGPU backend incomplete on some browsers | Fallback to WebGL2; test on Chrome + Firefox + Safari |
| M6 | `lto = "fat"` makes compile times 5-10× slower | Only use in release CI, not dev workflow |
