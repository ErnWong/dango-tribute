//! # Naia Server Socket
//! Provides an abstraction of a Socket capable of sending/receiving to many
//! clients, using either an underlying UdpSocket or a service that can
//! communicate via unreliable WebRTC datachannels

#![deny(
    missing_docs,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

extern crate log;

#[macro_use]
extern crate cfg_if;

cfg_if! {
    if #[cfg(feature = "use-wbindgen")] {
        extern crate serde_derive;
    }
    else {
    }
}

pub use naia_socket_shared::LinkConditionerConfig;

mod error;
mod impls;
// mod link_conditioner;
mod message_sender;
mod packet;
mod server_socket_trait;

pub use error::NaiaServerSocketError;
pub use impls::ServerSocket;
pub use message_sender::MessageSender;
pub use naia_socket_shared::find_my_ip_address;
pub use packet::Packet;
pub use server_socket_trait::ServerSocketTrait;

cfg_if! {
    if #[cfg(all(feature = "use-udp", feature = "use-webrtc"))]
    {
        // Use both protocols...
        compile_error!("Naia Server Socket can only use UDP or WebRTC, you must pick one");
    }
    else if #[cfg(all(not(feature = "use-udp"), not(feature = "use-webrtc"), not(feature="use-wbindgen")))]
    {
        // Use no protocols...
        compile_error!("Naia Server Socket requires either the 'use-udp' or 'use-webrtc' or 'use-wbindgen' feature to be enabled, you must pick one.");
    }
}
