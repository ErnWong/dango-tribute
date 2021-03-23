use bevy::{
    prelude::*,
    render::{mesh::Indices, pipeline::PrimitiveTopology},
};
use rand::prelude::*;

pub struct SakuraPlugin;

impl Plugin for SakuraPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(spawn_sakura.system());
    }
}

pub fn spawn_sakura(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    const WIDTH: f32 = 40.0;
    const THICKNESS: f32 = 2.5;
    const HEIGHT: f32 = 1.0;
    const LEAF_DENSITY: f32 = 60.0;
    const DISTANCE: f32 = 5.0;
    const LEAF_SIZE_MIN: f32 = 0.2;
    const LEAF_SIZE_MAX: f32 = 0.4;
    const TRUNK_DENSITY: f32 = 0.5;
    const TRUNK_THICKNESS: f32 = 0.2;
    const GROUND_FAR: f32 = -2.0;
    const GROUND_CLOSE: f32 = 10.0;

    let mut leaf_mesh = Mesh::new(PrimitiveTopology::TriangleList);
    leaf_mesh.set_indices(Some(Indices::U32([0, 1, 2].into())));
    let leaf_vertices: Vec<[f32; 3]> = [[0.0, 0.5, 0.0], [1.0, 0.0, 0.0], [0.0, -0.5, 0.0]].into();
    leaf_mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, leaf_vertices);
    leaf_mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0; 3]; 3]);
    leaf_mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0; 3]; 3]);
    let leaf_mesh_handle = meshes.add(leaf_mesh);

    for i in 0..((WIDTH * LEAF_DENSITY) as i32) {
        let x = i as f32 / LEAF_DENSITY - WIDTH * 0.5;
        let y = rand::thread_rng().gen_range(HEIGHT..(HEIGHT + THICKNESS));
        let z = rand::thread_rng()
            .gen_range((-DISTANCE - THICKNESS * 0.5)..(-DISTANCE + THICKNESS * 0.5));
        let angle = rand::thread_rng().gen_range(0.0..(2.0 * std::f32::consts::PI));
        let transform = Transform {
            translation: Vec3::new(x, y, z),
            rotation: Quat::from_rotation_z(angle),
            scale: Vec3::one() * rand::thread_rng().gen_range(LEAF_SIZE_MIN..LEAF_SIZE_MAX),
        };
        let brightness = rand::thread_rng().gen_range(0.5..1.0);
        let color = Color::rgb(1.0 * brightness, 0.4 * brightness, 0.5 * brightness);
        commands.spawn(SpriteBundle {
            sprite: Sprite {
                size: Vec2::one(),
                ..Default::default()
            },
            mesh: leaf_mesh_handle.clone(),
            material: materials.add(color.into()),
            transform,
            ..Default::default()
        });
    }

    let mut trunk_mesh = Mesh::new(PrimitiveTopology::TriangleList);
    trunk_mesh.set_indices(Some(Indices::U32([0, 1, 2, 2, 1, 3].into())));
    let trunk_vertices: Vec<[f32; 3]> = [
        [-0.5, 1.0, 0.0],
        [0.5, 1.0, 0.0],
        [-0.5, 0.0, 0.0],
        [0.5, 0.0, 0.0],
    ]
    .into();
    trunk_mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, trunk_vertices);
    trunk_mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0; 4]; 4]);
    trunk_mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0; 4]; 4]);
    let trunk_mesh_handle = meshes.add(trunk_mesh);

    for i in 0..((WIDTH * TRUNK_DENSITY) as i32) {
        let x = (i as f32 + rand::thread_rng().gen_range(-0.2..0.2)) / TRUNK_DENSITY - WIDTH * 0.5;
        let z = rand::thread_rng()
            .gen_range((-DISTANCE - THICKNESS * 0.3)..(-DISTANCE + THICKNESS * 0.3));
        let height = rand::thread_rng().gen_range(HEIGHT..(HEIGHT + THICKNESS * 0.5));
        let transform = Transform {
            translation: Vec3::new(x, 0.0, z),
            rotation: Quat::default(),
            scale: Vec3::new(TRUNK_THICKNESS, height, 1.0),
        };
        let brightness = rand::thread_rng().gen_range(0.7..1.0);
        let color = Color::rgb(0.4 * brightness, 0.1 * brightness, 0.2 * brightness);
        commands.spawn(SpriteBundle {
            sprite: Sprite {
                size: Vec2::one(),
                ..Default::default()
            },
            mesh: trunk_mesh_handle.clone(),
            material: materials.add(color.into()),
            transform,
            ..Default::default()
        });
    }

    // let mut ground_mesh = Mesh::new(PrimitiveTopology::TriangleList);
    // ground_mesh.set_indices(Some(Indices::U32([0, 1, 2, 2, 1, 3].into())));

    // // NOTE: We offset by -distance so it stays behind other objects in the scene.
    // let ground_vertices: Vec<[f32; 3]> = [
    //     [-WIDTH * 0.5, 0.0, GROUND_FAR + DISTANCE],
    //     [WIDTH, 0.0, GROUND_FAR + DISTANCE],
    //     [-WIDTH, 0.0, GROUND_CLOSE + DISTANCE],
    //     [WIDTH, 0.0, GROUND_CLOSE + DISTANCE],
    // ]
    // .into();

    // ground_mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, ground_vertices);
    // ground_mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0; 4]; 4]);
    // ground_mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0; 4]; 4]);
    // let ground_mesh_handle = meshes.add(ground_mesh);
    // commands.spawn(SpriteBundle {
    //     sprite: Sprite {
    //         size: Vec2::one(),
    //         ..Default::default()
    //     },
    //     mesh: ground_mesh_handle.clone(),
    //     material: materials.add(Color::rgb(0.3, 0.3, 0.3).into()),

    //     // NOTE: We offset by -distance so it stays behind other objects in the scene.
    //     transform: Transform::from_translation(Vec3::unit_z() * (-DISTANCE)),

    //     ..Default::default()
    // });
}
