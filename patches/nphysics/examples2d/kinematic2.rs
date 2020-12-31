extern crate nalgebra as na;

use na::{Point2, RealField, Vector2};
use ncollide2d::shape::{Ball, Cuboid, ShapeHandle};
use nphysics2d::force_generator::DefaultForceGeneratorSet;
use nphysics2d::joint::{DefaultJointConstraintSet, RevoluteJoint};
use nphysics2d::math::Velocity;
use nphysics2d::object::{
    Body, BodyPartHandle, BodyStatus, ColliderDesc, DefaultBodySet, DefaultColliderSet, Ground,
    MultibodyDesc, RigidBodyDesc,
};
use nphysics2d::world::{DefaultGeometricalWorld, DefaultMechanicalWorld};
use nphysics_testbed2d::Testbed;

/*
 * NOTE: The `r` macro is only here to convert from f64 to the `N` scalar type.
 * This simplifies experimentation with various scalar types (f32, fixed-point numbers, etc.)
 */
pub fn init_world<N: RealField>(testbed: &mut Testbed<N>) {
    /*
     * World
     */
    let mechanical_world = DefaultMechanicalWorld::new(Vector2::new(r!(0.0), r!(-9.81)));
    let geometrical_world = DefaultGeometricalWorld::new();
    let mut bodies = DefaultBodySet::new();
    let mut colliders = DefaultColliderSet::new();
    let joint_constraints = DefaultJointConstraintSet::new();
    let force_generators = DefaultForceGeneratorSet::new();

    /*
     * Ground
     */
    let ground_size = r!(25.0);
    let ground_shape = ShapeHandle::new(Cuboid::new(Vector2::new(ground_size, r!(1.0))));

    let ground_handle = bodies.insert(Ground::new());
    let co = ColliderDesc::new(ground_shape)
        .translation(-Vector2::y())
        .build(BodyPartHandle(ground_handle, 0));
    colliders.insert(co);

    /*
     * Create boxes
     */
    let num = 10;
    let rad = r!(0.2);

    let cuboid = ShapeHandle::new(Cuboid::new(Vector2::repeat(rad)));
    let collider_desc = ColliderDesc::new(cuboid.clone()).density(r!(1.0));

    let shift = (rad + ColliderDesc::<N>::default_margin()) * r!(2.0);
    let centerx = shift * r!(num as f64) / r!(2.0);
    let centery = shift / r!(2.0) + r!(3.04);

    for i in 0usize..num {
        for j in 0usize..num {
            let x = r!(i as f64) * shift - centerx;
            let y = r!(j as f64) * shift + centery;

            // Build the rigid body.
            let rb = RigidBodyDesc::new().translation(Vector2::new(x, y)).build();
            let rb_handle = bodies.insert(rb);

            // Build the collider.
            let co = collider_desc.build(BodyPartHandle(rb_handle, 0));
            colliders.insert(co);
        }
    }

    /*
     * Setup a kinematic rigid body.
     */
    let platform_body = RigidBodyDesc::new()
        .translation(Vector2::new(r!(0.0), r!(1.5)))
        .velocity(Velocity::linear(r!(1.0), r!(0.0)))
        .status(BodyStatus::Kinematic)
        .build();
    let platform_handle = bodies.insert(platform_body);

    let platform_geom = ShapeHandle::new(Cuboid::new(Vector2::new(rad * r!(10.0), rad)));
    let platform_collider = ColliderDesc::new(platform_geom)
        .density(r!(1.0))
        .build(BodyPartHandle(platform_handle, 0));
    colliders.insert(platform_collider);

    /*
     * Setup a kinematic multibody.
     */
    let joint = RevoluteJoint::new(r!(0.0));

    let mut mb = MultibodyDesc::new(joint)
        .body_shift(Vector2::x() * r!(2.0))
        .parent_shift(Vector2::new(r!(5.0), r!(2.0)))
        .build();

    mb.set_status(BodyStatus::Kinematic);
    mb.generalized_velocity_mut()[0] = r!(-3.0);

    let mb_handle = bodies.insert(mb);
    let mb_collider = collider_desc.build(BodyPartHandle(mb_handle, 0));
    colliders.insert(mb_collider);

    /*
     * Setup a motorized multibody.
     */
    let mut joint = RevoluteJoint::new(r!(0.0));
    joint.set_desired_angular_motor_velocity(r!(-2.0));
    joint.set_max_angular_motor_torque(r!(2.0));
    joint.enable_angular_motor();

    let mb = MultibodyDesc::new(joint)
        .body_shift(Vector2::x() * r!(2.0))
        .parent_shift(Vector2::new(r!(-4.0), r!(3.0)))
        .build();
    let mb_handle = bodies.insert(mb);

    let geom = ShapeHandle::new(Ball::new(r!(2.0) * rad));
    let ball_collider_desc = ColliderDesc::new(geom).density(r!(1.0));
    let mb_collider = ball_collider_desc.build(BodyPartHandle(mb_handle, 0));
    colliders.insert(mb_collider);

    /*
     * Setup a callback to control the platform.
     */
    testbed.add_callback(move |_, _, bodies, _, _, time| {
        if let Some(platform) = bodies.rigid_body_mut(platform_handle) {
            let platform_x = platform.position().translation.vector.x;

            let mut vel = *platform.velocity();
            vel.linear.y = (time * r!(5.0)).sin() * r!(0.8);

            if platform_x >= rad * r!(10.0) {
                vel.linear.x = r!(-1.0);
            }
            if platform_x <= -rad * r!(10.0) {
                vel.linear.x = r!(1.0);
            }

            platform.set_velocity(vel);
        }
    });

    /*
     * Run the simulation.
     */
    testbed.set_ground_handle(Some(ground_handle));
    testbed.set_world(
        mechanical_world,
        geometrical_world,
        bodies,
        colliders,
        joint_constraints,
        force_generators,
    );
    testbed.look_at(Point2::new(0.0, 5.0), 60.0);
}

fn main() {
    let testbed = Testbed::<f32>::from_builders(0, vec![("Kinematic body", init_world)]);
    testbed.run()
}
