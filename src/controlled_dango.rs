use bevy::prelude::*;
use bevy_contrib_inspector::{Inspectable, InspectorPlugin};
use nphysics2d::{
    force_generator::{DefaultForceGeneratorSet, ForceGenerator},
    math::{Force, ForceType},
    nalgebra::{Vector2, Vector3},
    object::{Body, BodySet, DefaultBodyHandle, DefaultColliderHandle, DefaultColliderSet},
    solver::IntegrationParameters,
    world::DefaultGeometricalWorld,
};
use std::f32::consts::PI;
use std::sync::{Arc, Mutex};

use super::physics::{NPhysicsBodyHandleComponent, NPhysicsColliderHandleComponent};
use super::RealField;

pub struct ControlledDangoPlugin;

impl Plugin for ControlledDangoPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(controlled_dango_system.system());

        if cfg!(feature = "inspect-control-config") {
            app.add_plugin(InspectorPlugin::<ControlledDangoConfig>::new())
                .add_system(update_config_system.system());
        }
    }
}

#[derive(Default)]
pub struct ControlledDangoComponent {
    // TODO: Is there a better way to restructure this?...
    state: Option<Arc<Mutex<ControlledDangoState>>>,
}

#[derive(Default)]
pub struct ControlledDangoState {
    applying_force: Vector2<RealField>,
    applying_drag: bool,
    lock_rotation: bool,
    center_of_mass: Vector2<RealField>,
    is_crawling: bool,
    crawl_side_timer: RealField,
    config: ControlledDangoConfig,
}

pub struct ControlledDangoForceGenerator {
    state: Arc<Mutex<ControlledDangoState>>,
    body_handle: DefaultBodyHandle,
}

#[derive(Inspectable, Clone)]
pub struct ControlledDangoConfig {
    #[inspectable(min = 0.0, max = 100.0)]
    variable_jump_force_initial: RealField,

    #[inspectable(min = 0.0, max = 1000.0)]
    variable_jump_force_decay: RealField,

    #[inspectable(min = 0.0, max = 0.2)]
    ground_drag: RealField,

    #[inspectable(min = 0.0, max = 100.0)]
    horizontal_crawling_movement_force: RealField,

    #[inspectable(min = 0.0, max = 100.0)]
    horizontal_rolling_movement_force: RealField,

    #[inspectable(min = 0.0, max = 100.0)]
    horizontal_air_movement_force: RealField,

    #[inspectable(min = 0.0, max = 1.0)]
    angular_momentum_compensation_ratio: RealField,

    #[inspectable(min = 0.0, max = 40.0)]
    angle_proportional_controller_coefficient: RealField,

    #[inspectable(min = 0.0, max=PI)]
    stable_angle_margin: RealField,

    #[inspectable(min = 1.0, max = 10.0)]
    crawl_side_frequency: RealField,

    #[inspectable(min = 0.0, max = 10.0)]
    crawl_side_amplitude: RealField,
}

impl Default for ControlledDangoConfig {
    fn default() -> Self {
        Self {
            variable_jump_force_initial: 80.0,
            variable_jump_force_decay: 500.0,
            ground_drag: 0.1,
            horizontal_crawling_movement_force: 7.0,
            horizontal_rolling_movement_force: 12.0,
            horizontal_air_movement_force: 2.0,
            angular_momentum_compensation_ratio: 0.16,
            angle_proportional_controller_coefficient: 25.0,
            stable_angle_margin: 0.3 * PI,
            crawl_side_frequency: 2.0,
            crawl_side_amplitude: 20.0,
        }
    }
}

impl ControlledDangoComponent {
    pub fn update_controls(
        &mut self,
        left: bool,
        right: bool,
        jump: bool,
        roll: bool,
        in_air: bool,
        translation: Vec3,
    ) {
        let mut state = self.state.as_ref().unwrap().lock().unwrap();
        if jump {
            if state.applying_force[1] == 0.0 && !in_air {
                state.applying_force[1] = state.config.variable_jump_force_initial;
            }
        } else {
            state.applying_force[1] = 0.0;
        }
        state.applying_force[0] = ((right as i32 as RealField) - (left as i32 as RealField))
            * if in_air {
                state.config.horizontal_air_movement_force
            } else if roll {
                state.config.horizontal_rolling_movement_force
            } else {
                state.config.horizontal_crawling_movement_force
            };
        state.lock_rotation = !roll;
        state.is_crawling = !roll && !in_air;
        state.applying_drag = !in_air;
        state.center_of_mass = Vector2::new(translation.x, translation.y);
    }
}

