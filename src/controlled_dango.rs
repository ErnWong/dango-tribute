use super::physics::NPhysicsBodyHandleComponent;
use bevy::prelude::*;
use nphysics2d::{
    force_generator::{DefaultForceGeneratorSet, ForceGenerator},
    math::{Force, ForceType},
    nalgebra::Vector2,
    object::{BodySet, DefaultBodyHandle},
    solver::IntegrationParameters,
};
use std::sync::{Arc, Mutex};

use super::RealField;

#[derive(Default)]
pub struct ControlledDangoComponent {
    // TODO: Is there a better way to restructure this?...
    state: Option<Arc<Mutex<ControlledDangoState>>>,
}

#[derive(Default)]
pub struct ControlledDangoState {
    applying_force: Vector2<RealField>,
}

pub struct ControlledDangoForceGenerator {
    state: Arc<Mutex<ControlledDangoState>>,
    body_handle: DefaultBodyHandle,
}

pub const VARIABLE_JUMP_FORCE_INITIAL: RealField = 10.0;
pub const VARIABLE_JUMP_FORCE_DECAY: RealField = 10.0;
pub const HORIZONTAL_MOVEMENT_FORCE: RealField = 10.0;

impl ControlledDangoComponent {
    pub fn update_controls(&mut self, left: bool, right: bool, jump: bool) {
        // TODO: Don't jump in midair.
        let mut state = self.state.as_ref().unwrap().lock().unwrap();
        if jump {
            if state.applying_force[1] == 0.0 {
                state.applying_force[1] = VARIABLE_JUMP_FORCE_INITIAL;
            }
        } else {
            state.applying_force[1] = 0.0;
        }
        // TODO: Change the horizontal force during midair.
        state.applying_force[0] =
            ((right as i32 as RealField) - (left as i32 as RealField)) * HORIZONTAL_MOVEMENT_FORCE;
    }
}

impl ForceGenerator<RealField, DefaultBodyHandle> for ControlledDangoForceGenerator {
    fn apply(
        &mut self,
        parameters: &IntegrationParameters<RealField>,
        bodies: &mut dyn BodySet<RealField, Handle = DefaultBodyHandle>,
    ) {
        let mut state = self.state.lock().unwrap();
        state.applying_force[1] -= VARIABLE_JUMP_FORCE_DECAY * parameters.dt();
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
    mut force_generators: ResMut<DefaultForceGeneratorSet<RealField>>,
    mut query: Query<(&mut ControlledDangoComponent, &NPhysicsBodyHandleComponent)>,
) {
    for (mut controlled_dango, body_handle) in query.iter_mut() {
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

        // Note: We handle force application in a dedicated ForceGenerator because the physics
        // simulation could go through several integration steps between each time this system
        // is called.
        controlled_dango.update_controls(left, right, jump);
    }
}
