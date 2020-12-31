/*!
 * # Expected behaviour:
 * The box stands vertically until it falls asleep.
 * The box should not fall (horizontally) on the ground.
 * The box should not traverse the ground.
 *
 * # Symptoms:
 * The long, thin, box fails to collide with the plane: it just ignores it.
 *
 * # Cause:
 * The one shot contact manifold generator was incorrect in this case. This generator rotated the
 * object wrt its center to sample the contact manifold. If the object is long and the theoretical
 * contact surface is small, all contacts will be invalidated whenever the incremental contact
 * manifold will get a new point from the one-shot generator.
 *
 * # Solution:
 * Rotate the object wrt the contact center, not wrt the object center.
 *
 * # Limitations of the solution:
 * This will create only a three-points manifold for a small axis-alligned cube, instead of four.
 */

extern crate nalgebra as na;
extern crate ncollide3d;
extern crate nphysics3d;
extern crate nphysics_testbed3d;

use na::{Point3, Vector3};
use ncollide3d::shape::{Cuboid, Plane, ShapeHandle};
use nphysics3d::object::{ColliderDesc, RigidBodyDesc};
use nphysics3d::world::World;
use nphysics_testbed3d::Testbed;

fn main() {
    /*
     * World
     */
    let mut world = World::new();
    world.set_gravity(Vector3::new(r!(0.0), r!(-9.81), r!(0.0)));

    /*
     * Plane
     */
    let ground_shape = ShapeHandle::new(Plane::new(Vector3::y_axis()));
    ColliderDesc::new(ground_shape).build(&mut world);

    /*
     * Create the box
     */
    let rad = r!(0.1);
    let x = rad;
    let y = rad + 10.0;
    let z = rad;

    let geom = ShapeHandle::new(Cuboid::new(Vector3::new(rad, rad * 10.0, rad)));
    let collider_desc = ColliderDesc::new(geom).density(r!(1.0));

    RigidBodyDesc::new()
        .collider(&collider_desc)
        .translation(Vector3::new(x, y, z))
        .build(&mut world);

    /*
     * Set up the testbed.
     */
    let mut testbed = Testbed::new(world);

    testbed.look_at(Point3::new(-30.0, 30.0, -30.0), Point3::new(0.0, 0.0, 0.0));
    testbed.run();
}
