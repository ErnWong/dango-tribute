use crate::physics_multiplayer::{PhysicsCommand, PhysicsSnapshot};
use bevy::prelude::*;
use bevy_networking_turbulence::{
    ConnectionChannelsBuilder, MessageChannelMode, MessageChannelSettings, NetworkResource,
    ReliableChannelSettings,
};
use bevy_prototype_networked_physics::{
    channels::ClockSyncMessage, timestamp::Timestamped, Config as NetworkedPhysicsConfig,
};
use std::time::Duration;

pub const TIMESTEP: f32 = 1.0 / 60.0;
pub const GRAVITY: f32 = -9.81 * 1.5;
pub type RealField = f32;
pub const NETWORKED_PHYSICS_CONFIG: NetworkedPhysicsConfig = NetworkedPhysicsConfig {
    timestep_seconds: TIMESTEP,
    ..NetworkedPhysicsConfig::new()
};

pub fn network_setup(mut net: ResMut<NetworkResource>) {
    net.set_channels_builder(|builder: &mut ConnectionChannelsBuilder| {
        builder
            .register::<Timestamped<PhysicsCommand>>(MessageChannelSettings {
                channel: 0,
                channel_mode: MessageChannelMode::Compressed {
                    reliability_settings: ReliableChannelSettings {
                        bandwidth: 4096,
                        recv_window_size: 1024,
                        send_window_size: 1024,
                        burst_bandwidth: 1024,
                        init_send: 512,
                        wakeup_time: Duration::from_millis(100),
                        initial_rtt: Duration::from_millis(200),
                        max_rtt: Duration::from_secs(2),
                        rtt_update_factor: 0.1,
                        rtt_resend_factor: 1.5,
                    },
                    max_chunk_len: 1024,
                },
                message_buffer_size: 64,
                packet_buffer_size: 64,
            })
            .unwrap();

        builder
            .register::<Timestamped<PhysicsSnapshot>>(MessageChannelSettings {
                channel: 1,
                channel_mode: MessageChannelMode::Unreliable,
                message_buffer_size: 64,
                packet_buffer_size: 64,
            })
            .unwrap();

        builder
            .register::<ClockSyncMessage>(MessageChannelSettings {
                channel: 2,
                channel_mode: MessageChannelMode::Unreliable,
                message_buffer_size: 64,
                packet_buffer_size: 64,
            })
            .unwrap();
    });
}
