#[cfg(feature = "dim3")]
use crate::objects::node::GraphicsNode;
use kiss3d::window::Window;
#[cfg(feature = "dim3")]
use na::Vector3;
use na::{Point3, RealField};
#[cfg(feature = "dim2")]
use nphysics::math::{Point, Vector};
use nphysics::object::{DefaultColliderHandle, DefaultColliderSet};
#[cfg(feature = "dim3")]
use num::Zero;

#[cfg(feature = "dim3")]
pub struct Plane {
    gfx: GraphicsNode,
    collider: DefaultColliderHandle,
}

#[cfg(feature = "dim2")]
pub struct Plane {
    color: Point3<f32>,
    base_color: Point3<f32>,
    position: Point<f32>,
    normal: na::Unit<Vector<f32>>,
    collider: DefaultColliderHandle,
}

impl Plane {
    #[cfg(feature = "dim2")]
    pub fn new<N: RealField>(
        collider: DefaultColliderHandle,
        colliders: &DefaultColliderSet<N>,
        position: &Point<f32>,
        normal: &Vector<f32>,
        color: Point3<f32>,
        _: &mut Window,
    ) -> Plane {
        let mut res = Plane {
            color,
            base_color: color,
            position: *position,
            normal: na::Unit::new_normalize(*normal),
            collider,
        };

        res.update(colliders);
        res
    }

    #[cfg(feature = "dim3")]
    pub fn new<N: RealField>(
        collider: DefaultColliderHandle,
        colliders: &DefaultColliderSet<N>,
        world_pos: &Point3<f32>,
        world_normal: &Vector3<f32>,
        color: Point3<f32>,
        window: &mut Window,
    ) -> Plane {
        let mut res = Plane {
            gfx: window.add_quad(100.0, 100.0, 10, 10),
            collider,
        };

        if colliders
            .get(collider)
            .unwrap()
            .query_type()
            .is_proximity_query()
        {
            res.gfx.set_surface_rendering_activation(false);
            res.gfx.set_lines_width(1.0);
        }

        res.gfx.set_color(color.x, color.y, color.z);

        let up = if world_normal.z.is_zero() && world_normal.y.is_zero() {
            Vector3::z()
        } else {
            Vector3::x()
        };

        res.gfx
            .reorient(world_pos, &(*world_pos + *world_normal), &up);

        res.update(colliders);

        res
    }

    pub fn select(&mut self) {}

    pub fn unselect(&mut self) {}

    pub fn update<N: RealField>(&mut self, _: &DefaultColliderSet<N>) {
        // FIXME: atm we assume the plane does not move
    }

    #[cfg(feature = "dim3")]
    pub fn set_color(&mut self, color: Point3<f32>) {
        self.gfx.set_color(color.x, color.y, color.z);
    }

    #[cfg(feature = "dim2")]
    pub fn set_color(&mut self, color: Point3<f32>) {
        self.color = color;
        self.base_color = color;
    }

    #[cfg(feature = "dim3")]
    pub fn scene_node(&self) -> &GraphicsNode {
        &self.gfx
    }

    #[cfg(feature = "dim3")]
    pub fn scene_node_mut(&mut self) -> &mut GraphicsNode {
        &mut self.gfx
    }

    pub fn object(&self) -> DefaultColliderHandle {
        self.collider
    }

    #[cfg(feature = "dim2")]
    pub fn draw(&mut self, window: &mut Window) {
        let orth = Vector::new(-self.normal.y, self.normal.x);
        window.draw_planar_line(
            &(self.position - orth * 50.0),
            &(self.position + orth * 50.0),
            &self.color,
        );
    }
}
