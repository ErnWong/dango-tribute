#[macro_use]
extern crate log;

use naia_server_socket::{LinkConditionerConfig, Packet, ServerSocket};
use simple_logger;
use smol::io;

const PING_MSG: &str = "ping";
const PONG_MSG: &str = "pong";

fn main() -> io::Result<()> {
    // IP Address to listen on for the signaling portion of WebRTC
    let session_listen_addr = "127.0.0.1:14191"
        .parse()
        .expect("could not parse HTTP address/port");

    // IP Address to listen on for UDP WebRTC data channels
    let webrtc_listen_addr = "127.0.0.1:14192"
        .parse()
        .expect("could not parse WebRTC data address/port");

    // The public WebRTC IP address to advertise
    let public_webrtc_addr = "127.0.0.1:14192"
        .parse()
        .expect("could not parse advertised public WebRTC data address/port");

    smol::block_on(async {
        simple_logger::init_with_level(log::Level::Info).expect("A logger was already initialized");

        info!("Naia Server Socket Example Started");

        let mut server_socket =
            ServerSocket::listen(session_listen_addr, webrtc_listen_addr, public_webrtc_addr)
                .await
                .with_link_conditioner(&LinkConditionerConfig::good_condition());

        let mut sender = server_socket.get_sender();

        loop {
            match server_socket.receive().await {
                Ok(packet) => {
                    let address = packet.address();
                    let message = String::from_utf8_lossy(packet.payload());
                    info!("Server recv <- {}: {}", address, message);

                    if message.eq(PING_MSG) {
                        let to_client_message: String = PONG_MSG.to_string();
                        info!("Server send -> {}: {}", address, to_client_message);
                        sender
                            .send(Packet::new(address, to_client_message.into_bytes()))
                            .await
                            .expect("send error");
                    }
                }
                Err(error) => {
                    info!("Server Error: {}", error);
                }
            }
        }
    })
}
