use bevy::{
    prelude::*,
    render::mesh::{Indices, VertexAttributeValues},
};
use bevy_rapier2d::{
    physics::{RigidBodyHandleComponent, TRANSFORM_SYNC_STAGE},
    rapier::dynamics::RigidBodySet,
};
// use lyon::tesselation::basic_shapes::*;
use lyon::{
    math::point,
    path::{builder::*, Path},
    tessellation::{
        basic_shapes::fill_circle, BuffersBuilder, FillAttributes, FillOptions, FillTessellator,
        VertexBuffers,
    },
};
use nphysics2d::{
    force_generator::DefaultForceGeneratorSet,
    joint::DefaultJointConstraintSet,
    nalgebra::{Point2, Point3, Vector2},
    ncollide2d::shape::Polyline,
    object::{
        BodyPartHandle, BodyStatus, ColliderDesc, DefaultBodyHandle, DefaultBodySet,
        DefaultColliderHandle, DefaultColliderSet, FEMSurfaceDesc, RigidBodyDesc,
    },
    world::{DefaultGeometricalWorld, DefaultMechanicalWorld},
};

use splines::{Interpolation, Key, Spline};

use num::NumCast;

pub const RAPIER_TO_NPHYSICS_SYNC_STAGE: &'static str = "rapier_to_nphysics_sync_stage";
pub const NPHYSICS_STEP_STAGE: &'static str = "nphysics_step_stage";
pub const NPHYSICS_TRANSFORM_SYNC_STAGE: &'static str = "nphysics_transform_sync_stage";

use super::RealField;

pub struct NPhysicsPlugin {
    gravity: Vector2<RealField>,
}

impl NPhysicsPlugin {
    pub fn new(gravity: Vector2<RealField>) -> NPhysicsPlugin {
        NPhysicsPlugin { gravity }
    }
}

const USE_EXISTING_MESH: bool = false;

pub struct BlobPhysicsComponent {
    x: f32,
    y: f32,
    r: f32,
}

impl BlobPhysicsComponent {
    pub fn new(x: f32, y: f32, r: f32) -> BlobPhysicsComponent {
        BlobPhysicsComponent { x, y, r }
    }
}

impl Plugin for NPhysicsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_resource(DefaultMechanicalWorld::<RealField>::new(self.gravity))
            .add_resource(DefaultGeometricalWorld::<RealField>::new())
            .add_resource(DefaultBodySet::<RealField>::new())
            .add_resource(DefaultColliderSet::<RealField>::new())
            .add_resource(DefaultJointConstraintSet::<RealField>::new())
            .add_resource(DefaultForceGeneratorSet::<RealField>::new())
            .add_resource(SimulationToRenderTime::default())
            .add_system_to_stage(stage::PRE_UPDATE, create_body_and_collider_system.system())
            .add_stage_after(
                TRANSFORM_SYNC_STAGE,
                RAPIER_TO_NPHYSICS_SYNC_STAGE,
                SystemStage::parallel(),
            )
            .add_system_to_stage(
                RAPIER_TO_NPHYSICS_SYNC_STAGE,
                sync_rapier_to_nphysics_system.system(),
            )
            .add_stage_after(
                RAPIER_TO_NPHYSICS_SYNC_STAGE,
                NPHYSICS_STEP_STAGE,
                SystemStage::parallel(),
            )
            .add_system_to_stage(NPHYSICS_STEP_STAGE, step_system.system())
            .add_stage_after(
                NPHYSICS_STEP_STAGE,
                NPHYSICS_TRANSFORM_SYNC_STAGE,
                SystemStage::parallel(),
            )
            .add_system_to_stage(
                NPHYSICS_TRANSFORM_SYNC_STAGE,
                sync_transform_system.system(),
            );
    }
}

pub struct NPBodyHandleComponent(DefaultBodyHandle);
pub struct NPColliderHandleComponent(DefaultColliderHandle);

