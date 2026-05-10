use crate::{Season, TimeResource};
use voxel_core::game_tick::TICKS_PER_SECOND;

#[test]
fn initial_state() {
    let t = TimeResource::new();
    assert_eq!(t.hour, 6);
    assert_eq!(t.minute, 0);
    assert!(t.is_daytime);
    assert_eq!(t.season, Season::Spring);
}

#[test]
fn advance_one_second() {
    let mut t = TimeResource::new();
    t.advance_ticks(TICKS_PER_SECOND);
    assert_eq!(t.second, 1);
    assert_eq!(t.hour, 6);
}

#[test]
fn day_night_transition() {
    let mut t = TimeResource::new();
    // Advance from hour 6 → hour 18 (12 hours)
    t.advance_ticks(12 * 3600 * TICKS_PER_SECOND);
    assert!(!t.is_daytime, "should be night after 18:00");
    assert_eq!(t.hour, 18);
}

#[test]
fn full_year_wraps_to_spring() {
    let mut t = TimeResource::new();
    t.advance_ticks(360 * 86400 * TICKS_PER_SECOND);
    assert_eq!(t.year, 2);
    assert_eq!(t.season, Season::Spring);
}

#[test]
fn season_cycle() {
    let tpd = 86400 * TICKS_PER_SECOND; // ticks per day
    let cases = [
        (1, Season::Spring),
        (91, Season::Summer),
        (181, Season::Autumn),
        (271, Season::Winter),
        (361, Season::Spring), // wraps
    ];
    for (day, expected) in cases {
        let mut t = TimeResource::new();
        t.advance_ticks((day - 1) * tpd);
        assert_eq!(t.season, expected, "day {day} should be {expected:?}");
    }
}
