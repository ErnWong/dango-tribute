#[cfg(feature = "dim3")]
use kiss3d::camera::ArcBall as Camera;
#[cfg(feature = "dim2")]
use kiss3d::planar_camera::Sidescroll as Camera;
use kiss3d::window::Window;
use na;
#[cfg(feature = "dim2")]
use na::Translation2 as Translation;
#[cfg(feature = "dim3")]
use na::Translation3 as Translation;
use na::{Point3, RealField};
use ncollide::shape::{self, Compound, Cuboid, Shape};

use crate::objects::ball::Ball;
use crate::objects::box_node::Box;
use crate::objects::capsule::Capsule;
use crate::objects::convex::Convex;
#[cfg(feature = "fluids")]
use crate::objects::fluid::Fluid as FluidNode;
use crate::objects::heightfield::HeightField;
#[cfg(feature = "dim3")]
use crate::objects::mesh::Mesh;
use crate::objects::node::{GraphicsNode, Node};
use crate::objects::plane::Plane;
#[cfg(feature = "dim2")]
use crate::objects::polyline::Polyline;
#[cfg(feature = "fluids")]
use crate::objects::FluidRenderingMode;
use ncollide::pipeline::CollisionGroups;
use ncollide::query::Ray;
#[cfg(feature = "dim2")]
use ncollide::shape::ConvexPolygon;
#[cfg(feature = "dim3")]
use ncollide::shape::{ConvexHull, TriMesh};
#[cfg(feature = "dim3")]
use ncollide::transformation;
use nphysics::math::{Isometry, Point, Vector};
use nphysics::object::{
    ColliderAnchor, DefaultBodyHandle, DefaultBodyPartHandle, DefaultColliderHandle,
    DefaultColliderSet,
};
use nphysics::world::DefaultGeometricalWorld;
use rand::{rngs::StdRng, Rng, SeedableRng};
#[cfg(feature = "fluids")]
use salva::object::{Boundary, BoundaryHandle, Fluid, FluidHandle};
#[cfg(feature = "fluids")]
use salva::LiquidWorld;
use std::collections::HashMap;

pub trait GraphicsWindow {
    fn remove_graphics_node(&mut self, node: &mut GraphicsNode);
    fn draw_graphics_line(&mut self, p1: &Point<f32>, p2: &Point<f32>, color: &Point3<f32>);
}

impl GraphicsWindow for Window {
    fn remove_graphics_node(&mut self, node: &mut GraphicsNode) {
        #[cfg(feature = "dim2")]
        self.remove_planar_node(node);
        #[cfg(feature = "dim3")]
        self.remove_node(node);
    }

    fn draw_graphics_line(&mut self, p1: &Point<f32>, p2: &Point<f32>, color: &Point3<f32>) {
        #[cfg(feature = "dim2")]
        self.draw_planar_line(p1, p2, color);
        #[cfg(feature = "dim3")]
        self.draw_line(p1, p2, color);
    }
}

pub struct GraphicsManager {
    rand: StdRng,
    b2sn: HashMap<DefaultBodyHandle, Vec<Node>>,
    #[cfg(feature = "fluids")]
    f2sn: HashMap<FluidHandle, FluidNode>,
    #[cfg(feature = "fluids")]
    boundary2sn: HashMap<BoundaryHandle, FluidNode>,
    b2color: HashMap<DefaultBodyHandle, Point3<f32>>,
    c2color: HashMap<DefaultColliderHandle, Point3<f32>>,
    b2wireframe: HashMap<DefaultBodyHandle, bool>,
    #[cfg(feature = "fluids")]
    f2color: HashMap<FluidHandle, Point3<f32>>,
    ground_color: Point3<f32>,
    rays: Vec<Ray<f32>>,
    camera: Camera,
    aabbs: Vec<(DefaultColliderHandle, GraphicsNode)>,
    ground_handle: Option<DefaultBodyHandle>,
    #[cfg(feature = "fluids")]
    fluid_rendering_mode: FluidRenderingMode,
    #[cfg(feature = "fluids")]
    render_boundary_particles: bool,
}

