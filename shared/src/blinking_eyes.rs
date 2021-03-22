use bevy::{prelude::*, render::mesh::Indices};
use lyon::{
    math::point,
    path::Path,
    tessellation::{
        BuffersBuilder, LineCap, LineJoin, StrokeAttributes, StrokeOptions, StrokeTessellator,
        VertexBuffers,
    },
};
use rand::prelude::*;

pub struct BlinkingEyes {
    pub eye_meshes: Vec<Handle<Mesh>>,
    pub state: EyeState,
    pub state_remaining_seconds: f32,
}

pub enum EyeState {
    Open,
    BlinkClosedFirst,
    BlinkOpen,
    BlinkClosedSecond,
}

impl BlinkingEyes {
    pub fn new(eye_meshes: Vec<Handle<Mesh>>, meshes: &mut Assets<Mesh>) -> BlinkingEyes {
        let blinking_eyes = BlinkingEyes {
            eye_meshes,
            state: EyeState::BlinkClosedSecond,
            state_remaining_seconds: 0.0,
        };

        blinking_eyes.update_meshes(meshes);

        blinking_eyes
    }

    pub fn update_meshes(&self, meshes: &mut Assets<Mesh>) {
        let path = match self.state {
            EyeState::Open | EyeState::BlinkOpen => {
                let mut builder = Path::builder();
                builder.move_to(point(0.0, -0.2));
                builder.line_to(point(0.0, 0.2));
                builder.close();
                builder.build()
            }
            EyeState::BlinkClosedFirst | EyeState::BlinkClosedSecond => {
                let mut builder = Path::builder();
                builder.move_to(point(-0.15, -0.1));
                builder.line_to(point(0.0, -0.2));
                builder.line_to(point(0.15, -0.1));
                builder.close();
                builder.build()
            }
        };

        let mut stroke_tesselator = StrokeTessellator::new();
        let mut stroke_geometry: VertexBuffers<[f32; 3], u32> = VertexBuffers::new();
        stroke_tesselator
            .tessellate_path(
                &path,
                &StrokeOptions::default()
                    .with_line_width(0.07)
                    .with_line_join(LineJoin::Round)
                    .with_line_cap(LineCap::Round),
                &mut BuffersBuilder::new(
                    &mut stroke_geometry,
                    |pos: lyon::math::Point, _: StrokeAttributes| [pos.x, pos.y, 0.0],
                ),
            )
            .unwrap();
        let outline_vertex_count = stroke_geometry.vertices.len();

        for eye_mesh_handle in self.eye_meshes.iter() {
            let mesh = meshes.get_mut(eye_mesh_handle).unwrap();
            mesh.set_indices(Some(Indices::U32(stroke_geometry.indices.clone())));
            mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, stroke_geometry.vertices.clone());
            mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0; 3]; outline_vertex_count]);
            mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0; 2]; outline_vertex_count]);
        }
    }
}

pub fn blinking_eyes_system(
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut query: Query<&mut BlinkingEyes>,
) {
    for mut blinking_eyes in query.iter_mut() {
        blinking_eyes.state_remaining_seconds -= time.delta_seconds();
        if blinking_eyes.state_remaining_seconds < 0.0 {
            let (next_state, state_duration) = match blinking_eyes.state {
                EyeState::Open => {
                    if rand::thread_rng().gen_bool(0.3) {
                        (EyeState::BlinkClosedFirst, 0.2)
                    } else {
                        (EyeState::BlinkClosedSecond, 0.2)
                    }
                }
                EyeState::BlinkClosedFirst => (EyeState::BlinkOpen, 0.2),
                EyeState::BlinkOpen => (EyeState::BlinkClosedSecond, 0.2),
                EyeState::BlinkClosedSecond => {
                    (EyeState::Open, rand::thread_rng().gen_range(3.0..10.0))
                }
            };
            blinking_eyes.state = next_state;
            blinking_eyes.state_remaining_seconds = state_duration;
        }

        blinking_eyes.update_meshes(&mut meshes);
    }
}
