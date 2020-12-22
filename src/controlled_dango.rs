use super::physics::NPhysicsBodyHandleComponent;
use bevy::prelude::*;
use nphysics2d::{nalgebra::Vector2, object::DefaultBodySet};

use super::RealField;

pub struct ControlledDangoComponent {}

pub fn controlled_dango_system(
    input: Res<Input<KeyCode>>,
    mut bodies: ResMut<DefaultBodySet<RealField>>,
    query: Query<(&ControlledDangoComponent, &NPhysicsBodyHandleComponent)>,
) {
    for (_, body_handle) in &mut query.iter() {
        if let Some(body) = bodies.rigid_body_mut(body_handle.handle()) {
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
