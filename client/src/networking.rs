use bevy::prelude::*;
use bevy_networking_turbulence::{NetworkEvent, NetworkResource, NetworkingPlugin};
use bevy_prototype_networked_physics::{
    client::{Client, ClientState},
    world::World,
    Config,
};
use shared::{networking::WrappedNetworkResource, settings::network_setup};
use std::marker::PhantomData;

#[derive(Debug)]
pub enum ClientConnectionEvent {
    Connected(usize),
    Disconnected(usize),
}

#[derive(Default)]
pub struct ClientSystemState {
    network_event_reader: EventReader<NetworkEvent>,
    is_ready: bool,
}

pub fn client_system<WorldType: World>(
    mut client_system_state: Local<ClientSystemState>,
    mut client: ResMut<Client<WorldType>>,
    time: Res<Time>,
    mut net: ResMut<NetworkResource>,
    network_events: Res<Events<NetworkEvent>>,
    mut client_connection_events: ResMut<Events<ClientConnectionEvent>>,
) {
    // TODO: For now, disconnection events are fatal and we do not attempt to reconnect. This
    // is why it is handled specially, in this location, rather than as a ClientState
    // transition.
    for network_event in client_system_state
        .network_event_reader
        .iter(&network_events)
    {
        match network_event {
            NetworkEvent::Disconnected(handle) => {
                client_connection_events
                    .send(ClientConnectionEvent::Disconnected(*handle as usize));
            }
            _ => {}
        }
    }
    client.update(
        time.delta_seconds(),
        time.seconds_since_startup(),
        &mut WrappedNetworkResource(&mut *net),
    );
    match client.state() {
        ClientState::Ready(client) => {
            if !client_system_state.is_ready {
                client_connection_events.send(ClientConnectionEvent::Connected(client.client_id()));
            }
            client_system_state.is_ready = true;
        }
        _ => {
            client_system_state.is_ready = false;
        }
    }
}

pub struct EndpointUrl(pub String);

pub fn client_setup<WorldType: World>(
    mut net: ResMut<NetworkResource>,
    endpoint_url: Res<EndpointUrl>,
) {
    info!("Starting client - connecting to {}", endpoint_url.0.clone());
    net.connect(endpoint_url.0.clone());
}

#[derive(Default)]
pub struct NetworkedPhysicsClientPlugin<WorldType: World> {
    config: Config,
    endpoint_url: String,
    _world_type: PhantomData<WorldType>,
}

impl<WorldType: World> NetworkedPhysicsClientPlugin<WorldType> {
    pub fn new(config: Config, endpoint_url: String) -> Self {
        Self {
            config,
            endpoint_url,
            _world_type: PhantomData,
        }
    }
}

impl<WorldType: World> Plugin for NetworkedPhysicsClientPlugin<WorldType> {
    fn build(&self, app: &mut AppBuilder) {
        app.add_plugin(NetworkingPlugin::default())
            .add_event::<ClientConnectionEvent>()
            .add_resource(Client::<WorldType>::new(self.config.clone()))
            .add_resource(EndpointUrl(self.endpoint_url.clone()))
            .add_startup_system(network_setup.system())
            .add_startup_system(client_setup::<WorldType>.system())
            .add_system(client_system::<WorldType>.system());
    }
}
