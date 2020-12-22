use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, PrintDiagnosticsPlugin},
    prelude::*,
    render::{pass::ClearColor, render_graph::base::BaseRenderGraphConfig},
};
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::{
    na::Vector2,
    physics::{RapierConfiguration, RapierPhysicsPlugin},
    rapier::dynamics::RigidBodyBuilder,
    rapier::geometry::ColliderBuilder,
};
use nphysics2d::{
    ncollide2d::shape::{Ball, Cuboid, ShapeHandle},
    object::{BodyStatus, ColliderDesc, FEMSurfaceDesc, RigidBodyDesc},
};

mod controlled_dango;
mod nphysics_sync;
mod reshade;
mod window_random_texture_node;

use controlled_dango::{controlled_dango_system, ControlledDangoComponent};
use nphysics_sync::{BlobPhysicsComponent, NPhysicsPlugin};
use reshade::ReshadePlugin;

pub const GRAVITY: f32 = -9.81 * 1.0;
pub type RealField = f32;

fn main() {
    App::build()
        .add_resource(ClearColor(Color::WHITE))
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
        .add_plugin(RapierPhysicsPlugin)
        .add_plugin(NPhysicsPlugin::new(Vector2::new(0.0, GRAVITY)))
        .add_plugin(DangoLand)
        .run();
}

pub struct DangoLand;

impl Plugin for DangoLand {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup_nphysics.system())
            .add_system(controlled_dango_system.system());
    }
}

fn setup_nphysics(
    commands: &mut Commands,
    asset_server: ResMut<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    asset_server.watch_for_changes().unwrap();

    let yellow = materials.add(Color::rgb(0.8672, 0.8438, 0.7266).into());
    let green = materials.add(Color::rgb(0.7813, 0.8673, 0.7656).into());
    let red = materials.add(Color::rgb(0.9023, 0.8400, 0.8400).into());
    let material_ball = materials.add(Color::rgb(0.8, 0.6, 0.0).into());

    commands
        //.spawn(Camera2dBundle::default())
        .spawn(Camera2dBundle {
            transform: Transform::from_scale(Vec3::one() / 60.0),
            ..Default::default()
        })
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
        .spawn(primitive(
            red.clone(),
            &mut meshes,
            ShapeType::Circle(0.8),
            TessellationMode::Fill(&FillOptions::default()),
            Vec3::zero().into(),
        ))
        .with(BlobPhysicsComponent::new(0.0, 0.0, 0.8))
        //.with(FEMSurfaceDesc::<RealField>::quad(3, 3))
        .spawn(primitive(
            material_ball.clone(),
            &mut meshes,
            ShapeType::Circle(0.8),
            TessellationMode::Fill(&FillOptions::default()),
            Vec3::zero().into(),
        ))
        .with(BlobPhysicsComponent::new(2.0, 0.0, 0.5))
        //.with(
        //    RigidBodyDesc::<RealField>::new()
        //        .mass(1.0)
        //        .translation(Vector2::new(30.0, 10.0)),
        //)
        //.with(ColliderDesc::<RealField>::new(ShapeHandle::new(Ball::new(
        //    40.0,
        //))))
        .with(ControlledDangoComponent {});
}

fn setup_rapier(
    commands: &mut Commands,
    asset_server: ResMut<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut rapier_config: ResMut<RapierConfiguration>,
) {
    asset_server.watch_for_changes().unwrap();

    rapier_config.gravity = Vector2::new(0.0, GRAVITY);
    let yellow = materials.add(Color::rgb(0.8672, 0.8438, 0.7266).into());
    let green = materials.add(Color::rgb(0.7813, 0.8673, 0.7656).into());
    let red = materials.add(Color::rgb(0.9023, 0.8400, 0.8400).into());
    let material_ball = materials.add(Color::rgb(0.8, 0.6, 0.0).into());

    commands
        .spawn(Camera2dBundle::default())
        .spawn(primitive(
            yellow.clone(),
            &mut meshes,
            ShapeType::Rectangle {
                width: 100.0,
                height: 10.0,
            },
            TessellationMode::Fill(&FillOptions::default()),
            Vec3::zero().into(),
        ))
        .with(
            RigidBodyBuilder::new_static()
                .translation(10.0, -100.0)
                .rotation(0.5),
        )
        .with(ColliderBuilder::cuboid(50.0, 5.0).translation(50.0, 5.0))
        .spawn(primitive(
            green.clone(),
            &mut meshes,
            ShapeType::Rectangle {
                width: 100.0,
                height: 10.0,
            },
            TessellationMode::Fill(&FillOptions::default()),
            Vec3::zero().into(),
        ))
        .with(
            RigidBodyBuilder::new_static()
                .translation(-60.0, -50.0)
                .rotation(-0.5),
        )
        .with(ColliderBuilder::cuboid(50.0, 5.0).translation(50.0, 5.0))
        .spawn(primitive(
            red.clone(),
            &mut meshes,
            ShapeType::Circle(40.0),
            TessellationMode::Fill(&FillOptions::default()),
            Vec3::zero().into(),
        ))
        .with(RigidBodyBuilder::new_dynamic())
        .with(ColliderBuilder::ball(40.0))
        .spawn(primitive(
            material_ball.clone(),
            &mut meshes,
            ShapeType::Circle(40.0),
            TessellationMode::Fill(&FillOptions::default()),
            Vec3::zero().into(),
        ))
        .with(
            RigidBodyBuilder::new_dynamic()
                .translation(30.0, 10.0)
                .lock_rotations(),
        )
        .with(ColliderBuilder::ball(40.0))
        .with(ControlledDangoComponent {});
}
