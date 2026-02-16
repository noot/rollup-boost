# rollup-boost

## Design of p2p implementation

See [DESIGN.md](./DESIGN.md).

## Test setup

To test the libp2p PoC, you will need to clone and build `builder-playground`:

```sh
git clone https://github.com/noot/builder-playground
cd builder-playground
go build -o builder-playground main.go
```

Then, run the test environment with the p2p override images:
```sh
./builder-playground cook opstack \
    --external-builder op-rbuilder \
    --flashblocks \
    --override rollup-boost=noot99/rollup-boost:p2p-poc \
    --override op-rbuilder=noot99/op-rbuilder:p2p-poc \
    --timeout 10m \
    --watchdog
```

This will start rollup-boost and rbuilder with a p2p node that will automatically 
discover each other and connect. rbuilder will open a stream with rollup-boost
when it connects, and open a stream to transmit `FlashblocksPayloadV1`s.

You should be able to see a stream being opened in the `rollup-boost` logs:

```sh
$ docker logs devnet-rollup-boost-1  | grep peer
2025-09-04T01:46:44.146036Z  INFO libp2p_swarm: local_peer_id=12D3KooWEhv5wbBmAo1WfSF5xXsCLjHpNbYveG7zo5pHd6DmskGK
2025-09-04T01:46:44.146703Z  INFO rollup_boost::p2p: node starting with peer ID 12D3KooWEhv5wbBmAo1WfSF5xXsCLjHpNbYveG7zo5pHd6DmskGK
2025-09-04T01:46:44.361104Z  INFO libp2p_mdns::behaviour: discovered peer on address peer=12D3KooW9sn2ZidTANAmQB1paiKBPGkF5DVusZXxaZCapbW94G44 address=/ip4/172.20.0.11/tcp/9001/p2p/12D3KooW9sn2ZidTANAmQB1paiKBPGkF5DVusZXxaZCapbW94G44
2025-09-04T01:46:44.361132Z  INFO rollup_boost::p2p::behaviour: discovered peer 12D3KooW9sn2ZidTANAmQB1paiKBPGkF5DVusZXxaZCapbW94G44 at /ip4/172.20.0.11/tcp/9001/p2p/12D3KooW9sn2ZidTANAmQB1paiKBPGkF5DVusZXxaZCapbW94G44
2025-09-04T01:46:44.363396Z  INFO rollup_boost::p2p: connection established with peer 12D3KooW9sn2ZidTANAmQB1paiKBPGkF5DVusZXxaZCapbW94G44
2025-09-04T01:46:44.363973Z  INFO rollup_boost::p2p: new incoming stream from peer 12D3KooW9sn2ZidTANAmQB1paiKBPGkF5DVusZXxaZCapbW94G44
```

You should also see flashblocks being re-broadcasted upon receipt to connected RPC providers:

```sh
$ docker logs devnet-rollup-boost-1  | grep flashblocks
2025-09-04T01:49:24.951231Z  INFO get_payload_v3{otel.kind=Server payload_id=0x03f8b4e61591ea2d builder_has_payload=true flashblocks_count=5}: rollup_boost::flashblocks::service: Returning fb payload
2025-09-04T01:49:24.951284Z  INFO get_payload_v3{otel.kind=Server payload_id=0x03f8b4e61591ea2d builder_has_payload=true flashblocks_count=5}:new_payload_v3{otel.kind=Client target="l2" url=http://op-geth:8551/ block_hash=0x9c7b05f048cc03440d7ddb1f1352bd9b87417a9f2343efd94fa80363c45c4a06}: rollup_boost::client::rpc: Sending new_payload_v3 to l2
2025-09-04T01:49:24.955650Z  INFO get_payload_v3{otel.kind=Server payload_id=0x03f8b4e61591ea2d builder_has_payload=true flashblocks_count=5 gas_delta="0" tx_count_delta="0" payload_source="builder"}: rollup_boost::server: returning block hash=0x9c7b05f048cc03440d7ddb1f1352bd9b87417a9f2343efd94fa80363c45c4a06 number=77 context=builder payload_id=0x03f8b4e61591ea2d
2025-09-04T01:49:25.010878Z  INFO rollup_boost::flashblocks::outbound: Broadcasted payload: Utf8Bytes(b"{\"payload_id\":\"0x034dc8a8a8d6b81c\",\"index\":0,\"base\":{\"parent_beacon_block_root\":\"0xfe4d991580f343beeff46c230f6325bfc56ebca43407c974edc63b0021128561\",\"parent_hash\":\"0x9c7b05f048cc03440d7ddb1f1352bd9b87417a9f2343efd94fa80363c45c4a06\",\"fee_recipient\":\"0x4200000000000000000000000000000000000011\",\"prev_randao\":\"0xee4bc6957a1cd5e7c3b7878f6cf84e6594551cf638fde338beff6863db00318d\",\"block_number\":\"0x4e\",\"gas_limit\":\"0x3938700\",\"timestamp\":\"0x68b8f027\",\"extra_data\":\"0x00000000fa00000006\",\"base_fee_per_gas\":\"0x2baa49a1\"},\"diff\":{\"state_root\":\"0xffadd3977a9e62a7009d3db91ea67f3f176b8df584a4fb7abb560e6c2065a5e2\",\"receipts_root\":\"0x2e99181e6bb810d887ca5b79326ddc3fe363bac8e0fa6194a0fbcc5a7806fd9b\",\"logs_bloom\":\"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\",\"gas_used\":\"0xab4e\",\"block_hash\":\"0x7998073005f70040b3bad12d2cbdce3329f1c56ffd509464a36b4f4fdc65b203\",\"transactions\":[\"0x7ef8f8a04769bd9d160d7483ec80c1aa6cc8b3bc7649ada9b5cf87fe01ae6d789b0250c494deaddeaddeaddeaddeaddeaddeaddeaddead00019442000000000000000000000000000000000000158080830f424080b8a4440a5e2000000558000c5fc500000000000000050000000068b8f019000000000000000c000000000000000000000000000000000000000000000000000000000c09a42800000000000000000000000000000000000000000000000000000000000000015fa7bccda30295afe55020e302ef9c1f1e7c376c265e173f5feabb67243b8c25000000000000000000000000aff0ca253b97e54440965855cec0a8a2e2399896\"],\"withdrawals\":[],\"withdrawals_root\":\"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421\"},\"metadata\":{\"block_number\":78,\"new_account_balances\":{\"0x000f3df6d732807ef1319fb7b8bb8522d0beac02\":\"0x0\",\"0x4200000000000000000000000000000000000015\":\"0x0\",\"0xdeaddeaddeaddeaddeaddeaddeaddeaddead0001\":\"0x0\"},\"receipts\":{\"0x2ed48b489ea40a32cdbd64cd2f4aae468fc12add7c50debc833ddbced97c22f0\":{\"Deposit\":{\"cumulativeGasUsed\":\"0xab4e\",\"depositNonce\":\"0x4d\",\"depositReceiptVersion\":\"0x1\",\"logs\":[],\"status\":\"0x1\"}}}}}")
```

In rbuilder, you should also see that it has a p2p peer and it sending payloads to it:

```sh
$ docker logs devnet-op-rbuilder-1  | grep peer
2025-09-04T01:50:54.013547Z  INFO received new payload to broadcast to peers peer_count=1
```
