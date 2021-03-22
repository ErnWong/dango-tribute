use crate::{
    player::{Player, PlayerDisplayState, PlayerId, PlayerInputCommand, PlayerSnapshot},
    settings,
    settings::RealField,
};
use bevy::prelude::*;
use bevy_prototype_networked_physics::{
    command::Command,
    fixed_timestepper::Stepper,
    world::{DisplayState, World},
};
use nphysics2d::{
    force_generator::DefaultForceGeneratorSet,
    joint::DefaultJointConstraintSet,
    nalgebra::Vector2,
    ncollide2d::shape::{Cuboid, ShapeHandle},
    object::{BodyPartHandle, ColliderDesc, DefaultBodySet, DefaultColliderSet, Ground},
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
pub struct PhysicsSnapshot {
    players: HashMap<PlayerId, PlayerSnapshot>,
}

//#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[derive(Clone, Default)]
pub struct PhysicsDisplayState {
    players: HashMap<PlayerId, PlayerDisplayState>,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        let mut physics_world = Self {
            mechanical_world: DefaultMechanicalWorld::<RealField>::new(Vector2::new(
                0.0,
                settings::GRAVITY,
            )),
            geometrical_world: DefaultGeometricalWorld::<RealField>::new(),
            bodies: DefaultBodySet::<RealField>::new(),
            colliders: DefaultColliderSet::<RealField>::new(),
            joint_constraints: DefaultJointConstraintSet::<RealField>::new(),
            force_generators: DefaultForceGeneratorSet::<RealField>::new(),
            players: HashMap::new(),
        };

        physics_world
            .mechanical_world
            .set_timestep(settings::TIMESTEP);

        // TODO: Source from a scene.
        let ground = physics_world.bodies.insert(Ground::new());
        physics_world.colliders.insert(
            ColliderDesc::<RealField>::new(ShapeHandle::new(Cuboid::new(Vector2::new(500.0, 5.0))))
                .translation(Vector2::new(0.0, -5.0))
                .build(BodyPartHandle(ground, 0)),
        );

        physics_world
    }
}

impl World for PhysicsWorld {
    type CommandType = PhysicsCommand;
    type SnapshotType = PhysicsSnapshot;
    type DisplayStateType = PhysicsDisplayState;

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
                if let Some(existing_player) = self.players.get(player_id) {
                    existing_player.deregister(&mut self.bodies, &mut self.colliders);
                }
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

    fn apply_snapshot(&mut self, snapshot: PhysicsSnapshot) {
        let bodies = &mut self.bodies;
        let colliders = &mut self.colliders;
        self.players.retain(|player_id, player| {
            if !snapshot.players.contains_key(player_id) {
                player.deregister(bodies, colliders);
                false
            } else {
                true
            }
        });
        for (player_id, player_snapshot) in &snapshot.players {
            let player = if let Some(player) = self.players.get_mut(player_id) {
                player
            } else {
                self.players.insert(
                    *player_id,
                    Player::new(
                        player_snapshot.color,
                        player_snapshot.size,
                        Vector2::new(0.0, 0.0),
                        bodies,
                        colliders,
                    ),
                );
                self.players.get_mut(player_id).unwrap()
            };
            player.apply_snapshot(player_snapshot, bodies);
        }
    }

    fn snapshot(&self) -> PhysicsSnapshot {
        let mut players = HashMap::new();
        for (player_id, player) in &self.players {
            players.insert(*player_id, player.snapshot(&self.bodies));
        }
        PhysicsSnapshot { players }
    }

    fn display_state(&self) -> PhysicsDisplayState {
        let mut players = HashMap::new();
        for (player_id, player) in &self.players {
            players.insert(*player_id, player.display_state(&self.bodies));
        }
        PhysicsDisplayState { players }
    }
}

impl Stepper for PhysicsWorld {
    fn step(&mut self) -> f32 {
        for player in &mut self.players.values_mut() {
            player.pre_step(self.mechanical_world.timestep(), &mut self.bodies);
        }
        self.mechanical_world.step(
            &mut self.geometrical_world,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.joint_constraints,
            &mut self.force_generators,
        );
        for player in &mut self.players.values_mut() {
            player.post_step(&self.colliders, &self.geometrical_world);
        }
        self.mechanical_world.timestep()
    }
}

impl Command for PhysicsCommand {}

impl PhysicsDisplayState {
    pub fn players(&self) -> &HashMap<PlayerId, PlayerDisplayState> {
        &self.players
    }
}

impl DisplayState for PhysicsDisplayState {
    fn from_interpolation(old_state: &Self, new_state: &Self, t: f32) -> Self {
        let mut players = HashMap::new();

        // Use the new state as the overall structure.
        for (player_id, new_player_state) in &new_state.players {
            if let Some(old_player_state) = old_state.players.get(player_id) {
                players.insert(
                    *player_id,
                    PlayerDisplayState::from_interpolation(old_player_state, new_player_state, t),
                );
            } else {
                players.insert(*player_id, new_player_state.clone());
            }
        }

        Self { players }
    }
}
