use clap::Parser;

#[derive(Parser, Clone, Debug)]
pub struct FlashblocksArgs {
    /// Enable Flashblocks client
    #[arg(long, env, default_value = "false")]
    pub flashblocks: bool,

    /// Flashblocks p2p port
    #[arg(long, env, default_value = "9000")]
    pub flashblocks_p2p_port: u16,

    /// Flashblocks WebSocket host for outbound connections
    #[arg(long, env, default_value = "127.0.0.1")]
    pub flashblocks_host: String,

    /// Flashblocks WebSocket port for outbound connections
    #[arg(long, env, default_value = "1112")]
    pub flashblocks_port: u16,
}
