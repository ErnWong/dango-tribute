extern crate log;
use log::info;

use std::{collections::VecDeque, net::SocketAddr};

use crate::{error::NaiaClientSocketError, Packet};

use naia_socket_shared::Ref;

use wasm_bindgen::{prelude::*, JsCast, JsValue};
use web_sys::{
    ErrorEvent, MessageEvent, ProgressEvent, RtcConfiguration, RtcDataChannel, RtcDataChannelInit,
    RtcDataChannelType, RtcIceCandidate, RtcIceCandidateInit, RtcPeerConnection,
    RtcPeerConnectionIceEvent, RtcSdpType, RtcSessionDescription, RtcSessionDescriptionInit,
    XmlHttpRequest,
};

#[derive(Deserialize, Debug, Clone)]
pub struct SessionAnswer {
    pub sdp: String,

    #[serde(rename = "type")]
    pub _type: String,
}

#[derive(Deserialize, Debug)]
pub struct SessionCandidate {
    pub candidate: String,
    #[serde(rename = "sdpMLineIndex")]
    pub sdp_m_line_index: u16,
    #[serde(rename = "sdpMid")]
    pub sdp_mid: String,
}

#[derive(Deserialize, Debug)]
pub struct JsSessionResponse {
    pub answer: SessionAnswer,
    pub candidate: SessionCandidate,
}

#[derive(Serialize)]
pub struct IceServerConfig {
    pub urls: [String; 1],
}

