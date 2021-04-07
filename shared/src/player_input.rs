use super::{
    physics_multiplayer::{PhysicsCommand, PhysicsWorld},
    player::{PlayerId, PlayerInputCommand, PlayerInputState},
};
use crate::networking::WrappedNetworkResource;
use bevy::prelude::*;
use bevy_networking_turbulence::NetworkResource;
use bevy_prototype_networked_physics::client::{Client, ClientState};

pub const INPUT_RESYNC_INTERVAL: f64 = 3.0;

#[derive(Default)]
pub struct LastResetTime(f64);

pub fn player_input_system(
    mut state: Local<PlayerInputState>,
    mut last_resync: Local<LastResetTime>,
    input: Res<Input<KeyCode>>,
    time: Res<Time>,
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

        let resync = if time.seconds_since_startup() - last_resync.0 > INPUT_RESYNC_INTERVAL {
            last_resync.0 = time.seconds_since_startup();
            true
        } else {
            false
        };

        if resync || left != state.left {
            ready_client.issue_command(
                PhysicsCommand::PlayerInput {
                    player_id,
                    command: PlayerInputCommand::Left(left),
                },
                &mut WrappedNetworkResource(&mut *net),
            );
        }
        if resync || right != state.right {
            ready_client.issue_command(
                PhysicsCommand::PlayerInput {
                    player_id,
                    command: PlayerInputCommand::Right(right),
                },
                &mut WrappedNetworkResource(&mut *net),
            );
        }
        if resync || jump != state.jump {
            ready_client.issue_command(
                PhysicsCommand::PlayerInput {
                    player_id,
                    command: PlayerInputCommand::Jump(jump),
                },
                &mut WrappedNetworkResource(&mut *net),
            );
        }
        if resync || roll != state.roll {
            ready_client.issue_command(
                PhysicsCommand::PlayerInput {
                    player_id,
                    command: PlayerInputCommand::Roll(roll),
                },
                &mut WrappedNetworkResource(&mut *net),
            );
        }
    }

    state.left = left;
    state.right = right;
    state.jump = jump;
    state.roll = roll;
}
