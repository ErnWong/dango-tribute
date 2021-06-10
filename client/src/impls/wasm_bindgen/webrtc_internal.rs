extern crate log;
use log::info;

use std::{collections::VecDeque, net::SocketAddr};

use crate::{error::NaiaClientSocketError, Packet};

use naia_socket_shared::Ref;

use wasm_bindgen::{prelude::*, JsCast, JsValue};
use web_sys::{
    ErrorEvent, MessageEvent, ProgressEvent, RtcConfiguration, RtcDataChannel, RtcDataChannelInit,
    RtcDataChannelType, RtcIceCandidate, RtcIceCandidateInit, RtcPeerConnection, RtcSdpType,
    RtcSessionDescriptionInit, XmlHttpRequest,
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
    socket_address: SocketAddr,
    msg_queue: Ref<VecDeque<Result<Option<Packet>, NaiaClientSocketError>>>,
) -> RtcDataChannel {
    let server_url_str = format!("http://{}/new_rtc_session", socket_address);

    let peer: RtcPeerConnection = RtcPeerConnection::new().unwrap();

    let mut data_channel_config: RtcDataChannelInit = RtcDataChannelInit::new();
    data_channel_config.ordered(false);
    data_channel_config.max_retransmits(0);

    let channel: RtcDataChannel =
        peer.create_data_channel_with_data_channel_dict("data", &data_channel_config);
    channel.set_binary_type(RtcDataChannelType::Arraybuffer);

    let cloned_channel = channel.clone();
    let msg_queue_clone = msg_queue.clone();
    let channel_onopen_func: Box<dyn FnMut(JsValue)> = Box::new(move |_| {
        let msg_queue_clone_2 = msg_queue_clone.clone();
        let channel_onmsg_func: Box<dyn FnMut(MessageEvent)> =
            Box::new(move |evt: MessageEvent| {
                if let Ok(arraybuf) = evt.data().dyn_into::<js_sys::ArrayBuffer>() {
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
        let session_description = e.into();
        let peer_clone_2 = peer_clone.clone();
        let server_url_msg_clone = server_url_msg.clone();
        let peer_desc_func: Box<dyn FnMut(JsValue)> = Box::new(move |_: JsValue| {
            let request = XmlHttpRequest::new().expect("can't create new XmlHttpRequest");

            request
                .open("POST", &server_url_msg_clone.borrow())
                .unwrap_or_else(|err| {
                    info!(
                        "WebSys, can't POST to server url. Original Error: {:?}",
                        err
                    )
                });

            let request_2 = request.clone();
            let peer_clone_3 = peer_clone_2.clone();
            let request_func: Box<dyn FnMut(ProgressEvent)> = Box::new(move |_: ProgressEvent| {
                if request_2.status().unwrap() == 200 {
                    let response_string = request_2.response_text().unwrap().unwrap();
                    let response_js_value = js_sys::JSON::parse(response_string.as_str()).unwrap();
                    let session_response: JsSessionResponse =
                        response_js_value.into_serde().unwrap();
                    let session_response_answer: SessionAnswer = session_response.answer.clone();

                    let peer_clone_4 = peer_clone_3.clone();
                    let remote_desc_success_func: Box<dyn FnMut(JsValue)> = Box::new(
                        move |e: JsValue| {
                            let mut candidate_init_dict: RtcIceCandidateInit =
                                RtcIceCandidateInit::new(
                                    session_response.candidate.candidate.as_str(),
                                );
                            candidate_init_dict.sdp_m_line_index(Some(
                                session_response.candidate.sdp_m_line_index,
                            ));
                            candidate_init_dict
                                .sdp_mid(Some(session_response.candidate.sdp_mid.as_str()));
                            let candidate: RtcIceCandidate =
                                RtcIceCandidate::new(&candidate_init_dict).unwrap();

                            let peer_add_success_func: Box<dyn FnMut(JsValue)> =
                                Box::new(move |_: JsValue| {
                                    //Client add ice candidate success
                                });
                            let peer_add_success_callback = Closure::wrap(peer_add_success_func);
                            let peer_add_failure_func: Box<dyn FnMut(JsValue)> =
                                Box::new(move |_: JsValue| {
                                    info!("Client error during 'addIceCandidate': {:?}", e);
                                });
                            let peer_add_failure_callback = Closure::wrap(peer_add_failure_func);

                            peer_clone_4.add_ice_candidate_with_rtc_ice_candidate_and_success_callback_and_failure_callback(
                                &candidate,
                                peer_add_success_callback.as_ref().unchecked_ref(),
                                peer_add_failure_callback.as_ref().unchecked_ref());
                            peer_add_success_callback.forget();
                            peer_add_failure_callback.forget();
                        },
                    );
                    let remote_desc_success_callback = Closure::wrap(remote_desc_success_func);

                    let remote_desc_failure_func: Box<dyn FnMut(JsValue)> =
                        Box::new(move |_: JsValue| {
                            info!(
                                "Client error during 'setRemoteDescription': TODO, put value here"
                            );
                        });
                    let remote_desc_failure_callback = Closure::wrap(remote_desc_failure_func);

                    let mut rtc_session_desc_init_dict: RtcSessionDescriptionInit =
                        RtcSessionDescriptionInit::new(RtcSdpType::Answer);

                    rtc_session_desc_init_dict.sdp(session_response_answer.sdp.as_str());

                    peer_clone_3.set_remote_description_with_success_callback_and_failure_callback(
                        &rtc_session_desc_init_dict,
                        remote_desc_success_callback.as_ref().unchecked_ref(),
                        remote_desc_failure_callback.as_ref().unchecked_ref(),
                    );
                    remote_desc_success_callback.forget();
                    remote_desc_failure_callback.forget();
                }
            });
            let request_callback = Closure::wrap(request_func);
            request.set_onload(Some(request_callback.as_ref().unchecked_ref()));
            request_callback.forget();

            request
                .send_with_opt_str(Some(
                    peer_clone_2.local_description().unwrap().sdp().as_str(),
                ))
                .unwrap_or_else(|err| {
                    info!("WebSys, can't sent request str. Original Error: {:?}", err)
                });
        });
        let peer_desc_callback = Closure::wrap(peer_desc_func);

        peer_clone
            .set_local_description(&session_description)
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
