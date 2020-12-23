use bevy::prelude::*;
use nphysics2d::{
    force_generator::DefaultForceGeneratorSet,
    joint::DefaultJointConstraintSet,
    math::Inertia,
    nalgebra::{Point2, Vector2},
    object::{
        BodyPartHandle, ColliderDesc, DefaultBodyHandle, DefaultBodySet, DefaultColliderHandle,
        DefaultColliderSet, RigidBodyDesc,
    },
    world::{DefaultGeometricalWorld, DefaultMechanicalWorld},
};

pub const NPHYSICS_TRANSFORM_SYNC_STAGE: &'static str = "nphysics_transform_sync_stage";

use super::RealField;

pub struct PhysicsPlugin {
    gravity: Vector2<RealField>,
}

impl PhysicsPlugin {
    pub fn new(gravity: Vector2<RealField>) -> PhysicsPlugin {
        PhysicsPlugin { gravity }
    }
}

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_resource(DefaultMechanicalWorld::<RealField>::new(self.gravity))
            .add_resource(DefaultGeometricalWorld::<RealField>::new())
            .add_resource(DefaultBodySet::<RealField>::new())
            .add_resource(DefaultColliderSet::<RealField>::new())
            .add_resource(DefaultJointConstraintSet::<RealField>::new())
            .add_resource(DefaultForceGeneratorSet::<RealField>::new())
            .add_resource(SimulationToRenderTime::default())
            .add_system_to_stage(stage::PRE_UPDATE, create_body_and_collider_system.system())
            .add_system_to_stage(stage::UPDATE, step_system.system())
            .add_stage_before(
                stage::POST_UPDATE,
                NPHYSICS_TRANSFORM_SYNC_STAGE,
                SystemStage::parallel(),
            )
            .add_system_to_stage(
                NPHYSICS_TRANSFORM_SYNC_STAGE,
                sync_transform_system.system(),
            )
            .add_system_to_stage(NPHYSICS_TRANSFORM_SYNC_STAGE, log_body_system.system());
    }
}

pub struct NPhysicsBodyHandleComponent(DefaultBodyHandle);
pub struct NPhysicsColliderHandleComponent(DefaultColliderHandle);

impl From<DefaultBodyHandle> for NPhysicsBodyHandleComponent {
    fn from(handle: DefaultBodyHandle) -> Self {
        Self(handle)
    }
}

impl NPhysicsBodyHandleComponent {
    pub fn handle(&self) -> DefaultBodyHandle {
        self.0
    }
}

impl From<DefaultColliderHandle> for NPhysicsColliderHandleComponent {
    fn from(handle: DefaultColliderHandle) -> Self {
        Self(handle)
    }
}

impl NPhysicsColliderHandleComponent {
    pub fn handle(&self) -> DefaultColliderHandle {
        self.0
    }
}

pub fn create_body_and_collider_system(
    commands: &mut Commands,
    mut bodies: ResMut<DefaultBodySet<RealField>>,
    mut colliders: ResMut<DefaultColliderSet<RealField>>,
    query: Query<(Entity, &RigidBodyDesc<RealField>, &ColliderDesc<RealField>)>,
) {
    for (entity, body_desc, collider_desc) in query.iter() {
        let body_handle = bodies.insert(body_desc.build());
        commands.insert_one(entity, NPhysicsBodyHandleComponent::from(body_handle));
        commands.remove_one::<RigidBodyDesc<RealField>>(entity);

        let collider_handle = colliders.insert(collider_desc.build(BodyPartHandle(body_handle, 0)));
        commands.insert_one(
            entity,
            NPhysicsColliderHandleComponent::from(collider_handle),
        );
        commands.remove_one::<ColliderDesc<RealField>>(entity);
    }
}

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

    // TODO: Limit this.
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

pub fn sync_transform_system(
    bodies: Res<DefaultBodySet<RealField>>,
    mut query: Query<(&NPhysicsBodyHandleComponent, &mut Transform)>,
) {
    for (body_handle, mut transform) in query.iter_mut() {
        if let Some(body) = bodies.get(body_handle.handle()) {
            let mut pos = Point2::<RealField>::new(0.0, 0.0);
            let mut angle = 0.0;
            let mut total_inertia = Inertia::zero();
            for i in 0..body.num_parts() {
                let part = body.part(i).unwrap();
                let inertia = part.inertia();
                if inertia.linear == 0.0 || inertia.angular == 0.0 {
                    // Non-dynamic part.
                    pos += part.position().translation.vector;
                    angle += part.position().rotation.angle();
                } else {
                    pos += part.position().translation.vector * inertia.linear;
                    angle += part.position().rotation.angle() * inertia.angular;
                }
                total_inertia += part.inertia();
            }
            if total_inertia.linear != 0.0 && total_inertia.angular != 0.0 {
                pos /= total_inertia.linear;
                angle /= total_inertia.angular;
            }
            transform.translation.x = pos.x;
            transform.translation.y = pos.y;
            transform.rotation = Quat::from_rotation_z(angle);
        }
    }
}

pub struct LogBodyComponent;

pub fn log_body_system(
    bodies: Res<DefaultBodySet<RealField>>,
    query: Query<(&NPhysicsBodyHandleComponent, &LogBodyComponent)>,
) {
    for (body_handle, _) in query.iter() {
        if let Some(body) = bodies.get(body_handle.handle()) {
            info!("=======================");
            info!("Body with {} parts", body.num_parts());
            for i in 0..body.num_parts() {
                let part = body.part(i).unwrap();
                info!(
                    "Part {} angle: {} degrees",
                    i,
                    part.position().rotation.angle() / 3.14159265 * 180.0
                );
            }
            info!("=======================");
        }
    }
}
