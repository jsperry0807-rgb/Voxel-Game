use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use voxel_core::game_tick::{DAYS_PER_YEAR, GameTick, TICKS_PER_SECOND};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

#[derive(Resource, Clone, Serialize, Deserialize, Debug)]
pub struct TimeResource {
    pub current_tick: GameTick,
    pub year: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub season: Season,
    pub is_daytime: bool,
    pub solar_angle: f32,
}

impl TimeResource {
    pub fn new() -> Self {
        let mut t = Self {
            current_tick: 6 * 3600 * TICKS_PER_SECOND, // Start at 6:00 AM
            year: 1,
            day: 1,
            hour: 0,   // recalculate will set this
            minute: 0, // recalculate will set this
            second: 0, // recalculate will set this
            season: Season::Spring,
            is_daytime: false,
            solar_angle: 0.0,
        };
        t.recalculate();
        t
    }

    pub fn advance_tick(&mut self) {
        self.current_tick += 1;
        self.recalculate();
    }

    pub fn advance_ticks(&mut self, ticks: u64) {
        self.current_tick += ticks;
        self.recalculate();
    }

    fn recalculate(&mut self) {
        let total_seconds = self.current_tick / TICKS_PER_SECOND;
        let seconds_in_day = total_seconds % 86400;

        self.hour = (seconds_in_day / 3600) as u32;
        self.minute = ((seconds_in_day % 3600) / 60) as u32;
        self.second = (seconds_in_day % 60) as u32;

        self.day = ((total_seconds / 86400) + 1) as u32;
        self.year = ((total_seconds / (86400 * 360)) + 1) as u32;

        self.is_daytime = self.hour >= 6 && self.hour < 18;
        self.solar_angle = self.calculate_solar_angle();
        self.season = self.calculate_season();
    }

    fn calculate_solar_angle(&self) -> f32 {
        let progress =
            (self.hour as f64 * 3600.0 + self.minute as f64 * 60.0 + self.second as f64) / 86400.0;
        (progress * std::f64::consts::PI * 2.0 - std::f64::consts::PI / 2.0) as f32
    }

    fn calculate_season(&self) -> Season {
        let days_per_year = DAYS_PER_YEAR as u32;
        let day_of_year = ((self.day - 1) % days_per_year) + 1;
        match (day_of_year - 1) / (days_per_year / 4) {
            0 => Season::Spring,
            1 => Season::Summer,
            2 => Season::Autumn,
            _ => Season::Winter,
        }
    }
}

impl Default for TimeResource {
    fn default() -> Self {
        Self::new()
    }
}
