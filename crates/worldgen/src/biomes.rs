use crate::gen_input::GenInput;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Biome {
    Ocean,
    Beach,
    Plains,
    Forest,
    Mountains,
    SnowyPeaks,
    Desert,
}

pub fn classify_biome(temperature: f32, moisture: f32) -> Biome {
    match (temperature, moisture) {
        (t, _) if t < 0.2 => Biome::SnowyPeaks,
        (t, m) if t >= 0.8 && m < 0.3 => Biome::Desert,
        (_, m) if m >= 0.6 => Biome::Forest,
        _ => Biome::Plains,
    }
}

/// Stub — returns a flat biome grid. Wire noise in Tier 2.
pub fn generate_biomes(_input: &GenInput, width: usize) -> Vec<Biome> {
    vec![Biome::Plains; width * width]
}
