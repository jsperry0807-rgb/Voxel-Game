use bevy::prelude::*;
use std::collections::HashMap;
use voxel_core::voxel::VoxelMaterial;

// ── SFX kind ───────────────────────────────────────────────────────────────────

/// Every discrete sound effect the engine can trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SfxKind {
    BlockBreak(VoxelMaterial),
    BlockPlace(VoxelMaterial),
    Footstep(VoxelMaterial),
    AmbientWind,
    AmbientWater,
}

// ── Events ─────────────────────────────────────────────────────────────────────

/// Fire this event to play a one-shot sound effect.
#[derive(Event, Debug, Clone)]
pub struct PlaySfxEvent {
    pub kind: SfxKind,
    /// Volume scalar, 0.0 – 1.0.
    pub volume: f32,
}

impl PlaySfxEvent {
    pub fn new(kind: SfxKind) -> Self {
        Self { kind, volume: 1.0 }
    }

    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume;
        self
    }
}

/// Fire this event to start or stop looping ambient audio.
#[derive(Event, Debug, Clone)]
pub struct SetAmbientEvent {
    pub kind: SfxKind,
    pub playing: bool,
    pub volume: f32,
}

// ── Audio registry resource ────────────────────────────────────────────────────

/// Maps SfxKind → loaded asset handles.
/// Populated at startup; handles are cheap to clone.
#[derive(Resource, Default)]
pub struct AudioRegistry {
    handles: HashMap<SfxKind, Handle<AudioSource>>,
}

impl AudioRegistry {
    pub fn register(&mut self, kind: SfxKind, handle: Handle<AudioSource>) {
        self.handles.insert(kind, handle);
    }

    pub fn get(&self, kind: SfxKind) -> Option<&Handle<AudioSource>> {
        self.handles.get(&kind)
    }
}

// ── Ambient marker ─────────────────────────────────────────────────────────────

/// Marker component on ambient audio entities so we can query and stop them.
#[derive(Component)]
pub struct AmbientAudio {
    pub kind: SfxKind,
}

// ── Systems ────────────────────────────────────────────────────────────────────

/// Load all audio assets and register them.
/// Call this from your Startup schedule after asset server is available.
/// In production you'd load real .ogg/.wav files; we register paths here
/// and the asset server handles the rest.
pub fn setup_audio_registry(mut registry: ResMut<AudioRegistry>, asset_server: Res<AssetServer>) {
    // Block break sounds per material
    let break_map = [
        (VoxelMaterial::Stone, "audio/break_stone.ogg"),
        (VoxelMaterial::Dirt, "audio/break_dirt.ogg"),
        (VoxelMaterial::Sand, "audio/break_sand.ogg"),
        (VoxelMaterial::Grass, "audio/break_grass.ogg"),
    ];
    for (mat, path) in break_map {
        registry.register(SfxKind::BlockBreak(mat), asset_server.load(path));
    }

    // Block place sounds
    let place_map = [
        (VoxelMaterial::Stone, "audio/place_stone.ogg"),
        (VoxelMaterial::Dirt, "audio/place_dirt.ogg"),
        (VoxelMaterial::Sand, "audio/place_sand.ogg"),
        (VoxelMaterial::Grass, "audio/place_grass.ogg"),
    ];
    for (mat, path) in place_map {
        registry.register(SfxKind::BlockPlace(mat), asset_server.load(path));
    }

    // Footstep sounds
    let step_map = [
        (VoxelMaterial::Stone, "audio/step_stone.ogg"),
        (VoxelMaterial::Dirt, "audio/step_dirt.ogg"),
        (VoxelMaterial::Sand, "audio/step_sand.ogg"),
        (VoxelMaterial::Grass, "audio/step_grass.ogg"),
        (VoxelMaterial::Water, "audio/step_water.ogg"),
    ];
    for (mat, path) in step_map {
        registry.register(SfxKind::Footstep(mat), asset_server.load(path));
    }

    // Ambient
    registry.register(
        SfxKind::AmbientWind,
        asset_server.load("audio/ambient_wind.ogg"),
    );
    registry.register(
        SfxKind::AmbientWater,
        asset_server.load("audio/ambient_water.ogg"),
    );
}

/// Listens for `PlaySfxEvent` and spawns a one-shot `AudioPlayer` entity.
/// Bevy automatically despawns it when playback finishes (PlaybackSettings::DESPAWN).
pub fn on_play_sfx(
    mut events: EventReader<PlaySfxEvent>,
    registry: Res<AudioRegistry>,
    mut commands: Commands,
) {
    for event in events.read() {
        let Some(handle) = registry.get(event.kind) else {
            // Asset not registered yet (file missing or not loaded) — skip silently
            continue;
        };

        commands.spawn((
            AudioPlayer(handle.clone()),
            PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Despawn,
                volume: bevy::audio::Volume::new(event.volume),
                ..default()
            },
        ));
    }
}

/// Listens for `SetAmbientEvent` and either spawns a looping entity or
/// stops/despawns existing ambient audio of that kind.
pub fn on_set_ambient(
    mut events: EventReader<SetAmbientEvent>,
    registry: Res<AudioRegistry>,
    mut commands: Commands,
    ambient_query: Query<(Entity, &AmbientAudio, &AudioSink)>,
) {
    for event in events.read() {
        if event.playing {
            // Don't double-spawn the same ambient kind
            let already_playing = ambient_query.iter().any(|(_, a, _)| a.kind == event.kind);
            if already_playing {
                continue;
            }

            let Some(handle) = registry.get(event.kind) else {
                continue;
            };

            commands.spawn((
                AmbientAudio { kind: event.kind },
                AudioPlayer(handle.clone()),
                PlaybackSettings {
                    mode: bevy::audio::PlaybackMode::Loop,
                    volume: bevy::audio::Volume::new(event.volume),
                    ..default()
                },
            ));
        } else {
            // Stop and despawn existing ambient of this kind
            for (entity, ambient, sink) in ambient_query.iter() {
                if ambient.kind == event.kind {
                    sink.stop();
                    commands.entity(entity).despawn();
                }
            }
        }
    }
}

// ── Plugin ─────────────────────────────────────────────────────────────────────

pub struct VoxelAudioPlugin;

impl Plugin for VoxelAudioPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AudioRegistry::default())
            .add_event::<PlaySfxEvent>()
            .add_event::<SetAmbientEvent>()
            .add_systems(Startup, setup_audio_registry)
            .add_systems(Update, (on_play_sfx, on_set_ambient));
    }
}
