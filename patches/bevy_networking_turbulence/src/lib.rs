use bevy_app::{AppBuilder, Events, Plugin};
use bevy_ecs::prelude::*;
use bevy_tasks::{IoTaskPool, TaskPool};

// #[cfg(not(target_arch = "wasm32"))]
use crossbeam_channel::{unbounded, Receiver, Sender};
// #[cfg(not(target_arch = "wasm32"))]
use std::sync::RwLock;
use std::{
    collections::HashMap,
    error::Error,
    fmt::Debug,
    net::SocketAddr,
    sync::{atomic, Arc, Mutex},
};

use naia_client_socket::ClientSocket;
// #[cfg(not(target_arch = "wasm32"))]
use naia_server_socket::{MessageSender as ServerSender, NextEvent, ServerSocket};

pub use naia_client_socket::LinkConditionerConfig;
// #[cfg(not(target_arch = "wasm32"))]
pub use naia_server_socket::find_my_ip_address;

use turbulence::{
    buffer::BufferPacketPool,
    message_channels::ChannelMessage,
    packet::{Packet as PoolPacket, PacketPool, MAX_PACKET_LEN},
    packet_multiplexer::MuxPacketPool,
};
pub use turbulence::{
    message_channels::{MessageChannelMode, MessageChannelSettings},
    reliable_channel::Settings as ReliableChannelSettings,
};

use wasm_bindgen_futures::spawn_local;

mod channels;
mod transport;
use self::channels::{SimpleBufferPool, TaskPoolRuntime};
pub use transport::{Connection, ConnectionChannelsBuilder, Packet};

pub type ConnectionHandle = u32;

#[derive(Default)]
pub struct NetworkingPlugin {
    pub link_conditioner: Option<LinkConditionerConfig>,
}

impl Plugin for NetworkingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let task_pool = app
            .resources()
            .get::<IoTaskPool>()
            .expect("IoTaskPool resource not found")
            .0
            .clone();

        app.add_resource(NetworkResource::new(
            task_pool,
            self.link_conditioner.clone(),
        ))
        .add_event::<NetworkEvent>()
        .add_system(receive_packets.system());
    }
}

pub struct NetworkResource {
    task_pool: TaskPool,

    pending_connections: Arc<Mutex<Vec<Box<dyn Connection>>>>,
    pending_disconnections: Arc<Mutex<Vec<SocketAddr>>>,
    connection_sequence: atomic::AtomicU32,
    pub connections: HashMap<ConnectionHandle, Box<dyn Connection>>,

    // #[cfg(not(target_arch = "wasm32"))]
    listeners: Arc<Mutex<Vec<ServerListener>>>,
    // #[cfg(not(target_arch = "wasm32"))]
    server_channels: Arc<RwLock<HashMap<SocketAddr, Sender<Packet>>>>,

    runtime: TaskPoolRuntime,
    packet_pool: MuxPacketPool<BufferPacketPool<SimpleBufferPool>>,
    channels_builder_fn: Option<Box<dyn Fn(&mut ConnectionChannelsBuilder) + Send + Sync>>,

    link_conditioner: Option<LinkConditionerConfig>,

    endpoint_id: Arc<Mutex<Option<String>>>,
}

// #[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)] // FIXME: remove this struct?
struct ServerListener {
    //receiver_task: bevy_tasks::Task<()>, // needed to keep receiver_task alive
    //receiver_task: FakeTask, // needed to keep receiver_task alive
    sender: ServerSender,
}

#[derive(Debug)]
pub enum NetworkEvent {
    Connected(ConnectionHandle),
    Disconnected(ConnectionHandle),
    Packet(ConnectionHandle, Packet),
    Hosted(String),
    // Error(NetworkError),
}

#[cfg(target_arch = "wasm32")]
unsafe impl Send for NetworkResource {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for NetworkResource {}

impl NetworkResource {
    pub fn new(task_pool: TaskPool, link_conditioner: Option<LinkConditionerConfig>) -> Self {
        let runtime = TaskPoolRuntime::new(task_pool.clone());
        let packet_pool =
            MuxPacketPool::new(BufferPacketPool::new(SimpleBufferPool(MAX_PACKET_LEN)));

        NetworkResource {
            task_pool,
            connections: HashMap::new(),
            connection_sequence: atomic::AtomicU32::new(0),
            pending_connections: Arc::new(Mutex::new(Vec::new())),
            pending_disconnections: Arc::new(Mutex::new(Vec::new())),
            // #[cfg(not(target_arch = "wasm32"))]
            listeners: Arc::new(Mutex::new(Vec::new())),
            // #[cfg(not(target_arch = "wasm32"))]
            server_channels: Arc::new(RwLock::new(HashMap::new())),
            runtime,
            packet_pool,
            channels_builder_fn: None,

            link_conditioner,
            endpoint_id: Arc::new(Mutex::new(None)),
        }
    }

