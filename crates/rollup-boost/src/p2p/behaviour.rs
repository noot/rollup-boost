use libp2p::{
    autonat, connection_limits, connection_limits::ConnectionLimits, identify, identity, ping,
    swarm::NetworkBehaviour,
};
use std::{convert::Infallible, time::Duration};

const DEFAULT_MAX_PEER_COUNT: u32 = 200; // TODO: make configurable based on node type
const ROLLUP_BOOST_STREAM_PROTOCOL: &str = "/rollup-boost/1.0.0";

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "BehaviourEvent")]
pub(crate) struct Behaviour {
    // connection gating
    connection_limits: connection_limits::Behaviour,

    // protocols
    identify: identify::Behaviour,
    ping: ping::Behaviour,
    stream: libp2p_stream::Behaviour,

    // nat traversal
    autonat: autonat::Behaviour,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub(crate) enum BehaviourEvent {
    Autonat(autonat::Event),
    Identify(identify::Event),
    Ping(ping::Event),
}

impl From<()> for BehaviourEvent {
    fn from(_: ()) -> Self {
        unreachable!("() cannot be converted to BehaviourEvent")
    }
}

impl From<Infallible> for BehaviourEvent {
    fn from(_: Infallible) -> Self {
        unreachable!("Infallible cannot be converted to BehaviourEvent")
    }
}

impl From<autonat::Event> for BehaviourEvent {
    fn from(event: autonat::Event) -> Self {
        BehaviourEvent::Autonat(event)
    }
}

impl From<ping::Event> for BehaviourEvent {
    fn from(event: ping::Event) -> Self {
        BehaviourEvent::Ping(event)
    }
}

impl From<identify::Event> for BehaviourEvent {
    fn from(event: identify::Event) -> Self {
        BehaviourEvent::Identify(event)
    }
}

impl Behaviour {
    pub(crate) fn new(keypair: &identity::Keypair, agent_version: String) -> eyre::Result<Self> {
        let peer_id = keypair.public().to_peer_id();

        let autonat = autonat::Behaviour::new(peer_id, autonat::Config::default());
        let connection_limits = connection_limits::Behaviour::new(
            ConnectionLimits::default().with_max_established(Some(DEFAULT_MAX_PEER_COUNT)),
        );

        let identify = identify::Behaviour::new(
            identify::Config::new(ROLLUP_BOOST_STREAM_PROTOCOL.to_string(), keypair.public())
                .with_agent_version(agent_version),
        );
        let ping = ping::Behaviour::new(ping::Config::new().with_interval(Duration::from_secs(10)));
        let stream = libp2p_stream::Behaviour::new();

        Ok(Self {
            autonat,
            connection_limits,
            identify,
            ping,
            stream,
        })
    }

    pub(crate) fn new_control(&mut self) -> libp2p_stream::Control {
        self.stream.new_control()
    }
}

impl BehaviourEvent {
    pub(crate) async fn handle(self) {
        match self {
            BehaviourEvent::Autonat(_event) => {}
            BehaviourEvent::Identify(_event) => {}
            BehaviourEvent::Ping(_event) => {}
        }
    }
}
