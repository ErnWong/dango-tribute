use bevy::prelude::*;
use bevy_contrib_inspector::{Inspectable, InspectorPlugin};
use nphysics2d::{
    force_generator::{DefaultForceGeneratorSet, ForceGenerator},
    math::{Force, ForceType},
    nalgebra::Vector2,
    object::{BodySet, DefaultBodyHandle, DefaultColliderSet},
    solver::IntegrationParameters,
    world::DefaultGeometricalWorld,
};
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
    #[inspectable(min = 0.0, max = 100.0)]
    horizontal_ground_movement_force: RealField,
    #[inspectable(min = 0.0, max = 100.0)]
    horizontal_air_movement_force: RealField,
}

impl Default for ControlledDangoConfig {
    fn default() -> Self {
        Self {
            variable_jump_force_initial: 80.0,
            variable_jump_force_decay: 600.0,
            horizontal_ground_movement_force: 10.0,
            horizontal_air_movement_force: 2.0,
        }
    }
}

impl ControlledDangoComponent {
    pub fn update_controls(&mut self, left: bool, right: bool, jump: bool, in_air: bool) {
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
            } else {
                state.config.horizontal_ground_movement_force
            };
    }
}

impl ForceGenerator<RealField, DefaultBodyHandle> for ControlledDangoForceGenerator {
    fn apply(
        &mut self,
        parameters: &IntegrationParameters<RealField>,
        bodies: &mut dyn BodySet<RealField, Handle = DefaultBodyHandle>,
    ) {
        let mut state = self.state.lock().unwrap();
        state.applying_force[1] -= state.config.variable_jump_force_decay * parameters.dt();
        if state.applying_force[1] <= 0.0 {
            state.applying_force[1] = 0.0;
        }
        // Note: We skip force application if the desired control force is zero. This is to allow
        // bodies to enter sleep mode in the physics engine whenever they get the chance.
        if state.applying_force != Vector2::new(0.0, 0.0) {
            if let Some(body) = bodies.get_mut(self.body_handle) {
                // TODO: Implement non-uniform force distribution.
                let force_per_part = state.applying_force / body.num_parts() as RealField;
                for i in 0..body.num_parts() {
                    body.apply_force(i, &Force::linear(force_per_part), ForceType::Force, true);
                }
            }
        }
    }
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

        let mut in_air = true;
        if let Some(collider) = colliders.get(collider_handle.handle()) {
            if collider.graph_index().is_some() {
                for (_, _, _, _, _, manifold) in geometrical_world
                    .contacts_with(&*colliders, collider_handle.handle(), true)
                    .unwrap()
                {
                    for contact in manifold.contacts() {
                        if contact.contact.world1[1] < transform.translation.y {
                            in_air = false;
                        }
                    }
                }
            }
        }

        // Note: We handle force application in a dedicated ForceGenerator because the physics
        // simulation could go through several integration steps between each time this system
        // is called.
        controlled_dango.update_controls(left, right, jump, in_air);
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
