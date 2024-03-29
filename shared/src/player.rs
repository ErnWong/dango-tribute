use crate::settings::RealField;
use bevy::prelude::*;
use crystalorb_bevy_networking_turbulence::crystalorb::world::DisplayState;
use nphysics2d::{
    math::{Force, ForceType},
    nalgebra::{Point2, Point3, Similarity2, Vector2, Vector3},
    ncollide2d::shape::Polyline,
    object::{
        Body, DefaultBodyHandle, DefaultBodySet, DefaultColliderHandle, DefaultColliderSet,
        FEMSurfaceDesc,
    },
    world::DefaultGeometricalWorld,
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub struct ControlledDangoConfig {
    variable_jump_force_initial: RealField,
    variable_jump_force_decay: RealField,
    ground_drag: RealField,
    horizontal_crawling_movement_force: RealField,
    horizontal_rolling_movement_force: RealField,
    horizontal_air_movement_force: RealField,
    angular_momentum_compensation_ratio: RealField,
    angle_proportional_controller_coefficient: RealField,
    stable_angle_margin: RealField,
    crawl_side_frequency: RealField,
    crawl_side_amplitude: RealField,
}

const PHYSICS_CONFIG: ControlledDangoConfig = ControlledDangoConfig {
    variable_jump_force_initial: 80.0,
    variable_jump_force_decay: 500.0,
    ground_drag: 0.1,
    horizontal_crawling_movement_force: 8.5,
    horizontal_rolling_movement_force: 12.0,
    horizontal_air_movement_force: 2.0,
    angular_momentum_compensation_ratio: 0.16,
    angle_proportional_controller_coefficient: 25.0,
    stable_angle_margin: 0.3 * std::f32::consts::PI,
    crawl_side_frequency: 2.0,
    crawl_side_amplitude: 8.0,
};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerId(pub usize);

pub struct Player {
    size: f32,
    color: Color,
    body: DefaultBodyHandle,
    collider: DefaultColliderHandle,
    inputs: PlayerInputState,
    forces: PlayerForcesState,

    semiderived_collision_state: PlayerCollisionState,

    derived_measurements: PhysicsBodyMeasurements,
    derived_mesh_indices: Vec<u32>,
    derived_boundary_indices: Vec<usize>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PlayerSnapshot {
    pub color: Color,
    pub size: f32,
    pub positions: Vec<RealField>,
    pub velocities: Vec<RealField>,
    pub inputs: PlayerInputState,
    pub forces: PlayerForcesState,

    // Note: While this information can be derived from the colliders,
    // we don't sync collider information with the server, so we need
    // to compute all the desired collision information that will affect
    // the game logic immediately after the physics update in the same timestep,
    // and have this collision information stored in the player state.
    pub semiderived_collision_state: PlayerCollisionState,
}

#[derive(Default, Clone, Debug)]
pub struct PlayerDisplayState {
    pub color: Color,
    pub size: f32,
    pub measurements: PhysicsBodyMeasurements,
    pub local_positions: Vec<Vector2<RealField>>,
    pub local_velocities: Vec<Vector2<RealField>>,
    pub mesh_indices: Vec<u32>,
    pub boundary_indices: Vec<usize>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PlayerInputState {
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub roll: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PlayerInputCommand {
    Left(bool),
    Right(bool),
    Jump(bool),
    Roll(bool),
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PlayerForcesState {
    horizontal_force: RealField,
    jump_force: RealField,
    crawl_side_timer: RealField,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PlayerCollisionState {
    has_feet_contact: bool,
}

const PLAYER_FEM_MESH_VERTICES: [[RealField; 2]; 9] = [
    // Center
    [0.0, 0.0],
    // 8 corners of an octagon starting from x axis going counter clockwise
    [0.924, 0.383],
    [0.383, 0.924],
    [-0.383, 0.924],
    [-0.924, 0.383],
    [-0.924, -0.383],
    [-0.383, -0.924],
    [0.383, -0.924],
    [0.924, -0.383],
];

const PLAYER_FEM_MESH_INDICES: [[usize; 3]; 8] = [
    // 8 triangles from x axis going counter clockwise
    [0, 1, 2],
    [0, 2, 3],
    [0, 3, 4],
    [0, 4, 5],
    [0, 5, 6],
    [0, 6, 7],
    [0, 7, 8],
    [0, 8, 1],
];

#[derive(Default, Debug, Clone)]
pub struct PhysicsBodyMeasurements {
    pub angular_momentum: RealField,
    pub velocity: Vector2<RealField>,
    pub sum_of_radius_squared: RealField,
    pub mass: RealField,
    pub center_of_mass: Vector2<RealField>,
    pub mean_angle: RealField,
}

impl Player {
    const COLLISION_MARGIN: RealField = 0.07;

    pub fn new(
        color: Color,
        size: f32,
        position: Vector2<RealField>,
        bodies: &mut DefaultBodySet<RealField>,
        colliders: &mut DefaultColliderSet<RealField>,
    ) -> Self {
        let mut fem_surface = FEMSurfaceDesc::<RealField>::new(
            &PLAYER_FEM_MESH_VERTICES
                .iter()
                .map(|[x, y]| Point2::new(*x, *y))
                .collect::<Vec<Point2<RealField>>>(),
            &PLAYER_FEM_MESH_INDICES
                .iter()
                .map(|[i, j, k]| Point3::new(*i, *j, *k))
                .collect::<Vec<Point3<usize>>>(),
        )
        .translation(position)
        .scale(Vector2::repeat(size))
        .young_modulus(1.0e2)
        .mass_damping(0.2)
        .build();
        let collider_desc = fem_surface
            .boundary_collider_desc()
            .margin(Self::COLLISION_MARGIN);

        let derived_mesh_indices = fem_surface
            .deformed_indices()
            .unwrap()
            // The generalized positions are chunked by two, so divide the index by 2 for
            // use by the mesh indices.
            .map(|i| i as u32 / 2)
            .collect();

        let body_handle = bodies.insert(fem_surface);
        let collider = collider_desc.build(body_handle);

        let polyline = collider.shape().as_shape::<Polyline<RealField>>().unwrap();
        let mut polyline_adj = vec![0; PLAYER_FEM_MESH_VERTICES.len()];
        for edge in polyline.edges() {
            polyline_adj[edge.indices[0]] = edge.indices[1];
        }
        let mut derived_boundary_indices = vec![];
        let start = polyline.edges()[0].indices[0];
        let mut i = start;
        while derived_boundary_indices.len() < polyline.points().len() {
            derived_boundary_indices.push(i);
            i = polyline_adj[i];
        }

        let collider_handle = colliders.insert(collider);
        Self {
            color,
            size,
            body: body_handle,
            collider: collider_handle,
            inputs: Default::default(),
            forces: Default::default(),
            derived_measurements: Default::default(),
            derived_mesh_indices,
            derived_boundary_indices,
            semiderived_collision_state: Default::default(),
        }
    }

    pub fn deregister(
        &self,
        bodies: &mut DefaultBodySet<RealField>,
        colliders: &mut DefaultColliderSet<RealField>,
    ) {
        colliders.remove(self.collider);
        bodies.remove(self.body);
    }

    pub fn apply_snapshot(
        &mut self,
        snapshot: &PlayerSnapshot,
        bodies: &mut DefaultBodySet<RealField>,
    ) {
        let body = bodies.get_mut(self.body).unwrap();
        for (i, body_position) in body
            .deformed_positions_mut()
            .unwrap()
            .1
            .iter_mut()
            .enumerate()
        {
            if let Some(snapshot_position) = snapshot.positions.get(i) {
                *body_position = *snapshot_position;
            } else {
                warn!("Not enough position values from snapshot to fill body");
            }
        }
        for (i, body_velocity) in body.generalized_velocity_mut().iter_mut().enumerate() {
            if let Some(snapshot_velocity) = snapshot.velocities.get(i) {
                *body_velocity = *snapshot_velocity;
            } else {
                warn!("Not enough velocity values from snapshot to fill body");
            }
        }
        self.inputs = snapshot.inputs.clone();
        self.forces = snapshot.forces.clone();
        self.semiderived_collision_state = snapshot.semiderived_collision_state.clone();
    }

    pub fn apply_command(&mut self, command: &PlayerInputCommand) {
        match command {
            PlayerInputCommand::Left(state) => self.inputs.left = *state,
            PlayerInputCommand::Right(state) => self.inputs.right = *state,
            PlayerInputCommand::Jump(state) => self.inputs.jump = *state,
            PlayerInputCommand::Roll(state) => self.inputs.roll = *state,
        }
    }

    fn update_measurements(&mut self, body: &dyn Body<RealField>) {
        let mut center_of_mass = Vector2::<RealField>::new(0.0, 0.0);
        let mut sum_of_sine_angle = 0.0;
        let mut sum_of_cosine_angle = 0.0;
        let mut mass: RealField = 0.0;
        for i in 0..body.num_parts() {
            let part = body.part(i).unwrap();
            let part_mass = part.inertia().linear;
            center_of_mass += part.position().translation.vector * part_mass;
            sum_of_sine_angle += part.position().rotation.angle().sin();
            sum_of_cosine_angle += part.position().rotation.angle().cos();
            mass += part_mass;
        }
        center_of_mass /= mass;
        let mean_angle = sum_of_sine_angle.atan2(sum_of_cosine_angle);

        // We pretend that each vertex represents an equal amount of mass (which is
        // obviously not true due to the design of Lyon's tessellator).
        let mut angular_momentum: RealField = 0.0;
        let mut sum_of_radius_squared: RealField = 0.0;
        let mut velocity: Vector2<RealField> = Vector2::new(0.0, 0.0);
        let generalized_velocities = body.generalized_velocity();
        let generalized_positions = body.deformed_positions().unwrap().1;
        for i in (0..generalized_velocities.len()).step_by(2) {
            let vertex = Vector3::new(generalized_positions[i], generalized_positions[i + 1], 0.0);
            let part_velocity = Vector3::new(
                generalized_velocities[i],
                generalized_velocities[i + 1],
                0.0,
            );
            let radius = vertex - Vector3::new(center_of_mass.x, center_of_mass.y, 0.0);

            // We don't divide by r^2 here because (I think) it cancels out
            // with the moment of inertia, so later we just multiply by the linear mass.
            angular_momentum += radius.cross(&part_velocity).z;
            sum_of_radius_squared += radius.dot(&radius);

            velocity += part_velocity.xy();
        }
        angular_momentum *= mass;

        self.derived_measurements = PhysicsBodyMeasurements {
            angular_momentum,
            velocity,
            sum_of_radius_squared,
            mass,
            center_of_mass,
            mean_angle,
        };
    }

    fn has_feet_contact(
        &self,
        colliders: &DefaultColliderSet<RealField>,
        geometrical_world: &DefaultGeometricalWorld<RealField>,
    ) -> bool {
        let collider = colliders.get(self.collider).unwrap();
        if collider.graph_index().is_some() {
            for (_, _, _, _, _, manifold) in geometrical_world
                .contacts_with(colliders, self.collider, true)
                .unwrap()
            {
                for contact in manifold.contacts() {
                    if contact.contact.world1.y < self.derived_measurements.center_of_mass.y {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    fn step_variable_jump_force(&mut self, dt: RealField) {
        self.forces.jump_force -= PHYSICS_CONFIG.variable_jump_force_decay * dt;
        if !self.inputs.jump {
            self.forces.jump_force = 0.0;
        } else if self.forces.jump_force <= 0.0 && self.semiderived_collision_state.has_feet_contact
        {
            self.forces.jump_force = PHYSICS_CONFIG.variable_jump_force_initial;
        } else if self.forces.jump_force < 0.0 {
            self.forces.jump_force = 0.0;
        }
    }

    fn step_horizontal_force(&mut self) {
        self.forces.horizontal_force = ((self.inputs.right as i32 as RealField)
            - (self.inputs.left as i32 as RealField))
            * if !self.semiderived_collision_state.has_feet_contact {
                PHYSICS_CONFIG.horizontal_air_movement_force
            } else if self.inputs.roll {
                PHYSICS_CONFIG.horizontal_rolling_movement_force
            } else {
                PHYSICS_CONFIG.horizontal_crawling_movement_force
            };
    }

    fn step_crawl_force_peak_pos(&mut self, dt: RealField) -> RealField {
        // Timer counts upwards from 0 and wraps around at 1.0
        self.forces.crawl_side_timer += PHYSICS_CONFIG.crawl_side_frequency * dt;
        self.forces.crawl_side_timer %= 1.0;

        // We want peak pos to oscillate between -1.0 and 1.0
        if self.forces.crawl_side_timer > 0.5 {
            3.0 - 4.0 * self.forces.crawl_side_timer
        } else {
            4.0 * self.forces.crawl_side_timer - 1.0
        }
    }

    fn apply_linear_forces(
        &self,
        body: &mut dyn Body<RealField>,
        should_crawl: bool,
        crawl_force_peak_pos: RealField,
        drag: RealField,
    ) {
        let jump_force_per_part = self.forces.jump_force / body.num_parts() as RealField;
        let average_horizontal_force_per_part =
            self.forces.horizontal_force / body.num_parts() as RealField;
        for i in 0..body.num_parts() {
            let horizontal_force = if should_crawl {
                let pos = body.part(i).unwrap().position().translation.vector.x
                    - self.derived_measurements.center_of_mass.x;

                let multiplier =
                    PHYSICS_CONFIG.crawl_side_amplitude * pos * crawl_force_peak_pos + 1.0;

                average_horizontal_force_per_part * multiplier
            } else {
                average_horizontal_force_per_part
            };

            // Don't apply ground drag to y axis to prevent artificial bounciness due to the
            // impact incidence velocity.
            let force = Vector2::new(horizontal_force + drag, jump_force_per_part);
            body.apply_force(i, &Force::linear(force), ForceType::Force, true);
        }
    }

    fn apply_rotation_lock(&self, body: &mut dyn Body<RealField>) {
        // We want to get rid of all this angular momentum, so let's apply
        // linear impulses to each triangular element of this FEMSurface
        // proportional to the distance from the center of mass.

        // For stability reasons, we will only apply a fraction of the
        // impulse needed to "apply angular braking" over several timesteps.
        let angular_momentum_compensation_per_radius = self.derived_measurements.angular_momentum
            / self.derived_measurements.sum_of_radius_squared
            * PHYSICS_CONFIG.angular_momentum_compensation_ratio;

        // Finally, apply a proportional controller to correct the dango's angle
        // so it is facing upright.
        let angle_compensation_per_radius = self.derived_measurements.mean_angle
            * PHYSICS_CONFIG.angle_proportional_controller_coefficient;

        // If the dango's angle is more than some angle offset and has sufficient angular
        // momentum going away from the 0 degree offset, then invert the angle compensator
        // to allow the dango to naturally roll back to the 0 degree. This is to prevent
        // weird jerky motions by the compensator. I.e., go with the flow.
        // TODO: I think this is dimensionally incorrect - missing sum(r^2)
        // This means that when the number of body parts change, the behaviour will
        // become very different. We may need to recallibrate and rethink about this
        // expression the next time we change the tessellator's tolerance parameter.
        let past_stable_angle =
            self.derived_measurements.mean_angle.abs() > PHYSICS_CONFIG.stable_angle_margin;
        let is_rolling_away = self.derived_measurements.angular_momentum.signum()
            * self.derived_measurements.mean_angle.signum()
            > 0.0;
        let angle_natural_compensation_per_radius = if past_stable_angle && is_rolling_away {
            -angle_compensation_per_radius
        } else {
            angle_compensation_per_radius
        };

        // We now apply these two compensators to each triangular element.
        for i in 0..body.num_parts() {
            let part = body.part(i).unwrap();
            let radius: Vector2<RealField> =
                part.position().translation.vector - self.derived_measurements.center_of_mass;

            let impulse = Force::linear(Vector2::new(
                // Apply it tangentially, i.e. perpendicular to the direction to the
                // center of mass.
                radius.y * angular_momentum_compensation_per_radius,
                -radius.x * angular_momentum_compensation_per_radius,
            ));
            body.apply_force(i, &impulse, ForceType::Impulse, true);

            let force = Force::linear(Vector2::new(
                // Apply it tangentially, i.e. perpendicular to the direction to the
                // center of mass.
                radius.y * angle_natural_compensation_per_radius,
                -radius.x * angle_natural_compensation_per_radius,
            ));
            body.apply_force(i, &force, ForceType::Force, true);
        }
    }

    pub fn pre_step(&mut self, dt: RealField, bodies: &mut DefaultBodySet<RealField>) {
        let body = bodies.get_mut(self.body).unwrap();

        self.update_measurements(body);

        let should_lock_rotation = !self.inputs.roll;
        let should_crawl = !self.inputs.roll && self.semiderived_collision_state.has_feet_contact;
        let should_apply_drag = self.semiderived_collision_state.has_feet_contact;

        self.step_variable_jump_force(dt);
        self.step_horizontal_force();
        let crawl_force_peak_pos = self.step_crawl_force_peak_pos(dt);

        // Add in some fictitious ground drag to prevent dangos from accelerating to infinity.
        let drag = if should_apply_drag {
            // Don't apply ground drag to y axis to prevent artificial bounciness due to the
            // impact incidence velocity.
            -PHYSICS_CONFIG.ground_drag * self.derived_measurements.velocity.x
                / body.num_parts() as RealField
        } else {
            0.0
        };

        self.apply_linear_forces(body, should_crawl, crawl_force_peak_pos, drag);

        if should_lock_rotation {
            self.apply_rotation_lock(body);
        }
    }

    pub fn post_step(
        &mut self,
        colliders: &DefaultColliderSet<RealField>,
        geometrical_world: &DefaultGeometricalWorld<RealField>,
    ) {
        self.semiderived_collision_state.has_feet_contact =
            self.has_feet_contact(colliders, geometrical_world);
    }

    pub fn snapshot(&self, bodies: &DefaultBodySet<RealField>) -> PlayerSnapshot {
        let body = bodies.get(self.body).unwrap();
        PlayerSnapshot {
            color: self.color,
            size: self.size,
            positions: body.deformed_positions().unwrap().1.into(),
            velocities: body.generalized_velocity().iter().copied().collect(),
            inputs: self.inputs.clone(),
            forces: self.forces.clone(),
            semiderived_collision_state: self.semiderived_collision_state.clone(),
        }
    }

    pub fn display_state(&self, bodies: &DefaultBodySet<RealField>) -> PlayerDisplayState {
        let body = bodies.get(self.body).unwrap();
        let to_local_coords = Similarity2::new(
            self.derived_measurements.center_of_mass,
            self.derived_measurements.mean_angle,
            self.size,
        )
        .inverse();
        PlayerDisplayState {
            color: self.color,
            size: self.size,
            measurements: self.derived_measurements.clone(),
            local_positions: body
                .deformed_positions()
                .unwrap()
                .1
                .chunks_exact(2)
                .map(|pos| {
                    to_local_coords.transform_point(&Point2::new(pos[0], pos[1]))
                        - Point2::new(0.0, 0.0)
                })
                .map(|pos| {
                    let norm = pos.norm();
                    if norm > 0.0 {
                        pos * (norm + Self::COLLISION_MARGIN * 2.0) / norm
                    } else {
                        pos
                    }
                })
                .collect(),
            local_velocities: body
                .generalized_velocity()
                .iter()
                .collect::<Vec<_>>()
                .chunks_exact(2)
                .map(|v| to_local_coords.transform_vector(&Vector2::new(*v[0], *v[1])))
                .collect(),
            mesh_indices: self.derived_mesh_indices.clone(),
            boundary_indices: self.derived_boundary_indices.clone(),
        }
    }
}

impl DisplayState for PlayerDisplayState {
    fn from_interpolation(old_state: &Self, new_state: &Self, t_f64: f64) -> Self {
        let t = t_f64 as f32;
        let mut state = new_state.clone();

        // Nonlinearly interpolate the derived mean angles.
        let weighted_old_sine = (1.0 - t) * old_state.measurements.mean_angle.sin();
        let weighted_old_cosine = (1.0 - t) * old_state.measurements.mean_angle.cos();
        let weighted_new_sine = t * new_state.measurements.mean_angle.sin();
        let weighted_new_cosine = t * new_state.measurements.mean_angle.cos();
        let interpolated_sine = weighted_old_sine + weighted_new_sine;
        let interpolated_cosine = weighted_old_cosine + weighted_new_cosine;
        state.measurements.mean_angle = interpolated_sine.atan2(interpolated_cosine);

        // Lerp the other derived measurements.
        state.measurements.angular_momentum = (1.0 - t) * old_state.measurements.angular_momentum
            + t * new_state.measurements.angular_momentum;
        state.measurements.velocity =
            (1.0 - t) * old_state.measurements.velocity + t * new_state.measurements.velocity;
        state.measurements.sum_of_radius_squared = (1.0 - t)
            * old_state.measurements.sum_of_radius_squared
            + t * new_state.measurements.sum_of_radius_squared;
        state.measurements.mass =
            (1.0 - t) * old_state.measurements.mass + t * new_state.measurements.mass;
        state.measurements.center_of_mass = (1.0 - t) * old_state.measurements.center_of_mass
            + t * new_state.measurements.center_of_mass;

        // Lerp the positions and velocities in local coordinates.
        // We don't lerp the global positions because that would cause the interpolated body to
        // implode if there is a rotation difference between old and new states.
        // TODO: We can possibly vectorize out the loops.
        // TODO: We can possible cache the local coordinate results since that's what we use in the
        // systems later on.
        let positions_zipped = state.local_positions.iter_mut().zip(
            old_state
                .local_positions
                .iter()
                .zip(new_state.local_positions.iter()),
        );
        for (position, (old_position, new_position)) in positions_zipped {
            *position = (1.0 - t) * old_position + t * new_position;
        }
        let velocities_zipped = state.local_velocities.iter_mut().zip(
            old_state
                .local_velocities
                .iter()
                .zip(new_state.local_velocities.iter()),
        );
        for (velocity, (old_velocity, new_velocity)) in velocities_zipped {
            *velocity = (1.0 - t) * old_velocity + t * new_velocity;
        }

        state
    }
}
