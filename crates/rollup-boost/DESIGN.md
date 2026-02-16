# DESIGN.md

## 1. Problem

Currently, rollup-boost and rbuilder communicate over websockets for streaming of flashblocks. This design, while functional, has several drawbacks:
- centralization of the network, as many rollup-boost nodes are connected and reliant on a single builder
- scalability bottlenecks as each builder must maintain and manage many TCP connections, potentially overloading the builder
- complicated reconnection logic for connection drops
- direct TCP connections are required, which may not work under complex network environments. 

We want to replace the websockets connections with a p2p network, which will address some of these drawbacks.

## 2. Possible designs and trade-offs

The following sections assumes usage of the libp2p library, which is a fully-featured p2p library that includes features such as NAT traversal and hole-punching, connections via relay, a kademlia DHT, and gossip. It also has support for connection management for persistent connections.

### 2.1 Direct p2p connections with builder

A simple implementation would be to have each builder and rollup-boost instance run a p2p node, and rollup-boost nodes connect directly (or via relay, in the case a direct connection is not possible) to a builder node. On creation of a flashblock, the builder would broadcast this to all connected nodes.

The benefits of this approach are its simplicity, and due to usage of libp2p, will remove the need for reconnection management. It also doesn't add communication hops compared to the existing websocket approach, as messages are sent directly. However, it doesn't help with the centralization or scalability bottlenecks, as each builder is still a single node which must maintain many connections.

### 2.2 Gossip network

A potential improvement would be to use a gossip network for dissemination of flashblocks. The main benefit of this is to reduce the bottleneck on the builder of having to maintain many connections. With a gossip network, the builder can maintain fewer connections, while the flashblock will still eventually reach each node. As well, once a message is initially sent out, if the builder goes down, the message will still reach more nodes, which is not the case in the completely centralized version above. 

The main drawback of this is potential added latency. Nodes which don't have a direct connection to the builder will experience added latency due to added hops. Flashblocks are latency-critical as the whole point is to provide fast preconfirmations to users, so if a gossip network is used, there should be measures to reduse latency as much as possible. For example, a group of co-located nodes can be grouped, and this whole group can have a single connection to the builder. Once one node in the group receives a block, it can reach the other nodes in the group relatively quickly.  

### 2.3 Peer discovery

The above approaches are agnostic to how discovery happens. Node discovery can happen in one of three ways:
- a known peer address list
- mDNS discovery of nodes on the local network
- DHT discovery of nodes on the entire network (which requires known bootnodes)

In the case of direct connections (2.1), rollup-boost nodes only need to connect to the builder node. It can either know its address directly, or discovery it over the DHT. However, since nodes only need to connect to the builder, an entire DHT is not necessary.

In the case of a gossip network (2.2), each node needs to maintain connections to many other nodes, so it makes sense to implement a DHT for peer discovery.  Each node should have a minimum number of connected peers. Additionally, extra peer information can be stored in the DHT for future improvements, such as location.

Another consideration here is privacy and security. Having a public network with a publicly-advertised builder p2p address can allow for potential DOS attacks on the builder or on the rollup-boost nodes (and thus sequencers).  A public DHT could also be sybilled, for instance if many malicious nodes join the routing table. The libp2p kademlia implementation has some protections against this such as internal peer scoring. If the network doesn't need to be public, it should be invite-only and bootnodes can be shared only with known parties. Since the flashblocks spec mentions the rollup-boost endpoint should be private, the p2p should likely also remain private.

## 3. PoC implementation

### Assumptions

- there is only one builder per network. Multiple builders on one network causes significant complexity as flashblock equivocation is not allowed, so some sort of builder rotation algorithm must be implemented.
- the p2p address of the builder is well-known.
- the builder going down is out of scope for the PoC.

### Topology

I will implement the direct p2p connection approach in 2.1 for the PoC, as it's the most straightforward and can be extended easily. A gossip network, while having benefits, 
has latency concerns which may make it infeasible without further research and design. For discovery, I will implement connecting to known peers via config as well as mDNS.

### Message protocol

The protocol uses one message type, the existing `FlashblocksPayloadV1`:
```rust!
pub struct FlashblocksPayloadV1 {
    /// The payload id of the flashblock
    pub payload_id: PayloadId,
    /// The index of the flashblock in the block
    pub index: u64,
    /// The base execution payload configuration
    pub base: Option<ExecutionPayloadBaseV1>,
    /// The delta/diff containing modified portions of the execution payload
    pub diff: ExecutionPayloadFlashblockDeltaV1,
    /// Additional metadata associated with the flashblock
    pub metadata: Value,
}
```

To send it over the wire, it will be JSON-encoded, same as the current implementation. (Note: the flashblocks spec says SSZ will be used, but that doesn't seem to be the case in the implementation).

[`tokio_util::codec::LinesCodec`](https://caolan.github.io/tamawiki/tokio/codec/struct.LinesCodec.html) is used to read and write one message
at a time over the wire. It's a simple codec which separates each message by a line. 

## 5. Extensions and productionization

On the code-level, the following is needed for productionization:
- update the CLI so that each p2p value can be configured, such as listen addresses, known peers, and private key.
- the private key, if provided, should be read from a file.
- use a `CancellationToken` for signal handling to properly shut down the p2p node.
- export constants such as `FLASHBLOCKS_STREAM_PROTOCOL` for import into `rbuilder`.
- add metrics such as peer count, re-add metrics such as flashblocks received count

On the features level:
- connection handling, such as [keep-alive](https://docs.rs/libp2p/latest/libp2p/swarm/trait.ConnectionHandler.html#method.connection_keep_alive) for rbuilder nodes, as we wish to 
permanently stay connected to them.
- a builder failure protocol in case the single builder fails. 

To extend the network to a gossip network, the following should be implemented:
- signing of the `FlashblocksPayloadV1` by the builder
- a DHT for discovery of distributed peers

To extend the network to multiple builders, the following should be implemented:
- a builder-selection algorithm which determines which builder is authorized to build flashblocks at a point in time. 
- since the list of potential builders is well-known, it should be straightforward to implement round-robin, with fallback to the next builder in the sequence in the case of failure.
