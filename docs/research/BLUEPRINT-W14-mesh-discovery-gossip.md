# BLUEPRINT W14 — discovery / gossip (MESH-02/03, bebop proto-wire)

Status: FULLY offline-verifiable. Zero new deps (reuses `quinn`/`QuicTransport`).
Decart: hand-rolled QUIC gossip — see NEXT-PHASES-research-decisions.md.

## New file: `bebop2/proto-wire/src/discovery.rs`

### `AnchorRoster` reuse
`QuicTransport::with_roster(AnchorRoster)` already carries the trusted peer set.
Discovery = learning peer endpoints; gossip = propagating learned rosters.

### `PeerDirectory` (content-addressed, in-memory)
```
pub struct PeerDirectory {
    peers: BTreeMap<PeerId, SignedEndpoint>,  // deterministic order
    revoked: RevocationSet,
}
impl PeerDirectory {
    pub fn merge(&mut self, other: &PeerDirectory) -> Vec<PeerId>; // returns newly-learned
    pub fn evict_revoked(&mut self, revs: &RevocationSet);
    pub fn snapshot_root(&self) -> String; // FNV-1a over sorted (id,endpoint)
}
```

### `GossipAgent` (periodic roster anti-entropy)
```
pub struct GossipAgent { dir: PeerDirectory, transport: QuicTransport }
impl GossipAgent {
    pub fn tick(&mut self) -> Vec<PeerId>;  // push own roster to known peers,
                                             // merge their responses, return new peers
}
```
MESH-02 = first roster fetch from anchors. MESH-03 = re-gossip learned peers
(recursion depth bounded by roster TTL / revocation). NO DHT — full-roster
exchange only (matches anchored allow-list model).

## Tests (RED→GREEN, offline)
1. `peer_directory_merge_dedup` — merge two dirs, no duplicate; snapshot_root
   deterministic regardless of insertion order.
2. `revocation_evicts` — a revoked peer is dropped on `evict_revoked`.
3. `gossip_converges_3node` (extends `mesh_sync_integration.rs` harness) — spin 3
   `QuicTransport` endpoints, each a `GossipAgent` seeded with ONE distinct anchor;
   after N ticks all 3 `PeerDirectory::snapshot_root` are IDENTICAL (full mesh
   discovery via gossip). Real QUIC, no mocks.

## Cargo.toml
No new dependencies. `discovery.rs` is part of `bebop-proto-wire` default build
(std + quinn only).

## Verify (parent)
`cargo test -p bebop-proto-wire --features insecure-test` → all prior mesh tests
+ 3 new gossip tests GREEN. `cargo test -p bebop2-core --features host` still 232+.

## Honest note
Gossip here is periodic full-roster anti-entropy, NOT a scalable DHT. That is the
correct, falsifiable scope for an anchored allow-list mesh; scaling to thousands of
peers is a future wave (would revisit libp2p only if cache + need justify it).
