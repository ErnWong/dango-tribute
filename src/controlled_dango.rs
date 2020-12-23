use super::physics::NPhysicsBodyHandleComponent;
use bevy::prelude::*;
use nphysics2d::{
    math::{Force, ForceType},
    nalgebra::Vector2,
    object::DefaultBodySet,
};

use super::RealField;

pub struct ControlledDangoComponent {}

pub fn controlled_dango_system(
    input: Res<Input<KeyCode>>,
    mut bodies: ResMut<DefaultBodySet<RealField>>,
    query: Query<(&ControlledDangoComponent, &NPhysicsBodyHandleComponent)>,
) {
    for (_, body_handle) in &mut query.iter() {
        if let Some(body) = bodies.get_mut(body_handle.handle()) {
            let horizontal_movement = ((input.pressed(KeyCode::D) || input.pressed(KeyCode::Right))
                as i32
                - (input.pressed(KeyCode::A) || input.pressed(KeyCode::Left)) as i32)
                as f32;
            // TODO: Apply to whole body, not a single part.
            body.apply_force(
                0,
                &Force::linear(Vector2::x() * horizontal_movement * 7.0),
                ForceType::Force,
                true,
            );
            if input.just_pressed(KeyCode::W)
                || input.just_pressed(KeyCode::Space)
                || input.just_pressed(KeyCode::Up)
            {
                body.apply_force(
                    0,
                    &Force::linear(Vector2::y() * 7.0),
                    ForceType::Impulse,
                    true,
                );
            }
        }
    }
}