impl ForceGenerator<RealField, DefaultBodyHandle> for ControlledDangoForceGenerator {
    fn apply(
        &mut self,
        parameters: &IntegrationParameters<RealField>,
        bodies: &mut dyn BodySet<RealField, Handle = DefaultBodyHandle>,
    ) {
        let mut state = self.state.lock().unwrap();

        // Update variable jump force.
        state.applying_force[1] -= state.config.variable_jump_force_decay * parameters.dt();
        if state.applying_force[1] <= 0.0 {
            state.applying_force[1] = 0.0;
        }

        // Timer counts upwards from 0 and wraps around at 1.0
        state.crawl_side_timer += state.config.crawl_side_frequency * parameters.dt();
        state.crawl_side_timer %= 1.0;

        // We want peak pos to oscillate between -1.0 and 1.0
        let crawl_force_peak_pos = if state.crawl_side_timer > 0.5 {
            3.0 - 4.0 * state.crawl_side_timer
        } else {
            4.0 * state.crawl_side_timer - 1.0
        };

        if let Some(body) = bodies.get_mut(self.body_handle) {
            // Dear physicists and mechanical engineers, please don't hate me for what
            // I'm about to do...
            let estimate = estimate_dango_state(body, &state);

            // Add in some fictitious ground drag to prevent dangos from accelerating to infinity.
            let drag = if state.applying_drag {
                // Don't apply ground drag to y axis to prevent artificial bounciness due to the
                // impact incidence velocity.
                -state.config.ground_drag * estimate.velocity.x / body.num_parts() as RealField
            } else {
                0.0
            };

            let force_per_part = state.applying_force / body.num_parts() as RealField;
            for i in 0..body.num_parts() {
                let mut force = force_per_part.clone();
                if state.is_crawling {
                    let pos = body.part(i).unwrap().position().translation.vector.x
                        - state.center_of_mass.x;

                    let multiplier =
                        state.config.crawl_side_amplitude * pos * crawl_force_peak_pos + 1.0;

                    force.x *= multiplier;
                }
                // Don't apply ground drag to y axis to prevent artificial bounciness due to the
                // impact incidence velocity.
                force.x += drag;
                body.apply_force(i, &Force::linear(force), ForceType::Force, true);
            }

            if state.lock_rotation {
                // We want to get rid of all this angular momentum, so let's apply
                // linear impulses to each triangular element of this FEMSurface
                // proportional to the distance from the center of mass.

                // For stability reasons, we will only apply a fraction of the
                // impulse needed to "apply angular braking" over several timesteps.
                let angular_momentum_compensation_per_radius = estimate.angular_momentum
                    / estimate.sum_of_radius_squared
                    * state.config.angular_momentum_compensation_ratio;

                // Finally, apply a proportional controller to correct the dango's angle
                // so it is facing upright.
                let angle = body.part(0).unwrap().position().rotation.angle();
                let angle_compensation_per_radius =
                    angle * state.config.angle_proportional_controller_coefficient;

                // If the dango's angle is more than some angle offset and has sufficient angular
                // momentum going away from the 0 degree offset, then invert the angle compensator
                // to allow the dango to naturally roll back to the 0 degree. This is to prevent
                // weird jerky motions by the compensator. I.e., go with the flow.
                // TODO: I think this is dimensionally incorrect - missing sum(r^2)
                // This means that when the number of body parts change, the behaviour will
                // become very different. We may need to recallibrate and rethink about this
                // expression the next time we change the tessellator's tolerance parameter.
                let past_stable_angle = angle.abs() > state.config.stable_angle_margin;
                let is_rolling_away = estimate.angular_momentum.signum() * angle.signum() > 0.0;
                let angle_natural_compensation_per_radius = if past_stable_angle && is_rolling_away
                {
                    -angle_compensation_per_radius
                } else {
                    angle_compensation_per_radius
                };

                // We now apply these two compensators to each triangular element.
                for i in 0..body.num_parts() {
                    let part = body.part(i).unwrap();
                    let radius = part.position().translation.vector - state.center_of_mass;

                    let impulse = Force::linear(Vector2::new(
                        // Apply it tangentially, i.e. perpendicular to the direction to the
                        // center of mass.
                        radius[1] * angular_momentum_compensation_per_radius,
                        -radius[0] * angular_momentum_compensation_per_radius,
                    ));
                    body.apply_force(i, &impulse, ForceType::Impulse, true);

                    let force = Force::linear(Vector2::new(
                        // Apply it tangentially, i.e. perpendicular to the direction to the
                        // center of mass.
                        radius[1] * angle_natural_compensation_per_radius,
                        -radius[0] * angle_natural_compensation_per_radius,
                    ));
                    body.apply_force(i, &force, ForceType::Force, true);
                }
            }
        }
    }
}

