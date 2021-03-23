extern crate log;

use std::{
    io::ErrorKind,
    net::{SocketAddr, UdpSocket},
};

use naia_socket_shared::{find_available_port, find_my_ip_address, LinkConditionerConfig, Ref};

use crate::{link_conditioner::LinkConditioner, ClientSocketTrait, MessageSender};

use crate::{error::NaiaClientSocketError, Packet};

use std::net::ToSocketAddrs;

/// A client-side socket which communicates with an underlying unordered &
/// unreliable protocol
#[derive(Debug)]
pub struct ClientSocket {
    address: SocketAddr,
    socket: Ref<UdpSocket>,
    receive_buffer: Vec<u8>,
    message_sender: MessageSender,
}

impl ClientSocket {
    /// Returns a new ClientSocket, connected to the given socket address
    pub fn connect(server_socket_address_string: String) -> Box<dyn ClientSocketTrait> {
        let client_ip_address = find_my_ip_address().expect("cannot find current ip address");
        let free_socket = find_available_port(&client_ip_address).expect("no available ports");
        let client_socket_address = format!("{}:{}", client_ip_address, free_socket);

        let socket = Ref::new(UdpSocket::bind(client_socket_address).unwrap());
        socket
            .borrow()
            .set_nonblocking(true)
            .expect("can't set socket to non-blocking!");

        let server_socket_address = server_socket_address_string
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap();

        let message_sender = MessageSender::new(server_socket_address, socket.clone());

        Box::new(ClientSocket {
            address: server_socket_address,
            socket,
            receive_buffer: vec![0; 1472],
            message_sender,
        })
    }
}

impl ClientSocketTrait for ClientSocket {
    fn receive(&mut self) -> Result<Option<Packet>, NaiaClientSocketError> {
        let buffer: &mut [u8] = self.receive_buffer.as_mut();
        match self
            .socket
            .borrow()
            .recv_from(buffer)
            .map(move |(recv_len, address)| (&buffer[..recv_len], address))
        {
            Ok((payload, address)) => {
                if address == self.address {
                    return Ok(Some(Packet::new(payload.to_vec())));
                } else {
                    return Err(NaiaClientSocketError::Message(
                        "Unknown sender.".to_string(),
                    ));
                }
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                //just didn't receive anything this time
                return Ok(None);
            }
            Err(e) => {
                return Err(NaiaClientSocketError::Wrapped(Box::new(e)));
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
