use crate::chunk::Chunk;
use crate::coordinate::CHUNK_SIZE;
use crate::voxel::VoxelMaterial;
use block_mesh::{
    GreedyQuadsBuffer, MergeVoxel, RIGHT_HANDED_Y_UP_CONFIG, Voxel, VoxelVisibility, greedy_quads,
};
use ndshape::{ConstShape, ConstShape3u32};
use std::collections::HashMap;

// block_mesh requires a 1-voxel padding border → (CHUNK_SIZE + 2)³
type PaddedShape = ConstShape3u32<34, 34, 34>;

#[derive(Default)]
pub struct ChunkMesh {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    /// Face index (0–5) per vertex; all 4 verts in a quad share the same value.
    pub face_indices: Vec<u8>,
    pub material_ids: Vec<u32>,
    pub indices: Vec<u32>,
    pub quad_count: usize,
}

// ── block_mesh voxel adapter ──────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
struct BmVoxel(VoxelMaterial);

impl Voxel for BmVoxel {
    fn get_visibility(&self) -> VoxelVisibility {
        match self.0 {
            VoxelMaterial::Air => VoxelVisibility::Empty,
            VoxelMaterial::Water => VoxelVisibility::Translucent,
            _ => VoxelVisibility::Opaque,
        }
    }
}

impl MergeVoxel for BmVoxel {
    type MergeValue = VoxelMaterial;
    fn merge_value(&self) -> Self::MergeValue {
        self.0
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Run greedy meshing and return one `ChunkMesh` per material present in the chunk.
pub fn generate_meshes_per_material(chunk: &Chunk) -> HashMap<VoxelMaterial, ChunkMesh> {
    let voxels = build_padded_voxels(chunk);

    let mut buffer = GreedyQuadsBuffer::new(voxels.len());
    greedy_quads(
        &voxels,
        &PaddedShape {},
        [0; 3],
        [33; 3],
        &RIGHT_HANDED_Y_UP_CONFIG.faces,
        &mut buffer,
    );

    let mut per_material: HashMap<VoxelMaterial, ChunkMesh> = HashMap::new();

    for (face_idx, (group, face)) in buffer
        .quads
        .groups
        .iter()
        .zip(RIGHT_HANDED_Y_UP_CONFIG.faces.iter())
        .enumerate()
    {
        for quad in group.iter() {
            let mat = voxels[PaddedShape::linearize(quad.minimum) as usize].0;
            if mat == VoxelMaterial::Air {
                continue;
            }

            let mesh = per_material.entry(mat).or_default();
            let idx_base = mesh.vertices.len() as u32;

            // Shift positions from padded space [1..33] back to chunk space [0..32]
            let verts = face
                .quad_mesh_positions(quad, 1.0)
                .map(|[x, y, z]| [x - 1.0, y - 1.0, z - 1.0]);

            mesh.vertices.extend_from_slice(&verts);
            mesh.normals.extend_from_slice(&face.quad_mesh_normals());
            mesh.uvs.extend_from_slice(&face.tex_coords(
                RIGHT_HANDED_Y_UP_CONFIG.u_flip_face,
                true,
                quad,
            ));
            mesh.face_indices.extend([face_idx as u8; 4]);
            mesh.material_ids.extend([mat as u32; 4]);

            // block_mesh corner layout:
            //   2 ── 3
            //   │  ╱ │
            //   0 ── 1
            // Two CCW triangles: (0,1,2) and (1,3,2)
            mesh.indices.extend([
                idx_base,
                idx_base + 1,
                idx_base + 2,
                idx_base + 1,
                idx_base + 3,
                idx_base + 2,
            ]);
            mesh.quad_count += 1;
        }
    }

    per_material
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn build_padded_voxels(chunk: &Chunk) -> Vec<BmVoxel> {
    let mut voxels = vec![BmVoxel(VoxelMaterial::Air); PaddedShape::SIZE as usize];
    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let src = crate::coordinate::VoxelIndex::new(x, y, z).linearize();
                let dst = PaddedShape::linearize([x + 1, y + 1, z + 1]) as usize;
                voxels[dst] = BmVoxel(chunk.materials[src]);
            }
        }
    }
    voxels
}
