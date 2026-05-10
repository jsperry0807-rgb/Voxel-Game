use crate::{
    acoustic_props::AcousticProps, block_id::BlockId, mechanical_props::MechanicalProps,
    thermal_props::ThermalProps,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDefinition {
    pub id: BlockId,
    pub name: String,

    // Visual
    pub albedo_rgb: [u8; 3],
    pub emissive_rgb: [u8; 3],

    // Simulation properties
    pub thermal: ThermalProps,
    pub mechanical: MechanicalProps,
    pub acoustic: AcousticProps,

    // Electrical
    pub electrical_conductivity: f32, // in Siemens per meter (S/m)
    pub magnetic_susceptibility: f32, // dimensionless

    // Chemical
    pub chemical_reactivity: Option<Vec<String>>,

    // Geologic / hydrology
    pub rock_type: Option<String>,
    pub permeability: f32, // in Darcy units
    pub porosity: f32,

    // Seimic
    pub seismic_resistance: f32, // in Richter scale units
}
