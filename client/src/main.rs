use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, PrintDiagnosticsPlugin},
    prelude::*,
    render::{pass::ClearColor, render_graph::base::BaseRenderGraphConfig},
};

use bevy_prototype_frameshader::FrameshaderPlugin;
use bevy_prototype_networked_physics::NetworkedPhysicsClientPlugin;
use bevy_prototype_transform_tracker::{TransformTrackingFollower, TransformTrackingPlugin};

#[cfg(feature = "debug-fly-camera")]
use bevy_fly_camera::{FlyCamera, FlyCameraPlugin};

#[cfg(feature = "web")]
use bevy_webgl2;

use shared::{
    physics_multiplayer::PhysicsWorld, physics_multiplayer_systems, player_input, settings,
};

#[cfg(feature = "web")]
const VERTEX_SHADER_PATH: &str = "shaders/frameshader.webgl2.vert";

#[cfg(feature = "native")]
const VERTEX_SHADER_PATH: &str = "shaders/frameshader.wgpu.vert";

#[cfg(feature = "web")]
const FRAGMENT_SHADER_PATH: &str = "shaders/frameshader.webgl2.frag";

#[cfg(feature = "native")]
const FRAGMENT_SHADER_PATH: &str = "shaders/frameshader.wgpu.frag";

fn main() {
    let mut app = App::build();

    // Note: Setup hot reloading first before loading other plugins.
    // Shader assets loaded via ReshadePlugin won't get watched otherwise.
    // TODO: Fix this by loading all assets from the main setup startup system.
    app.add_startup_system(setup_hot_reloading.system());

    app.add_resource(ClearColor(Color::WHITE))
        .add_resource(bevy::log::LogSettings {
            level: bevy::log::Level::INFO,
            ..Default::default()
        })
        .add_plugin(bevy::log::LogPlugin::default())
        .add_plugin(bevy::reflect::ReflectPlugin::default())
        .add_plugin(bevy::core::CorePlugin::default())
        .add_plugin(bevy::transform::TransformPlugin::default())
        .add_plugin(bevy::diagnostic::DiagnosticsPlugin::default())
        .add_plugin(bevy::input::InputPlugin::default())
        .add_plugin(bevy::window::WindowPlugin::default())
        .add_plugin(bevy::asset::AssetPlugin::default())
        .add_plugin(bevy::scene::ScenePlugin::default())
        .add_plugin(bevy::render::RenderPlugin {
            base_render_graph_config: Some(BaseRenderGraphConfig {
                // So we can route main pass through our reshade plugin.
                connect_main_pass_to_swapchain: false,
                ..Default::default()
            }),
        })
        .add_plugin(bevy::sprite::SpritePlugin::default())
        .add_plugin(bevy::pbr::PbrPlugin::default())
        .add_plugin(bevy::ui::UiPlugin::default())
        .add_plugin(bevy::text::TextPlugin::default())
        .add_plugin(bevy::gltf::GltfPlugin::default())
        .add_plugin(bevy::winit::WinitPlugin::default());

    #[cfg(feature = "native")]
    app.add_plugin(bevy::wgpu::WgpuPlugin::default());

    #[cfg(feature = "web")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);

    app.add_plugin(NetworkedPhysicsClientPlugin::<PhysicsWorld>::new(
        settings::NETWORKED_PHYSICS_CONFIG,
    ));

    // Order is important.
    // The above plugins provide resources for the plugins below.

    app.add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(PrintDiagnosticsPlugin::default())
        .add_plugin(FrameshaderPlugin::new(
            VERTEX_SHADER_PATH.into(),
            FRAGMENT_SHADER_PATH.into(),
        ))
        .add_plugin(TransformTrackingPlugin)
        .add_system(player_input::player_input_system.system())
        .add_system(physics_multiplayer_systems::physics_multiplayer_client_sync_system.system())
        .add_system(physics_multiplayer_systems::physics_multiplayer_client_spawn_system.system())
        .add_startup_system(setup.system());

    #[cfg(feature = "debug-fly-camera")]
    app.add_plugin(FlyCameraPlugin);

    app.run();
}

fn setup_hot_reloading(asset_server: ResMut<AssetServer>) {
    asset_server.watch_for_changes().unwrap();
}

fn setup(commands: &mut Commands) {
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

    #[cfg(feature = "debug-fly-camera")]
    commands.with(FlyCamera::default());
}