impl GraphicsManager {
    pub fn new() -> GraphicsManager {
        let mut camera;

        #[cfg(feature = "dim3")]
        {
            camera = Camera::new(Point3::new(10.0, 10.0, 10.0), Point3::new(0.0, 0.0, 0.0));
            camera.set_rotate_modifiers(Some(kiss3d::event::Modifiers::Control));
        }

        #[cfg(feature = "dim2")]
        {
            camera = Camera::new();
            camera.set_zoom(50.0);
        }

        GraphicsManager {
            camera,
            rand: StdRng::seed_from_u64(0),
            b2sn: HashMap::new(),
            #[cfg(feature = "fluids")]
            f2sn: HashMap::new(),
            #[cfg(feature = "fluids")]
            boundary2sn: HashMap::new(),
            b2color: HashMap::new(),
            c2color: HashMap::new(),
            #[cfg(feature = "fluids")]
            f2color: HashMap::new(),
            ground_color: Point3::new(0.5, 0.5, 0.5),
            b2wireframe: HashMap::new(),
            rays: Vec::new(),
            aabbs: Vec::new(),
            ground_handle: None,
            #[cfg(feature = "fluids")]
            fluid_rendering_mode: FluidRenderingMode::StaticColor,
            #[cfg(feature = "fluids")]
            render_boundary_particles: false,
        }
    }

    pub fn set_ground_handle(&mut self, handle: Option<DefaultBodyHandle>) {
        self.ground_handle = handle
    }

    #[cfg(feature = "fluids")]
    pub fn set_fluid_rendering_mode(&mut self, mode: FluidRenderingMode) {
        self.fluid_rendering_mode = mode;
    }

    #[cfg(feature = "fluids")]
    pub fn enable_boundary_particles_rendering(&mut self, enabled: bool) {
        self.render_boundary_particles = enabled;

        for sn in self.boundary2sn.values_mut() {
            sn.scene_node_mut().set_visible(enabled);
        }
    }

    pub fn clear(&mut self, window: &mut Window) {
        for sns in self.b2sn.values_mut() {
            for sn in sns.iter_mut() {
                if let Some(node) = sn.scene_node_mut() {
                    window.remove_graphics_node(node);
                }
            }
        }

        #[cfg(feature = "fluids")]
        for sn in self.f2sn.values_mut().chain(self.boundary2sn.values_mut()) {
            let node = sn.scene_node_mut();
            window.remove_graphics_node(node);
        }

        for aabb in self.aabbs.iter_mut() {
            window.remove_graphics_node(&mut aabb.1);
        }

        self.b2sn.clear();
        #[cfg(feature = "fluids")]
        self.f2sn.clear();
        #[cfg(feature = "fluids")]
        self.boundary2sn.clear();
        self.aabbs.clear();
        self.rays.clear();
        self.b2color.clear();
        self.c2color.clear();
        self.b2wireframe.clear();
        self.rand = StdRng::seed_from_u64(0);
    }

    pub fn remove_body_nodes(&mut self, window: &mut Window, body: DefaultBodyHandle) {
        if let Some(sns) = self.b2sn.get_mut(&body) {
            for sn in sns.iter_mut() {
                if let Some(node) = sn.scene_node_mut() {
                    window.remove_graphics_node(node);
                }
            }
        }

        self.b2sn.remove(&body);
    }

    pub fn remove_body_part_nodes<N: RealField>(
        &mut self,
        colliders: &DefaultColliderSet<N>,
        window: &mut Window,
        part: DefaultBodyPartHandle,
    ) -> DefaultBodyPartHandle {
        let mut delete_array = true;

        if let Some(sns) = self.b2sn.get_mut(&part.0) {
            sns.retain(|sn| {
                if let ColliderAnchor::OnBodyPart { body_part, .. } =
                    colliders.get(sn.collider()).unwrap().anchor()
                {
                    if *body_part == part {
                        if let Some(node) = sn.scene_node() {
                            window.remove_graphics_node(&mut node.clone());
                        }
                        false
                    } else {
                        delete_array = false;
                        true
                    }
                } else {
                    delete_array = false;
                    true
                }
            });
        }

        if delete_array {
            self.b2sn.remove(&part.0);
        }

        part
    }

