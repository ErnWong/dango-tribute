use bevy::prelude::*;
use bevy_networking_turbulence::{NetworkEvent, NetworkResource, NetworkingPlugin};
use bevy_prototype_networked_physics::{server::Server, world::World, Config};
use shared::{networking::WrappedNetworkResource, settings::network_setup};
use std::marker::PhantomData;

pub fn server_system<WorldType: World>(
    mut server: ResMut<Server<WorldType>>,
    time: Res<Time>,
    mut net: ResMut<NetworkResource>,
) {
    server.update(
        time.delta_seconds(),
        time.seconds_since_startup(),
        &mut WrappedNetworkResource(&mut *net),
    );
}

pub struct EndpointUrl(pub String);

pub fn server_setup<WorldType: World>(
    mut server: ResMut<Server<WorldType>>,
    time: Res<Time>,
    mut net: ResMut<NetworkResource>,
    endpoint_url: Res<EndpointUrl>,
) {
    server.update_timestamp(time.seconds_since_startup());
    info!("Starting server - listening at {}", endpoint_url.0.clone());
    net.listen(endpoint_url.0.clone());
}

#[derive(Default)]
pub struct NetworkedPhysicsServerPlugin<WorldType: World> {
    config: Config,
    endpoint_url: String,
    _world_type: PhantomData<WorldType>,
}

impl<WorldType: World> NetworkedPhysicsServerPlugin<WorldType> {
    pub fn new(config: Config, endpoint_url: String) -> Self {
        Self {
            config,
            endpoint_url,
            _world_type: PhantomData,
        }
    }
}

impl<WorldType: World> Plugin for NetworkedPhysicsServerPlugin<WorldType> {
    fn build(&self, app: &mut AppBuilder) {
        app.add_plugin(NetworkingPlugin::default())
            .add_resource(Server::<WorldType>::new(self.config.clone()))
            .add_resource(EndpointUrl(self.endpoint_url.clone()))
            .add_startup_system(network_setup.system())
            .add_startup_system(server_setup::<WorldType>.system())
            .add_system(server_system::<WorldType>.system());
    }
}