impl From<DefaultBodyHandle> for NPBodyHandleComponent {
    fn from(handle: DefaultBodyHandle) -> Self {
        Self(handle)
    }
}

impl NPBodyHandleComponent {
    pub fn handle(&self) -> DefaultBodyHandle {
        self.0
    }
}

impl From<DefaultColliderHandle> for NPColliderHandleComponent {
    fn from(handle: DefaultColliderHandle) -> Self {
        Self(handle)
    }
}

impl NPColliderHandleComponent {
    pub fn handle(&self) -> DefaultColliderHandle {
        self.0
    }
}

pub fn create_body_and_collider_system(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut bodies: ResMut<DefaultBodySet<RealField>>,
    mut colliders: ResMut<DefaultColliderSet<RealField>>,
    rigid_body_query: Query<(Entity, &RigidBodyDesc<RealField>, &ColliderDesc<RealField>)>,
    fem_surface_query: Query<
        (Entity, &BlobPhysicsComponent, &Handle<Mesh>),
        Without<NPBodyHandleComponent>,
    >,
) {
    for (entity, body_desc, collider_desc) in rigid_body_query.iter() {
        info!("Found new rigid body");
        let body_handle = bodies.insert(body_desc.build());
        commands.insert_one(entity, NPBodyHandleComponent::from(body_handle));
        commands.remove_one::<RigidBodyDesc<RealField>>(entity);

        let collider_handle = colliders.insert(collider_desc.build(BodyPartHandle(body_handle, 0)));
        commands.insert_one(entity, NPColliderHandleComponent::from(collider_handle));
        commands.remove_one::<ColliderDesc<RealField>>(entity);
    }

    for (entity, blob, mesh_handle) in fem_surface_query.iter() {
        info!("Found new FEM surface");
        let mut mesh_vertices: Vec<Point2<RealField>> = vec![];
        let mut mesh_indices: Vec<Point3<usize>> = vec![];
        let hexagon_vertices = VertexAttributeValues::Float3(HEXAGON_VERTICES.into());
        let hexagon_indices = Indices::U32(HEXAGON_INDICES.into());
        let mut circle_geometry: VertexBuffers<[f32; 3], u32> = VertexBuffers::new();
        fill_circle(
            lyon::math::Point::zero(),
            blob.r,
            &FillOptions::tolerance(0.1),
            &mut BuffersBuilder::new(&mut circle_geometry, |pos: lyon::math::Point| {
                [pos.x, pos.y, 0.0]
            }),
        )
        .unwrap();
        info!("Circle has {} vertices", circle_geometry.vertices.len());
        info!("- geometry: {:?}", circle_geometry);
        let circle_vertices = VertexAttributeValues::Float3(circle_geometry.vertices.into());
        let circle_indices = Indices::U32(circle_geometry.indices.into());
        let (desired_vertices, desired_indices) = if USE_EXISTING_MESH {
            let mesh = meshes.get(mesh_handle).unwrap();
            (
                mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap(),
                mesh.indices().unwrap(),
            )
        } else {
            // let mesh = meshes.get_mut(mesh_handle).unwrap();
            // mesh.set_indices(Some(Indices::U32(HEXAGON_INDICES.into())));
            // mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, hexagon_vertices.clone());
            // mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0; 3]; HEXAGON_VERTICES.len()]);
            // mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0; 2]; HEXAGON_VERTICES.len()]);

            //(&hexagon_vertices, &hexagon_indices)
            (&circle_vertices, &circle_indices)
        };
        fn convert_indices<SourceType: NumCast + Copy>(
            source: &Vec<SourceType>,
            dest: &mut Vec<Point3<usize>>,
        ) {
            for i in (0..source.len()).step_by(3) {
                dest.push(Point3::<usize>::new(
                    num::cast(source[i]).unwrap(),
                    num::cast(source[i + 1]).unwrap(),
                    num::cast(source[i + 2]).unwrap(),
                ));
            }
        }
        match desired_indices {
            Indices::U16(v) => convert_indices(v, &mut mesh_indices),
            Indices::U32(v) => convert_indices(v, &mut mesh_indices),
        }
        if let VertexAttributeValues::Float3(positions) = desired_vertices {
            for point in positions {
                mesh_vertices.push(Point2::<RealField>::new(-point[0], point[1]));
            }
        }
        let mut fem_surface = FEMSurfaceDesc::<RealField>::new(&mesh_vertices, &mesh_indices)
            .translation(Vector2::new(blob.x, blob.y))
            //.scale(Vector2::new(20.0, 20.0))
            //.young_modulus(4.0e2)
            .young_modulus(1.0e2)
            .mass_damping(0.2)
            .build();
        let collider_desc = fem_surface.boundary_collider_desc();
        let body_handle = bodies.insert(fem_surface);
        let collider_handle = colliders.insert(collider_desc.build(body_handle));
        commands.insert_one(entity, NPBodyHandleComponent::from(body_handle));
        commands.insert_one(entity, NPColliderHandleComponent::from(collider_handle));
    }
}