    pub fn update_after_body_key_change(
        &mut self,
        colliders: &DefaultColliderSet<f32>,
        body_key: DefaultBodyHandle,
    ) {
        if let Some(color) = self.b2color.remove(&body_key) {
            if let Some(sns) = self.b2sn.remove(&body_key) {
                for sn in sns {
                    let sn_key = colliders.get(sn.collider()).unwrap().body();

                    let _ = self.b2color.entry(sn_key).or_insert(color);
                    let new_sns = self.b2sn.entry(sn_key).or_insert_with(Vec::new);
                    new_sns.push(sn);
                }
            }
        }
    }

    #[cfg(feature = "fluids")]
    pub fn set_fluid_color(&mut self, f: FluidHandle, color: Point3<f32>) {
        self.f2color.insert(f, color);

        if let Some(n) = self.f2sn.get_mut(&f) {
            n.set_color(color)
        }
    }

    pub fn set_body_color(&mut self, b: DefaultBodyHandle, color: Point3<f32>) {
        self.b2color.insert(b, color);

        if let Some(ns) = self.b2sn.get_mut(&b) {
            for n in ns.iter_mut() {
                n.set_color(color)
            }
        }
    }

    pub fn set_body_wireframe(&mut self, b: DefaultBodyHandle, enabled: bool) {
        self.b2wireframe.insert(b, enabled);

        if let Some(ns) = self.b2sn.get_mut(&b) {
            for n in ns.iter_mut().filter_map(|n| n.scene_node_mut()) {
                if enabled {
                    n.set_surface_rendering_activation(true);
                    n.set_lines_width(1.0);
                } else {
                    n.set_surface_rendering_activation(false);
                    n.set_lines_width(1.0);
                }
            }
        }
    }

    pub fn set_collider_color(&mut self, handle: DefaultColliderHandle, color: Point3<f32>) {
        self.c2color.insert(handle, color);
    }

    fn gen_color(rng: &mut StdRng) -> Point3<f32> {
        let mut color: Point3<f32> = rng.gen();
        color *= 1.5;
        color.x = color.x.min(1.0);
        color.y = color.y.min(1.0);
        color.z = color.z.min(1.0);
        color
    }

    fn alloc_color(&mut self, handle: DefaultBodyHandle) -> Point3<f32> {
        let mut color = self.ground_color;

        match self.b2color.get(&handle).cloned() {
            Some(c) => color = c,
            None => {
                if Some(handle) != self.ground_handle {
                    color = Self::gen_color(&mut self.rand)
                }
            }
        }

        self.set_body_color(handle, color);

        color
    }

    pub fn toggle_wireframe_mode<N: RealField>(
        &mut self,
        colliders: &DefaultColliderSet<N>,
        enabled: bool,
    ) {
        for n in self.b2sn.values_mut().flat_map(|val| val.iter_mut()) {
            let force_wireframe = if let Some(collider) = colliders.get(n.collider()) {
                collider.is_sensor()
                    || self
                        .b2wireframe
                        .get(&collider.body())
                        .cloned()
                        .unwrap_or(false)
            } else {
                false
            };

            if let Some(node) = n.scene_node_mut() {
                if force_wireframe || enabled {
                    node.set_lines_width(1.0);
                    node.set_surface_rendering_activation(false);
                } else {
                    node.set_lines_width(0.0);
                    node.set_surface_rendering_activation(true);
                }
            }
        }
    }

    pub fn add_ray(&mut self, ray: Ray<f32>) {
        self.rays.push(ray)
    }

    #[cfg(feature = "fluids")]
    pub fn add_fluid<N: RealField>(
        &mut self,
        window: &mut Window,
        handle: FluidHandle,
        fluid: &Fluid<N>,
        particle_radius: N,
    ) {
        let rand = &mut self.rand;
        let color = *self
            .f2color
            .entry(handle)
            .or_insert_with(|| Self::gen_color(rand));

        self.add_fluid_with_color(window, handle, fluid, particle_radius, color);
    }

    #[cfg(feature = "fluids")]
    pub fn add_boundary<N: RealField>(
        &mut self,
        window: &mut Window,
        handle: BoundaryHandle,
        boundary: &Boundary<N>,
        particle_radius: N,
    ) {
        let color = self.ground_color;
        let node = FluidNode::new(
            na::convert_unchecked(particle_radius) as f32,
            &boundary.positions,
            color,
            window,
        );
        self.boundary2sn.insert(handle, node);
    }