pub struct EstimatedWholeDangoState {
    angular_momentum: RealField,
    velocity: Vector2<RealField>,
    sum_of_radius_squared: RealField,
}

pub fn estimate_dango_state(
    body: &dyn Body<RealField>,
    state: &ControlledDangoState,
) -> EstimatedWholeDangoState {
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
        let radius = vertex - Vector3::new(state.center_of_mass[0], state.center_of_mass[1], 0.0);

        // We don't divide by r^2 here because (I think) it cancels out
        // with the moment of inertia, so later we just multiply by the linear mass.
        angular_momentum += radius.cross(&part_velocity).z;
        sum_of_radius_squared += radius.dot(&radius);

        velocity += part_velocity.xy();
    }

    // How heavy is our dango? I don't know... so how about we just
    // recalculate it every single frame? Because... why not.
    // Real game devs please don't hate me.
    let mut mass: RealField = 0.0;
    for i in 0..body.num_parts() {
        mass += body.part(i).unwrap().inertia().linear;
    }
    angular_momentum *= mass;

    EstimatedWholeDangoState {
        angular_momentum,
        velocity,
        sum_of_radius_squared,
    }
}

pub fn has_feet_contact(
    transform: &Transform,
    collider_handle: DefaultColliderHandle,
    colliders: &DefaultColliderSet<RealField>,
    geometrical_world: &DefaultGeometricalWorld<RealField>,
) -> bool {
    if let Some(collider) = colliders.get(collider_handle) {
        if collider.graph_index().is_some() {
            for (_, _, _, _, _, manifold) in geometrical_world
                .contacts_with(&*colliders, collider_handle, true)
                .unwrap()
            {
                for contact in manifold.contacts() {
                    if contact.contact.world1[1] < transform.translation.y {
                        return true;
                    }
                }
            }
        }
    }
    return false;
}

pub fn controlled_dango_system(
    input: Res<Input<KeyCode>>,
    colliders: Res<DefaultColliderSet<RealField>>,
    geometrical_world: Res<DefaultGeometricalWorld<RealField>>,
    mut force_generators: ResMut<DefaultForceGeneratorSet<RealField>>,
    mut query: Query<(
        &mut ControlledDangoComponent,
        &NPhysicsBodyHandleComponent,
        &NPhysicsColliderHandleComponent,
        &Transform,
    )>,
) {
    for (mut controlled_dango, body_handle, collider_handle, transform) in query.iter_mut() {
        if controlled_dango.state.is_none() {
            let state = Arc::new(Mutex::new(ControlledDangoState::default()));
            controlled_dango.state = Some(state.clone());
            force_generators.insert(Box::new(ControlledDangoForceGenerator {
                state,
                body_handle: body_handle.handle(),
            }));
        }

        let right = input.pressed(KeyCode::D) || input.pressed(KeyCode::Right);
        let left = input.pressed(KeyCode::A) || input.pressed(KeyCode::Left);
        let jump = input.pressed(KeyCode::W)
            || input.pressed(KeyCode::Space)
            || input.pressed(KeyCode::Up);
        let roll = input.pressed(KeyCode::LShift) || input.pressed(KeyCode::RShift);
        let in_air = !has_feet_contact(
            transform,
            collider_handle.handle(),
            &*colliders,
            &*geometrical_world,
        );

        // Note: We handle force application in a dedicated ForceGenerator because the physics
        // simulation could go through several integration steps between each time this system
        // is called.
        controlled_dango.update_controls(left, right, jump, roll, in_air, transform.translation);
    }
}

pub fn update_config_system(
    changed_config: ChangedRes<ControlledDangoConfig>,
    query: Query<&ControlledDangoComponent>,
) {
    for controlled_dango in query.iter() {
        controlled_dango
            .state
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .config = changed_config.clone();
    }
}
