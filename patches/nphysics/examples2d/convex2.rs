extern crate nalgebra as na;

use rand::distributions::{Distribution, Standard};
use rand::{rngs::StdRng, SeedableRng};

use na::{Point2, RealField, Vector2};
use ncollide2d::shape::{ConvexPolygon, Cuboid, ShapeHandle};
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
    let ground_size = r!(50.0);
    let ground_shape = ShapeHandle::new(Cuboid::new(Vector2::new(ground_size, r!(1.0))));

    let ground_handle = bodies.insert(Ground::new());
    let co = ColliderDesc::new(ground_shape)
        .translation(-Vector2::y())
        .build(BodyPartHandle(ground_handle, 0));
    colliders.insert(co);

    /*
     * Create the convex geometries.
     */
    let npts = 10usize;
    let num = 25;
    let shift = r!(0.4);
    let centerx = shift * r!(num as f64) / r!(2.0);
    let centery = shift;
    let mut rng = StdRng::seed_from_u64(0);
    let distribution = Standard;

    for i in 0usize..num {
        for j in 0usize..num {
            let x = r!(i as f64) * shift - centerx;
            let y = r!(j as f64) * shift + centery;

            let mut pts = Vec::with_capacity(npts);

            for _ in 0..npts {
                let pt: Point2<f64> = distribution.sample(&mut rng);
                pts.push(na::convert::<_, Point2<N>>(pt) * r!(0.4));
            }

            // Build the rigid body.
            let rb = RigidBodyDesc::new().translation(Vector2::new(x, y)).build();
            let rb_handle = bodies.insert(rb);

            // Build the collider.
            let geom = ShapeHandle::new(ConvexPolygon::try_from_points(&pts).unwrap());
            let co = ColliderDesc::new(geom)
                .density(r!(1.0))
                .build(BodyPartHandle(rb_handle, 0));
            colliders.insert(co);
        }
    }

    /*
     * Set up the testbed.
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
    testbed.look_at(Point2::new(0.0, 2.5), 95.0);
}

fn main() {
    let testbed = Testbed::<f32>::from_builders(0, vec![("Convex", init_world)]);
    testbed.run()
}
