pub type GasConcentration = f32;
pub type WindVelocity = glam::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct AirCell {
    pub pressure: f32,
    pub temperature: f32,
    pub humidity: f32,
    pub wind: WindVelocity,
    pub gases: [GasConcentration; 4], // Example: [O2, N2, CO2, H2O, Other]
    pub cloud_cover: f32,
    pub rainfall_rate: f32,
}

impl Default for AirCell {
    fn default() -> Self {
        Self {
            pressure: 101325.0,  // Sea level standard atmospheric pressure in Pascals
            temperature: 293.15, // 20°C in Kelvin
            humidity: 0.5,       // 50% relative humidity
            wind: WindVelocity::ZERO,
            gases: [0.232, 0.0006, 0.7555, 0.0124], // Approximate concentrations of O2, N2, CO2, H2O
            cloud_cover: 0.0,
            rainfall_rate: 0.0,
        }
    }
}

impl AirCell {
    pub fn should_condense(&self) -> bool {
        self.humidity > 1.0
    }

    pub fn dew_point(&self) -> f32 {
        let t = self.temperature - 273.15;
        let h = self.humidity.min(1.0).max(1e-6);
        let gamma = (17.27 * t) / (237.7 + t) + h.ln();
        (237.7 * gamma) / (17.27 - gamma) + 273.15
    }

    pub fn saturation_vapor_pressure(&self) -> f32 {
        let t = self.temperature - 273.15;
        610.78 * (17.27 * t / (t + 237.3)).exp()
    }

    pub fn water_vapor_pressure(&self) -> f32 {
        self.humidity * self.saturation_vapor_pressure()
    }

    pub fn process_humidity(&mut self, _dt: f32) {
        if self.should_condense() {
            let excess = self.humidity - 1.0;
            self.humidity = 1.0;
            self.cloud_cover = (self.cloud_cover + excess * 0.5).min(1.0);
            self.rainfall_rate += excess * 0.001;
        } else {
            self.cloud_cover = (self.cloud_cover - 0.001).max(0.0);
            self.rainfall_rate = (self.rainfall_rate - 0.0001).max(0.0);
        }
    }
}

#[derive(Debug, Clone)]
pub struct AtmosphereResource {
    pub scale_height: f32,
    pub lapse_rate: f32,
    pub sea_level_pressure: f32,
    pub sea_level_temperature: f32,
}

impl Default for AtmosphereResource {
    fn default() -> Self {
        Self {
            scale_height: 8434.0,
            lapse_rate: 0.0065,
            sea_level_pressure: 101325.0,
            sea_level_temperature: 288.15,
        }
    }
}

impl AtmosphereResource {
    pub fn pressure_at_altitude(&self, altitude: f32) -> f32 {
        self.sea_level_pressure * (-altitude / self.scale_height).exp()
    }

    pub fn temperature_at_altitude(&self, altitude: f32) -> f32 {
        self.sea_level_temperature - self.lapse_rate * altitude
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cell() {
        let cell = AirCell::default();
        assert_eq!(cell.pressure, 101325.0);
        assert_eq!(cell.gases.len(), 4);
    }

    #[test]
    fn condensation_detected() {
        let mut cell = AirCell::default();
        assert!(!cell.should_condense());
        cell.humidity = 1.5;
        assert!(cell.should_condense());
    }

    #[test]
    fn pressure_decreases_with_altitude() {
        let res = AtmosphereResource::default();
        assert!(res.pressure_at_altitude(1000.0) < res.pressure_at_altitude(0.0));
    }

    #[test]
    fn temperature_decreases_with_altitude() {
        let res = AtmosphereResource::default();
        assert!(res.temperature_at_altitude(2000.0) < res.temperature_at_altitude(0.0));
    }

    #[test]
    fn dew_point_below_temp() {
        let cell = AirCell {
            humidity: 0.5,
            ..Default::default()
        };
        assert!(cell.dew_point() < cell.temperature);
    }
}
