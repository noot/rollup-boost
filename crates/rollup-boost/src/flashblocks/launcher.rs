use crate::{FlashblocksService, RpcClient, p2p::NodeBuilder};
use core::net::SocketAddr;

pub struct Flashblocks {}

impl Flashblocks {
    pub fn run(
        builder_url: RpcClient,
        p2p_port: u16,
        known_peers: Vec<String>,
        outbound_addr: SocketAddr,
    ) -> eyre::Result<FlashblocksService> {
        let known_peers: Vec<libp2p::Multiaddr> = known_peers
            .into_iter()
            .filter_map(|addr| addr.parse().ok())
            .collect();
        let (node, rx, _) = NodeBuilder::new()
            .with_port(p2p_port)
            .with_known_peers(known_peers)
            .try_build()
            .unwrap();
        tokio::spawn(node.run());

        let service = FlashblocksService::new(builder_url, outbound_addr)?;
        let mut service_handle = service.clone();
        tokio::spawn(async move {
            service_handle.run(rx).await;
        });

        Ok(service)
    }
}
