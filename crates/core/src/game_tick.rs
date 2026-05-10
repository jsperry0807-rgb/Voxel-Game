pub type GameTick = u64;

pub const TICKS_PER_SECOND: GameTick = 20;
pub const SECONDS_PER_MINUTE: u64 = 60;
pub const MINUTES_PER_HOUR: u64 = 60;
pub const HOURS_PER_DAY: u64 = 24;
pub const DAYS_PER_YEAR: u64 = 360;

pub const TICKS_PER_DAY: GameTick =
    TICKS_PER_SECOND * SECONDS_PER_MINUTE * MINUTES_PER_HOUR * HOURS_PER_DAY;

pub const TICKS_PER_YEAR: GameTick = TICKS_PER_DAY * DAYS_PER_YEAR;
