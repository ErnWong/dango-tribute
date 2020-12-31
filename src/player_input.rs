use super::{
    physics_multiplayer::{PhysicsCommand, PhysicsWorld},
    player::{PlayerId, PlayerInputCommand, PlayerInputState},
};
use bevy::prelude::*;
use bevy_prototype_networked_physics::{
    client::{Client, ClientState},
    net::NetworkResource,
};

pub fn player_input_system(
    mut state: Local<PlayerInputState>,
    input: Res<Input<KeyCode>>,
    mut client: ResMut<Client<PhysicsWorld>>,
    mut net: ResMut<NetworkResource>,
) {
    let left = input.pressed(KeyCode::A) || input.pressed(KeyCode::Left);
    let right = input.pressed(KeyCode::D) || input.pressed(KeyCode::Right);
    let jump =
        input.pressed(KeyCode::W) || input.pressed(KeyCode::Space) || input.pressed(KeyCode::Up);
    let roll = input.pressed(KeyCode::LShift) || input.pressed(KeyCode::RShift);

    if let ClientState::Ready(ready_client) = client.state_mut() {
        let player_id = PlayerId(ready_client.client_id());

        if left != state.left {
            ready_client.issue_command(
                PhysicsCommand::PlayerInput {
                    player_id,
                    command: PlayerInputCommand::Left(left),
                },
                &mut *net,
            );
        }
        if right != state.right {
            ready_client.issue_command(
                PhysicsCommand::PlayerInput {
                    player_id,
                    command: PlayerInputCommand::Right(right),
                },
                &mut *net,
            );
        }
        if jump != state.jump {
            ready_client.issue_command(
                PhysicsCommand::PlayerInput {
                    player_id,
                    command: PlayerInputCommand::Jump(jump),
                },
                &mut *net,
            );
        }
        if roll != state.roll {
            ready_client.issue_command(
                PhysicsCommand::PlayerInput {
                    player_id,
                    command: PlayerInputCommand::Roll(roll),
                },
                &mut *net,
            );
        }
    }

    state.left = left;
    state.right = right;
    state.jump = jump;
    state.roll = roll;
}
