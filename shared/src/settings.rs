use bevy_prototype_networked_physics::Config as NetworkedPhysicsConfig;

pub const TIMESTEP: f32 = 1.0 / 30.0;
pub const GRAVITY: f32 = -9.81 * 1.5;
pub type RealField = f32;
pub const NETWORKED_PHYSICS_CONFIG: NetworkedPhysicsConfig = NetworkedPhysicsConfig {
    lag_compensation_latency: 0.3,
    interpolation_latency: 0.2,
    timestep_seconds: TIMESTEP,
    heartbeat_period: 5.0,
    snapshot_send_period: 0.1,
    ..NetworkedPhysicsConfig::new()
};