#[allow(unused_must_use)]
pub fn webrtc_initialize(
    server_url_str: String,
    msg_queue: Ref<VecDeque<Result<Option<Packet>, NaiaClientSocketError>>>,
) -> RtcDataChannel {
    let mut peer_config: RtcConfiguration = RtcConfiguration::new();
    let ice_server_config = IceServerConfig {
        urls: ["stun:stun.l.google.com:19302".to_string()],
    };
    let ice_server_config_list = [ice_server_config];

    peer_config.ice_servers(&JsValue::from_serde(&ice_server_config_list).unwrap());

    let peer: RtcPeerConnection = RtcPeerConnection::new_with_configuration(&peer_config).unwrap();

    let mut data_channel_config: RtcDataChannelInit = RtcDataChannelInit::new();
    data_channel_config.ordered(false);
    data_channel_config.max_retransmits(0);
    data_channel_config.negotiated(true);
    data_channel_config.id(0);

    let channel: RtcDataChannel =
        peer.create_data_channel_with_data_channel_dict("webudp", &data_channel_config);
    channel.set_binary_type(RtcDataChannelType::Arraybuffer);

    let cloned_channel = channel.clone();
    let msg_queue_clone = msg_queue.clone();
    let channel_onopen_func: Box<dyn FnMut(JsValue)> = Box::new(move |_| {
        let msg_queue_clone_2 = msg_queue_clone.clone();
        let channel_onmsg_func: Box<dyn FnMut(MessageEvent)> =
            Box::new(move |evt: MessageEvent| {
                //web_sys::console::log_1(&"Rtc channel onmessage".into());
                if let Ok(arraybuf) = evt.data().dyn_into::<js_sys::ArrayBuffer>() {
                    //web_sys::console::log_1(&"received data".into());
                    let uarray: js_sys::Uint8Array = js_sys::Uint8Array::new(&arraybuf);
                    let mut body = vec![0; uarray.length() as usize];
                    uarray.copy_to(&mut body[..]);
                    msg_queue_clone_2
                        .borrow_mut()
                        .push_back(Ok(Some(Packet::new(body))));
                }
            });
        let channel_onmsg_closure = Closure::wrap(channel_onmsg_func);

        cloned_channel.set_onmessage(Some(channel_onmsg_closure.as_ref().unchecked_ref()));
        channel_onmsg_closure.forget();
    });
    let channel_onopen_closure = Closure::wrap(channel_onopen_func);
    channel.set_onopen(Some(channel_onopen_closure.as_ref().unchecked_ref()));
    channel_onopen_closure.forget();

    let onerror_func: Box<dyn FnMut(ErrorEvent)> = Box::new(move |e: ErrorEvent| {
        info!("data channel error event: {:?}", e);
    });
    let onerror_callback = Closure::wrap(onerror_func);
    channel.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();

    let peer_clone = peer.clone();
    let server_url_msg = Ref::new(server_url_str);
    let peer_offer_func: Box<dyn FnMut(JsValue)> = Box::new(move |e: JsValue| {
        //web_sys::console::log_1(&"created offer".into());
        let session_description = e.dyn_into::<RtcSessionDescription>().unwrap();
        let peer_clone_2 = peer_clone.clone();
        let server_url_msg_clone = server_url_msg.clone();
        let peer_desc_func: Box<dyn FnMut(JsValue)> = Box::new(move |_: JsValue| {
            //web_sys::console::log_1(
            //    &"set local description done, waiting for ice candidates...".into(),
            //);
            let peer_clone_3 = peer_clone_2.clone();
            let server_url_msg_clone_1 = server_url_msg_clone.clone();
            let ice_candidate_func: Box<dyn FnMut(RtcPeerConnectionIceEvent)> = Box::new(
                move |event: RtcPeerConnectionIceEvent| {
                    // null candidate represents end-of-candidates.
                    if event.candidate().is_none() {
                        //web_sys::console::log_1(&"got all candidates - sending offer".into());
                        let request =
                            XmlHttpRequest::new().expect("can't create new XmlHttpRequest");

                        request
                            .open("POST", &server_url_msg_clone_1.borrow())
                            .unwrap_or_else(|err| {
                                info!(
                                    "WebSys, can't POST to server url. Original Error: {:?}",
                                    err
                                )
                            });

                        let request_2 = request.clone();
                        let peer_clone_4 = peer_clone_3.clone();
                        let request_func: Box<dyn FnMut(ProgressEvent)> = Box::new(
                            move |_: ProgressEvent| {
                                if request_2.status().unwrap() == 200 {
                                    //web_sys::console::log_1(&"got answer".into());
                                    let answer_sdp_string =
                                        request_2.response_text().unwrap().unwrap();

                                    let remote_desc_success_func: Box<dyn FnMut(JsValue)> =
                                        Box::new(move |_: JsValue| {
                                            // Done.
                                        });
                                    let remote_desc_success_callback =
                                        Closure::wrap(remote_desc_success_func);

                                    let remote_desc_failure_func: Box<dyn FnMut(JsValue)> =
                                        Box::new(move |_: JsValue| {
                                            info!(
                                        "Client error during 'setRemoteDescription': TODO, put value here"
                                    );
                                        });
                                    let remote_desc_failure_callback =
                                        Closure::wrap(remote_desc_failure_func);

                                    let mut rtc_session_desc_init_dict: RtcSessionDescriptionInit =
                                        RtcSessionDescriptionInit::new(RtcSdpType::Answer);

                                    rtc_session_desc_init_dict.sdp(answer_sdp_string.as_str());

                                    peer_clone_4.set_remote_description_with_success_callback_and_failure_callback(
                                &rtc_session_desc_init_dict,
                                remote_desc_success_callback.as_ref().unchecked_ref(),
                                remote_desc_failure_callback.as_ref().unchecked_ref(),
                            );
                                    remote_desc_success_callback.forget();
                                    remote_desc_failure_callback.forget();
                                }
                            },
                        );
                        let request_callback = Closure::wrap(request_func);
                        request.set_onload(Some(request_callback.as_ref().unchecked_ref()));
                        request_callback.forget();

                        request
                            .send_with_opt_str(Some(
                                peer_clone_3.local_description().unwrap().sdp().as_str(),
                            ))
                            .unwrap_or_else(|err| {
                                info!("WebSys, can't sent request str. Original Error: {:?}", err)
                            });
                    }
                },
            );
            let ice_candidate_callback = Closure::wrap(ice_candidate_func);
            peer_clone_2.set_onicecandidate(Some(ice_candidate_callback.as_ref().unchecked_ref()));
            ice_candidate_callback.forget();
        });
        let peer_desc_callback = Closure::wrap(peer_desc_func);

        let mut session_description_init: RtcSessionDescriptionInit =
            RtcSessionDescriptionInit::new(session_description.type_());
        session_description_init.sdp(session_description.sdp().as_str());
        peer_clone
            .set_local_description(&session_description_init)
            .then(&peer_desc_callback);
        peer_desc_callback.forget();
    });
    let peer_offer_callback = Closure::wrap(peer_offer_func);

    let peer_error_func: Box<dyn FnMut(JsValue)> = Box::new(move |_: JsValue| {
        info!("Client error during 'createOffer': e value here? TODO");
    });
    let peer_error_callback = Closure::wrap(peer_error_func);

    peer.create_offer().then(&peer_offer_callback);

    peer_offer_callback.forget();
    peer_error_callback.forget();

    return channel;
}
