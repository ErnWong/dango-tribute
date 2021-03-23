use async_trait::async_trait;

use naia_socket_shared::LinkConditionerConfig;

use super::{message_sender::MessageSender, packet::Packet};
use crate::error::NaiaServerSocketError;

/// Defines the functionality of a Naia Server Socket
#[async_trait]
pub trait ServerSocketTrait: Send + Sync {
    /// Receive a new packet from the socket, or a tick event
    async fn receive(&mut self) -> Result<Packet, NaiaServerSocketError>;
    /// Gets a MessageSender you can use to send messages through the Server
    /// Socket
    fn get_sender(&mut self) -> MessageSender;
    /// Wraps the current socket in a LinkConditioner
    fn with_link_conditioner(
        self: Box<Self>,
        config: &LinkConditionerConfig,
    ) -> Box<dyn ServerSocketTrait>;
}
