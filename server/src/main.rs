use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, PrintDiagnosticsPlugin},
    prelude::*,
};
use bevy_prototype_networked_physics::NetworkedPhysicsServerPlugin;
use bevy_prototype_transform_tracker::TransformTrackingFollower;
use shared::{physics_multiplayer::PhysicsWorld, physics_multiplayer_systems, settings};

const SHOW_DEBUG_WINDOW: bool = true;

fn main() {
    let mut app = App::build();

    // Note: Setup hot reloading first before loading other plugins.
    // Shader assets loaded via ReshadePlugin won't get watched otherwise.
    // TODO: Fix this by loading all assets from the main setup startup system.
    app.add_startup_system(setup_hot_reloading.system());

    app.add_resource(bevy::log::LogSettings {
        level: bevy::log::Level::INFO,
        ..Default::default()
    })
    .add_plugin(bevy::log::LogPlugin::default())
    .add_plugin(bevy::reflect::ReflectPlugin::default())
    .add_plugin(bevy::core::CorePlugin::default())
    .add_plugin(bevy::diagnostic::DiagnosticsPlugin::default())
    .add_plugin(bevy::asset::AssetPlugin::default())
    .add_plugin(bevy::scene::ScenePlugin::default());

    if SHOW_DEBUG_WINDOW {
        app.add_resource(ClearColor(Color::WHITE))
            .add_plugin(bevy::transform::TransformPlugin::default())
            .add_plugin(bevy::input::InputPlugin::default())
            .add_plugin(bevy::window::WindowPlugin::default())
            .add_plugin(bevy::render::RenderPlugin::default())
            .add_plugin(bevy::sprite::SpritePlugin::default())
            .add_plugin(bevy::pbr::PbrPlugin::default())
            .add_plugin(bevy::ui::UiPlugin::default())
            .add_plugin(bevy::text::TextPlugin::default())
            .add_plugin(bevy::gltf::GltfPlugin::default())
            .add_plugin(bevy::winit::WinitPlugin::default())
            .add_plugin(bevy::wgpu::WgpuPlugin::default());
    } else {
        app.add_plugin(bevy::app::ScheduleRunnerPlugin::default());
    }

    app.add_plugin(NetworkedPhysicsServerPlugin::<PhysicsWorld>::new(
        settings::NETWORKED_PHYSICS_CONFIG,
    ));

    // Order is important.
    // The above plugins provide resources for the plugins below.

    app.add_plugin(FrameTimeDiagnosticsPlugin::default())
        //.add_plugin(PrintDiagnosticsPlugin::default())
        .add_system(
            physics_multiplayer_systems::physics_multiplayer_server_despawn_system.system(),
        );

    if SHOW_DEBUG_WINDOW {
        app.add_system(
            physics_multiplayer_systems::physics_multiplayer_server_diagnostic_sync_system.system(),
        )
        .add_startup_system(debug_window_setup.system());
    }

    app.run();
}

fn setup_hot_reloading(asset_server: ResMut<AssetServer>) {
    asset_server.watch_for_changes().unwrap();
}

fn debug_window_setup(commands: &mut Commands) {
    commands
        .spawn(Camera2dBundle {
            transform: Transform {
                scale: Vec3::one() / 60.0,
                translation: Vec3::unit_z() * (1000.0 / 60.0 - 0.1),
                rotation: Default::default(),
            },
            ..Default::default()
        })
        .with(TransformTrackingFollower);
}