    pub fn listen(&mut self, signalling_server_url: String) {
        let server_channels = self.server_channels.clone();
        let pending_connections = self.pending_connections.clone();
        let pending_disconnections = self.pending_disconnections.clone();
        let task_pool = self.task_pool.clone();
        let listeners = self.listeners.clone();
        let endpoint_id = self.endpoint_id.clone();

        let link_conditioner = self.link_conditioner.take();

        spawn_local(async move {
            let mut server_socket = {
                let (id, socket) = ServerSocket::listen(signalling_server_url).await;
                endpoint_id.lock().unwrap().replace(id);

                if let Some(ref conditioner) = link_conditioner {
                    socket.with_link_conditioner(conditioner)
                } else {
                    socket
                }
            };
            let sender = server_socket.get_sender();

            let task_pool_clone = task_pool.clone();
            let receiver_task = task_pool.spawn(async move {
                loop {
                    match server_socket.receive().await {
                        NextEvent::ReceivedPacket(Ok(packet)) => {
                            let address = packet.address();
                            let message = String::from_utf8_lossy(packet.payload());
                            log::debug!(
                                "Server recv <- {}:{}: {}",
                                address,
                                packet.payload().len(),
                                message
                            );

                            match server_channels.write() {
                                Ok(mut server_channels) => {
                                    if !server_channels.contains_key(&address) {
                                        let (packet_tx, packet_rx): (
                                            Sender<Packet>,
                                            Receiver<Packet>,
                                        ) = unbounded();
                                        pending_connections.lock().unwrap().push(Box::new(
                                            transport::ServerConnection::new(
                                                task_pool_clone.clone(),
                                                packet_rx,
                                                server_socket.get_sender(),
                                                address,
                                            ),
                                        ));
                                        server_channels.insert(address, packet_tx);
                                    }
                                }
                                Err(err) => {
                                    log::error!("Error locking server channels: {}", err);
                                }
                            }

                            match server_channels
                                .read()
                                .unwrap()
                                .get(&address)
                                .unwrap()
                                .send(Packet::copy_from_slice(packet.payload()))
                            {
                                Ok(()) => {}
                                Err(error) => {
                                    log::error!("Server Send Error: {}", error);
                                }
                            }
                        }
                        NextEvent::ReceivedPacket(Err(error)) => {
                            log::error!("Server Receive Error: {}", error);
                        }
                        NextEvent::Disconnected(socket_address) => {
                            log::info!("Received a disconnection event");
                            server_channels.write().unwrap().remove(&socket_address);
                            pending_disconnections.lock().unwrap().push(socket_address);
                        }
                    }
                }
            });

            listeners.lock().unwrap().push(ServerListener {
                //receiver_task,
                sender,
            });
        });
    }

    pub fn connect(&mut self, socket_address: String) {
        let mut client_socket = {
            let socket = ClientSocket::connect(socket_address);

            if let Some(ref conditioner) = self.link_conditioner {
                socket.with_link_conditioner(conditioner)
            } else {
                socket
            }
        };
        let sender = client_socket.get_sender();

        self.pending_connections
            .lock()
            .unwrap()
            .push(Box::new(transport::ClientConnection::new(
                self.task_pool.clone(),
                client_socket,
                sender,
            )));
    }

    pub fn send(
        &mut self,
        handle: ConnectionHandle,
        payload: Packet,
    ) -> Result<(), Box<dyn Error + Send>> {
        match self.connections.get_mut(&handle) {
            Some(connection) => connection.send(payload),
            None => Err(Box::new(std::io::Error::new(
                // FIXME: move to enum Error
                std::io::ErrorKind::NotFound,
                "No such connection",
            ))),
        }
    }

    pub fn broadcast(&mut self, payload: Packet) {
        for (_handle, connection) in self.connections.iter_mut() {
            connection.send(payload.clone()).unwrap();
        }
    }

