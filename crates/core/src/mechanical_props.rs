use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct MechanicalProps {
    pub compressive_strength: f32, // MPa
    pub tensile_strength: f32,
    pub shear_strength: f32,
    pub density: f32, // kg/m^3
    pub friction: f32,
}