pub fn sync_rapier_to_nphysics_system(
    mut nphysics_bodies: ResMut<DefaultBodySet<RealField>>,
    rapier_bodies: Res<RigidBodySet>,
    query: Query<(&NPBodyHandleComponent, &RigidBodyHandleComponent)>,
) {
    for (nphysics_body_handle, rapier_body_handle) in query.iter() {
        // Let nphysics know that it doesn't have control of this body.
        if let Some(nphysics_body) = nphysics_bodies.get_mut(nphysics_body_handle.handle()) {
            nphysics_body.set_status(BodyStatus::Kinematic);
        }

        // Copy over the transform updates from rapier to nphysics.
        if let Some(nphysics_rigid_body) =
            nphysics_bodies.rigid_body_mut(nphysics_body_handle.handle())
        {
            if let Some(rapier_body) = rapier_bodies.get(rapier_body_handle.handle()) {
                nphysics_rigid_body.set_position(rapier_body.position().clone());
                nphysics_rigid_body.set_linear_velocity(rapier_body.linvel().clone());
                nphysics_rigid_body.set_angular_velocity(rapier_body.angvel());
            }
        }
    }
}

// /// A component to store the previous position of a body to use for
// /// interpolation between steps
// pub struct PhysicsInterpolationComponent(pub Option<Isometry<f32>>);
//
// impl Default for PhysicsInterpolationComponent {
//     fn default() -> Self {
//         PhysicsInterpolationComponent(None)
//     }
// }
//
// impl PhysicsInterpolationComponent {
//     /// Create a new PhysicsInterpolationComponent from a translation and rotation
//     #[cfg(feature = "dim2")]
//     pub fn new(translation: Vec2, rotation_angle: f32) -> Self {
//         Self(Some(Isometry::from_parts(
//             Translation::from(Vector::new(translation.x, translation.y)),
//             UnitComplex::new(rotation_angle),
//         )))
//     }
//
//     /// Create a new PhysicsInterpolationComponent from a translation and rotation
//     #[cfg(feature = "dim3")]
//     pub fn new(translation: Vec3, rotation: Quat) -> Self {
//         Self(Some(Isometry::from_parts(
//             Translation::from(Vector::new(translation.x, translation.y, translation.z)),
//             UnitQuaternion::from_quaternion(Quaternion::new(
//                 rotation.x, rotation.y, rotation.z, rotation.w,
//             )),
//         )))
//     }
// }

/// Difference between simulation and rendering time
#[derive(Default)]
pub struct SimulationToRenderTime {
    /// Difference between simulation and rendering time
    pub diff: f32,
}

