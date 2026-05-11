use rand::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RockType {
    Granite,
    Basalt,
    Sandstone,
    Limestone,
    Shale,
    Marble,
    Slate,
    Gneiss,
    OreBearing,
}

impl RockType {
    pub fn density(&self) -> f32 {
        match self {
            RockType::Granite => 2700.0,
            RockType::Basalt => 3000.0,
            RockType::Sandstone => 2200.0,
            RockType::Limestone => 2500.0,
            RockType::Shale => 2400.0,
            RockType::Marble => 2700.0,
            RockType::Slate => 2800.0,
            RockType::Gneiss => 2750.0,
            RockType::OreBearing => 3500.0,
        }
    }

    pub fn erosion_resistance(&self) -> f32 {
        match self {
            RockType::Granite => 0.8,
            RockType::Basalt => 0.9,
            RockType::Sandstone => 0.4,
            RockType::Limestone => 0.5,
            RockType::Shale => 0.3,
            RockType::Marble => 0.6,
            RockType::Slate => 0.7,
            RockType::Gneiss => 0.75,
            RockType::OreBearing => 0.85,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StratigraphicFormation {
    pub rock_type: RockType,
    pub base_depth: f32,
    pub thinkness: f32,
    pub age_mya: f64,
}

impl StratigraphicFormation {
    pub fn new(rock_type: RockType, base: f32, think: f32, age: f64) -> Self {
        Self {
            rock_type,
            base_depth: base,
            thinkness: think,
            age_mya: age,
        }
    }

    pub fn top_depth(&self) -> f32 {
        self.base_depth - self.thinkness
    }
}

pub struct OreDeposit {
    pub ore_type: u8,
    pub concentration: f32,
    pub mass_estimate: f32,
}

impl OreDeposit {
    pub fn new<R: Rng>(rng: &mut R) -> Self {
        Self {
            ore_type: rng.gen_range(0..20),
            concentration: rng.gen_range(0.05..0.5),
            mass_estimate: rng.gen_range(10.0..10000.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_xoshiro::Xoshiro256Plus;

    #[test]
    fn basalt_denser_than_sandstone() {
        assert!(RockType::Basalt.density() > RockType::Sandstone.density());
    }

    #[test]
    fn formation_top_depth() {
        let f = StratigraphicFormation::new(RockType::Granite, 100.0, 50.0, 200.0);
        assert_eq!(f.top_depth(), 50.0);
    }

    #[test]
    fn ore_concentration_in_range() {
        let mut rng = Xoshiro256Plus::seed_from_u64(42);
        let ore = OreDeposit::new(&mut rng);
        assert!(ore.concentration > 0.0 && ore.concentration <= 0.5);
    }
}
