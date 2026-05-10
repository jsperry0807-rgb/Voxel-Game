use crate::TimeResource;
use bevy::prelude::*;

pub struct TimePlugin;

impl Plugin for TimePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TimeResource::new());
        app.add_systems(Update, advance_time);
    }
}

fn advance_time(mut time: ResMut<TimeResource>) {
    time.advance_tick();
}