pub fn step_system(
    (time, mut sim_to_render_time): (Res<Time>, ResMut<SimulationToRenderTime>),
    mut mechanical_world: ResMut<DefaultMechanicalWorld<RealField>>,
    mut geometrical_world: ResMut<DefaultGeometricalWorld<RealField>>,
    mut bodies: ResMut<DefaultBodySet<RealField>>,
    mut colliders: ResMut<DefaultColliderSet<RealField>>,
    mut joint_constraints: ResMut<DefaultJointConstraintSet<RealField>>,
    mut force_generators: ResMut<DefaultForceGeneratorSet<RealField>>,
) {
    sim_to_render_time.diff += time.delta_seconds();

    let sim_dt = mechanical_world.timestep();
    while sim_to_render_time.diff >= sim_dt {
        mechanical_world.step(
            &mut *geometrical_world,
            &mut *bodies,
            &mut *colliders,
            &mut *joint_constraints,
            &mut *force_generators,
        );
        sim_to_render_time.diff -= sim_dt;
    }
}

// pub fn step_system(
//     mut mechanical_world: ResMut<DefaultMechanicalWorld<RealField>>,
//     mut geometrical_world: ResMut<DefaultGeometricalWorld<RealField>>,
//     mut bodies: ResMut<DefaultBodySet<RealField>>,
//     mut colliders: ResMut<DefaultColliderSet<RealField>>,
//     mut joint_constraints: ResMut<DefaultJointConstraintSet<RealField>>,
//     mut force_generators: ResMut<DefaultForceGeneratorSet<RealField>>,
// ) {
//     mechanical_world.step(
//         &mut *geometrical_world,
//         &mut *bodies,
//         &mut *colliders,
//         &mut *joint_constraints,
//         &mut *force_generators,
//     );
// }

