#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum VoxelMaterial {
    #[default]
    Air = 0,
    Stone,
    Dirt,
    Sand,
    Grass,
    Water,
}

impl VoxelMaterial {
    #[inline]
    pub fn is_solid(self) -> bool {
        !matches!(self, Self::Air | Self::Water)
    }

    #[inline]
    pub fn is_fluid(self) -> bool {
        matches!(self, Self::Water)
    }

    #[inline]
    pub fn is_opaque(self) -> bool {
        matches!(self, Self::Stone | Self::Dirt | Self::Sand | Self::Grass)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct VoxelDiffusionState {
    pub water_level: f32,
    pub sediment: f32,
    pub temperature: f32,
    pub velocity: [f32; 2],
}
