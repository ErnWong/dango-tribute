use bevy::{
    asset::LoadState,
    diagnostic::FrameTimeDiagnosticsPlugin,
    prelude::*,
    render::{pass::ClearColor, render_graph::base::BaseRenderGraphConfig},
};
use bevy_kira_audio::{Audio, AudioPlugin, AudioSource};
use bevy_web_fullscreen::FullViewportPlugin;
use wasm_bindgen::prelude::*;
use web_sys::UrlSearchParams;

use bevy_prototype_frameshader::FrameshaderPlugin;
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
    wasm_print_diagnostics_plugin::WasmPrintDiagnosticsPlugin,
};

pub mod sakura;

use crystalorb_bevy_networking_turbulence::{
    bevy_networking_turbulence::NetworkResource, ClientConnectionEvent, CrystalOrbClientPlugin,
};
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
    show_load_complete("wasm");

    log::set_logger(&wasm_bindgen_console_logger::DEFAULT_LOGGER);
    log::set_max_level(log::LevelFilter::Info);

    let mut app = App::build();

    // Note: Setup hot reloading first before loading other plugins.
    // Shader assets loaded via ReshadePlugin won't get watched otherwise.
    // TODO: Fix this by loading all assets from the main setup startup system.
    app.add_startup_system(setup_hot_reloading.system());

    app.insert_resource(ClearColor(Color::rgba(0.0, 0.0, 0.0, 0.0)))
        .insert_resource(bevy::log::LogSettings {
            level: bevy::log::Level::INFO,
            ..Default::default()
        })
        .add_plugin(bevy::log::LogPlugin::default())
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

    app.add_plugin(CrystalOrbClientPlugin::<PhysicsWorld>::new(
        settings::NETWORKED_PHYSICS_CONFIG,
    ));

    // Order is important.
    // The above plugins provide resources for the plugins below.

    app.add_plugin(FrameTimeDiagnosticsPlugin::default())
        //.add_plugin(WasmPrintDiagnosticsPlugin::default())
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
        .add_system(blinking_eyes::blinking_eyes_system.system())
        .add_system(test_load_progress_system.system())
        .add_system(update_status_system.system())
        .add_startup_system(setup.system());

    #[cfg(feature = "debug-fly-camera")]
    app.add_plugin(FlyCameraPlugin);

    app.run();
}

fn setup_hot_reloading(asset_server: ResMut<AssetServer>) {
    asset_server.watch_for_changes().unwrap();
}

fn setup(
    mut commands: Commands,
    asset_server: ResMut<AssetServer>,
    audio: Res<Audio>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut net: ResMut<NetworkResource>,
) {
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
    let endpoint_url = format!("http://dango-daikazoku.herokuapp.com/join/{}", host_id);
    info!("Starting client - connecting to {}", endpoint_url);
    net.connect(endpoint_url);

    let bgm = asset_server.load("soundtracks/untitled.ogg");
    audio.play_looped(bgm);

    commands
        .spawn()
        .insert_bundle(Camera2point5dBundle {
            transform: Transform::from_translation(Vec3::Z * 8.0).looking_at(-Vec3::Z, Vec3::Y),
            ..Default::default()
        })
        .insert(TransformTrackingFollower);

    #[cfg(feature = "debug-fly-camera")]
    commands.with(FlyCamera::default());
}

fn test_load_progress_system(
    audio_asset_events: EventReader<AssetEvent<AudioSource>>,
    audio_sources: Res<Assets<AudioSource>>,
    shaders: Res<Assets<Shader>>,
    shader_asset_events: EventReader<AssetEvent<Shader>>,
    asset_server: ResMut<AssetServer>,
    client_connection_events: EventReader<ClientConnectionEvent>,
) {
    for _ in audio_asset_events.iter() {
        let mut all_audio_loaded = false;
        for audio_source in audio_sources.ids() {
            // Note: only consider when there's at least one audio source registered.
            all_audio_loaded = true;
            if asset_server.get_load_state(audio_source) != LoadState::Loaded {
                all_audio_loaded = false;
                break;
            }
        }
        if all_audio_loaded {
            show_load_complete("audio");
        }
        break;
    }

    //for _ in shader_asset_events.iter() {
    // let mut all_shaders_loaded = false;
    // info!("Testing shaders {:?}", shaders);
    // for shader in shaders.ids() {
    //     // Note: only consider when there's at least one shader registered.
    //     info!("Testing shader {:?}", shader);
    //     all_shaders_loaded = true;
    //     if asset_server.get_load_state(shader) != LoadState::Loaded {
    //         info!(
    //             "Shader {:?} is not loaded. State: {:?}",
    //             shader,
    //             asset_server.get_load_state(shader)
    //         );
    //         all_shaders_loaded = false;
    //         break;
    //     }
    // }
    // if all_shaders_loaded {
    //     show_load_complete("shaders");
    // }
    //    break;
    //}
    // TODO Not all shaders get loaded. To debug. Also - test shader compilation.
    show_load_complete("shaders");

    for client_connection_event in client_connection_events.iter() {
        if let ClientConnectionEvent::Connected(client_id) = client_connection_event {
            show_load_complete("connection");
            break;
        }
    }
}

fn show_load_complete(component_name: &str) {
    web_sys::window()
        .expect("should have global window")
        .document()
        .expect("window should have document")
        .get_element_by_id("loading-screen")
        .expect("there should be a loading screen")
        .class_list()
        .add_1(&format!("load-complete-{}", component_name))
        .expect("should be able to add a class to span");
}

fn update_status_system(client_connection_events: EventReader<ClientConnectionEvent>) {
    for client_connection_event in client_connection_events.iter() {
        let class_name = match client_connection_event {
            ClientConnectionEvent::Connected(_) => "status-connected",
            ClientConnectionEvent::Disconnected(_) => "status-disconnected",
        };
        web_sys::window()
            .expect("should have global window")
            .document()
            .expect("window should have document")
            .get_element_by_id("status")
            .expect("there should be a status span")
            .set_class_name(class_name);
    }
}
