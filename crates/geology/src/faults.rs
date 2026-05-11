use glam::Vec3;

#[derive(Debug, Clone)]
pub struct Fold {
    pub axis: Vec3,
    pub axis_direction: Vec3,
    pub wavelength: f32,
    pub amplitude: f32,
    pub plunge: f32,
}

impl Fold {
    pub fn displacement_at(&self, position: Vec3) -> f32 {
        let axis_norm = self.axis_direction.normalize_or_zero();
        if axis_norm.length() < 0.001 {
            return 0.0;
        }
        let relative = position - self.axis;
        let along = relative.dot(axis_norm);
        let perp = relative - axis_norm * along;
        let dist = perp.length();
        let phase = 2.0 * std::f32::consts::PI * dist / self.wavelength + along * 0.01;
        self.amplitude * phase.sin() * (-dist / (self.wavelength * 2.0)).exp()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FaultType {
    Normal,
    Reverse,
    StrikeSlip,
}

#[derive(Debug, Clone)]
pub struct Fault {
    pub plane_origin: Vec3,
    pub plane_normal: Vec3,
    pub dip: f32,
    pub displacement: f32,
    pub fault_type: FaultType,
}

impl Fault {
    pub fn distance_to_plane(&self, position: Vec3) -> f32 {
        let n = self.plane_normal.normalize_or_zero();
        if n.length() < 0.001 {
            return f32::MAX;
        }
        (position - self.plane_origin).dot(n)
    }

    pub fn displacement_vector(&self, position: Vec3) -> Vec3 {
        let d = self.distance_to_plane(position);
        let side = match self.fault_type {
            FaultType::Normal => {
                if d > 0.0 {
                    1.0
                } else {
                    0.0
                }
            }
            FaultType::Reverse => {
                if d > 0.0 {
                    1.0
                } else {
                    0.0
                }
            }
            FaultType::StrikeSlip => {
                if d > 0.0 {
                    1.0
                } else {
                    -1.0
                }
            }
        };
        self.plane_normal.normalize_or(Vec3::Y) * self.displacement * side
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_at_axis_zero() {
        let f = Fold {
            axis: Vec3::ZERO,
            axis_direction: Vec3::Z,
            wavelength: 100.0,
            amplitude: 20.0,
            plunge: 0.0,
        };
        assert!(f.displacement_at(Vec3::ZERO).abs() < 1.0);
    }

    #[test]
    fn fold_has_displacement_away_from_axis() {
        let f = Fold {
            axis: Vec3::ZERO,
            axis_direction: Vec3::Z,
            wavelength: 100.0,
            amplitude: 50.0,
            plunge: 0.0,
        };
        assert!(f.displacement_at(Vec3::new(30.0, 0.0, 0.0)).abs() > 0.0);
    }

    #[test]
    fn fault_plane_distance() {
        let f = Fault {
            plane_origin: Vec3::ZERO,
            plane_normal: Vec3::X,
            dip: 0.78,
            displacement: 10.0,
            fault_type: FaultType::Normal,
        };
        assert_eq!(f.distance_to_plane(Vec3::X * 5.0), 5.0);
    }

    #[test]
    fn normal_fault_zero_on_negative_side() {
        let f = Fault {
            plane_origin: Vec3::ZERO,
            plane_normal: Vec3::X,
            dip: 0.78,
            displacement: 10.0,
            fault_type: FaultType::Normal,
        };
        assert!(f.displacement_vector(Vec3::NEG_X * 5.0).length() < 0.001);
    }

    #[test]
    fn strike_slip_displaces_both_sides() {
        let f = Fault {
            plane_origin: Vec3::ZERO,
            plane_normal: Vec3::X,
            dip: 0.78,
            displacement: 10.0,
            fault_type: FaultType::StrikeSlip,
        };
        assert!(f.displacement_vector(Vec3::X * 5.0).length() > 0.0);
        assert!(f.displacement_vector(Vec3::NEG_X * 5.0).length() > 0.0);
    }
}
