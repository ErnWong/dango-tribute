extern crate nalgebra as na;

use na::{Point2, RealField, Vector2};
use ncollide2d::shape::{Cuboid, ShapeHandle};
use nphysics2d::force_generator::DefaultForceGeneratorSet;
use nphysics2d::joint::DefaultJointConstraintSet;
use nphysics2d::object::{
    BodyPartHandle, ColliderDesc, DefaultBodySet, DefaultColliderSet, Ground, RigidBodyDesc,
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
     * Create the boxes
     */
    let num = 20;
    let rad = r!(0.1);

    let cuboid = ShapeHandle::new(Cuboid::new(Vector2::repeat(rad)));

    let shift = r!(2.0) * (rad + ColliderDesc::<N>::default_margin());
    let centerx = shift * r!(num as f64) / r!(2.0);
    let centery = rad + ColliderDesc::<N>::default_margin() * r!(2.0);

    for i in 0usize..num {
        for j in i..num {
            let fj = j as f64;
            let fi = i as f64;
            let x = (fi * shift / r!(2.0)) + (fj - fi) * shift - centerx;
            let y = fi * shift + centery;

            // Build the rigid body.
            let rb = RigidBodyDesc::new().translation(Vector2::new(x, y)).build();
            let rb_handle = bodies.insert(rb);

            // Build the collider.
            let co = ColliderDesc::new(cuboid.clone())
                .density(r!(1.0))
                .build(BodyPartHandle(rb_handle, 0));
            colliders.insert(co);
        }
    }

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
    testbed.look_at(Point2::new(0.0, 3.0), 95.0);
}

fn main() {
    let testbed = Testbed::<f32>::from_builders(0, vec![("Pyramid", init_world)]);
    testbed.run()
}
