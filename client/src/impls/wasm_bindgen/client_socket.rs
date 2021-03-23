extern crate log;
use log::info;

use std::{collections::VecDeque, net::SocketAddr};

use crate::{
    error::NaiaClientSocketError, link_conditioner::LinkConditioner, ClientSocketTrait,
    MessageSender, Packet,
};

use naia_socket_shared::{LinkConditionerConfig, Ref};

use super::webrtc_internal::webrtc_initialize;

/// A client-side socket which communicates with an underlying unordered &
/// unreliable protocol
#[derive(Debug)]
pub struct ClientSocket {
    address: SocketAddr,
    message_queue: Ref<VecDeque<Result<Option<Packet>, NaiaClientSocketError>>>,
    message_sender: MessageSender,
    dropped_outgoing_messages: Ref<VecDeque<Packet>>,
}

impl ClientSocket {
    /// Returns a new ClientSocket, connected to the given socket address
    pub fn connect(server_socket_address: SocketAddr) -> Box<dyn ClientSocketTrait> {
        let message_queue = Ref::new(VecDeque::new());
        let data_channel = webrtc_initialize(server_socket_address, message_queue.clone());

        let dropped_outgoing_messages = Ref::new(VecDeque::new());

        let message_sender =
            MessageSender::new(data_channel.clone(), dropped_outgoing_messages.clone());

        Box::new(ClientSocket {
            address: server_socket_address,
            message_queue,
            message_sender,
            dropped_outgoing_messages,
        })
    }
}

#[allow(unsafe_code)]
#[cfg(feature = "multithread")]
unsafe impl Send for ClientSocket {}
#[allow(unsafe_code)]
#[cfg(feature = "multithread")]
unsafe impl Sync for ClientSocket {}

impl ClientSocketTrait for ClientSocket {
    fn receive(&mut self) -> Result<Option<Packet>, NaiaClientSocketError> {
        if !self.dropped_outgoing_messages.borrow().is_empty() {
            if let Some(dropped_packets) = {
                let mut dom = self.dropped_outgoing_messages.borrow_mut();
                let dropped_packets: Vec<Packet> = dom.drain(..).collect::<Vec<Packet>>();
                Some(dropped_packets)
            } {
                for dropped_packet in dropped_packets {
                    self.message_sender
                        .send(dropped_packet)
                        .unwrap_or_else(|err| {
                            info!("Can't send dropped packet. Original Error: {:?}", err)
                        });
                }
            }
        }

        loop {
            if self.message_queue.borrow().is_empty() {
                return Ok(None);
            }

            match self
                .message_queue
                .borrow_mut()
                .pop_front()
                .expect("message queue shouldn't be empty!")
            {
                Ok(Some(packet)) => {
                    return Ok(Some(packet));
                }
                Ok(inner) => {
                    return Ok(inner);
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
    }

    fn get_sender(&mut self) -> MessageSender {
        return self.message_sender.clone();
    }

    fn with_link_conditioner(
        self: Box<Self>,
        config: &LinkConditionerConfig,
    ) -> Box<dyn ClientSocketTrait> {
        Box::new(LinkConditioner::new(config, self))
    }
}
