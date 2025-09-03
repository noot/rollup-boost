mod behaviour;

use behaviour::Behaviour;

use eyre::Context;
use futures::stream::FuturesUnordered;
use libp2p::{
    Multiaddr, PeerId, StreamProtocol, Swarm, Transport as _, identity, noise, swarm::SwarmEvent,
    tcp, yamux,
};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::debug;

use crate::FlashblocksPayloadV1;

const FLASHBLOCKS_STREAM_PROTOCOL: StreamProtocol = StreamProtocol::new("/flashblocks/1.0.0");
const DEFAULT_AGENT_VERSION: &str = "rollup-boost/1.0.0";

pub(crate) struct Node {
    peer_id: PeerId,
    listen_addrs: Vec<libp2p::Multiaddr>,
    swarm: Swarm<Behaviour>,
    known_peers: Vec<Multiaddr>,
    payload_tx: mpsc::Sender<FlashblocksPayloadV1>,
    cancellation_token: tokio_util::sync::CancellationToken,
}

impl Node {
    pub(crate) fn peer_id(&self) -> PeerId {
        self.peer_id
    }

    pub(crate) fn listen_addrs(&self) -> &[libp2p::Multiaddr] {
        &self.listen_addrs
    }

    /// Returns the multiaddresses that this node is listening on, with the peer ID included.
    pub(crate) fn multiaddrs(&self) -> Vec<libp2p::Multiaddr> {
        self.listen_addrs
            .iter()
            .map(|addr| {
                addr.clone()
                    .with_p2p(self.peer_id)
                    .expect("can add peer ID to multiaddr")
            })
            .collect()
    }

    pub(crate) async fn run(self) -> eyre::Result<()> {
        use libp2p::futures::StreamExt as _;

        let Node {
            peer_id,
            listen_addrs,
            mut swarm,
            known_peers,
            payload_tx,
            cancellation_token,
        } = self;

        for addr in listen_addrs {
            swarm
                .listen_on(addr)
                .wrap_err("swarm failed to listen on multiaddr")?;
        }

        for mut address in known_peers {
            let peer_id = match address.pop() {
                Some(multiaddr::Protocol::P2p(peer_id)) => peer_id,
                _ => {
                    eyre::bail!("no peer ID for known peer");
                }
            };
            swarm.add_peer_address(peer_id, address.clone());
        }

        let mut control = swarm.behaviour_mut().new_control();
        let mut incoming = control
            .accept(FLASHBLOCKS_STREAM_PROTOCOL)
            .wrap_err("failed to accept incoming streams")?;

        let mut stream_handles = FuturesUnordered::new();

        loop {
            tokio::select! {
                biased;
                _ = cancellation_token.cancelled() => {
                    debug!("cancellation token triggered, shutting down node");
                    break Ok(());
                }
                event = swarm.select_next_some() => {
                    match event {
                        SwarmEvent::NewListenAddr {
                            address,
                            ..
                        } => {
                            debug!("new listen address: {address}");
                        }
                        SwarmEvent::ExternalAddrConfirmed { address } => {
                            debug!("external address confirmed: {address}");
                        }
                        SwarmEvent::ConnectionEstablished {
                            peer_id,
                            ..
                        } => {
                            debug!("connection established with peer {peer_id}");
                        }
                        SwarmEvent::ConnectionClosed {
                            peer_id,
                            cause,
                            ..
                        } => {
                            debug!("connection closed with peer {peer_id}: {cause:?}");
                        }
                        SwarmEvent::Behaviour(event) => event.handle().await,
                        _ => continue,
                    }
                },
                stream = incoming.next() => {
                    match stream {
                        Some((peer_id, stream)) => {
                            debug!("new incoming stream from peer {peer_id}");
                            let payload_tx = payload_tx.clone();
                            let handle = tokio::spawn(async move {
                                handle_incoming_stream(peer_id, stream, payload_tx).await
                            });
                            stream_handles.push(handle);
                        }
                        None => {
                            debug!("incoming stream channel closed");
                            break Ok(());
                        }
                    }
                }
                Some(res) = stream_handles.next() => {
                    match res {
                        Ok(Ok(())) => {
                            // stream handled successfully
                        }
                        Ok(Err(e)) => {
                            debug!("failed to handle incoming stream: {e:?}");
                        }
                        Err(e) => {
                            debug!("stream handling task failed: {e:?}");
                        }
                    }
                }
            }
        }
    }
}

pub(crate) struct NodeBuilder {
    port: Option<u16>,
    listen_addrs: Vec<libp2p::Multiaddr>,
    keypair: Option<identity::Keypair>,
    known_peers: Vec<Multiaddr>,
    cancellation_token: Option<tokio_util::sync::CancellationToken>,
}

impl Default for NodeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeBuilder {
    pub(crate) fn new() -> Self {
        Self {
            port: None,
            listen_addrs: Vec::new(),
            keypair: None,
            known_peers: Vec::new(),
            cancellation_token: None,
        }
    }

    pub(crate) fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub(crate) fn with_listen_addr(mut self, addr: libp2p::Multiaddr) -> Self {
        self.listen_addrs.push(addr);
        self
    }

    pub(crate) fn with_keypair(mut self, keypair: identity::Keypair) -> Self {
        self.keypair = Some(keypair);
        self
    }

