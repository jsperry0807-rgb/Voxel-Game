use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct ThermalProps {
    pub thermal_conductivity: f32, // W/(m*K)
    pub specific_heat: f32,        // J/(kg*K)
    pub emissivity: f32,           // dimensionless, between 0 and 1
    pub ignition_temperature: f32, // Celsius
    pub fuel_value: f32,           // MJ/kg
    pub latent_heat_fusion: Option<f32>,
    pub phase_change_temperature: Option<f32>,
}
