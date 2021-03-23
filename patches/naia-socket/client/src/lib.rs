//! # Naia Client Socket
//! A Socket abstraction over either a UDP socket on native Linux, or a
//! unreliable WebRTC datachannel on the browser

#![deny(
    missing_docs,
    missing_debug_implementations,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

#[macro_use]
extern crate cfg_if;

cfg_if! {
    if #[cfg(all(target_arch = "wasm32", feature = "wbindgen"))] {
        #[macro_use]
        extern crate serde_derive;
    }
    else {
    }
}

pub use naia_socket_shared::LinkConditionerConfig;

mod client_socket;
mod error;
mod impls;
mod link_conditioner;
mod packet;

pub use client_socket::ClientSocketTrait;
pub use error::NaiaClientSocketError;
pub use impls::{ClientSocket, MessageSender};
pub use naia_socket_shared::find_my_ip_address;
pub use packet::Packet;
