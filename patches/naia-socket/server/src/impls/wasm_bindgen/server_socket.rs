// XXX: This is a hacky frackenstein of:
//  - client wasm_bindgen socket/internals
//  - server webrtc socket
//  - and more
// Minimal effort was put into understanding the nuances of the different storage types employed.

extern crate log;
extern crate serde_derive;
use log::info;

use std::{
    collections::HashMap,
    io::Error as IoError,
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener},
};

use async_trait::async_trait;

use futures_channel::mpsc;
use futures_util::{pin_mut, select, FutureExt, StreamExt};

use naia_socket_shared::LinkConditionerConfig;

use crate::{
    error::NaiaServerSocketError,
    //link_conditioner::LinkConditioner,
    message_sender::MessageSender,
    Packet,
    ServerSocketTrait,
};
use wasm_bindgen::{prelude::*, JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;
use web_sys::{
    ErrorEvent, MessageEvent, RtcConfiguration, RtcDataChannel, RtcDataChannelInit,
    RtcDataChannelType, RtcPeerConnection, RtcPeerConnectionIceEvent, RtcSdpType,
    RtcSessionDescription, RtcSessionDescriptionInit, WebSocket,
};

use rand::Rng;

const CLIENT_CHANNEL_SIZE: usize = 8;

use serde_derive::Serialize;

#[derive(Serialize)]
pub struct IceServerConfig {
    pub urls: [String; 1],
}

/// TODO
#[derive(Debug)]
pub struct ServerSocket {
    to_client_sender: mpsc::Sender<Packet>,
    from_client_receiver: mpsc::Receiver<Result<Packet, IoError>>,
}

impl ServerSocket {
    /// TODO
    pub async fn listen(signalling_server_url: String) -> Box<dyn ServerSocketTrait> {
        let (to_client_sender, mut to_client_receiver) =
            mpsc::channel::<Packet>(CLIENT_CHANNEL_SIZE);
        let (from_client_sender, from_client_receiver) = mpsc::channel(CLIENT_CHANNEL_SIZE);

        let evil = std::thread::spawn(move || {
            spawn_local(async move {
                let (mut new_client_sender, mut new_client_receiver) =
                    mpsc::channel(CLIENT_CHANNEL_SIZE);
                let mut clients: HashMap<SocketAddr, RtcDataChannel> = HashMap::new();
                let signalling_socket = WebSocket::new(signalling_server_url.as_str()).unwrap();

                let signalling_socket_clone = signalling_socket.clone();
                let signalling_socket_onmessage_func: Box<dyn FnMut(MessageEvent)> = Box::new(
                    move |event: MessageEvent| {
                        if let Ok(offer_sdp_string) = event.data().dyn_into::<js_sys::JsString>() {
                            // TODO: we don't know the address unless we try parsing sdp. Too
                            // much effort.
                            let fake_socket_address = SocketAddr::new(
                                IpAddr::V4(Ipv4Addr::new(
                                    rand::thread_rng().gen_range(0..255),
                                    rand::thread_rng().gen_range(0..255),
                                    rand::thread_rng().gen_range(0..255),
                                    rand::thread_rng().gen_range(0..255),
                                )),
                                rand::thread_rng().gen_range(0..65353),
                            );

                            let mut peer_config: RtcConfiguration = RtcConfiguration::new();
                            let ice_server_config = IceServerConfig {
                                urls: ["stun:stun.l.google.com:19302".to_string()],
                            };
                            let ice_server_config_list = [ice_server_config];

                            peer_config.ice_servers(
                                &JsValue::from_serde(&ice_server_config_list).unwrap(),
                            );
                            let peer =
                                RtcPeerConnection::new_with_configuration(&peer_config).unwrap();

                            let mut data_channel_config: RtcDataChannelInit =
                                RtcDataChannelInit::new();
                            data_channel_config.ordered(false);
                            data_channel_config.max_retransmits(0);

                            let channel: RtcDataChannel = peer
                                .create_data_channel_with_data_channel_dict(
                                    "webudp",
                                    &data_channel_config,
                                );
                            channel.set_binary_type(RtcDataChannelType::Arraybuffer);

                            // XXX: timing: wait for new_client_sender to be ready, or
                            // notify error.
                            new_client_sender.try_send((fake_socket_address, channel.clone()));

                            let cloned_channel = channel.clone();
                            let from_client_sender_clone = from_client_sender.clone();
                            let channel_onopen_func: Box<dyn FnMut(JsValue)> =
                                Box::new(move |_| {
                                    let mut from_client_sender_clone_2 =
                                        from_client_sender_clone.clone();
                                    let channel_onmsg_func: Box<dyn FnMut(MessageEvent)> =
                                        Box::new(move |evt: MessageEvent| {
                                            if let Ok(arraybuf) =
                                                evt.data().dyn_into::<js_sys::ArrayBuffer>()
                                            {
                                                let uarray: js_sys::Uint8Array =
                                                    js_sys::Uint8Array::new(&arraybuf);
                                                let mut body = vec![0; uarray.length() as usize];
                                                uarray.copy_to(&mut body[..]);
                                                from_client_sender_clone_2.try_send(Ok(
                                                    Packet::new(fake_socket_address, body),
                                                ));
                                            }
                                        });
                                    let channel_onmsg_closure = Closure::wrap(channel_onmsg_func);

                                    cloned_channel.set_onmessage(Some(
                                        channel_onmsg_closure.as_ref().unchecked_ref(),
                                    ));
                                    channel_onmsg_closure.forget();
                                });
                            let channel_onopen_closure = Closure::wrap(channel_onopen_func);
                            channel
                                .set_onopen(Some(channel_onopen_closure.as_ref().unchecked_ref()));
                            channel_onopen_closure.forget();

                            let onerror_func: Box<dyn FnMut(ErrorEvent)> =
                                Box::new(move |e: ErrorEvent| {
                                    info!("data channel error event: {:?}", e);
                                });
                            let onerror_callback = Closure::wrap(onerror_func);
                            channel.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
                            onerror_callback.forget();

                            let signalling_socket_clone_2 = signalling_socket_clone.clone();
                            let peer_clone = peer.clone();
                            let remote_desc_success_func: Box<dyn FnMut(JsValue)> = Box::new(
                                move |_| {
                                    let signalling_socket_clone_3 =
                                        signalling_socket_clone_2.clone();
                                    let peer_clone_2 = peer_clone.clone();
                                    let peer_answer_func: Box<dyn FnMut(JsValue)> = Box::new(
                                        move |session_description: JsValue| {
                                            let local_session_description = session_description
                                                .dyn_into::<RtcSessionDescription>()
                                                .unwrap();

                                            let signalling_socket_clone_4 =
                                                signalling_socket_clone_3.clone();
                                            let peer_clone_3 = peer_clone_2.clone();
                                            let local_desc_success_func: Box<dyn FnMut(JsValue)> =
                                                Box::new(move |_| {
                                                    let signalling_socket_clone_5 =
                                                        signalling_socket_clone_4.clone();
                                                    let peer_clone_4 = peer_clone_3.clone();
                                                    let ice_candidate_func: Box<
                                                        dyn FnMut(RtcPeerConnectionIceEvent),
                                                    > = Box::new(
                                                        move |event: RtcPeerConnectionIceEvent| {
                                                            // null candidate represents end-of-candidates.
                                                            if event.candidate().is_none() {
                                                                let answer_sdp_string =
                                                                    peer_clone_4
                                                                        .local_description()
                                                                        .unwrap()
                                                                        .sdp();
                                                                signalling_socket_clone_5
                                                                    .send_with_str(
                                                                        answer_sdp_string.as_str(),
                                                                    );
                                                            }
                                                        },
                                                    );
                                                    let ice_candidate_callback =
                                                        Closure::wrap(ice_candidate_func);
                                                    peer_clone_3.set_onicecandidate(Some(
                                                        ice_candidate_callback
                                                            .as_ref()
                                                            .unchecked_ref(),
                                                    ));
                                                    ice_candidate_callback.forget();
                                                });
                                            let local_desc_success_callback =
                                                Closure::wrap(local_desc_success_func);

                                            let mut session_description_init: RtcSessionDescriptionInit =
                                        RtcSessionDescriptionInit::new(
                                            local_session_description.type_(),
                                        );
                                            session_description_init
                                                .sdp(local_session_description.sdp().as_str());
                                            peer_clone_2
                                                .set_local_description(&session_description_init)
                                                .then(&local_desc_success_callback);
                                            local_desc_success_callback.forget();
                                        },
                                    );
                                    let peer_answer_callback = Closure::wrap(peer_answer_func);

                                    peer_clone.create_answer().then(&peer_answer_callback);
                                    peer_answer_callback.forget();
                                },
                            );
                            let remote_desc_success_callback =
                                Closure::wrap(remote_desc_success_func);

                            let remote_desc_failure_func: Box<dyn FnMut(JsValue)> = Box::new(
                                move |_: JsValue| {
                                    info!(
                                    "Server error during 'setRemoteDescription': TODO, put value here"
                                );
                                },
                            );
                            let remote_desc_failure_callback =
                                Closure::wrap(remote_desc_failure_func);

                            let mut rtc_session_desc_init_dict: RtcSessionDescriptionInit =
                                RtcSessionDescriptionInit::new(RtcSdpType::Offer);

                            rtc_session_desc_init_dict
                                .sdp(offer_sdp_string.as_string().unwrap().as_str());

                            peer.set_remote_description_with_success_callback_and_failure_callback(
                                &rtc_session_desc_init_dict,
                                remote_desc_success_callback.as_ref().unchecked_ref(),
                                remote_desc_failure_callback.as_ref().unchecked_ref(),
                            );
                            remote_desc_success_callback.forget();
                            remote_desc_failure_callback.forget();
                        }
                    },
                );
                let signalling_socket_onmessage_closure =
                    Closure::wrap(signalling_socket_onmessage_func);
                signalling_socket.set_onmessage(Some(
                    signalling_socket_onmessage_closure.as_ref().unchecked_ref(),
                ));
                signalling_socket_onmessage_closure.forget();

                // TODO: WebSocket error and disconnection handling.

                enum Next {
                    NewClientMessage((SocketAddr, RtcDataChannel)),
                    ToClientMessage(Packet),
                }
                loop {
                    let next = {
                        let to_client_receiver_next = to_client_receiver.next().fuse();
                        pin_mut!(to_client_receiver_next);

                        let new_client_message_receiver_next = new_client_receiver.next().fuse();
                        pin_mut!(new_client_message_receiver_next);

                        select! {
                            new_client_result = new_client_message_receiver_next => {
                                Next::NewClientMessage(new_client_result.expect("new client message receiver closed"))
                            }
                            to_client_message = to_client_receiver_next => {
                                Next::ToClientMessage(
                                    to_client_message.expect("to client message receiver closed")
                                )
                            }
                        }
                    };

                    match next {
                        Next::NewClientMessage((socket_address, data_channel)) => {
                            clients.insert(socket_address, data_channel);
                        }
                        Next::ToClientMessage(packet) => {
                            clients
                                .get(&packet.address())
                                .unwrap()
                                .send_with_u8_array(&packet.payload())
                                .unwrap();
                        }
                    }
                }
            });
        });

        let socket = ServerSocket {
            to_client_sender,
            from_client_receiver,
        };
        Box::new(socket)
    }
}

#[async_trait]
impl ServerSocketTrait for ServerSocket {
    async fn receive(&mut self) -> Result<Packet, NaiaServerSocketError> {
        self.from_client_receiver
            .next()
            .await
            .unwrap()
            .map_err(|e| NaiaServerSocketError::Wrapped(Box::new(e)))

        /*
        enum Next {
            // NewClientMessage((SocketAddr, RtcDataChannel)),
            FromClientMessage(Result<Packet, IoError>),
            // ToClientMessage(Packet),
        }
        loop {
            let next = {
                // let to_client_receiver_next = self.to_client_receiver.next().fuse();
                // pin_mut!(to_client_receiver_next);

                let from_client_message_receiver_next = self.from_client_receiver.next().fuse();
                pin_mut!(from_client_message_receiver_next);

                select! {
                    from_client_result = from_client_message_receiver_next => {
                        Next::FromClientMessage(
                            match from_client_result {
                                Some(Ok(msg)) => {
                                    Ok(Packet::new(msg.remote_addr, msg.message.as_ref().to_vec()))
                                }
                                Some(Err(err)) => { Err(err) }
                            }
                        )
                    }
                    // to_client_message = to_client_receiver_next => {
                    //     Next::ToClientMessage(
                    //         to_client_message.expect("to server message receiver closed")
                    //     )
                    // }
                }
            };

            match next {
                Next::FromClientMessage(from_client_message) => match from_client_message {
                    Ok(packet) => {
                        return Ok(packet);
                    }
                    Err(err) => {
                        return Err(NaiaServerSocketError::Wrapped(Box::new(err)));
                    }
                },
                // Next::ToClientMessage(packet) => {
                //     let address = packet.address();

                //     match self
                //         .clients
                //         .get(address)
                //         .send_with_u8_array(&packet.payload())
                //     {
                //         Err(_) => {
                //             return Err(NaiaServerSocketError::SendError(address));
                //         }
                //         _ => {}
                //     }
                // }
            }
        }
        */
    }

    fn get_sender(&mut self) -> MessageSender {
        return MessageSender::new(self.to_client_sender.clone());
    }

    fn with_link_conditioner(
        self: Box<Self>,
        config: &LinkConditionerConfig,
    ) -> Box<dyn ServerSocketTrait> {
        unimplemented!();
        // Box::new(LinkConditioner::new(config, self))
    }
}
