use crate::{
    blinking_eyes::{BlinkingEyes, EyeState},
    physics_multiplayer::{PhysicsCommand, PhysicsDisplayState, PhysicsWorld},
    player::{PlayerDisplayState, PlayerId},
};
use bevy::{
    prelude::*,
    render::{
        mesh::Indices,
        pipeline::{PrimitiveTopology, RenderPipeline},
        render_graph::base::MainPass,
    },
    sprite::SPRITE_PIPELINE_HANDLE,
};
use bevy_prototype_lyon::{
    basic_shapes::{primitive, ShapeType},
    TessellationMode,
};
use crystalorb_bevy_networking_turbulence::{
    bevy_networking_turbulence::{NetworkEvent, NetworkResource},
    crystalorb::{
        client::{stage::Stage as ClientStage, Client},
        server::Server,
    },
    WrappedNetworkResource,
};

use bevy_prototype_transform_tracker::TransformTrackingTarget;
use lyon::{
    math::point,
    path::Path,
    tessellation::{
        BuffersBuilder, FillAttributes, FillOptions, FillTessellator, StrokeAttributes,
        StrokeOptions, StrokeTessellator, VertexBuffers,
    },
};
use splines::{Interpolation, Key, Spline};
use std::{
    collections::{HashMap, HashSet},
    convert::TryInto,
};

//#[cfg(not(target_arch = "wasm32"))]
pub fn physics_multiplayer_server_spawn_despawn_system(
    mut server: ResMut<Server<PhysicsWorld>>,
    mut network_events: EventReader<NetworkEvent>,
    mut net: ResMut<NetworkResource>,
) {
    for network_event in network_events.iter() {
        match network_event {
            NetworkEvent::Connected(client_id) => {
                server.issue_command(
                    PhysicsCommand::SpawnPlayer {
                        player_id: PlayerId((*client_id).try_into().unwrap()),
                        // TODO: Dynamically chosen...
                        size: 0.6,
                        x: 0.0,
                        y: 5.0,
                        color: match *client_id % 4 {
                            0 => Color::rgb(1.0, 0.3, 0.3),
                            1 => Color::rgb(0.3, 0.8, 0.4),
                            2 => Color::rgb(1.0, 0.8, 0.3),
                            3 => Color::rgb(0.3, 0.6, 1.0),
                            _ => unreachable!(),
                        },
                    },
                    &mut WrappedNetworkResource(&mut net),
                );
            }
            NetworkEvent::Disconnected(client_id) => {
                server.issue_command(
                    PhysicsCommand::DespawnPlayer(PlayerId((*client_id).try_into().unwrap())),
                    &mut WrappedNetworkResource(&mut net),
                );
            }
            _ => {}
        }
    }
}

#[derive(Clone)]
pub struct OutlineMesh(Handle<Mesh>);

#[derive(Bundle)]
pub struct PlayerBundle {
    // TODO: Is sprite needed? Can we not use the spritesheet pipeline?
    pub sprite: Sprite,
    pub mesh: Handle<Mesh>,
    pub outline_mesh: OutlineMesh,
    pub material: Handle<ColorMaterial>,
    pub main_pass: MainPass,
    pub draw: Draw,
    pub visible: Visible,
    pub render_pipelines: RenderPipelines,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub player: PlayerComponent,
    pub blinking_eyes: BlinkingEyes,
}

pub struct PlayerComponent;

#[derive(Default)]
pub struct PlayerMap(HashMap<PlayerId, Entity>);

pub struct Shadow(Entity);

#[derive(Copy, Clone)]
enum DrawMode {
    Poly,
    Spline,
}

const PLAYER_Z_SPACE: f32 = 0.1;
const MAX_PLAYERS: f32 = 100.0;
const SHADOW_Z_OFFSET: f32 = 1.0;