    #[cfg(feature = "fluids")]
    pub fn add_fluid_with_color<N: RealField>(
        &mut self,
        window: &mut Window,
        handle: FluidHandle,
        fluid: &Fluid<N>,
        particle_radius: N,
        color: Point3<f32>,
    ) {
        let node = FluidNode::new(
            na::convert_unchecked(particle_radius) as f32,
            &fluid.positions,
            color,
            window,
        );
        self.f2sn.insert(handle, node);
    }

    pub fn add<N: RealField>(
        &mut self,
        window: &mut Window,
        id: DefaultColliderHandle,
        colliders: &DefaultColliderSet<N>,
    ) {
        let collider = colliders.get(id).unwrap();

        let color = if let Some(c) = self.c2color.get(&id).cloned() {
            c
        } else if let Some(c) = self.b2color.get(&collider.body()).cloned() {
            c
        } else {
            self.alloc_color(collider.body())
        };

        self.add_with_color(window, id, colliders, color)
    }

    pub fn add_with_color<N: RealField>(
        &mut self,
        window: &mut Window,
        id: DefaultColliderHandle,
        colliders: &DefaultColliderSet<N>,
        color: Point3<f32>,
    ) {
        let collider = colliders.get(id).unwrap();
        let key = collider.body();
        let shape = collider.shape();

        // NOTE: not optimal allocation-wise, but it is not critical here.
        let mut new_nodes = Vec::new();
        self.add_shape(
            window,
            id,
            colliders,
            na::one(),
            shape,
            color,
            &mut new_nodes,
        );

        {
            for node in new_nodes.iter_mut().filter_map(|n| n.scene_node_mut()) {
                if self
                    .b2wireframe
                    .get(&collider.body())
                    .cloned()
                    .unwrap_or(false)
                {
                    node.set_lines_width(1.0);
                    node.set_surface_rendering_activation(false);
                } else {
                    node.set_lines_width(0.0);
                    node.set_surface_rendering_activation(true);
                }
            }

            let nodes = self.b2sn.entry(key).or_insert_with(Vec::new);
            nodes.append(&mut new_nodes);
        }
    }

    fn add_shape<N: RealField>(
        &mut self,
        window: &mut Window,
        object: DefaultColliderHandle,
        colliders: &DefaultColliderSet<N>,
        delta: Isometry<f32>,
        shape: &dyn Shape<N>,
        color: Point3<f32>,
        out: &mut Vec<Node>,
    ) {
        if let Some(s) = shape.as_shape::<shape::Plane<N>>() {
            self.add_plane(window, object, colliders, s, color, out)
        } else if let Some(s) = shape.as_shape::<shape::Ball<N>>() {
            self.add_ball(window, object, colliders, delta, s, color, out)
        } else if let Some(s) = shape.as_shape::<Cuboid<N>>() {
            self.add_box(window, object, colliders, delta, s, color, out)
        } else if let Some(s) = shape.as_shape::<shape::Capsule<N>>() {
            self.add_capsule(window, object, colliders, delta, s, color, out)
        } else if let Some(s) = shape.as_shape::<Compound<N>>() {
            for &(t, ref s) in s.shapes().iter() {
                let t: Isometry<f64> = na::convert_unchecked(t);
                let t: Isometry<f32> = na::convert(t);
                self.add_shape(window, object, colliders, delta * t, s.as_ref(), color, out)
            }
        }

        #[cfg(feature = "dim2")]
        {
            if let Some(s) = shape.as_shape::<ConvexPolygon<N>>() {
                self.add_convex(window, object, colliders, delta, s, color, out)
            } else if let Some(s) = shape.as_shape::<shape::Polyline<N>>() {
                self.add_polyline(window, object, colliders, delta, s, color, out);
            } else if let Some(s) = shape.as_shape::<shape::HeightField<N>>() {
                self.add_heightfield(window, object, colliders, delta, s, color, out);
            }
        }

        #[cfg(feature = "dim3")]
        {
            if let Some(s) = shape.as_shape::<ConvexHull<N>>() {
                self.add_convex(window, object, colliders, delta, s, color, out)
            } else if let Some(s) = shape.as_shape::<TriMesh<N>>() {
                self.add_mesh(window, object, colliders, delta, s, color, out);
            } else if let Some(s) = shape.as_shape::<shape::HeightField<N>>() {
                self.add_heightfield(window, object, colliders, delta, s, color, out);
            }
        }
    }

