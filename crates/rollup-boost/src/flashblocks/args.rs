use clap::Parser;

#[derive(Parser, Clone, Debug)]
pub struct FlashblocksArgs {
    /// Enable Flashblocks client
    #[arg(long, env, default_value = "false")]
    pub flashblocks: bool,

    /// Flashblocks p2p port
    #[arg(long, env, default_value = "9000")]
    pub flashblocks_p2p_port: u16,

    /// Comma-separated list of multiaddrs of known Flashblocks peers
    /// Example: "/ip4/104.131.131.82/tcp/4001/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ,/ip4/104.131.131.82/udp/4001/quic-v1/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ"
    #[arg(
        long,
        env,
        default_value = "/ip4/127.0.0.1/tcp/9001/p2p/12D3KooW9sn2ZidTANAmQB1paiKBPGkF5DVusZXxaZCapbW94G44"
    )]
    pub flashblocks_known_peers: String,

    /// Flashblocks WebSocket host for outbound connections
    #[arg(long, env, default_value = "127.0.0.1")]
    pub flashblocks_host: String,

    /// Flashblocks WebSocket port for outbound connections
    #[arg(long, env, default_value = "1112")]
    pub flashblocks_port: u16,
}
