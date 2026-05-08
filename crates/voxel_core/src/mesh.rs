use crate::chunk::Chunk;
use crate::coordinate::CHUNK_SIZE;
use crate::voxel::VoxelMaterial;
use block_mesh::{
    GreedyQuadsBuffer, MergeVoxel, RIGHT_HANDED_Y_UP_CONFIG, Voxel, VoxelVisibility, greedy_quads,
};
use ndshape::{ConstShape, ConstShape3u32};
use std::collections::HashMap;

// block-mesh needs a 1-voxel padding border, so shape is (CHUNK_SIZE + 2)³
type PaddedShape = ConstShape3u32<34, 34, 34>;

/// A mesh for a single material type (one chunk, one material).
#[derive(Default)]
pub struct ChunkMesh {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub material_ids: Vec<u32>, // all identical for this mesh (redundant but kept for compatibility)
    pub indices: Vec<u32>,
    pub quad_count: usize,
}

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

/// Generate a separate mesh for each material that appears in the chunk.
/// Use these meshes with individual texture files (e.g., stone.png, dirt.png).
pub fn generate_meshes_per_material(chunk: &Chunk) -> HashMap<VoxelMaterial, ChunkMesh> {
    // Build padded voxel buffer (air border around the chunk)
    let mut voxels = vec![BmVoxel(VoxelMaterial::Air); PaddedShape::SIZE as usize];
    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let src = crate::coordinate::VoxelIndex::new(x, y, z).linearize();
                let dst = <PaddedShape as ConstShape<3>>::linearize([x + 1, y + 1, z + 1]) as usize;
                voxels[dst] = BmVoxel(chunk.materials[src]);
            }
        }
    }

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

    for (group, face) in buffer
        .quads
        .groups
        .iter()
        .zip(RIGHT_HANDED_Y_UP_CONFIG.faces.iter())
    {
        for quad in group.iter() {
            let mat = voxels[<PaddedShape as ConstShape<3>>::linearize(quad.minimum) as usize].0;
            if mat == VoxelMaterial::Air {
                continue;
            }

            let mesh = per_material.entry(mat).or_insert_with(ChunkMesh::default);

            let idx_base = mesh.vertices.len() as u32;
            let verts = face.quad_mesh_positions(quad, 1.0);
            let normal = face.quad_mesh_normals();
            // UVs map the entire texture (0..1) – perfect for individual files
            let uvs = face.tex_coords(RIGHT_HANDED_Y_UP_CONFIG.u_flip_face, true, quad);

            mesh.vertices.extend_from_slice(&verts);
            mesh.normals.extend_from_slice(&normal);
            mesh.uvs.extend_from_slice(&uvs);
            mesh.material_ids.extend([mat as u32; 4]);
            mesh.indices
                .extend([0, 1, 2, 0, 2, 3].map(|i| idx_base + i));
            mesh.quad_count += 1;
        }
    }

    per_material
}

/// Generate a single combined mesh (all materials) for backwards compatibility.
/// For individual textures you should use `generate_meshes_per_material`.
#[allow(dead_code)]
pub fn generate_mesh(chunk: &Chunk) -> ChunkMesh {
    let per_mat = generate_meshes_per_material(chunk);
    let mut combined = ChunkMesh::default();
    for (_, mesh) in per_mat {
        let offset = combined.vertices.len() as u32;
        combined.vertices.extend(mesh.vertices);
        combined.normals.extend(mesh.normals);
        combined.uvs.extend(mesh.uvs);
        combined.material_ids.extend(mesh.material_ids);
        combined
            .indices
            .extend(mesh.indices.iter().map(|i| i + offset));
        combined.quad_count += mesh.quad_count;
    }
    combined
}