    fn add_plane<N: RealField>(
        &mut self,
        window: &mut Window,
        object: DefaultColliderHandle,
        colliders: &DefaultColliderSet<N>,
        shape: &shape::Plane<N>,
        color: Point3<f32>,
        out: &mut Vec<Node>,
    ) {
        let pos = colliders.get(object).unwrap().position();
        let position: Point<f64> = na::convert_unchecked(Point::from(pos.translation.vector));
        let normal: Vector<f64> = na::convert_unchecked(pos * shape.normal().into_inner());

        out.push(Node::Plane(Plane::new(
            object,
            colliders,
            &na::convert(position),
            &na::convert(normal),
            color,
            window,
        )))
    }

    #[cfg(feature = "dim2")]
    fn add_polyline<N: RealField>(
        &mut self,
        window: &mut Window,
        object: DefaultColliderHandle,
        colliders: &DefaultColliderSet<N>,
        delta: Isometry<f32>,
        shape: &shape::Polyline<N>,
        color: Point3<f32>,
        out: &mut Vec<Node>,
    ) {
        let vertices = shape
            .points()
            .iter()
            .map(|p| na::convert::<Point<f64>, Point<f32>>(na::convert_unchecked(*p)))
            .collect();
        let indices = shape.edges().iter().map(|e| e.indices).collect();

        out.push(Node::Polyline(Polyline::new(
            object, colliders, delta, vertices, indices, color, window,
        )))
    }

    #[cfg(feature = "dim3")]
    fn add_mesh<N: RealField>(
        &mut self,
        window: &mut Window,
        object: DefaultColliderHandle,
        colliders: &DefaultColliderSet<N>,
        delta: Isometry<f32>,
        shape: &TriMesh<N>,
        color: Point3<f32>,
        out: &mut Vec<Node>,
    ) {
        let is = shape
            .faces()
            .iter()
            .map(|f| {
                na::convert(Point3::new(
                    f.indices.x as u32,
                    f.indices.y as u32,
                    f.indices.z as u32,
                ))
            })
            .collect();

        let points = shape
            .points()
            .iter()
            .map(|&p| {
                let p: Point<f64> = na::convert_unchecked(p);
                na::convert(p)
            })
            .collect();

        out.push(Node::Mesh(Mesh::new(
            object, colliders, delta, points, is, color, window,
        )))
    }

    fn add_heightfield<N: RealField>(
        &mut self,
        window: &mut Window,
        object: DefaultColliderHandle,
        colliders: &DefaultColliderSet<N>,
        delta: Isometry<f32>,
        heightfield: &shape::HeightField<N>,
        color: Point3<f32>,
        out: &mut Vec<Node>,
    ) {
        out.push(Node::HeightField(HeightField::new(
            object,
            colliders,
            delta,
            heightfield,
            color,
            window,
        )))
    }

    fn add_capsule<N: RealField>(
        &mut self,
        window: &mut Window,
        object: DefaultColliderHandle,
        colliders: &DefaultColliderSet<N>,
        delta: Isometry<f32>,
        shape: &shape::Capsule<N>,
        color: Point3<f32>,
        out: &mut Vec<Node>,
    ) {
        let margin = colliders.get(object).unwrap().margin();
        out.push(Node::Capsule(Capsule::new(
            object,
            colliders,
            delta,
            na::convert_unchecked(shape.radius() + margin) as f32,
            na::convert_unchecked(shape.height()) as f32,
            color,
            window,
        )))
    }

    fn add_ball<N: RealField>(
        &mut self,
        window: &mut Window,
        object: DefaultColliderHandle,
        colliders: &DefaultColliderSet<N>,
        delta: Isometry<f32>,
        shape: &shape::Ball<N>,
        color: Point3<f32>,
        out: &mut Vec<Node>,
    ) {
        let margin = colliders.get(object).unwrap().margin();
        out.push(Node::Ball(Ball::new(
            object,
            colliders,
            delta,
            na::convert_unchecked(shape.radius() + margin) as f32,
            color,
            window,
        )))
    }

