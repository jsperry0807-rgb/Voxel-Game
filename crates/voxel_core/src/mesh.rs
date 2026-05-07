use crate::chunk::Chunk;
use crate::coordinate::CHUNK_SIZE;
use crate::voxel::VoxelMaterial;
use block_mesh::{
    GreedyQuadsBuffer, MergeVoxel, RIGHT_HANDED_Y_UP_CONFIG, Voxel, VoxelVisibility, greedy_quads,
};
use ndshape::{ConstShape, ConstShape3u32};

// block-mesh needs a 1-voxel padding border, so shape is (CHUNK_SIZE + 2)³
type PaddedShape = ConstShape3u32<34, 34, 34>;

pub struct ChunkMesh {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub material_ids: Vec<u32>,
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

pub fn generate_mesh(chunk: &Chunk) -> ChunkMesh {
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

    let mut mesh = ChunkMesh {
        vertices: Vec::new(),
        normals: Vec::new(),
        uvs: Vec::new(),
        material_ids: Vec::new(),
        indices: Vec::new(),
        quad_count: 0,
    };

    for (group, face) in buffer
        .quads
        .groups
        .iter()
        .zip(RIGHT_HANDED_Y_UP_CONFIG.faces.iter())
    {
        for quad in group.iter() {
            let idx_base = mesh.vertices.len() as u32;
            let verts = face.quad_mesh_positions(quad, 1.0);
            let normal = face.quad_mesh_normals();
            let uvs = face.tex_coords(RIGHT_HANDED_Y_UP_CONFIG.u_flip_face, true, quad);
            let mat = voxels[<PaddedShape as ConstShape<3>>::linearize(quad.minimum) as usize].0;

            mesh.vertices.extend_from_slice(&verts);
            mesh.normals.extend_from_slice(&normal);
            mesh.uvs.extend_from_slice(&uvs);
            mesh.material_ids.extend([mat as u32; 4]);
            mesh.indices
                .extend([0, 1, 2, 0, 2, 3].map(|i| idx_base + i));
            mesh.quad_count += 1
        }
    }

    mesh
}
