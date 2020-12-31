use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, PrintDiagnosticsPlugin},
    prelude::*,
};
use bevy_prototype_networked_physics::NetworkedPhysicsServerPlugin;
use shared::{physics_multiplayer::PhysicsWorld, physics_multiplayer_systems, settings};

fn main() {
    let mut app = App::build();

    // Note: Setup hot reloading first before loading other plugins.
    // Shader assets loaded via ReshadePlugin won't get watched otherwise.
    // TODO: Fix this by loading all assets from the main setup startup system.
    app.add_startup_system(setup_hot_reloading.system());

    app.add_plugin(bevy::log::LogPlugin::default())
        .add_plugin(bevy::reflect::ReflectPlugin::default())
        .add_plugin(bevy::core::CorePlugin::default())
        .add_plugin(bevy::diagnostic::DiagnosticsPlugin::default())
        .add_plugin(bevy::asset::AssetPlugin::default())
        .add_plugin(bevy::scene::ScenePlugin::default())
        .add_plugin(bevy::app::ScheduleRunnerPlugin::default());

    app.add_plugin(NetworkedPhysicsServerPlugin::<PhysicsWorld>::new(
        settings::NETWORKED_PHYSICS_CONFIG,
    ));

    // Order is important.
    // The above plugins provide resources for the plugins below.

    app.add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(PrintDiagnosticsPlugin::default())
        .add_system(
            physics_multiplayer_systems::physics_multiplayer_server_despawn_system.system(),
        );

    app.run();
}

fn setup_hot_reloading(asset_server: ResMut<AssetServer>) {
    asset_server.watch_for_changes().unwrap();
}
