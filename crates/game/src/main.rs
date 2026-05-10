use bevy::prelude::*;
use renderer::VoxelBevyPlugin;
use time::TimePlugin;
use worldgen::WorldgenPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(VoxelBevyPlugin)
        .add_plugins(TimePlugin)
        .add_plugins(WorldgenPlugin)
        .run();
}
