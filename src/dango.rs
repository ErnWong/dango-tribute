use bevy::{
    prelude::*,
    render::mesh::Indices,
    render::{
        pipeline::{PrimitiveTopology, RenderPipeline},
        render_graph::base::MainPass,
    },
    sprite::SPRITE_PIPELINE_HANDLE,
};
use lyon::{
    math::point,
    path::Path,
    tessellation::{
        basic_shapes::fill_circle, BuffersBuilder, FillAttributes, FillOptions, FillTessellator,
        VertexBuffers,
    },
};
use nphysics2d::{
    nalgebra::{Point2, Point3, Vector2},
    ncollide2d::shape::Polyline,
    object::{DefaultBodySet, DefaultColliderSet, FEMSurfaceDesc},
};
use splines::{Interpolation, Key, Spline};

use super::physics::{
    NPhysicsBodyHandleComponent, NPhysicsColliderHandleComponent, NPHYSICS_TRANSFORM_SYNC_STAGE,
};
use super::RealField;

pub struct DangoPlugin;

impl Plugin for DangoPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_resource(DangoPhysicsMesh::default())
            .add_system_to_stage(stage::PRE_UPDATE, create_dango_system.system())
            .add_system_to_stage(
                NPHYSICS_TRANSFORM_SYNC_STAGE,
                sync_dango_physics_system.system(),
            );
    }
}

pub mod colors {
    use bevy::prelude::*;
    pub const DANGO_YELLOW: Color = Color::rgb_linear(0.8672, 0.8438, 0.7266);
    pub const DANGO_GREEN: Color = Color::rgb_linear(0.7813, 0.8673, 0.7656);
    pub const DANGO_RED: Color = Color::rgb_linear(0.9023, 0.8400, 0.8400);
}

#[derive(Bundle)]
pub struct DangoBundle {
    // TODO: Is sprite needed? Can we not use the spritesheet pipeline?
    pub sprite: Sprite,
    pub mesh: Handle<Mesh>,
    pub material: Handle<ColorMaterial>,
    pub main_pass: MainPass,
    pub draw: Draw,
    pub visible: Visible,
    pub render_pipelines: RenderPipelines,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub body: NPhysicsBodyHandleComponent,
    pub collider: NPhysicsColliderHandleComponent,
}

pub struct DangoDescriptorComponent {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub color: Color,
}

pub struct DangoPhysicsMesh {
    vertices: Vec<Point2<RealField>>,
    indices: Vec<Point3<usize>>,
}

impl Default for DangoPhysicsMesh {
    fn default() -> DangoPhysicsMesh {
        let mut geometry: VertexBuffers<Point2<RealField>, usize> = VertexBuffers::new();
        fill_circle(
            lyon::math::Point::zero(),
            1.0,
            &FillOptions::tolerance(0.1),
            &mut BuffersBuilder::new(&mut geometry, |pos: lyon::math::Point| {
                // Note: Mirror the x coordinate to flip the triangleas needed for
                // Nphyiscs' FEMSurface simulation.
                Point2::new(-pos.x, pos.y)
            }),
        )
        .unwrap();
        let mut grouped_indices = vec![];
        for i in (0..geometry.indices.len()).step_by(3) {
            grouped_indices.push(Point3::<usize>::new(
                geometry.indices[i],
                geometry.indices[i + 1],
                geometry.indices[i + 2],
            ));
        }
        DangoPhysicsMesh {
            vertices: geometry.vertices,
            indices: grouped_indices,
        }
    }
}

pub fn create_dango_system(
    commands: &mut Commands,
    physics_mesh: Res<DangoPhysicsMesh>,
    mut bodies: ResMut<DefaultBodySet<RealField>>,
    mut colliders: ResMut<DefaultColliderSet<RealField>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    query: Query<(Entity, &DangoDescriptorComponent)>,
) {
    for (entity, dango_descriptor) in query.iter() {
        info!("Creating dango");
        let mut fem_surface =
            FEMSurfaceDesc::<RealField>::new(&physics_mesh.vertices, &physics_mesh.indices)
                .translation(Vector2::new(dango_descriptor.x, dango_descriptor.y))
                .scale(Vector2::new(dango_descriptor.size, dango_descriptor.size))
                .young_modulus(1.0e2)
                .mass_damping(0.2)
                .build();
        let collider_desc = fem_surface.boundary_collider_desc();
        let body_handle = bodies.insert(fem_surface);
        let collider_handle = colliders.insert(collider_desc.build(body_handle));
        commands.insert(
            entity,
            DangoBundle {
                sprite: Sprite {
                    size: Vec2::one(),
                    ..Default::default()
                },
                mesh: meshes.add(Mesh::new(PrimitiveTopology::TriangleList)),
                material: materials.add(dango_descriptor.color.into()),
                main_pass: MainPass,
                draw: Default::default(),
                visible: Visible {
                    is_transparent: true,
                    ..Default::default()
                },
                render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                    SPRITE_PIPELINE_HANDLE.typed(),
                )]),
                transform: Transform::default(),
                // transform: Transform {
                //     translation: Vec3::new(dango_descriptor.x, dango_descriptor.y, 0.0),
                //     scale: Vec3::new(dango_descriptor.size, dango_descriptor.size, 1.0),
                //     ..Default::default()
                // },
                global_transform: GlobalTransform::default(),
                body: NPhysicsBodyHandleComponent::from(body_handle),
                collider: NPhysicsColliderHandleComponent::from(collider_handle),
            },
        );
        commands.remove_one::<DangoDescriptorComponent>(entity);
    }
}

pub fn sync_dango_physics_system(
    colliders: Res<DefaultColliderSet<RealField>>,
    mut meshes: ResMut<Assets<Mesh>>,
    query: Query<(&NPhysicsColliderHandleComponent, &Handle<Mesh>)>,
) {
    for (collider_handle, mesh_handle) in query.iter() {
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
        }
    }
}
