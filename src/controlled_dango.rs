use bevy::prelude::*;
use bevy_rapier2d::{
    na::Vector2, physics::RigidBodyHandleComponent, rapier::dynamics::RigidBodySet,
};

pub struct ControlledDangoComponent {}

pub fn controlled_dango_system(
    input: Res<Input<KeyCode>>,
    mut bodies: ResMut<RigidBodySet>,
    query: Query<(&ControlledDangoComponent, &RigidBodyHandleComponent)>,
) {
    for (_, body_handle) in &mut query.iter() {
        let body = bodies.get_mut(body_handle.handle()).unwrap();
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
}
