use glam::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct HydrologyResource {
    pub max_water_content: f32,
    pub hydraulic_conductivity: f32,
    pub runoff_coefficient: f32,
    pub erodibility: f32,
    pub infiltration_rate: f32,
}

impl HydrologyResource {
    pub fn stone() -> Self {
        Self {
            max_water_content: 0.01,
            hydraulic_conductivity: 1e-7,
            runoff_coefficient: 0.85,
            erodibility: 0.05,
            infiltration_rate: 1e-8,
        }
    }
    pub fn dirt() -> Self {
        Self {
            max_water_content: 0.4,
            hydraulic_conductivity: 1e-5,
            runoff_coefficient: 0.3,
            erodibility: 0.6,
            infiltration_rate: 1e-5,
        }
    }
    pub fn sand() -> Self {
        Self {
            max_water_content: 0.35,
            hydraulic_conductivity: 1e-4,
            runoff_coefficient: 0.1,
            erodibility: 0.3,
            infiltration_rate: 1e-4,
        }
    }
    pub fn grass() -> Self {
        Self {
            max_water_content: 0.3,
            hydraulic_conductivity: 5e-6,
            runoff_coefficient: 0.2,
            erodibility: 0.15,
            infiltration_rate: 5e-6,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HydrologyEvent {
    pub position: Vec3,
    pub event_type: HydroEventType,
    pub magnitude: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HydroEventType {
    Flooding,
    Drainage,
    Infiltration,
    Runoff,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sand_more_conductive_than_stone() {
        assert!(
            HydrologyResource::sand().hydraulic_conductivity
                > HydrologyResource::stone().hydraulic_conductivity
        );
    }

    #[test]
    fn dirt_more_erodible_than_stone() {
        assert!(HydrologyResource::dirt().erodibility > HydrologyResource::stone().erodibility);
    }
}
