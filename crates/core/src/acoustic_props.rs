use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct AcousticProps {
    pub absorption_low: f32,
    pub absorption_mid: f32,
    pub absorption_high: f32,
    pub scattering: f32,
}
