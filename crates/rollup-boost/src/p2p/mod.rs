mod behaviour;

use behaviour::Behaviour;

use eyre::Context;
use futures::stream::FuturesUnordered;
use libp2p::{Multiaddr, PeerId, StreamProtocol, Swarm, swarm::SwarmEvent};
use tracing::debug;

const FLASHBLOCKS_STREAM_PROTOCOL: StreamProtocol = StreamProtocol::new("/flashblocks/1.0.0");

pub(crate) struct Node {
    peer_id: PeerId,
    listen_addrs: Vec<libp2p::Multiaddr>,
    swarm: Swarm<Behaviour>,
    bootnodes: Vec<Multiaddr>,
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
            peer_id: _,
            listen_addrs,
            mut swarm,
            bootnodes,
            cancellation_token,
        } = self;

        for addr in listen_addrs {
            swarm
                .listen_on(addr)
                .wrap_err("swarm failed to listen on multiaddr")?;
        }

        for bootnode in bootnodes {
            match swarm.dial(bootnode.clone()) {
                Ok(_) => {}
                Err(e) => {
                    debug!("failed to dial bootnode {bootnode}: {e:?}");
                }
            }
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
                stream = incoming.next() => {
                    match stream {
                        Some((peer_id, stream)) => {
                            debug!("new incoming stream from peer {peer_id}");
                            let handle = tokio::spawn(async move {
                                handle_incoming_stream(peer_id, stream).await
                            });
                            stream_handles.push(handle);
                        }
                        None => {
                            debug!("incoming stream channel closed");
                            break Ok(());
                        }
                    }
                }
                res = stream_handles.next() => {
                    match res {
                        Some(Ok(Ok(()))) => {
                            // stream handled successfully
                        }
                        Some(Ok(Err(e))) => {
                            debug!("failed to handle incoming stream: {e:?}");
                        }
                        Some(Err(e)) => {
                            debug!("stream handling task failed: {e:?}");
                        }
                        None => {
                            // no more stream handles
                        }
                    }
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
            }
        }
    }
}

async fn handle_incoming_stream(peer_id: PeerId, mut stream: libp2p::Stream) -> eyre::Result<()> {
    use futures::AsyncReadExt as _;

    let mut buf = vec![0u8; 1024];
    loop {
        match stream.read(&mut buf).await {
            Ok(0) => {
                // nothing was read
            }
            Ok(n) => {
                debug!("received {} bytes from peer {peer_id}", n);
            }
            Err(e) => {
                break Err(e).wrap_err(format!("failed to read from stream of peer {peer_id}"));
            }
        }
    }
}
