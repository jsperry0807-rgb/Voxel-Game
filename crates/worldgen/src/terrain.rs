use crate::{
    gen_input::GenInput,
    gen_output::{GenOutput, GEN_CHUNK_H, GEN_CHUNK_W},
};
use noise::{NoiseFn, Perlin};

// Block ID constants (wire to your BlockRegistry later)
pub const AIR: u16 = 0;
pub const STONE: u16 = 1;
pub const DIRT: u16 = 2;
pub const GRASS: u16 = 3;
pub const WATER: u16 = 5;

pub fn generate_heights(input: &GenInput) -> Vec<f64> {
    let perlin = Perlin::new(input.seed as u32);
    let w = GEN_CHUNK_W;
    let s = input.climate.terrain_scale;
    let mut heights = vec![0.0f64; w * w];

    for z in 0..w {
        for x in 0..w {
            let wx = input.chunk_pos.x as f64 * w as f64 + x as f64;
            let wz = input.chunk_pos.z as f64 * w as f64 + z as f64;

            // fBm: 3 octaves
            let mut e = 0.0f64;
            e += perlin.get([wx * s, wz * s]) * 1.00;
            e += perlin.get([wx * s * 2.0, wz * s * 2.0]) * 0.50;
            e += perlin.get([wx * s * 4.0, wz * s * 4.0]) * 0.25;
            // Normalise [-1.75, 1.75] → [0, 1]
            heights[z * w + x] = ((e + 1.75) / 3.5).clamp(0.0, 1.0);
        }
    }
    heights
}

pub fn apply_terrain(input: &GenInput, heights: &[f64], out: &mut GenOutput) {
    let w = GEN_CHUNK_W;
    let sea = input.climate.sea_level as usize;

    for z in 0..w {
        for x in 0..w {
            let idx = z * w + x;
            let surface = (heights[idx] * 128.0 + 64.0).floor() as usize;
            let surface = surface.min(GEN_CHUNK_H - 1);
            out.heightmap[idx] = surface as u8;

            for y in 0..GEN_CHUNK_H {
                let block = if y < surface.saturating_sub(4) {
                    STONE
                } else if y < surface {
                    DIRT
                } else if y == surface {
                    if surface >= sea {
                        GRASS
                    } else {
                        DIRT
                    }
                } else if y <= sea && surface < sea {
                    WATER
                } else {
                    AIR
                };
                out.blocks[GenOutput::block_idx(x, y, z)] = block;
            }
        }
    }
}
