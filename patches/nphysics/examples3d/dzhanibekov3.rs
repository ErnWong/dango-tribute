extern crate nalgebra as na;

use na::{Isometry3, Point3, RealField, Vector3};
use ncollide3d::shape::{Compound, Cuboid, ShapeHandle};
use nphysics3d::force_generator::DefaultForceGeneratorSet;
use nphysics3d::joint::DefaultJointConstraintSet;
use nphysics3d::math::Velocity;
use nphysics3d::object::{
    BodyPartHandle, ColliderDesc, DefaultBodySet, DefaultColliderSet, RigidBodyDesc,
};
use nphysics3d::world::{DefaultGeometricalWorld, DefaultMechanicalWorld};
use nphysics_testbed3d::Testbed;

/*
 * NOTE: The `r` macro is only here to convert from f64 to the `N` scalar type.
 * This simplifies experimentation with various scalar types (f32, fixed-point numbers, etc.)
 */
pub fn init_world<N: RealField>(testbed: &mut Testbed<N>) {
    /*
     * World
     */
    let mechanical_world = DefaultMechanicalWorld::new(Vector3::zeros());
    let geometrical_world = DefaultGeometricalWorld::new();
    let mut bodies = DefaultBodySet::new();
    let mut colliders = DefaultColliderSet::new();
    let joint_constraints = DefaultJointConstraintSet::new();
    let force_generators = DefaultForceGeneratorSet::new();

    /*
     * Create boxes to compute the inertia.
     */
    let mut shapes = Vec::new();
    shapes.push((
        Isometry3::identity(),
        ShapeHandle::new(Cuboid::new(Vector3::new(r!(1.0), r!(0.1), r!(0.1)))),
    ));
    shapes.push((
        Isometry3::translation(r!(0.0), r!(0.4), r!(0.0)),
        ShapeHandle::new(Cuboid::new(Vector3::new(r!(0.1), r!(0.2), r!(0.1)))),
    ));

    /*
     * Create the rigid body.
     */
    let rb = RigidBodyDesc::new()
        .velocity(Velocity::angular(r!(0.0), r!(10.0), r!(0.1)))
        .build();
    let rb_handle = bodies.insert(rb);

    /*
     * Create the collider from which the inertia will be automatically computed.
     */
    let geom = ShapeHandle::new(Compound::new(shapes));
    let co = ColliderDesc::new(geom)
        .density(r!(1.0))
        .build(BodyPartHandle(rb_handle, 0));
    colliders.insert(co);

    /*
     * Set up the testbed.
     */
    testbed.set_world(
        mechanical_world,
        geometrical_world,
        bodies,
        colliders,
        joint_constraints,
        force_generators,
    );
    testbed.look_at(Point3::new(0.0, 0.0, 5.0), Point3::new(0.0, 0.0, 0.0));
}

fn main() {
    let testbed = Testbed::<f32>::from_builders(0, vec![("Dzhanibekov effect", init_world)]);

    testbed.run()
}