    pub fn set_channels_builder<F>(&mut self, builder: F)
    where
        F: Fn(&mut ConnectionChannelsBuilder) + Send + Sync + 'static,
    {
        self.channels_builder_fn = Some(Box::new(builder));
    }

    pub fn send_message<M: ChannelMessage + Debug + Clone>(
        &mut self,
        handle: ConnectionHandle,
        message: M,
    ) -> Result<Option<M>, Box<dyn Error + Send>> {
        match self.connections.get_mut(&handle) {
            Some(connection) => {
                let channels = connection.channels().unwrap();
                let unsent = channels.send(message);
                channels.flush::<M>();
                Ok(unsent)
            }
            None => Err(Box::new(std::io::Error::new(
                // FIXME: move to enum Error
                std::io::ErrorKind::NotFound,
                "No such connection",
            ))),
        }
    }

    pub fn broadcast_message<M: ChannelMessage + Debug + Clone>(&mut self, message: M) {
        // log::info!("Broadcast:\n{:?}", message);
        for (handle, connection) in self.connections.iter_mut() {
            let channels = connection.channels().unwrap();
            let result = channels.send(message.clone());
            channels.flush::<M>();
            if let Some(msg) = result {
                log::error!("Failed broadcast to [{}]: {:?}", handle, msg);
            }
        }
    }

    pub fn recv_message<M: ChannelMessage + Debug + Clone>(
        &mut self,
        handle: ConnectionHandle,
    ) -> Option<M> {
        match self.connections.get_mut(&handle) {
            Some(connection) => {
                let channels = connection.channels().unwrap();
                channels.recv()
            }
            None => None,
        }
    }
}

pub fn receive_packets(
    mut net: ResMut<NetworkResource>,
    mut network_events: ResMut<Events<NetworkEvent>>,
) {
    if let Some(endpoint_id) = net.endpoint_id.lock().unwrap().take() {
        network_events.send(NetworkEvent::Hosted(endpoint_id));
    }

    let pending_connections: Vec<Box<dyn Connection>> =
        net.pending_connections.lock().unwrap().drain(..).collect();
    for mut conn in pending_connections {
        let handle: ConnectionHandle = net
            .connection_sequence
            .fetch_add(1, atomic::Ordering::Relaxed);
        if let Some(channels_builder_fn) = net.channels_builder_fn.as_ref() {
            conn.build_channels(
                channels_builder_fn,
                net.runtime.clone(),
                net.packet_pool.clone(),
            );
        }
        net.connections.insert(handle, conn);
        network_events.send(NetworkEvent::Connected(handle));
    }

    let pending_disconnections: Vec<SocketAddr> = net
        .pending_disconnections
        .lock()
        .unwrap()
        .drain(..)
        .collect();
    for mut disconnected_address in pending_disconnections {
        log::info!("Finding handles to remove...");
        let mut handles_to_disconnect = vec![];
        for (handle, connection) in net.connections.iter() {
            if connection.remote_address().unwrap() == disconnected_address {
                handles_to_disconnect.push(handle.clone());
            }
        }
        for handle in handles_to_disconnect {
            log::info!(
                "Removing handle {:?} and sending disconnected network event",
                handle
            );
            net.connections.remove(&handle);
            network_events.send(NetworkEvent::Disconnected(handle));
        }
    }

    let packet_pool = net.packet_pool.clone();
    for (handle, connection) in net.connections.iter_mut() {
        while let Some(result) = connection.receive() {
            match result {
                Ok(packet) => {
                    let message = String::from_utf8_lossy(&packet);
                    log::debug!("Received on [{}] {} RAW: {}", handle, packet.len(), message);
                    if let Some(channels_rx) = connection.channels_rx() {
                        log::debug!("Processing as message");
                        let mut pool_packet = packet_pool.acquire();
                        pool_packet.resize(packet.len(), 0);
                        pool_packet[..].copy_from_slice(&*packet);
                        match channels_rx.try_send(pool_packet) {
                            Ok(()) => {
                                // cool
                            }
                            Err(err) => {
                                log::error!("Channel Incoming Error: {}", err);
                                // FIXME:error_events.send(error);
                            }
                        }
                    } else {
                        log::debug!("Processing as packet");
                        network_events.send(NetworkEvent::Packet(*handle, packet));
                    }
                }
                Err(err) => {
                    log::error!("Receive Error: {}", err);
                    // FIXME:error_events.send(error);
                }
            }
        }
    }
}
