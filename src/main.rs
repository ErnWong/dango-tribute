use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, PrintDiagnosticsPlugin},
    prelude::*,
    render::{pass::ClearColor, render_graph::base::BaseRenderGraphConfig},
};
use bevy_fly_camera::{FlyCamera, FlyCameraPlugin};
use bevy_prototype_lyon::prelude::*;
use nphysics2d::{
    nalgebra::Vector2,
    ncollide2d::shape::{Cuboid, ShapeHandle},
    object::{BodyStatus, ColliderDesc, RigidBodyDesc},
};

mod controlled_dango;
mod dango;
mod physics;
mod reshade;
mod window_random_texture_node;

#[cfg(target_arch = "wasm32")]
use bevy_webgl2;

use controlled_dango::{ControlledDangoComponent, ControlledDangoPlugin};
use dango::{colors::*, DangoDescriptorComponent, DangoPlugin};
use physics::PhysicsPlugin;
use reshade::ReshadePlugin;

pub const GRAVITY: f32 = -9.81 * 1.5;
pub type RealField = f32;

fn main() {
    let mut app = App::build();

    // Note: Setup hot reloading first before loading other plugins.
    // Shader assets loaded via ReshadePlugin won't get watched otherwise.
    // TODO: Fix this by loading all assets from the main setup startup system.
    app.add_startup_system(setup_hot_reloading.system());

    app.add_resource(ClearColor(Color::WHITE))
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
        .add_plugin(bevy::audio::AudioPlugin::default())
        .add_plugin(bevy::gltf::GltfPlugin::default())
        .add_plugin(bevy::winit::WinitPlugin::default())
        .add_plugin(bevy::wgpu::WgpuPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(PrintDiagnosticsPlugin::default())
        .add_plugin(ReshadePlugin)
        .add_plugin(PhysicsPlugin::new(Vector2::new(0.0, GRAVITY)))
        .add_plugin(DangoPlugin)
        .add_plugin(ControlledDangoPlugin)
        .add_startup_system(setup.system());

    #[cfg(target_arch = "wasm32")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);

    if cfg!(feature = "debug-fly-camera") {
        app.add_plugin(FlyCameraPlugin);
    }

    app.run();
}

fn setup_hot_reloading(asset_server: ResMut<AssetServer>) {
    asset_server.watch_for_changes().unwrap();
}

fn setup(
    commands: &mut Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let yellow = materials.add(DANGO_YELLOW.into());
    let green = materials.add(DANGO_GREEN.into());

    commands
        .spawn(Camera2dBundle {
            transform: Transform {
                scale: Vec3::one() / 60.0,
                translation: Vec3::unit_z() * (1000.0 / 60.0 - 0.1),
                rotation: Default::default(),
            },
            ..Default::default()
        })
        .with(FlyCamera::default())
        .spawn(primitive(
            yellow.clone(),
            &mut meshes,
            ShapeType::Rectangle {
                width: 10.0,
                height: 0.3,
            },
            TessellationMode::Fill(&FillOptions::default()),
            Vec3::zero().into(),
        ))
        .with(
            RigidBodyDesc::<RealField>::new()
                .translation(Vector2::new(0.0, -5.0))
                .rotation(0.5)
                .status(BodyStatus::Static),
        )
        .with(
            ColliderDesc::<RealField>::new(ShapeHandle::new(Cuboid::new(Vector2::new(5.0, 0.15))))
                .translation(Vector2::new(5.0, 0.15)),
        )
        .spawn(primitive(
            green.clone(),
            &mut meshes,
            ShapeType::Rectangle {
                width: 10.0,
                height: 0.3,
            },
            TessellationMode::Fill(&FillOptions::default()),
            Vec3::zero().into(),
        ))
        .with(
            RigidBodyDesc::<RealField>::new()
                .translation(Vector2::new(-2.0, -1.0))
                .rotation(-0.5)
                .status(BodyStatus::Static),
        )
        .with(
            ColliderDesc::<RealField>::new(ShapeHandle::new(Cuboid::new(Vector2::new(5.0, 0.15))))
                .translation(Vector2::new(5.0, 0.15)),
        )
        .spawn((DangoDescriptorComponent {
            x: 0.0,
            y: 0.0,
            size: 0.8,
            color: DANGO_RED,
        },))
        .spawn((DangoDescriptorComponent {
            x: 2.0,
            y: 0.0,
            size: 0.5,
            color: DANGO_GREEN,
        },))
        .with(ControlledDangoComponent::default());
}
