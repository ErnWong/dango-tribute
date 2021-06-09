use crystalorb_bevy_networking_turbulence::crystalorb::Config as NetworkedPhysicsConfig;

pub const TIMESTEP: f64 = 1.0 / 60.0;
pub const GRAVITY: f32 = -9.81 * 1.5;
pub type RealField = f32;
pub const NETWORKED_PHYSICS_CONFIG: NetworkedPhysicsConfig = NetworkedPhysicsConfig {
    timestep_seconds: TIMESTEP,
    ..NetworkedPhysicsConfig::new()
};