pub fn sync_transform_system(
    bodies: Res<DefaultBodySet<RealField>>,
    colliders: Res<DefaultColliderSet<RealField>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut transform_query: Query<(&NPBodyHandleComponent, &mut Transform)>,
    mesh_query: Query<(&NPColliderHandleComponent, &Handle<Mesh>)>,
) {
    for (body_handle, mut transform) in transform_query.iter_mut() {
        if let Some(body) = bodies.get(body_handle.handle()) {
            if body.deformed_positions().is_some() {
                continue;
            }
            let pos = body.part(0).unwrap().position();
            transform.translation.x = pos.translation.vector.x;
            transform.translation.y = pos.translation.vector.y;
            transform.rotation = Quat::from_rotation_z(pos.rotation.angle());
        }
    }
    for (collider_handle, mesh_handle) in mesh_query.iter() {
        if let Some(collider) = colliders.get(collider_handle.handle()) {
            if let Some(shape) = collider.shape().as_shape::<Polyline<RealField>>() {
                let vertices = shape.points();
                let edges = shape.edges();
                let mut x_keys = vec![];
                let mut y_keys = vec![];
                let mut previous = None;
                let mut count = 0;
                // XXX: ugly and not proven to be correct (Assumes to/from order respected)
                while count < edges.len() + 2 {
                    for edge in edges.iter() {
                        let from_index = edge.indices[0];
                        let to_index = edge.indices[1];
                        let from = vertices[from_index];
                        let to = vertices[to_index];
                        let t = count as f32;
                        match previous {
                            None => {
                                x_keys.push(Key::new(t, from[0], Interpolation::CatmullRom));
                                y_keys.push(Key::new(t, from[1], Interpolation::CatmullRom));
                                x_keys.push(Key::new(t + 1.0, to[0], Interpolation::CatmullRom));
                                y_keys.push(Key::new(t + 1.0, to[1], Interpolation::CatmullRom));
                                previous = Some(to_index);
                                count += 1;
                            }
                            Some(index) if index == from_index => {
                                x_keys.push(Key::new(t + 1.0, to[0], Interpolation::CatmullRom));
                                y_keys.push(Key::new(t + 1.0, to[1], Interpolation::CatmullRom));
                                previous = Some(to_index);
                                count += 1;
                            }
                            Some(_) => {}
                        }
                    }
                }
                let mut path_builder = Path::builder();
                path_builder.move_to(point(x_keys[1].value, y_keys[1].value));
                let x_spline = Spline::from_vec(x_keys);
                let y_spline = Spline::from_vec(y_keys);
                const SUBDIVISIONS: usize = 4;
                for i in 0..(count * SUBDIVISIONS) {
                    let t = (i as f32) / (SUBDIVISIONS as f32);
                    if let Some(x) = x_spline.sample(t) {
                        if let Some(y) = y_spline.sample(t) {
                            path_builder.line_to(point(x, y));
                        }
                    }
                }
                path_builder.close();
                let path = path_builder.build();
                let mut tesselator = FillTessellator::new();
                let mut geometry: VertexBuffers<[f32; 3], u32> = VertexBuffers::new();
                tesselator
                    .tessellate_path(
                        &path,
                        &FillOptions::default(),
                        &mut BuffersBuilder::new(
                            &mut geometry,
                            |pos: lyon::math::Point, _: FillAttributes| [pos.x, pos.y, 0.0],
                        ),
                    )
                    .unwrap();

                let mesh = meshes.get_mut(mesh_handle).unwrap();
                let vertex_count = geometry.vertices.len();
                mesh.set_indices(Some(Indices::U32(geometry.indices)));
                mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, geometry.vertices);
                mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0; 3]; vertex_count]);
                mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0; 2]; vertex_count]);
            }
            // if let Some((_, deformed_positions)) = body.deformed_positions() {
            //     let mesh = meshes.get_mut(mesh_handle).unwrap();

            //     let mut positions: Vec<[f32; 3]> = Vec::new();
            //     for i in 0..(deformed_positions.len() / 2) {
            //         positions.push([deformed_positions[i], deformed_positions[i + 1], 0.0]);
            //     }
            //     info!("{:?}", positions);

            //     mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            // }
        }
    }
}
// const HEXAGON_VERTICES: [[f32; 3]; 3] = [[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]];
// const HEXAGON_INDICES: [u32; 3] = [1, 0, 2];
//const QUAD_VERTICES: [[f32; 3]; 8] = [
//    [0.0, 0.0, 0.0],
//    [0.0, 1.0, 0.0],
//    [1.0, 0.0, 0.0],
//    [1.0, 1.0, 0.0],
//    [2.0, 0.0, 0.0],
//    [2.0, 1.0, 0.0],
//    [3.0, 0.0, 0.0],
//    [3.0, 1.0, 0.0],
//];
//const QUAD_INDICES: [u32; 18] = [
//    0 + 1,
//    0 + 0,
//    0 + 2,
//    0 + 1,
//    0 + 2,
//    0 + 3,
//    2 + 1,
//    2 + 0,
//    2 + 2,
//    2 + 1,
//    2 + 2,
//    2 + 3,
//    4 + 1,
//    4 + 0,
//    4 + 2,
//    4 + 1,
//    4 + 2,
//    4 + 3,
//];
const HEXAGON_VERTICES: [[f32; 3]; 7] = [
    [-1.0, 1.0, 0.0],  // 0 top left
    [1.0, 1.0, 0.0],   // 1 top right
    [-2.0, 0.0, 0.0],  // 2 left
    [0.0, 0.0, 0.0],   // 3 mid
    [2.0, 0.0, 0.0],   // 4 right
    [-1.0, -1.0, 0.0], // 5 bottom left
    [1.0, -1.0, 0.0],  // 6 bottom right
];
const HEXAGON_INDICES: [u32; 18] = [
    0, 3, 1, // top
    0, 2, 3, // top left
    1, 3, 4, // top right
    2, 5, 3, // bottom left
    6, 4, 3, // bottom right
    5, 6, 3, // bottom
];
