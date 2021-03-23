use async_io::Timer;
use async_trait::async_trait;
use futures_util::{pin_mut, select, FutureExt};
use std::time::Duration;

use naia_socket_shared::{link_condition_logic, LinkConditionerConfig, TimeQueue};

use super::{
    error::NaiaServerSocketError, message_sender::MessageSender, packet::Packet,
    server_socket_trait::ServerSocketTrait,
};

pub struct LinkConditioner {
    config: LinkConditionerConfig,
    inner_socket: Box<dyn ServerSocketTrait>,
    time_queue: TimeQueue<Packet>,
}

impl LinkConditioner {
    pub fn new(config: &LinkConditionerConfig, socket: Box<dyn ServerSocketTrait>) -> Self {
        LinkConditioner {
            config: config.clone(),
            inner_socket: socket,
            time_queue: TimeQueue::new(),
        }
    }
}

#[async_trait]
impl ServerSocketTrait for LinkConditioner {
    async fn receive(&mut self) -> Result<Packet, NaiaServerSocketError> {
        enum Next {
            Event(Result<Packet, NaiaServerSocketError>),
            BufferedEvent,
        }

        loop {
            let next = {
                let mut queue_duration: Option<std::time::Instant> = None;
                if self.time_queue.len() != 0 {
                    if let Some(container) = self.time_queue.peek_entry() {
                        queue_duration = Some(container.instant.get_inner());
                    }
                }

                let buffered_next = {
                    match queue_duration {
                        Some(instant) => Timer::at(instant).fuse(),
                        None => Timer::at(
                            std::time::Instant::now()
                                + Duration::from_secs_f64(60.0 * 60.0 * 24.0 * 365.0), // delay for a year because I couldn't figure out how to get a never-completing future in here
                        )
                        .fuse(),
                    }
                };
                pin_mut!(buffered_next);

                let socket_next = self.inner_socket.receive().fuse();
                pin_mut!(socket_next);

                select! {
                    socket_result = socket_next => {
                        Next::Event(socket_result)
                    }

                    _ = buffered_next => {
                        Next::BufferedEvent
                    }
                }
            };

            match next {
                Next::Event(result) => match result {
                    Ok(packet) => {
                        self.process_packet(packet);
                    }
                    Err(err) => {
                        return Err(err);
                    }
                },
                Next::BufferedEvent => {
                    if let Some(packet) = self.time_queue.pop_item() {
                        return Ok(packet);
                    }
                }
            }
        }
    }

    fn get_sender(&mut self) -> MessageSender {
        self.inner_socket.get_sender()
    }

    fn with_link_conditioner(
        self: Box<Self>,
        config: &LinkConditionerConfig,
    ) -> Box<dyn ServerSocketTrait> {
        // Absolutely do not recommend decorating a socket with multiple link
        // conditioners... why would you do this??
        Box::new(LinkConditioner::new(config, self))
    }
}

impl LinkConditioner {
    fn process_packet(&mut self, packet: Packet) {
        link_condition_logic::process_packet(&self.config, &mut self.time_queue, packet);
    }
}