    fn add_box<N: RealField>(
        &mut self,
        window: &mut Window,
        object: DefaultColliderHandle,
        colliders: &DefaultColliderSet<N>,
        delta: Isometry<f32>,
        shape: &Cuboid<N>,
        color: Point3<f32>,
        out: &mut Vec<Node>,
    ) {
        let margin = colliders.get(object).unwrap().margin();

        out.push(Node::Box(Box::new(
            object,
            colliders,
            delta,
            na::convert(na::convert_unchecked::<_, Vector<f64>>(
                shape.half_extents() + Vector::repeat(margin),
            )),
            color,
            window,
        )))
    }

    #[cfg(feature = "dim2")]
    fn add_convex<N: RealField>(
        &mut self,
        window: &mut Window,
        object: DefaultColliderHandle,
        colliders: &DefaultColliderSet<N>,
        delta: Isometry<f32>,
        shape: &ConvexPolygon<N>,
        color: Point3<f32>,
        out: &mut Vec<Node>,
    ) {
        let points: Vec<_> = shape
            .points()
            .iter()
            .map(|p| na::convert::<Point<f64>, Point<f32>>(na::convert_unchecked(*p)))
            .collect();

        out.push(Node::Convex(Convex::new(
            object, colliders, delta, points, color, window,
        )))
    }

    #[cfg(feature = "dim3")]
    fn add_convex<N: RealField>(
        &mut self,
        window: &mut Window,
        object: DefaultColliderHandle,
        colliders: &DefaultColliderSet<N>,
        delta: Isometry<f32>,
        shape: &ConvexHull<N>,
        color: Point3<f32>,
        out: &mut Vec<Node>,
    ) {
        let points: Vec<_> = shape
            .points()
            .iter()
            .map(|p| {
                let p: Point<f64> = na::convert_unchecked(*p);
                na::convert(p)
            })
            .collect();
        let mut chull = transformation::convex_hull(&points);
        chull.replicate_vertices();
        chull.recompute_normals();

        out.push(Node::Convex(Convex::new(
            object, colliders, delta, &chull, color, window,
        )))
    }

    pub fn show_aabbs<N: RealField>(
        &mut self,
        _geometrical_world: &DefaultGeometricalWorld<N>,
        colliders: &DefaultColliderSet<N>,
        window: &mut Window,
    ) {
        for (_, ns) in self.b2sn.iter() {
            for n in ns.iter() {
                let handle = n.collider();
                if let Some(collider) = colliders.get(handle) {
                    let color = if let Some(c) = self.c2color.get(&handle).cloned() {
                        c
                    } else {
                        self.b2color[&collider.body()]
                    };

                    #[cfg(feature = "dim2")]
                    let mut cube = window.add_rectangle(1.0, 1.0);
                    #[cfg(feature = "dim3")]
                    let mut cube = window.add_cube(1.0, 1.0, 1.0);
                    cube.set_surface_rendering_activation(false);
                    cube.set_lines_width(5.0);
                    cube.set_color(color.x, color.y, color.z);
                    self.aabbs.push((handle, cube));
                }
            }
        }
    }

    pub fn hide_aabbs(&mut self, window: &mut Window) {
        for mut aabb in self.aabbs.drain(..) {
            window.remove_graphics_node(&mut aabb.1)
        }
    }

    #[cfg(feature = "fluids")]
    pub fn draw_fluids<N: RealField>(&mut self, liquid_world: &LiquidWorld<N>) {
        for (i, fluid) in liquid_world.fluids().iter() {
            if let Some(node) = self.f2sn.get_mut(&i) {
                node.update_with_fluid(fluid, self.fluid_rendering_mode)
            }
        }

        if self.render_boundary_particles {
            for (i, boundary) in liquid_world.boundaries().iter() {
                if let Some(node) = self.boundary2sn.get_mut(&i) {
                    node.update_with_boundary(boundary)
                }
            }
        }
    }

