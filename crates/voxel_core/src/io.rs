use std::{
    fs,
    io::{self, BufWriter},
    path::{Path, PathBuf},
};

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    chunk::Chunk,
    coordinate::ChunkCoordinate,
    voxel::{VoxelDiffusionState, VoxelMaterial},
};

// ── Error type ─────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum ChunkIoError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Serialization error: {0}")]
    Encode(bincode::error::EncodeError),

    #[error("Deserialization error: {0}")]
    Decode(bincode::error::DecodeError),

    #[error("Compression error: {0}")]
    Compress(io::Error),

    #[error("Decompression error: {0}")]
    Decompress(io::Error),

    #[error("Version mismatch: expected {expected}, got {found}")]
    VersionMismatch { expected: u16, found: u16 },

    #[error("Corrupt chunk data: {0}")]
    Corrupt(String),
}

impl From<bincode::error::EncodeError> for ChunkIoError {
    fn from(e: bincode::error::EncodeError) -> Self {
        Self::Encode(e)
    }
}

impl From<bincode::error::DecodeError> for ChunkIoError {
    fn from(e: bincode::error::DecodeError) -> Self {
        Self::Decode(e)
    }
}

// ── On-disk format ─────────────────────────────────────────────────────────────

const CHUNK_FORMAT_VERSION: u16 = 1;

#[derive(Serialize, Deserialize)]
struct SavedChunk {
    version: u16,
    coord: (i32, i32, i32),
    materials_compressed: Vec<u8>,
    diffusion_compressed: Option<Vec<u8>>,
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct DiffusionStateRaw {
    water_level: f32,
    sediment: f32,
    temperature: f32,
    velocity_x: f32,
    velocity_y: f32,
    _pad: f32,
}

impl From<VoxelDiffusionState> for DiffusionStateRaw {
    fn from(s: VoxelDiffusionState) -> Self {
        Self {
            water_level: s.water_level,
            sediment: s.sediment,
            temperature: s.temperature,
            velocity_x: s.velocity[0],
            velocity_y: s.velocity[1],
            _pad: 0.0,
        }
    }
}

impl From<DiffusionStateRaw> for VoxelDiffusionState {
    fn from(r: DiffusionStateRaw) -> Self {
        Self {
            water_level: r.water_level,
            sediment: r.sediment,
            temperature: r.temperature,
            velocity: [r.velocity_x, r.velocity_y],
        }
    }
}

// ── Path helpers ───────────────────────────────────────────────────────────────

pub fn chunk_path(world_dir: &Path, coord: ChunkCoordinate) -> PathBuf {
    world_dir
        .join("chunks")
        .join(format!("r_{}_{}_{}.chunk", coord.0.x, coord.0.y, coord.0.z))
}

// ── Save ───────────────────────────────────────────────────────────────────────

pub fn save_chunk(path: &Path, chunk: &Chunk, coord: ChunkCoordinate) -> Result<(), ChunkIoError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Serialize materials as u16 bytes then compress
    let materials_u16: Vec<u16> = chunk.materials.iter().map(|&m| m as u16).collect();
    let materials_bytes = bytemuck::cast_slice::<u16, u8>(&materials_u16);
    let materials_compressed =
        zstd::encode_all(materials_bytes, 3).map_err(ChunkIoError::Compress)?;

    // Serialize diffusion state if present
    let diffusion_compressed = chunk
        .diffusion
        .as_ref()
        .map(|diff| -> Result<Vec<u8>, ChunkIoError> {
            let raw: Vec<DiffusionStateRaw> =
                diff.iter().copied().map(DiffusionStateRaw::from).collect();
            let bytes = bytemuck::cast_slice::<DiffusionStateRaw, u8>(&raw);
            zstd::encode_all(bytes, 3).map_err(ChunkIoError::Compress)
        })
        .transpose()?;

    let saved = SavedChunk {
        version: CHUNK_FORMAT_VERSION,
        coord: (coord.0.x, coord.0.y, coord.0.z),
        materials_compressed,
        diffusion_compressed,
    };

    // Atomic write: .tmp then rename
    let tmp_path = path.with_extension("chunk.tmp");
    {
        let file = fs::File::create(&tmp_path)?;
        let mut writer = BufWriter::new(file);
        bincode::serde::encode_into_std_write(&saved, &mut writer, bincode::config::standard())?;
    }
    fs::rename(&tmp_path, path)?;

    Ok(())
}

// ── Load ───────────────────────────────────────────────────────────────────────

pub fn load_chunk(path: &Path) -> Result<(Chunk, ChunkCoordinate), ChunkIoError> {
    let file = fs::File::open(path)?;
    let mut reader = io::BufReader::new(file);

    let saved: SavedChunk =
        bincode::serde::decode_from_std_read(&mut reader, bincode::config::standard())?;

    if saved.version != CHUNK_FORMAT_VERSION {
        return Err(ChunkIoError::VersionMismatch {
            expected: CHUNK_FORMAT_VERSION,
            found: saved.version,
        });
    }

    // Decompress and deserialize materials
    let mat_bytes = zstd::decode_all(saved.materials_compressed.as_slice())
        .map_err(ChunkIoError::Decompress)?;

    if mat_bytes.len() % 2 != 0 {
        return Err(ChunkIoError::Corrupt(
            "material byte length is not a multiple of 2".into(),
        ));
    }
    let mat_u16: &[u16] = bytemuck::cast_slice(&mat_bytes);
    let materials: Box<[VoxelMaterial]> = mat_u16
        .iter()
        .map(|&v| u16_to_material(v))
        .collect::<Result<Vec<_>, _>>()
        .map_err(ChunkIoError::Corrupt)?
        .into_boxed_slice();

    // Decompress and deserialize diffusion state
    let diffusion = saved
        .diffusion_compressed
        .map(|blob| -> Result<Box<[VoxelDiffusionState]>, ChunkIoError> {
            let bytes = zstd::decode_all(blob.as_slice()).map_err(ChunkIoError::Decompress)?;

            let raw_size = std::mem::size_of::<DiffusionStateRaw>();
            if bytes.len() % raw_size != 0 {
                return Err(ChunkIoError::Corrupt(
                    "diffusion byte length not aligned".into(),
                ));
            }

            let raw: &[DiffusionStateRaw] = bytemuck::cast_slice(&bytes);
            let states: Box<[VoxelDiffusionState]> =
                raw.iter().copied().map(VoxelDiffusionState::from).collect();
            Ok(states)
        })
        .transpose()?;

    let coord = ChunkCoordinate::new(saved.coord.0, saved.coord.1, saved.coord.2);
    let chunk = Chunk {
        materials,
        diffusion,
        dirty: false,
    };

    Ok((chunk, coord))
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn u16_to_material(v: u16) -> Result<VoxelMaterial, String> {
    match v {
        0 => Ok(VoxelMaterial::Air),
        1 => Ok(VoxelMaterial::Stone),
        2 => Ok(VoxelMaterial::Dirt),
        3 => Ok(VoxelMaterial::Sand),
        4 => Ok(VoxelMaterial::Grass),
        5 => Ok(VoxelMaterial::Water),
        _ => Err(format!("unknown material id {v}")),
    }
}
