use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, PrintDiagnosticsPlugin},
    prelude::*,
    render::{pass::ClearColor, render_graph::base::BaseRenderGraphConfig},
};
use bevy_kira_audio::{Audio, AudioPlugin};
use bevy_web_fullscreen::FullViewportPlugin;
use wasm_bindgen::prelude::*;
use web_sys::UrlSearchParams;

use bevy_prototype_frameshader::FrameshaderPlugin;
use bevy_prototype_networked_physics::NetworkedPhysicsClientPlugin;
use bevy_prototype_transform_tracker::{TransformTrackingFollower, TransformTrackingPlugin};

#[cfg(feature = "debug-fly-camera")]
use bevy_fly_camera::{FlyCamera, FlyCameraPlugin};

#[cfg(feature = "web")]
use bevy_webgl2;

use shared::{
    blinking_eyes,
    camera_2point5d::{Camera2point5dBundle, Camera2point5dPlugin},
    physics_multiplayer::PhysicsWorld,
    physics_multiplayer_systems, player_input, settings,
};

pub mod sakura;

use sakura::SakuraPlugin;

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

    app.add_resource(ClearColor(Color::rgba(0.0, 0.0, 0.0, 0.0)))
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

    #[cfg(feature = "web")]
    app.add_plugin(FullViewportPlugin);

    let host_id = UrlSearchParams::new_with_str(
        &web_sys::window()
            .expect("should have global window")
            .location()
            .search()
            .expect("should have search string"),
    )
    .expect("should parse valid search params")
    .get("join")
    .expect("should have host id");

    let endpoint_url = format!("http://192.168.1.9:8080/join/{}", host_id);

    app.add_plugin(NetworkedPhysicsClientPlugin::<PhysicsWorld>::new(
        settings::NETWORKED_PHYSICS_CONFIG,
        endpoint_url,
    ));

    // Order is important.
    // The above plugins provide resources for the plugins below.

    app.add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(PrintDiagnosticsPlugin::default())
        .add_plugin(AudioPlugin)
        .add_plugin(FrameshaderPlugin::new(
            VERTEX_SHADER_PATH.into(),
            FRAGMENT_SHADER_PATH.into(),
        ))
        .add_plugin(TransformTrackingPlugin)
        .add_plugin(Camera2point5dPlugin)
        .add_plugin(SakuraPlugin)
        .add_system(player_input::player_input_system.system())
        .add_system(physics_multiplayer_systems::physics_multiplayer_client_sync_system.system())
        .add_system(physics_multiplayer_systems::physics_multiplayer_client_spawn_system.system())
        .add_system(blinking_eyes::blinking_eyes_system.system())
        .add_startup_system(setup.system());

    #[cfg(feature = "debug-fly-camera")]
    app.add_plugin(FlyCameraPlugin);

    app.run();
}

fn setup_hot_reloading(asset_server: ResMut<AssetServer>) {
    asset_server.watch_for_changes().unwrap();
}

fn setup(
    commands: &mut Commands,
    asset_server: ResMut<AssetServer>,
    audio: Res<Audio>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let bgm = asset_server.load("soundtracks/untitled.wav");
    audio.play_looped(bgm);

    commands
        .spawn(Camera2point5dBundle {
            transform: Transform::from_translation(Vec3::unit_z() * 8.0)
                .looking_at(-Vec3::unit_z(), Vec3::unit_y()),
            ..Default::default()
        })
        .with(TransformTrackingFollower);
    // .spawn(primitive(
    //     materials.add(Color::rgb(0.3, 0.7, 1.0).into()),
    //     &mut meshes,
    //     ShapeType::Rectangle {
    //         width: 1000.0,
    //         height: 1000.0,
    //     },
    //     TessellationMode::Fill(&FillOptions::default()),
    //     Vec3::new(-500.0, 0.0 - 5.0, 0.0),
    // ))
    // .spawn(primitive(
    //     materials.add(Color::rgb(1.0, 0.4, 0.5).into()),
    //     &mut meshes,
    //     ShapeType::Rectangle {
    //         width: 1000.0,
    //         height: 1000.0,
    //     },
    //     TessellationMode::Fill(&FillOptions::default()),
    //     Vec3::new(-500.0, -1000.0 - 5.0, 0.0),
    // ));

    #[cfg(feature = "debug-fly-camera")]
    commands.with(FlyCamera::default());
}
