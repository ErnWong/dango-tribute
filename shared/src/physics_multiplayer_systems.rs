use crate::{
    physics_multiplayer::{PhysicsCommand, PhysicsState, PhysicsWorld},
    player::{PlayerId, PlayerState},
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
use bevy_prototype_networked_physics::{
    client::{Client, ClientState},
    events::ClientConnectionEvent,
    net::NetworkResource,
};

#[cfg(not(target_arch = "wasm32"))]
use bevy_prototype_networked_physics::server::Server;

use bevy_prototype_transform_tracker::TransformTrackingTarget;
use lyon::{
    math::point,
    path::Path,
    tessellation::{BuffersBuilder, FillAttributes, FillOptions, FillTessellator, VertexBuffers},
};
use splines::{Interpolation, Key, Spline};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct SpawnSystemState {
    client_connection_event_reader: EventReader<ClientConnectionEvent>,
}

pub fn physics_multiplayer_client_spawn_system(
    mut state: Local<SpawnSystemState>,
    mut client: ResMut<Client<PhysicsWorld>>,
    client_connection_events: Res<Events<ClientConnectionEvent>>,
    mut net: ResMut<NetworkResource>,
) {
    for client_connection_event in state
        .client_connection_event_reader
        .iter(&client_connection_events)
    {
        if let ClientConnectionEvent::Connected(client_id) = client_connection_event {
            if let ClientState::Ready(ready_client) = client.state_mut() {
                ready_client.issue_command(
                    PhysicsCommand::SpawnPlayer {
                        player_id: PlayerId(*client_id),
                        // TODO: Dynamically chosen...
                        size: 0.6,
                        x: 0.0,
                        y: 0.0,
                        color: match *client_id % 4 {
                            0 => Color::RED,
                            1 => Color::GREEN,
                            2 => Color::ORANGE,
                            3 => Color::BLUE,
                            _ => Color::RED,
                        },
                    },
                    &mut net,
                );
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn physics_multiplayer_server_despawn_system(
    mut state: Local<SpawnSystemState>,
    mut server: ResMut<Server<PhysicsWorld>>,
    client_connection_events: Res<Events<ClientConnectionEvent>>,
    mut net: ResMut<NetworkResource>,
) {
    for client_connection_event in state
        .client_connection_event_reader
        .iter(&client_connection_events)
    {
        if let ClientConnectionEvent::Disconnected(client_id) = client_connection_event {
            server.issue_command(
                PhysicsCommand::DespawnPlayer(PlayerId(*client_id)),
                &mut net,
            );
        }
    }
}

#[derive(Bundle)]
pub struct PlayerBundle {
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
    pub player: PlayerComponent,
}

pub struct PlayerComponent;

#[derive(Default)]
pub struct PlayerMap(HashMap<PlayerId, Entity>);

#[derive(Copy, Clone)]
enum DrawMode {
    Poly,
    Spline,
}

pub fn physics_multiplayer_client_sync_system(
    mut player_map: Local<PlayerMap>,
    commands: &mut Commands,
    client: Res<Client<PhysicsWorld>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    meshes: ResMut<Assets<Mesh>>,
    query: Query<(&PlayerComponent, &Handle<Mesh>, &mut Transform)>,
) {
    if let ClientState::Ready(ready_client) = client.state() {
        sync_from_state(
            ready_client.world_state(),
            PlayerId(ready_client.client_id()),
            &mut player_map,
            commands,
            &mut materials,
            meshes,
            query,
            DrawMode::Spline,
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn physics_multiplayer_server_diagnostic_sync_system(
    mut player_map: Local<PlayerMap>,
    commands: &mut Commands,
    server: Res<Server<PhysicsWorld>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    meshes: ResMut<Assets<Mesh>>,
    query: Query<(&PlayerComponent, &Handle<Mesh>, &mut Transform)>,
) {
    sync_from_state(
        &server.world_state(),
        PlayerId(0),
        &mut player_map,
        commands,
        &mut materials,
        meshes,
        query,
        DrawMode::Poly,
    );
}

fn sync_from_state(
    world_state: &PhysicsState,
    player_to_track: PlayerId,
    player_map: &mut PlayerMap,
    commands: &mut Commands,
    materials: &mut Assets<ColorMaterial>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut query: Query<(&PlayerComponent, &Handle<Mesh>, &mut Transform)>,
    draw_mode: DrawMode,
) {
    let new_player_states = world_state.players();

    let new_player_ids: HashSet<PlayerId> = new_player_states.keys().copied().collect();
    let old_player_ids: HashSet<PlayerId> = player_map.0.keys().copied().collect();

    let to_spawn = new_player_ids.difference(&old_player_ids);
    let to_despawn = old_player_ids.difference(&new_player_ids);

    for player_id in to_spawn {
        info!("Spawning player {:?}", player_id);
        let player_state = new_player_states.get(player_id).unwrap();
        let mut transform = Transform::default();
        update_transform(&mut transform, player_state);
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        update_mesh(&mut mesh, player_state, &transform, draw_mode);
        let entity = commands
            .spawn(PlayerBundle {
                sprite: Sprite {
                    size: Vec2::one(),
                    ..Default::default()
                },
                mesh: meshes.add(mesh),
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
            })
            .current_entity()
            .unwrap();
        commands.set_current_entity(entity);
        commands.with_children(|parent| {
            parent
                .spawn(primitive(
                    materials.add(Color::BLACK.into()),
                    &mut meshes,
                    ShapeType::Rectangle {
                        width: 0.07,
                        height: 0.4,
                    },
                    TessellationMode::Fill(&FillOptions::default()),
                    Vec3::new(-0.1, 0.3, 1.0),
                ))
                .spawn(primitive(
                    materials.add(Color::BLACK.into()),
                    &mut meshes,
                    ShapeType::Rectangle {
                        width: 0.07,
                        height: 0.4,
                    },
                    TessellationMode::Fill(&FillOptions::default()),
                    Vec3::new(0.1, 0.3, 1.0),
                ));
        });

        if *player_id == player_to_track {
            commands.insert_one(entity, TransformTrackingTarget);
        }

        player_map.0.insert(*player_id, entity);
    }

    for player_id in to_despawn {
        info!("Despawning player {:?}", player_id);
        commands.despawn(player_map.0.remove(player_id).unwrap());
    }

    for (player_id, player_state) in new_player_states {
        let entity = player_map.0.get(player_id).unwrap();
        if let Ok((_, mesh_handle, mut transform)) = query.get_mut(*entity) {
            trace!("Updating player {:?}", player_id);
            update_transform(&mut transform, player_state);
            let mesh = meshes.get_mut(mesh_handle).unwrap();
            update_mesh(mesh, player_state, &transform, draw_mode);
        }
    }
}

fn update_transform(transform: &mut Transform, player_state: &PlayerState) {
    transform.scale = Vec3::one() * player_state.size;
    transform.translation.x = player_state.derived_measurements.center_of_mass.x;
    transform.translation.y = player_state.derived_measurements.center_of_mass.y;
    transform.rotation = Quat::from_rotation_z(player_state.derived_measurements.mean_angle);
}

fn update_mesh(
    mesh: &mut Mesh,
    player_state: &PlayerState,
    transform: &Transform,
    draw_mode: DrawMode,
) {
    let to_local_coords = transform.compute_matrix().inverse();
    let local_coords_iter = player_state
        .positions
        .chunks(2)
        .map(|pos| to_local_coords.transform_point3(Vec3::new(pos[0], pos[1], 0.0)));
    let vertex_count;

    match draw_mode {
        DrawMode::Poly => {
            mesh.set_indices(Some(Indices::U32(
                player_state.derived_mesh_indices.clone(),
            )));
            mesh.set_attribute(
                Mesh::ATTRIBUTE_POSITION,
                local_coords_iter
                    .map(|v| v.into())
                    .collect::<Vec<[f32; 3]>>(),
            );
            vertex_count = player_state.positions.len() / 2;
        }

        DrawMode::Spline => {
            let mut spline_keys = vec![];
            let local_coords = local_coords_iter.collect::<Vec<Vec3>>();

            // Note: We are repeating the first 3 vertices: 2 to provide context to the
            // cubic spline interpolation, and 1 more to close the loop.
            let wrapped_boundary_indices = player_state
                .derived_boundary_indices
                .iter()
                .cycle()
                .take(player_state.derived_boundary_indices.len() + 3);
            for boundary_index in wrapped_boundary_indices {
                spline_keys.push(Key::new(
                    spline_keys.len() as f32,
                    local_coords[*boundary_index],
                    Interpolation::CatmullRom,
                ));
            }
            let mut path_builder = Path::builder();

            // Note: Start at offset of 1 (since key 0 is used for interpolation context).
            path_builder.move_to(point(spline_keys[1].value.x, spline_keys[1].value.y));
            let spline = Spline::from_vec(spline_keys);

            const SUBDIVISIONS: usize = 4;
            for i in 0..(player_state.derived_boundary_indices.len() * SUBDIVISIONS) {
                // Note: Start at offset of 1 (since key 0 is used for interpolation context).
                let t = (i as f32) / (SUBDIVISIONS as f32) + 1.0;
                if let Some(p) = spline.sample(t) {
                    path_builder.line_to(point(p.x, p.y));
                } else {
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

            vertex_count = geometry.vertices.len();
            mesh.set_indices(Some(Indices::U32(geometry.indices)));
            mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, geometry.vertices);
        }
    }

    mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0; 3]; vertex_count]);
    mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0; 2]; vertex_count]);
}
