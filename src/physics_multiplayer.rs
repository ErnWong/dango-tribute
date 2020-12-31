use super::{
    player::{Player, PlayerId, PlayerInputCommand, PlayerState},
    RealField,
};
use bevy::prelude::*;
use bevy_prototype_networked_physics::{
    command::Command,
    world::{State, World},
};
use nphysics2d::{
    force_generator::DefaultForceGeneratorSet,
    joint::DefaultJointConstraintSet,
    nalgebra::Vector2,
    object::{DefaultBodySet, DefaultColliderSet},
    world::{DefaultGeometricalWorld, DefaultMechanicalWorld},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Debug};

pub struct PhysicsWorld {
    mechanical_world: DefaultMechanicalWorld<RealField>,
    geometrical_world: DefaultGeometricalWorld<RealField>,
    bodies: DefaultBodySet<RealField>,
    colliders: DefaultColliderSet<RealField>,
    joint_constraints: DefaultJointConstraintSet<RealField>,
    force_generators: DefaultForceGeneratorSet<RealField>,
    players: HashMap<PlayerId, Player>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PhysicsCommand {
    PlayerInput {
        player_id: PlayerId,
        command: PlayerInputCommand,
    },
    SpawnPlayer {
        player_id: PlayerId,
        color: Color,
        size: f32,
        x: RealField,
        y: RealField,
    },
    DespawnPlayer(PlayerId),
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PhysicsState {
    players: HashMap<PlayerId, PlayerState>,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        let mut physics_world = Self {
            mechanical_world: DefaultMechanicalWorld::<RealField>::new(Vector2::new(
                0.0,
                super::GRAVITY,
            )),
            geometrical_world: DefaultGeometricalWorld::<RealField>::new(),
            bodies: DefaultBodySet::<RealField>::new(),
            colliders: DefaultColliderSet::<RealField>::new(),
            joint_constraints: DefaultJointConstraintSet::<RealField>::new(),
            force_generators: DefaultForceGeneratorSet::<RealField>::new(),
            players: HashMap::new(),
        };

        physics_world.mechanical_world.set_timestep(super::TIMESTEP);

        physics_world
    }
}

impl World for PhysicsWorld {
    type CommandType = PhysicsCommand;
    type StateType = PhysicsState;

    fn command_is_valid(command: &PhysicsCommand, client_id: usize) -> bool {
        let player_id = match command {
            PhysicsCommand::PlayerInput { player_id, .. } => player_id,
            PhysicsCommand::SpawnPlayer { player_id, .. } => player_id,
            PhysicsCommand::DespawnPlayer(player_id) => player_id,
        };
        *player_id == PlayerId(client_id)
    }

    fn apply_command(&mut self, command: &PhysicsCommand) {
        match command {
            PhysicsCommand::PlayerInput { player_id, command } => {
                if let Some(player) = self.players.get_mut(player_id) {
                    player.apply_command(command);
                }
            }
            PhysicsCommand::SpawnPlayer {
                player_id,
                color,
                size,
                x,
                y,
            } => {
                self.players.insert(
                    *player_id,
                    Player::new(
                        *color,
                        *size,
                        Vector2::new(*x, *y),
                        &mut self.bodies,
                        &mut self.colliders,
                    ),
                );
            }
            PhysicsCommand::DespawnPlayer(player_id) => {
                if let Some(player) = self.players.remove(player_id) {
                    player.deregister(&mut self.bodies, &mut self.colliders);
                }
            }
        }
    }

    fn step(&mut self) {
        for player in &mut self.players.values_mut() {
            player.step(
                self.mechanical_world.timestep(),
                &mut self.bodies,
                &self.colliders,
                &self.geometrical_world,
            );
        }
        self.mechanical_world.step(
            &mut self.geometrical_world,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.joint_constraints,
            &mut self.force_generators,
        );
    }

    fn set_state(&mut self, target_state: PhysicsState) {
        let bodies = &mut self.bodies;
        let colliders = &mut self.colliders;
        self.players.retain(|player_id, player| {
            if !target_state.players.contains_key(player_id) {
                player.deregister(bodies, colliders);
                false
            } else {
                true
            }
        });
        for (player_id, player_state) in &target_state.players {
            let player = if let Some(player) = self.players.get_mut(player_id) {
                player
            } else {
                self.players.insert(
                    *player_id,
                    Player::new(
                        player_state.color,
                        player_state.size,
                        Vector2::new(0.0, 0.0),
                        bodies,
                        colliders,
                    ),
                );
                self.players.get_mut(player_id).unwrap()
            };
            player.set_state(player_state, bodies);
        }
    }

    fn state(&self) -> PhysicsState {
        let mut players = HashMap::new();
        for (player_id, player) in &self.players {
            players.insert(*player_id, PlayerState::from_player(player, &self.bodies));
        }
        PhysicsState { players }
    }
}

impl Command for PhysicsCommand {}

impl PhysicsState {
    pub fn players(&self) -> &HashMap<PlayerId, PlayerState> {
        &self.players
    }
}

impl State for PhysicsState {
    fn from_interpolation(old_state: &Self, new_state: &Self, t: f32) -> Self {
        let mut players = HashMap::new();

        // Use the new state as the overall structure.
        for (player_id, new_player_state) in &new_state.players {
            if let Some(old_player_state) = old_state.players.get(player_id) {
                players.insert(
                    *player_id,
                    PlayerState::from_interpolation(old_player_state, new_player_state, t),
                );
            } else {
                players.insert(*player_id, new_player_state.clone());
            }
        }

        Self { players }
    }
}
