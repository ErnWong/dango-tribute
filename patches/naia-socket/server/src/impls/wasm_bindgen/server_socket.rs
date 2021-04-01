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
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};

use async_trait::async_trait;

use futures_channel::{mpsc, oneshot};
use futures_util::{pin_mut, select, FutureExt, StreamExt};

use naia_socket_shared::LinkConditionerConfig;

use crate::{
    error::NaiaServerSocketError,
    //link_conditioner::LinkConditioner,
    message_sender::MessageSender,
    NextEvent,
    Packet,
    ServerSocketTrait,
};
use js_sys::Reflect;
use wasm_bindgen::{prelude::*, JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;
use web_sys::{
    ErrorEvent, MessageEvent, RtcConfiguration, RtcDataChannel, RtcDataChannelInit,
    RtcDataChannelType, RtcIceConnectionState, RtcPeerConnection, RtcPeerConnectionIceEvent,
    RtcSdpType, RtcSessionDescription, RtcSessionDescriptionInit, WebSocket,
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
    disconnected_client_receiver: mpsc::Receiver<SocketAddr>,
}

impl ServerSocket {
    /// TODO
    pub async fn listen(signalling_server_url: String) -> (String, Box<dyn ServerSocketTrait>) {
        web_sys::console::log_1(&"Server listening to new WebRTC connections...".into());

        let (to_client_sender, mut to_client_receiver) =
            mpsc::channel::<Packet>(CLIENT_CHANNEL_SIZE);
        let (from_client_sender, from_client_receiver) = mpsc::channel(CLIENT_CHANNEL_SIZE);

        let (endpoint_id_sender, mut endpoint_id_receiver) = mpsc::channel::<String>(1);

        let (disconnected_internal_sender, mut disconnected_internal_receiver) =
            mpsc::channel(CLIENT_CHANNEL_SIZE);
        let (mut disconnected_client_sender, disconnected_client_receiver) =
            mpsc::channel(CLIENT_CHANNEL_SIZE);

        //let evil = std::thread::spawn(move || {
        spawn_local(async move {
            let (mut new_client_sender, mut new_client_receiver) =
                mpsc::channel(CLIENT_CHANNEL_SIZE);
            let mut clients: HashMap<SocketAddr, RtcDataChannel> = HashMap::new();
            let signalling_socket = WebSocket::new(signalling_server_url.as_str()).unwrap();

            web_sys::console::log_1(&"Waiting for connections via websocket relay...".into());
            let signalling_socket_clone = signalling_socket.clone();
            let mut endpoint_id_sender_clone = endpoint_id_sender.clone();
            let disconnected_internal_sender_clone = disconnected_internal_sender.clone();
            let is_first_message = AtomicBool::new(true);
            let signalling_socket_onmessage_func: Box<dyn FnMut(MessageEvent)> = Box::new(
                move |event: MessageEvent| {
                    if let Ok(websocket_message_string) =
                        event.data().dyn_into::<js_sys::JsString>()
                    {
                        // TODO: We might be able to relax some of the ordering.
                        if is_first_message.compare_exchange(
                            true,
                            false,
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        ) == Ok(true)
                        {
                            endpoint_id_sender_clone.try_send(websocket_message_string.into());
                            return;
                        }
                        let offer_sdp_string = websocket_message_string;
                        web_sys::console::log_1(
                            &"Got an offer string - creating rtc channel".into(),
                        );
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

                        peer_config
                            .ice_servers(&JsValue::from_serde(&ice_server_config_list).unwrap());
                        let peer = RtcPeerConnection::new_with_configuration(&peer_config).unwrap();

                        let mut data_channel_config: RtcDataChannelInit = RtcDataChannelInit::new();
                        data_channel_config.ordered(false);
                        data_channel_config.max_retransmits(0);
                        data_channel_config.negotiated(true);
                        data_channel_config.id(0);

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
                        let channel_onopen_func: Box<dyn FnMut(JsValue)> = Box::new(move |_| {
                            web_sys::console::log_1(&"Rtc channel opened".into());
                            let mut from_client_sender_clone_2 = from_client_sender_clone.clone();
                            let channel_onmsg_func: Box<dyn FnMut(MessageEvent)> =
                                Box::new(move |evt: MessageEvent| {
                                    // web_sys::console::log_1(&"Rtc channel onmessage".into());
                                    if let Ok(arraybuf) =
                                        evt.data().dyn_into::<js_sys::ArrayBuffer>()
                                    {
                                        let uarray: js_sys::Uint8Array =
                                            js_sys::Uint8Array::new(&arraybuf);
                                        // web_sys::console::log_1(
                                        //     &"Receive data of length {}".into(),
                                        //     //uarray.length().into()
                                        // );
                                        let mut body = vec![0; uarray.length() as usize];
                                        uarray.copy_to(&mut body[..]);
                                        from_client_sender_clone_2
                                            .try_send(Ok(Packet::new(fake_socket_address, body)));
                                    }
                                });
                            let channel_onmsg_closure = Closure::wrap(channel_onmsg_func);

                            cloned_channel.set_onmessage(Some(
                                channel_onmsg_closure.as_ref().unchecked_ref(),
                            ));
                            channel_onmsg_closure.forget();
                        });
                        let channel_onopen_closure = Closure::wrap(channel_onopen_func);
                        channel.set_onopen(Some(channel_onopen_closure.as_ref().unchecked_ref()));
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
                                web_sys::console::log_1(
                                    &"successfully set remote description".into(),
                                );
                                let signalling_socket_clone_3 = signalling_socket_clone_2.clone();
                                let peer_clone_2 = peer_clone.clone();
                                let peer_answer_func: Box<dyn FnMut(JsValue)> = Box::new(
                                    move |answer: JsValue| {
                                        web_sys::console::log_1(&"generated answer".into());
                                        let local_sdp_string =
                                            Reflect::get(&answer, &JsValue::from_str("sdp"))
                                                .unwrap()
                                                .as_string()
                                                .unwrap();

                                        let signalling_socket_clone_4 =
                                            signalling_socket_clone_3.clone();
                                        let peer_clone_3 = peer_clone_2.clone();
                                        let local_desc_success_func: Box<dyn FnMut(JsValue)> =
                                            Box::new(move |_| {
                                                web_sys::console::log_1(
                                                    &"successfully set local description".into(),
                                                );
                                                let signalling_socket_clone_5 =
                                                    signalling_socket_clone_4.clone();
                                                let peer_clone_4 = peer_clone_3.clone();
                                                let ice_candidate_func: Box<
                                                    dyn FnMut(RtcPeerConnectionIceEvent),
                                                > = Box::new(
                                                    move |event: RtcPeerConnectionIceEvent| {
                                                        web_sys::console::log_1(
                                                            &"Found new ice candidate".into(),
                                                        );
                                                        // null candidate represents end-of-candidates.
                                                        // TODO: do we need to deregister handler?
                                                        if event.candidate().is_none() {
                                                            web_sys::console::log_1(&"Found all ice candidates - sending answer".into());
                                                            let answer_sdp_string = peer_clone_4
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
                                                    ice_candidate_callback.as_ref().unchecked_ref(),
                                                ));
                                                ice_candidate_callback.forget();
                                            });
                                        let local_desc_success_callback =
                                            Closure::wrap(local_desc_success_func);

                                        let mut session_description_init: RtcSessionDescriptionInit =
                                        RtcSessionDescriptionInit::new(
                                            RtcSdpType::Answer,
                                        );
                                        session_description_init.sdp(&local_sdp_string);
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
                        let remote_desc_success_callback = Closure::wrap(remote_desc_success_func);

                        let remote_desc_failure_func: Box<dyn FnMut(JsValue)> = Box::new(
                            move |_: JsValue| {
                                info!(
                                    "Server error during 'setRemoteDescription': TODO, put value here"
                                );
                            },
                        );
                        let remote_desc_failure_callback = Closure::wrap(remote_desc_failure_func);

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

                        let mut disconnected_internal_sender_clone_2 =
                            disconnected_internal_sender_clone.clone();
                        let peer_clone_disconnect_detect = peer.clone();
                        let connection_state_change_func: Box<dyn FnMut(JsValue)> = Box::new(
                            move |_| match peer_clone_disconnect_detect.ice_connection_state() {
                                RtcIceConnectionState::Failed
                                | RtcIceConnectionState::Closed
                                | RtcIceConnectionState::Disconnected => {
                                    web_sys::console::log_1(
                                            &"RtcIceConnectionState: failed/closed/disconnected - sending disconnect and closing peer connection".into(),
                                        );
                                    peer_clone_disconnect_detect.close();
                                    disconnected_internal_sender_clone_2
                                        .try_send(fake_socket_address);
                                }
                                _ => {}
                            },
                        );
                        let connection_state_change_closure =
                            Closure::wrap(connection_state_change_func);
                        peer.set_oniceconnectionstatechange(Some(
                            connection_state_change_closure.as_ref().unchecked_ref(),
                        ));
                        connection_state_change_closure.forget();
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
                DisconnectedClientMessage(SocketAddr),
                ToClientMessage(Packet),
            }
            loop {
                let next = {
                    let to_client_receiver_next = to_client_receiver.next().fuse();
                    pin_mut!(to_client_receiver_next);

                    let new_client_message_receiver_next = new_client_receiver.next().fuse();
                    pin_mut!(new_client_message_receiver_next);

                    let disconnected_client_message_receiver_next =
                        disconnected_internal_receiver.next().fuse();
                    pin_mut!(disconnected_client_message_receiver_next);

                    select! {
                        new_client_result = new_client_message_receiver_next => {
                            Next::NewClientMessage(new_client_result.expect("new client message receiver closed"))
                        }
                        disconnected_client_result = disconnected_client_message_receiver_next => {
                            Next::DisconnectedClientMessage(disconnected_client_result.expect("disconnected client message receiver closed"))
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
                    Next::DisconnectedClientMessage(socket_address) => {
                        web_sys::console::log_1(&"Removing client".into());
                        clients.remove(&socket_address);
                        disconnected_client_sender.try_send(socket_address);
                    }
                    Next::ToClientMessage(packet) => {
                        // web_sys::console::log_1(
                        //     &"Send data of length {}".into(),
                        //     //&packet.payload().len().into(),
                        // );
                        clients
                            .get(&packet.address())
                            .unwrap()
                            .send_with_u8_array(&packet.payload())
                            .unwrap();
                    }
                }
            }
        });
        //});

        let socket = ServerSocket {
            to_client_sender,
            from_client_receiver,
            disconnected_client_receiver,
        };
        (endpoint_id_receiver.next().await.unwrap(), Box::new(socket))
    }
}

#[async_trait]
impl ServerSocketTrait for ServerSocket {
    async fn receive(&mut self) -> NextEvent {
        let receive_packet_next = self.from_client_receiver.next().fuse();
        pin_mut!(receive_packet_next);

        let disconnected_next = self.disconnected_client_receiver.next().fuse();
        pin_mut!(disconnected_next);

        select! {
            receive_packet_result = receive_packet_next => {
                NextEvent::ReceivedPacket(receive_packet_result.expect("from client receiver closed").map_err(|e| NaiaServerSocketError::Wrapped(Box::new(e))))
            }
            disconnected_result = disconnected_next => {
                                web_sys::console::log_1(
                                    &"Returning NextEvent::Disconnected".into(),
                                );
                NextEvent::Disconnected(disconnected_result.expect("disconnected client receiver closed"))
            }
        }

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
