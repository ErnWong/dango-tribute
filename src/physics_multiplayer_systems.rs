use super::{
    physics_multiplayer::{PhysicsCommand, PhysicsWorld},
    player::PlayerId,
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
use bevy_prototype_networked_physics::{
    client::{Client, ClientState},
    events::ClientConnectionEvent,
    net::NetworkResource,
    server::Server,
};
use bevy_prototype_transform_tracker::TransformTrackingTarget;
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
                        size: 0.8,
                        x: 0.0,
                        y: 0.0,
                        color: Color::RED,
                    },
                    &mut net,
                );
            }
        }
    }
}

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

pub fn physics_multiplayer_client_sync_system(
    mut player_map: Local<PlayerMap>,
    commands: &mut Commands,
    client: Res<Client<PhysicsWorld>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut query: Query<(&PlayerComponent, &Handle<Mesh>, &mut Transform)>,
) {
    if let ClientState::Ready(ready_client) = client.state() {
        let new_player_states = ready_client.world_state().players();

        let new_player_ids: HashSet<PlayerId> = new_player_states.keys().copied().collect();
        let old_player_ids: HashSet<PlayerId> = player_map.0.keys().copied().collect();

        let to_spawn = new_player_ids.difference(&old_player_ids);
        let to_despawn = old_player_ids.difference(&new_player_ids);

        for player_id in to_spawn {
            let player_state = new_player_states.get(player_id).unwrap();
            let entity = commands
                .spawn(PlayerBundle {
                    sprite: Sprite {
                        size: Vec2::one(),
                        ..Default::default()
                    },
                    mesh: meshes.add(Mesh::new(PrimitiveTopology::TriangleList)),
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
                    transform: Transform {
                        translation: Vec3::new(
                            player_state.derived_measurements.center_of_mass.x,
                            player_state.derived_measurements.center_of_mass.y,
                            0.0,
                        ),
                        scale: Vec3::new(player_state.size, player_state.size, 1.0),
                        ..Default::default()
                    },
                    global_transform: GlobalTransform::default(),
                    player: PlayerComponent,
                })
                .current_entity()
                .unwrap();

            if *player_id == PlayerId(ready_client.client_id()) {
                commands.insert_one(entity, TransformTrackingTarget);
            }

            player_map.0.insert(*player_id, entity);
        }

        for player_id in to_despawn {
            commands.despawn(player_map.0.remove(player_id).unwrap());
        }

        for (player_id, player_state) in new_player_states {
            let entity = player_map.0.get(player_id).unwrap();
            if let Ok((_, mesh_handle, mut transform)) = query.get_mut(*entity) {
                transform.translation.x = player_state.derived_measurements.center_of_mass.x;
                transform.translation.y = player_state.derived_measurements.center_of_mass.y;

                let mesh = meshes.get_mut(mesh_handle).unwrap();

                mesh.set_indices(Some(Indices::U32(player_state.derived_indices.clone())));
                mesh.set_attribute(
                    Mesh::ATTRIBUTE_POSITION,
                    player_state
                        .positions
                        .chunks(2)
                        .map(|pos| [pos[0], pos[1], 0.0])
                        .collect::<Vec<[f32; 3]>>(),
                );

                let vertex_count = player_state.derived_indices.len();
                mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0; 3]; vertex_count]);
                mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0; 2]; vertex_count]);
            }
        }
    }
}