    pub(crate) fn with_known_peer(mut self, address: Multiaddr) -> Self {
        self.known_peers.push(address);
        self
    }

    pub(crate) fn with_known_peers<I, T>(mut self, addresses: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Multiaddr>,
    {
        for address in addresses {
            self.known_peers.push(address.into());
        }
        self
    }

    pub(crate) fn with_cancellation_token(
        mut self,
        cancellation_token: tokio_util::sync::CancellationToken,
    ) -> Self {
        self.cancellation_token = Some(cancellation_token);
        self
    }

    pub(crate) fn try_build(
        self,
    ) -> eyre::Result<(
        Node,
        tokio::sync::mpsc::Receiver<FlashblocksPayloadV1>,
        libp2p_stream::Control,
    )> {
        let Self {
            port,
            mut listen_addrs,
            keypair,
            known_peers,
            cancellation_token,
        } = self;

        let keypair = keypair.unwrap_or(identity::Keypair::generate_ed25519());
        let peer_id = keypair.public().to_peer_id();

        let transport = create_transport(&keypair)?;
        let mut behaviour = Behaviour::new(&keypair, DEFAULT_AGENT_VERSION.to_string())
            .context("failed to create behaviour")?;
        let control = behaviour.new_control();

        let swarm = libp2p::SwarmBuilder::with_existing_identity(keypair)
            .with_tokio()
            .with_other_transport(|_| transport)?
            .with_behaviour(|_| behaviour)?
            .with_swarm_config(|cfg| {
                cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX)) // don't disconnect from idle peers
            })
            .build();
        if listen_addrs.is_empty() {
            let port = port.unwrap_or(0);
            let listen_addr = format!("/ip4/0.0.0.0/tcp/{port}")
                .parse()
                .expect("can parse valid multiaddr");
            listen_addrs.push(listen_addr);
        }

        let (tx, rx) = tokio::sync::mpsc::channel(100);

        Ok((
            Node {
                peer_id,
                swarm,
                listen_addrs,
                known_peers,
                payload_tx: tx,
                cancellation_token: cancellation_token.unwrap_or_default(),
            },
            rx,
            control,
        ))
    }
}

fn create_transport(
    keypair: &identity::Keypair,
) -> eyre::Result<libp2p::core::transport::Boxed<(PeerId, libp2p::core::muxing::StreamMuxerBox)>> {
    let transport = tcp::tokio::Transport::new(tcp::Config::default())
        .upgrade(libp2p::core::upgrade::Version::V1)
        .authenticate(noise::Config::new(keypair)?)
        .multiplex(yamux::Config::default())
        .timeout(Duration::from_secs(20))
        .boxed();

    Ok(transport)
}

async fn handle_incoming_stream(
    peer_id: PeerId,
    stream: libp2p::Stream,
    payload_tx: mpsc::Sender<FlashblocksPayloadV1>,
) -> eyre::Result<()> {
    use futures::StreamExt as _;
    use tokio_util::codec::FramedRead;
    use tokio_util::codec::LinesCodec;
    use tokio_util::compat::FuturesAsyncReadCompatExt as _;

    let codec = LinesCodec::new();
    let mut reader = FramedRead::new(stream.compat(), codec);

    loop {
        match reader.next().await {
            Some(Ok(str)) => {
                let payload: FlashblocksPayloadV1 = serde_json::from_str(&str)
                    .wrap_err("failed to decode stream message into FlashblocksPayloadV1")?;
                let _ = payload_tx.send(payload).await; // TODO: error if receiver drops?
            }
            Some(Err(e)) => {
                return Err(e).wrap_err(format!("failed to read from stream of peer {peer_id}"));
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod test {
    use tokio_util::{
        codec::{FramedWrite, LinesCodec},
        compat::FuturesAsyncReadCompatExt as _,
    };

    use super::*;

    #[tokio::test]
    async fn two_nodes_can_connect_and_stream() {
        use futures::SinkExt as _;

        let (node1, mut rx1, _) = NodeBuilder::new()
            .with_listen_addr("/ip4/127.0.0.1/tcp/9000".parse().unwrap())
            .try_build()
            .unwrap();
        let node1_peer_id = node1.peer_id();
        let (node2, _, mut control2) = NodeBuilder::new()
            .with_known_peers(node1.multiaddrs())
            .with_listen_addr("/ip4/127.0.0.1/tcp/9001".parse().unwrap())
            .try_build()
            .unwrap();

        tokio::spawn(async move { node1.run().await });
        tokio::spawn(async move { node2.run().await });

        // sleep to allow nodes to connect; implementing a way to get peer count is better
        tokio::time::sleep(Duration::from_secs(2)).await;

        // open stream from node1->node2
        let stream = control2
            .open_stream(node1_peer_id, FLASHBLOCKS_STREAM_PROTOCOL)
            .await
            .unwrap();

        let mut writer = FramedWrite::new(stream.compat(), LinesCodec::new());
        let payload = serde_json::to_string(&FlashblocksPayloadV1::default()).unwrap();
        writer.send(payload).await.unwrap();

        let received = rx1.recv().await.unwrap();
        assert_eq!(received, FlashblocksPayloadV1::default());
    }
}