    pub fn draw<N: RealField>(
        &mut self,
        geometrical_world: &DefaultGeometricalWorld<N>,
        colliders: &DefaultColliderSet<N>,
        window: &mut Window,
    ) {
        //        use crate::kiss3d::camera::Camera;
        //        println!("eye: {}, at: {}", self.camera.eye(), self.camera.at());
        for (_, ns) in self.b2sn.iter_mut() {
            for n in ns.iter_mut() {
                n.update(colliders)
            }
        }

        for (_, ns) in self.b2sn.iter_mut() {
            for n in ns.iter_mut() {
                n.draw(window)
            }
        }

        for (handle, node) in &mut self.aabbs {
            if let Some(collider) = colliders.get(*handle) {
                let bf = geometrical_world.broad_phase();
                let aabb = collider
                    .proxy_handle()
                    .and_then(|h| bf.proxy(h))
                    .map(|p| p.0);

                if let Some(aabb) = aabb {
                    let mut w = aabb.half_extents();
                    w += w;

                    let center: Vector<f64> = na::convert_unchecked(aabb.center().coords);
                    let center: Vector<f32> = na::convert(center);
                    node.set_local_translation(Translation::from(center));

                    #[cfg(feature = "dim2")]
                    node.set_local_scale(
                        na::convert_unchecked(w.x) as f32,
                        na::convert_unchecked(w.y) as f32,
                    );
                    #[cfg(feature = "dim3")]
                    node.set_local_scale(
                        na::convert_unchecked::<_, f64>(w.x) as f32,
                        na::convert_unchecked::<_, f64>(w.y) as f32,
                        na::convert_unchecked::<_, f64>(w.z) as f32,
                    );
                }
            }
        }

        for &Ray { origin, dir } in &self.rays {
            let ray = Ray {
                origin: na::convert(na::convert_unchecked::<_, Point<f64>>(origin)),
                dir: na::convert(na::convert_unchecked::<_, Vector<f64>>(dir)),
            };
            let groups = CollisionGroups::new();
            let inter =
                geometrical_world.interferences_with_ray(colliders, &ray, N::max_value(), &groups);
            let hit = inter.fold(1000.0f32, |t, (_, _, hit)| {
                (na::convert_unchecked::<_, f64>(hit.toi) as f32).min(t)
            });
            let p1 = origin;
            let p2 = p1 + dir * hit;
            window.draw_graphics_line(&p1, &p2, &Point3::new(1.0, 0.0, 0.0));
        }
    }

    // pub fn draw_positions(&mut self, window: &mut Window, rbs: &RigidBodies<f32>) {
    //     for (_, ns) in self.b2sn.iter_mut() {
    //         for n in ns.iter_mut() {
    //             let object = n.object();
    //             let rb = rbs.get(object).expect("Rigid body not found.");

    //             // if let WorldObjectBorrowed::RigidBody(rb) = object {
    //                 let t      = rb.position();
    //                 let center = rb.center_of_mass();

    //                 let rotmat = t.rotation.to_rotation_matrix().unwrap();
    //                 let x = rotmat.column(0) * 0.25f32;
    //                 let y = rotmat.column(1) * 0.25f32;
    //                 let z = rotmat.column(2) * 0.25f32;

    //                 window.draw_line(center, &(*center + x), &Point3::new(1.0, 0.0, 0.0));
    //                 window.draw_line(center, &(*center + y), &Point3::new(0.0, 1.0, 0.0));
    //                 window.draw_line(center, &(*center + z), &Point3::new(0.0, 0.0, 1.0));
    //             // }
    //         }
    //     }
    // }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    #[cfg(feature = "dim3")]
    pub fn look_at(&mut self, eye: Point<f32>, at: Point<f32>) {
        self.camera.look_at(eye, at);
    }

    #[cfg(feature = "dim2")]
    pub fn look_at(&mut self, at: Point<f32>, zoom: f32) {
        self.camera.look_at(at, zoom);
    }

    pub fn body_nodes(&self, handle: DefaultBodyHandle) -> Option<&Vec<Node>> {
        self.b2sn.get(&handle)
    }

    pub fn body_nodes_mut(&mut self, handle: DefaultBodyHandle) -> Option<&mut Vec<Node>> {
        self.b2sn.get_mut(&handle)
    }

    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.b2sn.values().flat_map(|val| val.iter())
    }

    pub fn nodes_mut(&mut self) -> impl Iterator<Item = &mut Node> {
        self.b2sn.values_mut().flat_map(|val| val.iter_mut())
    }

    #[cfg(feature = "dim3")]
    pub fn set_up_axis(&mut self, up_axis: na::Vector3<f32>) {
        self.camera.set_up_axis(up_axis);
    }
}

impl Default for GraphicsManager {
    fn default() -> Self {
        Self::new()
    }
}
