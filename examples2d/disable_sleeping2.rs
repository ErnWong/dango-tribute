extern crate nalgebra as na;
extern crate ncollide2d;
extern crate nphysics2d;
extern crate nphysics_testbed2d;

use na::{Point2, RealField, Vector2};
use ncollide2d::shape::{Cuboid, ShapeHandle};
use nphysics2d::math::Velocity;
use nphysics2d::object::{ColliderDesc, RigidBodyDesc};
use nphysics2d::world::World;
use nphysics_testbed2d::Testbed;

/*
 * NOTE: The `r` macro is only here to convert from f64 to the `N` scalar type.
 * This simplifies experimentation with various scalar types (f32, fixed-point numbers, etc.)
 */
pub fn init_world<N: RealField>(testbed: &mut Testbed<N>) {
    /*
     * World
     */
    let mut world = World::new();
    world.set_gravity(Vector2::new(0.0, 0.0));

    /*
     * Create the box that will be deactivated.
     */
    let rad = r!(0.1);
    let cuboid = ShapeHandle::new(Cuboid::new(Vector2::new(rad, rad)));
    let collider_desc = ColliderDesc::new(cuboid).density(r!(1.0));

    /*
     * Create the body that will be deactivated.
     */
    RigidBodyDesc::new()
        .collider(&collider_desc)
        .velocity(Velocity::linear(0.099, 0.0))
        .build(&mut world);

    /*
     * Create the body that cannot be deactivated by
     * setting its sleep threshold to None.
     */
    RigidBodyDesc::new()
        .collider(&collider_desc)
        .translation(Vector2::y() * 0.3)
        .velocity(Velocity::linear(0.099, 0.0))
        .sleep_threshold(None)
        .build(&mut world);

    /*
     * Set up the testbed.
     */
    testbed.set_world(world);
    testbed.look_at(Point2::origin(), 95.0);
}

fn main() {
    let mut testbed = Testbed::new_empty();
    init_world(&mut testbed);
    testbed.run();
}
