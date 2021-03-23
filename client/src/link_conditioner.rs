use naia_socket_shared::{link_condition_logic, LinkConditionerConfig, TimeQueue};

use crate::MessageSender;

use super::{client_socket::ClientSocketTrait, error::NaiaClientSocketError, packet::Packet};

#[derive(Debug)]
pub struct LinkConditioner {
    config: LinkConditionerConfig,
    inner_socket: Box<dyn ClientSocketTrait>,
    time_queue: TimeQueue<Packet>,
}

impl LinkConditioner {
    pub fn new(config: &LinkConditionerConfig, socket: Box<dyn ClientSocketTrait>) -> Self {
        LinkConditioner {
            config: config.clone(),
            inner_socket: socket,
            time_queue: TimeQueue::new(),
        }
    }
}

impl ClientSocketTrait for LinkConditioner {
    fn receive(&mut self) -> Result<Option<Packet>, NaiaClientSocketError> {
        loop {
            match self.inner_socket.receive() {
                Ok(event) => match event {
                    None => {
                        break;
                    }
                    Some(packet) => {
                        self.process_packet(packet);
                    }
                },
                Err(error) => {
                    return Err(error);
                }
            }
        }

        if self.has_packet() {
            return Ok(Some(self.get_packet()));
        } else {
            return Ok(None);
        }
    }

    fn get_sender(&mut self) -> MessageSender {
        self.inner_socket.get_sender()
    }

    fn with_link_conditioner(
        self: Box<Self>,
        config: &LinkConditionerConfig,
    ) -> Box<dyn ClientSocketTrait> {
        // Absolutely do not recommend decorating a socket with multiple link
        // conditioners... why would you do this??
        Box::new(LinkConditioner::new(config, self))
    }
}

impl LinkConditioner {
    fn process_packet(&mut self, packet: Packet) {
        link_condition_logic::process_packet(&self.config, &mut self.time_queue, packet);
    }

    fn has_packet(&self) -> bool {
        self.time_queue.has_item()
    }

    fn get_packet(&mut self) -> Packet {
        self.time_queue.pop_item().unwrap()
    }
}
