use bevy::{
    app::ScheduleRunnerSettings,
    diagnostic::{FrameTimeDiagnosticsPlugin, PrintDiagnosticsPlugin},
    prelude::*,
};
use bevy_prototype_networked_physics::{net::NetworkEvent, NetworkedPhysicsServerPlugin};
use bevy_prototype_transform_tracker::TransformTrackingFollower;
use shared::{
    physics_multiplayer::PhysicsWorld, physics_multiplayer_systems, settings,
    wasm_print_diagnostics_plugin::WasmPrintDiagnosticsPlugin,
};
use std::time::Duration;
use wasm_bindgen::prelude::*;
use web_sys::Url;

// const SHOW_DEBUG_WINDOW: bool = false;

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

    // if SHOW_DEBUG_WINDOW {
    //     app.add_resource(ClearColor(Color::WHITE))
    //         .add_plugin(bevy::transform::TransformPlugin::default())
    //         .add_plugin(bevy::input::InputPlugin::default())
    //         .add_plugin(bevy::window::WindowPlugin::default())
    //         .add_plugin(bevy::render::RenderPlugin::default())
    //         .add_plugin(bevy::sprite::SpritePlugin::default())
    //         .add_plugin(bevy::pbr::PbrPlugin::default())
    //         .add_plugin(bevy::ui::UiPlugin::default())
    //         .add_plugin(bevy::text::TextPlugin::default())
    //         .add_plugin(bevy::gltf::GltfPlugin::default())
    //         .add_plugin(bevy::winit::WinitPlugin::default())
    //         .add_plugin(bevy::wgpu::WgpuPlugin::default());
    // } else {
    app.add_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f32(
        1.0 / 60.0,
    )))
    .add_plugin(bevy::app::ScheduleRunnerPlugin::default());
    // }

    app.add_plugin(NetworkedPhysicsServerPlugin::<PhysicsWorld>::new(
        settings::NETWORKED_PHYSICS_CONFIG,
        "ws://dango-daikazoku.herokuapp.com/host".to_string(),
    ));

    // Order is important.
    // The above plugins provide resources for the plugins below.

    app.add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(WasmPrintDiagnosticsPlugin::default())
        .add_system(show_shareable_url_system.system())
        .add_system(
            physics_multiplayer_systems::physics_multiplayer_server_despawn_system.system(),
        );

    // if SHOW_DEBUG_WINDOW {
    //     app.add_system(
    //         physics_multiplayer_systems::physics_multiplayer_server_diagnostic_sync_system.system(),
    //     )
    //     .add_startup_system(debug_window_setup.system());
    // }

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

#[derive(Default)]
pub struct ShowHostIdState {
    network_event_reader: EventReader<NetworkEvent>,
}

fn show_shareable_url_system(
    mut state: Local<ShowHostIdState>,
    network_events: Res<Events<NetworkEvent>>,
) {
    for network_event in state.network_event_reader.iter(&network_events) {
        if let NetworkEvent::Hosted(endpoint_id) = network_event {
            info!("Found endpoint id");
            let relative_url = format!("../client/?join={}", endpoint_id);
            let document = web_sys::window()
                .expect("should have global window")
                .document()
                .expect("window should have document");
            let document_location: String = document
                .location()
                .expect("document should have a location")
                .to_string()
                .into();
            let absolute_url = Url::new_with_base(&relative_url, &document_location)
                .expect("resulting url should be valid")
                .href();
            document
                .get_element_by_id("join-url")
                .expect("join-url input should exist")
                .set_attribute("value", &absolute_url)
                .expect("setting value attribute should succeed");
            // There seems to be severe performance problems running the client in the same tab...
            // document
            //     .get_element_by_id("client-iframe")
            //     .expect("client iframe should exist")
            //     .set_attribute("src", &absolute_url)
            //     .expect("setting src attribute should succeed");
        }
    }
}
