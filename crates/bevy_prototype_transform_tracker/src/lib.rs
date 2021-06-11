use bevy::prelude::*;

pub struct TransformTrackingPlugin;
pub struct TransformTrackingTarget;
pub struct TransformTrackingFollower;

impl Plugin for TransformTrackingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(transform_tracking_system.system());
    }
}

pub fn transform_tracking_system(
    target_query: Query<(&TransformTrackingTarget, &Transform)>,
    mut follower_query: Query<
        (&TransformTrackingFollower, &mut Transform),
        Without<TransformTrackingTarget>,
    >,
) {
    for (_, target_transform) in target_query.iter() {
        let target_pos = target_transform.translation;
        for (_, mut follower_transform) in follower_query.iter_mut() {
            let mut follower_pos = &mut follower_transform.translation;
            follower_pos.x += (target_pos.x - follower_pos.x) * 0.1;
            follower_pos.y += (target_pos.y - follower_pos.y) * 0.1;
        }
    }
}