pub fn physics_multiplayer_client_sync_system(
    mut player_map: Local<PlayerMap>,
    mut commands: Commands,
    client: Res<Client<PhysicsWorld>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    meshes: ResMut<Assets<Mesh>>,
    query: Query<(&PlayerComponent, &Handle<Mesh>, &OutlineMesh, &Shadow)>,
    transform_query: Query<&mut Transform>,
) {
    if let ClientStage::Ready(ready_client) = client.stage() {
        sync_from_state(
            ready_client.display_state(),
            PlayerId(ready_client.client_id()),
            &mut player_map,
            commands,
            &mut materials,
            meshes,
            query,
            transform_query,
            DrawMode::Spline,
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn physics_multiplayer_server_diagnostic_sync_system(
    mut player_map: Local<PlayerMap>,
    mut commands: Commands,
    server: Res<Server<PhysicsWorld>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    meshes: ResMut<Assets<Mesh>>,
    query: Query<(&PlayerComponent, &Handle<Mesh>, &OutlineMesh, &Shadow)>,
    transform_query: Query<&mut Transform>,
) {
    sync_from_state(
        &server.display_state(),
        PlayerId(0),
        &mut player_map,
        commands,
        &mut materials,
        meshes,
        query,
        transform_query,
        DrawMode::Poly,
    );
}

fn sync_from_state(
    world_state: &PhysicsDisplayState,
    player_to_track: PlayerId,
    player_map: &mut PlayerMap,
    mut commands: Commands,
    materials: &mut Assets<ColorMaterial>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut query: Query<(&PlayerComponent, &Handle<Mesh>, &OutlineMesh, &Shadow)>,
    mut transform_query: Query<&mut Transform>,
    draw_mode: DrawMode,
) {
    let new_player_states = world_state.players();

    let new_player_ids: HashSet<PlayerId> = new_player_states.keys().copied().collect();
    let old_player_ids: HashSet<PlayerId> = player_map.0.keys().copied().collect();
    trace!("new_player_ids: {:?}", new_player_ids);

    let to_spawn = new_player_ids.difference(&old_player_ids);
    let to_despawn = old_player_ids.difference(&new_player_ids);

    for player_id in to_spawn {
        info!("Spawning player {:?}", player_id);
        let player_state = new_player_states.get(player_id).unwrap();
        let mut transform = Transform::default();
        let mut shadow_transform = Transform::default();
        update_transform(
            &mut transform,
            &mut shadow_transform,
            player_id,
            player_state,
        );

        let mesh = Mesh::new(PrimitiveTopology::TriangleList);
        let outline_mesh = Mesh::new(PrimitiveTopology::TriangleList);
        let left_eye_mesh = Mesh::new(PrimitiveTopology::TriangleList);
        let right_eye_mesh = Mesh::new(PrimitiveTopology::TriangleList);

        // TODO: Move this section out.
        let mut shadow_mesh = Mesh::new(PrimitiveTopology::TriangleList);
        shadow_mesh.set_indices(Some(Indices::U32(
            [
                0, 1, 2, 0, 2, 3, 0, 3, 4, 0, 4, 5, 0, 5, 6, 0, 6, 7, 0, 7, 8, 0, 8, 1,
            ]
            .into(),
        )));
        let shadow_vertices: Vec<[f32; 3]> = [
            // Center
            [0.0, 0.0, 0.0 + SHADOW_Z_OFFSET],
            // 8 corners of an octagon starting from x axis
            [-1.0 * 0.924, 0.0, 0.383 + SHADOW_Z_OFFSET],
            [-1.0 * 0.383, 0.0, 0.924 + SHADOW_Z_OFFSET],
            [-1.0 * -0.383, 0.0, 0.924 + SHADOW_Z_OFFSET],
            [-1.0 * -0.924, 0.0, 0.383 + SHADOW_Z_OFFSET],
            [-1.0 * -0.924, 0.0, -0.383 + SHADOW_Z_OFFSET],
            [-1.0 * -0.383, 0.0, -0.924 + SHADOW_Z_OFFSET],
            [-1.0 * 0.383, 0.0, -0.924 + SHADOW_Z_OFFSET],
            [-1.0 * 0.924, 0.0, -0.383 + SHADOW_Z_OFFSET],
        ]
        .into();
        shadow_mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, shadow_vertices);
        shadow_mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0; 3]; 9]);
        shadow_mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0; 3]; 9]);

        let mesh_handle = meshes.add(mesh);
        let outline_mesh_handle = OutlineMesh(meshes.add(outline_mesh));
        let left_eye_mesh_handle = meshes.add(left_eye_mesh);
        let right_eye_mesh_handle = meshes.add(right_eye_mesh);
        let shadow_mesh_handle = meshes.add(shadow_mesh);
        update_mesh(
            &mut meshes,
            &mesh_handle,
            &outline_mesh_handle.0,
            player_state,
            draw_mode,
        );
        let entity = commands
            .spawn()
            .insert_bundle(PlayerBundle {
                sprite: Sprite {
                    size: Vec2::ONE,
                    ..Default::default()
                },
                mesh: mesh_handle,
                outline_mesh: outline_mesh_handle.clone(),
                material: materials.add(player_state.color.into()),
                main_pass: MainPass,
                draw: Default::default(),
                visible: Visible {
                    is_transparent: true,
                    ..Default::default()
                },
                render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                    SPRITE_PIPELINE_HANDLE.typed(),
                )]),
                transform,
                global_transform: GlobalTransform::default(),
                player: PlayerComponent,
                blinking_eyes: BlinkingEyes::new(
                    vec![left_eye_mesh_handle.clone(), right_eye_mesh_handle.clone()],
                    &mut meshes,
                ),
            })
            .id();
        commands
            .spawn()
            .insert_bundle(SpriteBundle {
                sprite: Sprite {
                    size: Vec2::ONE,
                    ..Default::default()
                },
                mesh: left_eye_mesh_handle.clone(),
                material: materials.add(Color::rgb(0.1, 0.1, 0.1).into()),
                transform: Transform::from_translation(Vec3::new(-0.2, 0.3, 0.5)),
                ..Default::default()
            })
            .insert(Parent(entity));
        commands
            .spawn()
            .insert_bundle(SpriteBundle {
                sprite: Sprite {
                    size: Vec2::ONE,
                    ..Default::default()
                },
                mesh: right_eye_mesh_handle.clone(),
                material: materials.add(Color::rgb(0.1, 0.1, 0.1).into()),
                transform: Transform::from_translation(Vec3::new(0.2, 0.3, 0.5)),
                ..Default::default()
            })
            .insert(Parent(entity));
        commands
            .spawn()
            .insert_bundle(SpriteBundle {
                sprite: Sprite {
                    size: Vec2::ONE,
                    ..Default::default()
                },
                mesh: outline_mesh_handle.0,
                material: materials.add(
                    Color::rgb(
                        player_state.color.r() * 0.5,
                        player_state.color.g() * 0.5,
                        player_state.color.b() * 0.5,
                    )
                    .into(),
                ),
                main_pass: MainPass,
                draw: Default::default(),
                visible: Visible {
                    is_transparent: true,
                    ..Default::default()
                },
                render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                    SPRITE_PIPELINE_HANDLE.typed(),
                )]),
                transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.5)),
                global_transform: GlobalTransform::default(),
            })
            .insert(Parent(entity));
        let shadow_entity = commands
            .spawn()
            .insert_bundle(SpriteBundle {
                sprite: Sprite {
                    size: Vec2::ONE,
                    ..Default::default()
                },
                mesh: shadow_mesh_handle.clone(),
                material: materials.add(
                    Color::rgb(
                        player_state.color.r() * 0.1,
                        player_state.color.g() * 0.1,
                        player_state.color.b() * 0.1,
                    )
                    .into(),
                ),
                main_pass: MainPass,
                draw: Default::default(),
                visible: Visible {
                    is_transparent: true,
                    ..Default::default()
                },
                render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                    SPRITE_PIPELINE_HANDLE.typed(),
                )]),
                transform: shadow_transform,
                global_transform: GlobalTransform::default(),
            })
            .id();
        commands.entity(entity).insert(Shadow(shadow_entity));

        if *player_id == player_to_track {
            commands.entity(entity).insert(TransformTrackingTarget);
        }

        player_map.0.insert(*player_id, entity);
    }

    for player_id in to_despawn {
        info!("Despawning player {:?}", player_id);
        let entity = player_map.0.remove(player_id).unwrap();
        if let Ok(shadow_entity) = query.get_component::<Shadow>(entity) {
            commands.entity(shadow_entity.0).despawn_recursive();
        }
        commands.entity(entity).despawn_recursive();
    }

    for (player_id, player_state) in new_player_states {
        let entity = player_map.0.get(player_id).unwrap();
        // SAFE: Shadow and it's shadow caster are different entities, so no aliasing occurs.
        if let Ok((_, mesh_handle, outline_mesh_handle, shadow)) = query.get_mut(*entity) {
            unsafe {
                if let Ok(mut transform) = transform_query.get_unchecked(*entity) {
                    if let Ok(mut shadow_transform) = transform_query.get_unchecked(shadow.0) {
                        trace!("Updating player {:?}", player_id);
                        update_transform(
                            &mut transform,
                            &mut shadow_transform,
                            player_id,
                            player_state,
                        );
                        update_mesh(
                            &mut meshes,
                            mesh_handle,
                            &outline_mesh_handle.0,
                            player_state,
                            draw_mode,
                        );
                    }
                }
            }
        }
    }
}

