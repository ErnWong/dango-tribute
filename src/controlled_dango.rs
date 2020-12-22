use super::nphysics_sync::NPBodyHandleComponent;
use bevy::prelude::*;
use bevy_rapier2d::{
    na::Vector2, physics::RigidBodyHandleComponent, rapier::dynamics::RigidBodySet,
};
use nphysics2d::object::DefaultBodySet;

use super::RealField;

pub struct ControlledDangoComponent {}

pub fn controlled_dango_system(
    input: Res<Input<KeyCode>>,
    mut rapier_bodies: ResMut<RigidBodySet>,
    mut nphysics_bodies: ResMut<DefaultBodySet<RealField>>,
    rapier_query: Query<(&ControlledDangoComponent, &RigidBodyHandleComponent)>,
    nphysics_query: Query<(&ControlledDangoComponent, &NPBodyHandleComponent)>,
) {
    for (_, body_handle) in &mut rapier_query.iter() {
        let body = rapier_bodies.get_mut(body_handle.handle()).unwrap();
        let horizontal_movement = ((input.pressed(KeyCode::D) || input.pressed(KeyCode::Right))
            as i32
            - (input.pressed(KeyCode::A) || input.pressed(KeyCode::Left)) as i32)
            as f32;
        body.set_linvel(
            body.linvel() + Vector2::new(horizontal_movement * 10.0, 0.0),
            true,
        );
        if input.just_pressed(KeyCode::W)
            || input.just_pressed(KeyCode::Space)
            || input.just_pressed(KeyCode::Up)
        {
            body.set_linvel(
                body.linvel().component_mul(&Vector2::x()) + Vector2::new(0.0, 200.0),
                true,
            );
        }
    }
    for (_, body_handle) in &mut nphysics_query.iter() {
        if let Some(body) = nphysics_bodies.rigid_body_mut(body_handle.handle()) {
            let horizontal_movement = ((input.pressed(KeyCode::D) || input.pressed(KeyCode::Right))
                as i32
                - (input.pressed(KeyCode::A) || input.pressed(KeyCode::Left)) as i32)
                as f32;
            body.set_linear_velocity(
                body.velocity().linear + Vector2::new(horizontal_movement * 10.0, 0.0),
            );
            if input.just_pressed(KeyCode::W)
                || input.just_pressed(KeyCode::Space)
                || input.just_pressed(KeyCode::Up)
            {
                body.set_linear_velocity(
                    body.velocity().linear.component_mul(&Vector2::x()) + Vector2::new(0.0, 200.0),
                );
            }
        }
    }
}