fn update_transform(
    transform: &mut Transform,
    shadow_transform: &mut Transform,
    player_id: &PlayerId,
    player_state: &PlayerDisplayState,
) {
    transform.scale = Vec3::new(
        player_state.size,
        player_state.size,
        PLAYER_Z_SPACE / MAX_PLAYERS,
    );
    transform.translation.x = player_state.measurements.center_of_mass.x;

    // HACK: Compensating outline's 0.1 thickness so it touches the ground at the right visual
    // place, so that the shadows look correct.
    transform.translation.y = player_state.measurements.center_of_mass.y + 0.05;

    // Ensure each player gets their own z-space for drawing, since we don't want
    // one players outline and fill to sandwich another player's.
    transform.translation.z = -(player_id.0 as f32) * PLAYER_Z_SPACE / MAX_PLAYERS;
    transform.rotation = Quat::from_rotation_z(player_state.measurements.mean_angle);

    shadow_transform.scale = Vec3::ONE * player_state.size;
    shadow_transform.translation.x = transform.translation.x;
    shadow_transform.translation.z =
        transform.translation.z - SHADOW_Z_OFFSET * shadow_transform.scale.z;
}

fn update_mesh(
    meshes: &mut Assets<Mesh>,
    mesh_handle: &Handle<Mesh>,
    outline_mesh_handle: &Handle<Mesh>,
    player_state: &PlayerDisplayState,
    draw_mode: DrawMode,
) {
    let vertex_count;

    match draw_mode {
        DrawMode::Poly => {
            let mesh = meshes.get_mut(mesh_handle).unwrap();
            mesh.set_indices(Some(Indices::U32(player_state.mesh_indices.clone())));
            mesh.set_attribute(
                Mesh::ATTRIBUTE_POSITION,
                player_state
                    .local_positions
                    .iter()
                    .map(|pos| [pos.x, pos.y, 0.0])
                    .collect::<Vec<[f32; 3]>>(),
            );
            vertex_count = player_state.local_positions.len() / 2;
        }

        DrawMode::Spline => {
            let mut spline_keys = vec![];

            // Note: We are repeating the first 3 vertices: 2 to provide context to the
            // cubic spline interpolation, and 1 more to close the loop.
            let wrapped_boundary_indices = player_state
                .boundary_indices
                .iter()
                .cycle()
                .take(player_state.boundary_indices.len() + 3);
            for boundary_index in wrapped_boundary_indices {
                spline_keys.push(Key::new(
                    spline_keys.len() as f32,
                    player_state.local_positions[*boundary_index],
                    Interpolation::CatmullRom,
                ));
            }
            let mut path_builder = Path::builder();

            // Note: Start at offset of 1 (since key 0 is used for interpolation context).
            path_builder.move_to(point(spline_keys[1].value.x, spline_keys[1].value.y));
            let spline = Spline::from_vec(spline_keys);

            const SUBDIVISIONS: usize = 4;
            for i in 0..(player_state.boundary_indices.len() * SUBDIVISIONS) {
                // Note: Start at offset of 1 (since key 0 is used for interpolation context).
                let t = (i as f32) / (SUBDIVISIONS as f32) + 1.0;
                if let Some(p) = spline.sample(t) {
                    path_builder.line_to(point(p.x, p.y));
                } else {
                }
            }
            path_builder.close();
            let path = path_builder.build();
            let mut fill_tesselator = FillTessellator::new();
            let mut fill_geometry: VertexBuffers<[f32; 3], u32> = VertexBuffers::new();
            fill_tesselator
                .tessellate_path(
                    &path,
                    &FillOptions::default(),
                    &mut BuffersBuilder::new(
                        &mut fill_geometry,
                        |pos: lyon::math::Point, _: FillAttributes| [pos.x, pos.y, 0.0],
                    ),
                )
                .unwrap();

            vertex_count = fill_geometry.vertices.len();
            let mesh = meshes.get_mut(mesh_handle).unwrap();
            mesh.set_indices(Some(Indices::U32(fill_geometry.indices)));
            mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, fill_geometry.vertices);

            let mut stroke_tesselator = StrokeTessellator::new();
            let mut stroke_geometry: VertexBuffers<[f32; 3], u32> = VertexBuffers::new();
            stroke_tesselator
                .tessellate_path(
                    &path,
                    &StrokeOptions::default().with_line_width(0.1),
                    &mut BuffersBuilder::new(
                        &mut stroke_geometry,
                        |pos: lyon::math::Point, _: StrokeAttributes| [pos.x, pos.y, 0.0],
                    ),
                )
                .unwrap();
            let outline_vertex_count = stroke_geometry.vertices.len();
            let outline_mesh = meshes.get_mut(outline_mesh_handle).unwrap();
            outline_mesh.set_indices(Some(Indices::U32(stroke_geometry.indices)));
            outline_mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, stroke_geometry.vertices);
            outline_mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0; 3]; outline_vertex_count]);
            outline_mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0; 2]; outline_vertex_count]);
        }
    }

    let mesh = meshes.get_mut(mesh_handle).unwrap();
    mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0; 3]; vertex_count]);
    mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0; 2]; vertex_count]);
}
