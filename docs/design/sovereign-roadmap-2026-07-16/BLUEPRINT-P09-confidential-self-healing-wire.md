# BLUEPRINT — PHASE 9: CONFIDENTIAL, SELF-HEALING WIRE

> Transport-substrate completion. The wire is authenticated today but **readable**; the mesh
> cannot heal a partition or route around a dropped node. This phase closes both — it turns the
> mesh from a "sync layer that survives if the network is honest" into a "confidential mesh that
> survives partition and node loss."
>
> **Roadmap position:** Phase 9 of 19 (`R2-MERGED-PHASE-ROADMAP.md`). Largest phase — **20 anchors**.
> On the critical path **P3 → P9 → P10 → P13**. Wave 2.
> **Canon:** `ARCHITECTURE.md` §0 (M3/M6/M7), §1 (D2), §6 (F11–F30), §8 (honest gaps).
> **Primary evidence:** `R1-A-mesh-crypto-gap-analysis.md` §Phase-B + per-anchor table;
> `R1-B-hub-autonomy-agent-infra-gap-analysis.md` §M7.
> **This is a planning blueprint. It writes no code and edits no source.** Every file:line below is
> an evidence citation inherited from R1-A/R1-B (bebop-repo tip `b87b7e2`), to be re-verified at build
> time against the live tree — the index may be stale.

---

## 0. Phase framing

### 0.1 What this phase IS and IS NOT

- **IS** the *integration* of confidentiality and self-healing onto the existing `Transport` trait and
  `SignedFrame` boundary. It wires primitives that already exist (canonical ML-KEM from Phase 3, the
  graph-math library from Phase 4) into the live wire.
- **IS NOT** the place where graph algorithms are invented. **All heal math — Dijkstra, A\*, DSU
  (union-find), Kruskal/MST — lands exactly once in Phase 4 (kernel, zero-dep) and is CONSUMED here.**
  If you find yourself writing a shortest-path loop in `mesh-node`, stop: that is a Phase 4 regression.
  Per R2 §1 "major merges," the M7 heal layer was named twice (A-Phase-B mesh-healing + B's M7 owner
  assignment) and is one work item; the math primitive is a Phase-4 deliverable, the mesh overlay that
  calls it is a Phase-9 deliverable.
- **IS NOT** the product wiring. dowiz does not ride the wire until Phase 13. F45/F46 (route, partition-
  tolerant delivery) consume the *same* Phase-4 math on the *product* surface; Phase 9 consumes it on the
  *mesh-topology* surface. Two consumers, one library.

### 0.2 Dependencies (hard) and consumed rulings

| Kind | Source | What Phase 9 needs from it |
|---|---|---|
| Hard dep | **Phase 3** (PQ trust-root hardened) | F16 must encrypt with the *canonical* ML-KEM-768, not one of the three divergent stacks. Revocation gossip ships the *finalized* `RevocationSet`. Session keys are established between hybrid-anchored identities whose delegation chain is no longer classically forgeable. **Do not start F16 until Phase 3 designates ONE canonical KEM.** |
| Hard dep | **Phase 4** (kernel graph-math) | Dijkstra/A\* over the peer graph (E32), DSU + Kruskal/MST (E33), consumed by the heal module (M7). Phase 4 ports the stranded router (`crates/bebop/src/cost_estimate.rs:205-290`) into the kernel onto `csr.rs` with a graph-ingestion port. **Do not start the heal module until Phase 4 exports these.** |
| Ruling | **Phase 2 · O5** (D2/iroh direction) | The §3 DECART executes whichever direction the operator chose: land `iroh` for real, or amend canon to "quinn primary + named network-unlock trigger." Phase 9 does not *decide* O5; it *executes* the decision as a written DECART. |
| Ruling | **Phase 2 · O6** (E35 3-tier locality) | E35 is **not** a Phase-9 anchor, but R2 §5 records "impl attaches to Phase 9 if defined." If O6 defines the three tiers, the locality model attaches to the discovery/topology layer built here; if O6 strikes E35, this attachment is dropped. Conditional — do not build a locality model on an undefined anchor. |
| Ruling | **Phase 2 · O11** (M12/F25 replay bound) | §4 implements F25 per the ruling: bless the per-gate in-process bound, or require persistence. Phase 9 does not choose; it executes. |

### 0.3 Parallel-safety and the one-sentence gap

Parallel-safe with **P5, P6, P7, P8, P12** (zero shared mutable surface — those touch routing, verifier,
money, observability, storage; Phase 9 touches `proto-wire` + a new heal module + `entropy.rs` toggle).

**The gap in one sentence:** the frames are signed but sent in the clear over a plaintext carrier that
accepts any certificate, and there is no mechanism by which the mesh notices a partition, merges two
divergent islands, or routes around a node that dropped.

### 0.4 Anchor roster (20) → owning sub-section

| Sub-section | Anchors | Headline |
|---|---|---|
| §1 Wire Confidentiality | **F16**, M6 (crypto leg) | ML-KEM session encryption to a just-met peer; kill `NoopPayloadEnc`/`insecure-tls`/`ws://` |
| §2 Mesh Healing | **M7**, E31 (HRW leg), **E32**, **E33**, **F15** | Partition detect (DSU) · re-route (Dijkstra/A\*) · gossip overlay (MST) · HRW split-brain merge — all consuming Phase 4 |
| §3 D2 Transport Decision | **D2**, **F20**, **E34**, E31 (iroh leg) | Written DECART: land iroh OR amend canon with a named network-unlock trigger |
| §4 Revocation & Replay | **F23**, **E38**, **F25** | Live revocation-gossip propagation loop; F25 replay bound per O11 |
| §5 QRNG Beacon | **M3**, **F13**, **F22** | Operator beacon toggle + beacon-down graceful-degrade test (never fail-closed on the whole system) |
| §6 At-Rest Encryption | **F30** | Wire stranded `vault.rs` XChaCha20-Poly1305 into the live per-hub store |
| §7 Batching & Transport-Trait Extension | **F18**, M6 (transport leg) | Library batching (not test-only); stdio/HTTP transports behind the existing Trait |
| §8 Regression guard | **F11**, **F12** | ALREADY BUILT — must survive the refactor, not be rebuilt |

M6 appears in §1 (zero-dep crypto leg), §7 (transport-swap leg); E31 appears in §2 (HRW leg) and §3
(iroh leg) — one anchor, two five-word conflations that R1-A flagged (`E31 iroh+HRW`).

---

## 1. WIRE CONFIDENTIALITY (F16, M6-crypto)

### 1.1 Current-state evidence

The wire is **authenticated but not confidential** — R1-A headline #3, HIGH.

- **ML-KEM is implemented but never invoked for transport.** `grep encaps|decaps` across
  `proto-wire`/`mesh-node` returns **zero**. The KEM is a finished library with no caller on the wire.
- **The default compiled feature is literally `insecure-tls`** (`Cargo.toml:38`). `InsecureAcceptAny`
  accepts *any* certificate (`iroh_transport.rs:149-155`).
- **The WSS carrier runs plaintext `ws://`, not `wss://`** — `wss_transport.rs:451-458` carries its own
  self-description: "authenticated but readable by a passive on-path observer."
- **A `NoopPayloadEnc` passthrough sits exactly where the ML-KEM→XChaCha20-Poly1305 layer belongs**
  (`transport_policy.rs:107-133`).

Net: a passive on-path observer reads every frame's plaintext. Frames are signed (integrity + origin),
so an attacker cannot forge, but confidentiality in transit is TLS-only, and the default TLS is a no-op.
**F16 ("encrypts traffic with ML-KEM to a peer it just met") is NOT BUILT.**

### 1.2 Target design

A confidentiality layer that replaces `NoopPayloadEnc` with a real ML-KEM → AEAD session, established on
first contact with **no prior enrollment** (F16 is a self-certified, 0-RTT-*ish* handshake — the peers
have "just met").

1. **Handshake (self-certified).** On `connect`/`accept`, each side already presents a hybrid-signed
   `SignedFrame` identity (Phase 3). The initiator generates an ML-KEM-768 encapsulation against the
   responder's advertised KEM public key (carried in / bound to the identity frame), yielding a shared
   secret + ciphertext. The responder decapsulates. **This is the first `encaps`/`decaps` call pair on
   the wire.** Because both identities are hybrid-anchored (Phase 3), the handshake inherits the
   trust-root fix — the KEM public key is bound into a chain that is no longer classically forgeable.
2. **Session-key derivation.** The ML-KEM shared secret is run through the canonical KDF (SHA3/SHAKE from
   `bebop2-core`, zero-dep — M6) to a symmetric session key. Optionally mixed with QRNG beacon entropy
   when `--features anu` is on (§5) — beacon *hardens* the key derivation, it is never the sole source.
3. **AEAD payload encryption.** Replace `NoopPayloadEnc` with an XChaCha20-Poly1305 payload encryptor
   keyed by the session key. This is the **same AEAD** as the at-rest layer (§6) — one cipher, two
   surfaces. Nonces come from the entropy floor (§5), never reused per key.
4. **Carrier hardening.** Remove `insecure-tls` as the *default* feature; the default build must not
   accept any certificate. The WSS carrier must be `wss://` (or, since the payload is now AEAD-sealed
   above the carrier, plaintext `ws://` becomes defensible *only* as a debug feature — decided in the
   DECART of §3 alongside the transport direction). Either way, **the default build emits ciphertext.**
5. **Canonical ML-KEM only.** F16 encrypts with the ONE ML-KEM-768 that Phase 3 designates canonical
   (the FIPS-203-exact `bebop2/core/src/pq_kem.rs`, `J(z‖c)` implicit rejection, η1=2). The non-FIPS
   `proto-crypto` stack (η1=3, `H(sk‖c)`, `G=SHAKE256`) must not be the one wired here.

### 1.3 Acceptance criteria

- **HEADLINE TEST (F16):** two nodes that have just met (no prior enrollment) derive a working ML-KEM
  session key, and a **passive on-path packet capture shows CIPHERTEXT, not plaintext**. This is the
  phase's flagship falsifiable test.
- `grep encaps|decaps` over `proto-wire`/`mesh-node` is now **non-zero** and reachable from the live
  `connect`/`accept` path (not test-only).
- The default `cargo build` no longer selects `insecure-tls`; a build that accepts any certificate
  requires an explicit opt-in feature.
- `NoopPayloadEnc` is deleted or demoted to an explicitly-named debug feature; the default `PayloadEnc`
  is the XChaCha20-Poly1305 session encryptor.
- A 1-bit flip in the ML-KEM ciphertext causes the session to fail closed (no plaintext fallback).

---

## 2. MESH HEALING (M7, E31-HRW, E32, E33, F15)

### 2.1 Current-state evidence

**The entire M7 self-healing layer is 0% built** — R1-B §M7 confirms `grep` across all of `bebop2/`
returns **zero** Dijkstra, A\*, Union-Find, or MST implementations *on the mesh surface*.

- **The math exists on the wrong surface.** A real zero-dep Dijkstra/A\*/contraction-hierarchy router is
  stranded in `crates/bebop/src/cost_estimate.rs:205-290` (courier *route cost*, not mesh topology).
  Union-find exists in `dowiz/kernel/src/order_machine.rs:611-656` (FSM cyclomatic number, not network
  membership). MST is found **nowhere**. HRW exists in `proto-cap/matcher.rs` (181 ln) for *courier
  assignment tie-break*, **not** as a partition-merge routine.
- **Partial substrate the heal layer will compose with:** peer discovery + revocation eviction
  (`discovery.rs:44-174`, `evict_revoked` at `:82`); `RevocationSet::merge` anti-entropy
  (`revocation.rs:14-16`).
- **"No leader election required for liveness" is vacuously satisfied** — no election code exists at all,
  which is *correct* by M7 (no leader needed), but it means there is also no partition-awareness.

R1-A per-anchor: **F15** (partition/HRW-merge) NOT BUILT; **E32** (Dijkstra/A\*) NOT BUILT for mesh;
**E33** (Union-Find/MST) NOT BUILT for mesh; **M7** heal = 0%.

### 2.2 Target design — a `heal` module over the peer graph that CONSUMES Phase 4

A new mesh-side module (in `mesh-node`, calling Phase 4's kernel exports) that maintains a live model of
the peer topology and reacts to change. **It contains no graph algorithms of its own** — it builds the
graph from live peer state and calls the Phase-4 library.

1. **Topology model.** Maintain a multi-peer graph from the existing `discovery.rs` peer directory:
   nodes = reachable hub identities, edges = live transport links (with a cost/latency weight). Feed this
   into Phase 4's CSR graph-ingestion port. This is the mesh's *live* graph; Phase 4 owns the *algorithm*.
2. **Partition detection (E33 · DSU).** Run Phase 4's DSU/union-find over the live-peer set. When the peer
   set splits into ≥2 connected components, the mesh has partitioned. DSU `c_components` is the exact
   primitive Phase 4 parity-swaps from the existing BFS `c_components` in `cgraph.rs` (byte-identical),
   so the partition-detection result is provable against a known oracle.
3. **Shortest-path re-route (E32 · Dijkstra/A\*).** When a peer drops, recompute the shortest path to
   each still-reachable peer over the remaining graph using Phase 4's Dijkstra (A\* == Dijkstra under an
   admissible heuristic — Phase 4's own acceptance test). Traffic that transited the dropped node is
   re-pointed onto the recomputed path. **No leader is elected;** each node recomputes locally.
4. **Gossip overlay (E33 · MST).** Maintain a minimum spanning tree (Kruskal, Phase 4) over the peer
   graph as the gossip/anti-entropy overlay, so revocation gossip (§4) and roster sync flood the mesh
   over a tree (no cycles, O(n) edges) rather than full mesh. When the topology changes, recompute the MST.
5. **Split-brain merge (F15/E31-HRW).** When two islands with **divergently-rooted state** reconnect,
   deterministically pick the surviving root via **Highest-Random-Weight (HRW / rendezvous hashing)** —
   the *same* HRW primitive as `matcher.rs`, applied to the *partition-merge* problem it was never wired
   for. HRW is deterministic given the same candidate set, so both sides independently compute the same
   winner and converge without a coordinator. State below the chosen root is merged via the existing
   content-addressed union (`sync_pull.rs` MerkleLog anti-entropy).
6. **Heal-time budget.** State a concrete heal-time budget (e.g. "traffic re-routes around a dropped node
   within N gossip rounds / T ms on a K-node fixture") and assert it in the test. A heal layer with no
   stated budget is unfalsifiable.

### 2.3 Acceptance criteria

- **Split-brain (F15/E31):** two mesh islands with divergently-rooted state deterministically
  converge/merge via HRW after reconnecting — *both sides pick the same root without a coordinator*.
- **Re-route (M7/E32/E33):** killing a node causes traffic to be re-routed around it via a **recomputed
  shortest path** plus a **maintained spanning tree**, within the **stated heal-time budget**.
- **DSU oracle:** partition detection's `c_components` result is byte-identical to the existing BFS
  connected-components on shared fixtures (inherited from Phase 4's acceptance).
- **No re-implementation:** a CI/grep check (or code review) confirms the heal module contains **no**
  Dijkstra/A\*/DSU/MST loop of its own — it calls Phase 4. If Phase 4's export is absent, the heal module
  does not compile (hard dependency, not a soft copy).

---

## 3. D2 TRANSPORT DECISION (D2, F20, E34, E31-iroh)

### 3.1 Current-state evidence

**D2 as written in canon is INVERTED versus reality** — R1-A headline #5, CANON-GAP.

- Canon (`ARCHITECTURE.md` §1): "zero-dep proto-cap + **iroh-QUIC primary, quinn fallback** via DECART."
- Code: `iroh` is **deliberately not a dependency** (offline build + an `ed25519-dalek 3.0.0-rc.0` pin
  conflict, `Cargo.toml:31,41-52`). **quinn 0.11.11 is the only QUIC carrier that actually runs.**
  `iroh_transport.rs` is quinn-under-the-hood (`iroh_transport.rs:1,7-15`); the "out of scope" NAT-punch
  note lives at `iroh_transport.rs:23-25` (innovate marker), and `discovery.rs` is explicitly "NOT a DHT"
  — no hole-punch/STUN/relay.
- **Therefore F20 ("Hub uses iroh for NAT punch, quinn if iroh down") is IMPOSSIBLE as literally written
  — there is no iroh to fall back FROM.** E34 (iroh-quinn NAT) and E31's iroh leg are the same gap.

### 3.2 Target — a WRITTEN DECART that executes Phase 2's O5 ruling

Phase 9 does **not** decide the direction — Phase 2 (O5) does. Phase 9 **executes** the chosen direction
as a written DECART decision document (per M6/§3 "DECART-gate new deps" and the D6 pattern). Exactly one
of two outcomes:

- **Outcome A — land `iroh` for real.** Resolve the `ed25519-dalek` pin conflict, add `iroh` as a
  real dependency, and implement NAT-punch/relay behind the `Transport` trait with quinn as the genuine
  fallback (F20 becomes true as written). This is a network-unlock-dependent action (the pin conflict +
  offline-build constraint are why it isn't already done); the DECART must record the added dep, the
  DECART bake-off, and the measured win that justifies the second dependency (F20's own CON: "2 deps").
- **Outcome B — amend the canon.** Formally amend D2/F20/E34 to **"quinn primary, no iroh, with a named
  future network-unlock trigger."** The trigger must be *checkable* (e.g. "when `cargo add iroh` succeeds
  offline-vendored AND the `ed25519-dalek` pin conflict is resolved upstream"). This makes F20 honest
  instead of impossible, and defers iroh to a real future condition rather than a standing lie in canon.

Either outcome is a real artifact: a dated `docs/adr/`-style DECART showing which way the operator went
and why. The canon edit itself (if Outcome B) is Phase 2's "merge, never append" action, relayed — Phase
9 produces the DECART that *justifies* it.

### 3.3 Acceptance criteria

- **A written D2 DECART document exists** in the roadmap/ADR tree showing **either** iroh landed for real
  (with the pin conflict resolved and F20's fallback working) **OR** a canon amendment with a **named,
  checkable network-unlock trigger**.
- If Outcome A: `iroh` appears in `Cargo.toml`, the `Transport` impl set includes a real iroh carrier,
  and killing iroh falls back to quinn in a test (F20 true as written).
- If Outcome B: canon D2/F20/E34 no longer say "iroh primary"; the trigger is a grep-checkable predicate,
  not prose.

---

## 4. REVOCATION & REPLAY (F23, E38, F25)

### 4.1 Current-state evidence

- **Revocation is named in the architecture but no propagation loop runs.** `RevocationSet` is BUILT at
  library level (`revocation.rs:49-138`) with a `gossip_payload` and a `merge` anti-entropy primitive
  (`:14-16`), and `discovery.rs` has `evict_revoked` (`:82`) — but **F23/E38 propagation is unwired**:
  there is no loop that actually gossips a new revocation to peers. A key can be revoked *locally* and no
  other node ever hears about it.
- **F25's replay-nonce ledger is in-process/volatile only.** The gate uses a per-instance
  `Mutex<HashSet>` nonce ledger with `MAX_SEEN_NONCES = 1<<20`, verify-then-record ordering
  (`hybrid_gate.rs`), explicitly **not distributed and lost on restart**. Whether that volatile bound is
  acceptable, or must be persisted, is **Phase 2's ruling O11** — Phase 9 does not decide it.

### 4.2 Target design

1. **Live revocation-gossip loop (F23/E38).** Wire a propagation loop that, on a local revocation, pushes
   the `gossip_payload` over the **MST gossip overlay from §2** (not full-mesh flood) to peers; peers
   `merge` the incoming `RevocationSet` (anti-entropy — commutative, idempotent, so ordering-agnostic)
   and `evict_revoked` from their peer directory. Within one propagation round, a revoked key reaches the
   MST-connected component. Frames signed by the revoked key are then dropped by the receiving node's
   existing fail-closed gate.
2. **F25 replay bound — execute O11, do not decide it.** Two concrete forks, one is built:
   - **O11 = bless the in-process bound:** keep the per-gate `Mutex<HashSet>` ledger; document in canon
     §8 that the replay window is *per-process, reset on restart* as an accepted bound, with the
     `MAX_SEEN_NONCES` prune policy stated. No new persistence.
   - **O11 = require persistence:** back the nonce ledger with the durable store (the at-rest store of §6
     / the Phase-12 `BlockStore`), so a restart does not re-open the replay window. The verify-then-record
     ordering is preserved; only the backing store changes.
   The design here is *conditional on the ruling* — the blueprint carries both; the builder implements one.

### 4.3 Acceptance criteria

- **Revocation propagation (F23/E38):** a revoked key is gossiped to a third node **within one
  propagation round**, and that node **subsequently rejects frames signed by the revoked key.**
- The propagation rides the §2 MST overlay (not a full-mesh flood) — verifiable by counting the gossip
  fan-out on a K-node fixture (O(n) edges, not O(n²)).
- **F25 per O11:** if blessed in-process — a restart-resets-window test passes *and* canon §8 records the
  bound; if persisted — a replay after restart is still rejected (the ledger survived the restart).

---

## 5. QRNG BEACON (M3, F13, F22)

### 5.1 Current-state evidence

**More built than canon implies, but unwired** — R1-A M3.

- `proto-cap/src/entropy.rs` (487 lines) exists: `OsEntropy` is a mandatory fail-closed floor
  (getrandom); `AnuQrng` models the ANU beacon API but is **disabled by default**, with real HTTP gated
  behind `--features anu` (off), returning `EntropyUnavailable` when off (`:107,154-157`). `SeedPool`
  mixes `SHA3-512(floor ‖ advisory-QRNG ‖ counter)` (`:285-292`). Tests already prove the floor alone is
  sufficient and that **QRNG can never replace the floor** (`:386-415,467-473`).
- **The gap:** the operator toggle is not wired into a *running node*, and there is no *live* beacon-down
  integration test. The beacon client exists feature-gated; nothing on the live path calls it, so
  M3/F13/F22 are effectively unwired.
- **Canon note (R1-A #6, M3-vs-M6 boundary):** a network-touching beacon may not live inside the zero-dep
  wire boundary. The resolution already exists in the API shape — the beacon is injected as
  *caller-supplied entropy from outside the boundary*, never a dependency of the signing path.

### 5.2 Target design

1. **Operator-facing beacon toggle.** Wire `--features anu` (and/or a runtime HubPolicy flag consumed in
   Phase 10) so an operator can enable beacon mixing on a running node. When on, `AnuQrng` bytes are mixed
   into the `SeedPool` for nonce/ephemeral-key generation (including the §1 F16 session-key derivation and
   §6 nonces) — **hardening, never replacing** the OS floor.
2. **Beacon-down graceful degrade (the critical safety property).** When the beacon is enabled but
   unreachable, the system **must degrade to ML-DSA-only / floor-only and keep running** — it must
   **NEVER fail closed on the whole system** because an *optional* entropy source is down. M3 is
   LOCK-as-optional; F13 is the explicit "QRNG down → ML-DSA-only" path. The floor is always present, so
   degrade is always possible.

### 5.3 Acceptance criteria

- Running with `--features anu` **mixes real beacon bytes into nonce generation** (verifiable: the seed
  differs from a floor-only seed given the same counter, and the beacon bytes are present in the mix).
- Running with the beacon **deliberately taken down still succeeds** using ML-DSA-only / floor-only
  fallback (M3/F13/F22) — a beacon outage never halts the node.
- The beacon client is *outside* the zero-dep signing boundary (M6) — injected as caller entropy, not a
  dependency of `proto-cap`'s signed path.

---

## 6. AT-REST ENCRYPTION (F30)

### 6.1 Current-state evidence

**BUILT but mislocated** — R1-A F30. A real at-rest layer exists in the **legacy** `crates/bebop/src/
vault.rs`: Argon2id key derivation + ML-KEM⊕X25519 hybrid + **XChaCha20-Poly1305**. It is stranded — it
protects a legacy product-node store, **not** the mesh substrate or the per-hub live DB. The live per-hub
store is unencrypted at rest.

### 6.2 Target design

Relocate/generalize the `vault.rs` XChaCha20-Poly1305 at-rest crypto into the **live per-hub store** (the
Phase-12 `BlockStore`/durable EventStore surface). Key management via EnvFile (F30's LOCK qualifier:
"EnvFile-key" — S3 secrets discipline, never in-repo). **One AEAD across the system:** the same
XChaCha20-Poly1305 used for the §1 wire session encryptor is used at rest — a single cipher, reviewed
once, with distinct keys (session key vs at-rest key) and distinct nonce domains.

> **Sequencing note:** the *live per-hub durable store itself* is a Phase-12 deliverable (E29 BlockStore,
> E28 durable EventStore). Phase 9 provides the **at-rest crypto layer** that Phase 12 wraps its store in.
> If Phase 12 has not yet landed the durable store when Phase 9 builds, F30 lands as a **ready-to-wire
> at-rest encryptor with a stated integration point**, not a store rebuild. Phase 9 owns the cipher;
> Phase 12 owns the store. This is called out explicitly so the phases don't collide on the same file.

### 6.3 Acceptance criteria

- The XChaCha20-Poly1305 at-rest encryptor is available to the live per-hub store (unwired legacy
  `vault.rs` is no longer the only home).
- The at-rest key derives from an EnvFile secret (never in-repo; gitleaks-clean per Phase 1).
- A tampered at-rest ciphertext fails the Poly1305 tag check (fail-closed, no plaintext read).
- The wire AEAD (§1) and at-rest AEAD share one reviewed implementation with separate keys and nonce
  domains (no nonce reuse across surfaces).

---

## 7. BATCHING & TRANSPORT-TRAIT EXTENSION (F18, M6-transport)

### 7.1 Current-state evidence

- **F18 frame batching exists only in a test, not the live path.** R1-A F18: NOT BUILT as a library
  primitive — only test-only manual batching in `mesh_sync_integration.rs:103-200`. A production hub
  cannot batch-then-flush; the capability is demonstrated but not shipped.
- **M6 transport-swap is real but the impl set is thin.** The `Transport` trait
  (`proto-wire/src/lib.rs:58`: `connect/accept/send/recv` over `SignedFrame`) has three impls —
  `QuicTransport` (`iroh_transport.rs:235`), `WssTransport` (`wss_transport.rs:379`), `MemTransport`
  (`bpv7.rs:332`) — and swap is a compile-time generic (`MeshNode<T: Transport>`, `node.rs:41`). The
  trait boundary is done; **stdio and HTTP transports are absent.**

### 7.2 Target design

1. **Library batching (F18).** Promote the test-only batching into a real library path: a batching
   `Transport` wrapper / policy that accumulates frames up to a tuned size/time bound then flushes (F18's
   LOCK: "+ tuned"). It sits above the carrier, batching `SignedFrame`s; each frame stays individually
   signed (no batch-level signature shortcut). Latency/throughput trade-off is a tuned parameter, not a
   hard-coded constant.
2. **Transport-Trait extension (M6).** Add **stdio** and **HTTP** transports behind the existing
   `Transport` trait, consistent with "zero protocol deps, transport swappable." stdio enables
   pipe/subprocess meshing and offline/local composition; HTTP enables edge/proxy traversal where QUIC is
   blocked. Both implement `connect/accept/send/recv` over `SignedFrame` — **the frame format and signing
   path are unchanged** (M6: the wire/trust boundary stays zero-dep; only the carrier is swapped).

### 7.3 Acceptance criteria

- F18 batching is reachable from the **live path** (not test-only): a hub batches N frames then flushes
  as a library call, tunable, with each frame independently signed.
- At least one new transport (stdio and/or HTTP) implements the `Transport` trait and passes the **same**
  integration suite the QUIC/WSS/Mem transports pass (the trait is the contract — a new carrier is a drop-in).
- Adding a transport adds **zero** dependencies to the signing path (M6 zero-dep boundary intact).

---

## 8. REGRESSION GUARD: F11 & F12 (already built — do NOT rebuild)

**F11 (wire-format rejection on disagreement) and F12 (island-mode / solo operation) are ALREADY BUILT.**
This phase refactors the transport layer heavily (new payload encryptor, new heal module, new transports,
carrier hardening) — the single largest risk is silently regressing these two working properties. They
must be **regression-guarded through the refactor, not rebuilt.**

### 8.1 What is built (evidence)

- **F11 — BUILT (strict, two layers).** Envelope: `ENVELOPE_VERSION=1`, version mismatch rejected
  fail-closed (`framing.rs:59-61`), 1 MiB cap before alloc. Frame codec: magic `b"BEBOPFRM"` + version
  `0x01`, unknown-field reject with no silent forward-compat skip (`wire_codec.rs:287-292`),
  trailing-byte reject, 200-iter hostile-byte fuzz (`:474-505`). (Known caveat: envelope version is
  unsigned/outer — a documented `innovate:` gap at `framing.rs:54-58`, not this phase's job to close.)
- **F12 — BUILT.** `bpv7.rs` BPv7-style custody store-and-forward with an exactly-once reconnect test
  (`:400`); `sync_pull.rs` MerkleLog anti-entropy pull with a convergence test (`:1032`).

### 8.2 What regresses if they are NOT guarded

- **If F11 regresses:** the new payload-encryption layer (§1) and the new stdio/HTTP transports (§7) add
  new byte paths. If the strict unknown-field / trailing-byte / version-mismatch rejection is not
  re-asserted **after** the refactor, a hub could **silently accept a malformed or version-skewed frame**
  — reintroducing exactly the wire-format ambiguity M6/F11 exists to make impossible. Confidentiality
  (§1) is worthless if the frame it wraps is parsed leniently: an attacker downgrades or smuggles fields
  under the AEAD. **F11's fuzz + reject suites must stay GREEN across every §1/§7 change.**
- **If F12 regresses:** the heal module (§2) changes how peers are tracked and how partitions are
  detected. If island-mode is broken, a hub that **loses all peers can no longer operate solo** — it
  would fail instead of entering island mode, and the exactly-once custody reconnect (the mechanism by
  which an island *rejoins*) could drop or duplicate frames on reconnect. **A partition-heal layer that
  breaks solo survival is a net regression** — the whole point of M7 healing is that nodes survive being
  cut off; if §2's partition detection makes a lone node think it is "broken" rather than "islanded," the
  phase has moved backwards. **F12's island-mode + exactly-once-reconnect tests must stay GREEN through
  the §2 heal work.**

### 8.3 Guard mechanism

- The existing F11 fuzz/reject suites and F12 custody/convergence tests are pinned as **regression gates**
  that must stay GREEN at every step of the §1/§2/§7 refactor (not just at phase exit).
- No new "lenient parse for the new transport" path is permitted — stdio/HTTP carriers reuse the **same**
  strict codec (§7).
- The §2 heal module must treat "zero reachable peers" as **island mode (F12), not failure** — asserted
  by a test that kills all peers and confirms solo operation continues.

---

## 9. CONSOLIDATED BUILD CHECKLIST

> Ordered so hard-deps land before consumers. Items 1–2 are gates on the whole phase. `[P3]`/`[P4]`/`[P2]`
> mark cross-phase preconditions. Each item is falsifiable at exit (§10).

1. **[GATE · P3]** Confirm Phase 3 has designated **ONE canonical ML-KEM-768** (FIPS-203-exact) and a
   non-forgeable hybrid trust root. Do not start §1 until true.
2. **[GATE · P4]** Confirm Phase 4 exports **Dijkstra/A\*, DSU, Kruskal/MST** from the kernel with a
   graph-ingestion port. Do not start §2 until true.
3. **[§1 · F16]** Replace `NoopPayloadEnc` (`transport_policy.rs:107-133`) with a real ML-KEM→
   XChaCha20-Poly1305 session encryptor; wire the first `encaps`/`decaps` pair into `connect`/`accept`.
4. **[§1 · F16]** Remove `insecure-tls` as the default feature; harden the carrier so the default build
   emits ciphertext and does not accept any certificate.
5. **[§1 · M6]** Derive the session key via the zero-dep canonical KDF; optionally mix QRNG (§5). No new
   dep on the signing path.
6. **[§2 · M7/E33]** Build the topology model in `mesh-node` from `discovery.rs`; run Phase-4 DSU for
   partition detection (`c_components` byte-identical to BFS oracle).
7. **[§2 · M7/E32]** Wire Phase-4 Dijkstra/A\* for shortest-path re-route on peer drop; state the
   heal-time budget.
8. **[§2 · M7/E33]** Maintain a Phase-4 Kruskal/MST gossip overlay; recompute on topology change.
9. **[§2 · F15/E31]** Implement HRW split-brain merge over the peer set; both islands pick the same root
   deterministically; merge state via existing MerkleLog union.
10. **[§3 · D2/F20/E34]** Execute Phase-2 O5 ruling as a **written DECART**: land iroh (real dep + F20
    fallback) OR amend canon to "quinn primary + named checkable network-unlock trigger."
11. **[§4 · F23/E38]** Wire the live revocation-gossip propagation loop over the §2 MST overlay; peers
    `merge` + `evict_revoked`; revoked-key frames dropped downstream.
12. **[§4 · F25 · P2 O11]** Implement the replay bound per O11: bless the in-process `Mutex<HashSet>`
    bound (+ canon §8 note) OR persist the nonce ledger (survives restart).
13. **[§5 · M3/F13/F22]** Wire the operator beacon toggle into a running node; mix beacon bytes when on;
    **degrade to floor-only when the beacon is down — never fail closed on the whole system.**
14. **[§6 · F30]** Relocate/generalize `vault.rs` XChaCha20-Poly1305 into a ready-to-wire at-rest
    encryptor for the live per-hub store (EnvFile key; one AEAD shared with §1, distinct keys/nonces).
15. **[§7 · F18]** Promote test-only batching (`mesh_sync_integration.rs:103-200`) into a tuned library
    batching path on the live wire; each frame independently signed.
16. **[§7 · M6]** Add stdio and/or HTTP transports behind the existing `Transport` trait; reuse the strict
    codec; zero new deps on the signing path.
17. **[§8 · F11 GUARD]** F11 envelope/codec fuzz + reject suites stay GREEN at every step of items 3–16.
18. **[§8 · F12 GUARD]** F12 island-mode + exactly-once custody-reconnect tests stay GREEN; "zero peers"
    means island mode, not failure.
19. **[E35 · CONDITIONAL · P2 O6]** If O6 defined the 3-tier locality model, attach it to the §2
    discovery/topology layer; if O6 struck E35, drop this item. Do not build on an undefined anchor.
20. **[EXIT]** All §10 falsifiable done-tests GREEN; DECART (item 10) merged; canon §8 updated with the
    F25/O11 bound and any D2 amendment relayed to Phase 2's "merge, never append."

---

## 10. FALSIFIABLE DONE-TESTS (phase exit gate)

Phase 9 is done when **all** of the following pass — these are the R2 done-tests, expanded:

1. **[HEADLINE · F16]** Two nodes that have JUST met derive a working ML-KEM session key, and a passive
   on-path packet capture shows **CIPHERTEXT, not plaintext**.
2. **[M3/F13/F22]** `--features anu` mixes real beacon bytes into nonce generation; with the beacon
   **deliberately taken down**, the node still succeeds using ML-DSA-only / floor-only fallback.
3. **[F15/E31]** Two mesh islands with divergently-rooted state **deterministically converge/merge via
   HRW** after reconnecting (both sides pick the same root, no coordinator).
4. **[F23/E38]** A revoked key is gossiped to a third node **within one propagation round**, and that node
   **subsequently rejects frames signed by the revoked key.**
5. **[M7/E32/E33]** Killing a node causes traffic to be **re-routed around it** via a recomputed shortest
   path plus a maintained spanning tree, **within the stated heal-time budget** — consuming Phase 4's math
   (no re-implemented graph algorithm in `mesh-node`).
6. **[D2/F20/E34]** A **written D2 DECART** exists showing **either** iroh landed for real **OR** a canon
   amendment with a **named, checkable network-unlock trigger.**
7. **[F25 · O11]** The replay bound is implemented per Phase-2's O11 ruling (in-process-blessed with a
   canon note, or persisted and surviving a restart).
8. **[F30]** The at-rest XChaCha20-Poly1305 encryptor is wired from the stranded `vault.rs` into a
   ready-to-use per-hub at-rest layer; a tampered ciphertext fails the tag check.
9. **[F18]** Batching is a live library path (not test-only), tuned, with per-frame signing intact.
10. **[M6]** A new stdio/HTTP transport passes the same integration suite as QUIC/WSS/Mem with zero new
    deps on the signing path.
11. **[F11/F12 GUARD]** The pre-existing F11 fuzz/reject and F12 island/custody suites are **still GREEN**
    — proven un-regressed through the entire refactor.

---

## 11. Cross-references & relays

- **Consumes Phase 4** for all graph math (E32/E33) — one implementation, two consumers (this phase's
  mesh overlay + Phase 13's product route F45/F46). Do not fork the math.
- **Consumes Phase 3** for the canonical ML-KEM (§1) and the non-forgeable hybrid trust root that F16's
  self-certified handshake rides on.
- **Consumes Phase 2 rulings:** O5 (D2/iroh, §3), O6 (E35 locality, conditional item 19), O11 (F25 bound,
  §4). None are decided here.
- **Feeds Phase 10** (hub runtime): the operator kill-verb (F28) uses **this phase's wire frame format**;
  the revocation-gossip loop (§4) is F5's missing half; the beacon toggle (§5) is consumed by HubPolicy.
- **Feeds Phase 13** (delivery on protocol): the confidential, self-healing wire is the substrate dowiz
  first rides. Wiring the product onto a plaintext, non-healing wire (the pre-Phase-9 state) would bake
  insecure, partition-fragile assumptions into the product — which is exactly why P9 precedes P13.
- **Relay to operator (Phase 2 "merge, never append"):** the D2 DECART (§3) and the F25/O11 canon §8 note
  (§4) are canon edits Phase 9 *justifies* but Phase 2 *lands*.

---

## 12. Planning-protocol completion appendix (2026-07-17, decorrelated pass)

Per the Detailed Planning Protocol (`AGENTS.md`) and the Anu/Ananke doctrine, applied as a decorrelated
audit pass. This blueprint already carries strong (a) evidence and DECART-shaped framing (§3); this
appendix supplies the missing (c) 2Q doubt audit and (d) Anu/Ananke check, plus fresh citation
verification.

### 12.1 — Citation verification against live repo (bebop-repo HEAD `397b8cd8`, dowiz current session)

Re-verified a representative, load-bearing sample of this blueprint's ~30 file:line citations. `bebop-
repo` has advanced 8 commits past the cited baseline `b87b7e2` (confirmed ancestor via `git merge-base
--is-ancestor`), 4 of which touch cited files (`iroh_transport.rs` +64 lines via later breach-domain-
separation work; `proto-cap/src/redline.rs` gained Auth/Secret/Migration mapping — not cited by P09, no
effect here).

**One material stale claim, corrected:**
- §1.1's third bullet — *"The WSS carrier runs plaintext `ws://`, not `wss://`"*, citing
  `wss_transport.rs:451-458` as "readable by a passive on-path observer" — **is false as production
  behavior today.** `wss_transport.rs:403-413` (`connect`) and `:465-479` (`accept`) implement **MESH-10**
  (commit `85fcee2`, predating even this blueprint's own citation baseline): production **rejects**
  plaintext `ws://`/`Listen` with `WireError::InsecureTransport` unless the test-only `insecure-test`
  feature is enabled, and `wss://` completes a **real client/server rustls TLS1.3 handshake** ("a real
  `wss://` connection now completes end-to-end"). The cited lines 451-458 are a **stale, orphaned doc-
  comment** (`innovate: H6`) never deleted when MESH-10 shipped — the evidence chain took a leftover
  comment at face value instead of the code five lines above it. **Corrected framing:** the wire IS
  confidential-in-transit today via classical TLS1.3; the real gap is specifically **post-quantum**
  confidentiality (harvest-now-decrypt-later resistance), not "every frame readable in plaintext." F16's
  headline verdict (NOT BUILT — zero `encaps`/`decaps` on the wire, re-confirmed fresh via `grep`) is
  unaffected; only the supporting threat-model bullet needed correction.
- Minor line drift only (content unaffected): `Cargo.toml:38` (`default = ["insecure-tls"]`) is now at
  line **50**; `iroh_transport.rs:149-155` (`InsecureAcceptAny`) is now at **~156-197**;
  `crates/bebop/src/cost_estimate.rs:205-290` → `pub fn route(` is at **209** (matches
  `SELF-CRITIQUE-2Q-DOUBT-AUDIT.md` §1.4's independent finding that this exact range is "loose but not
  wrong" across two other citing documents too).
- Confirmed accurate, unchanged: `iroh_transport.rs:23` (NAT-punch out-of-scope marker, exact line);
  `transport_policy.rs:107-133` (`NoopPayloadEnc`); `dowiz/kernel/src/order_machine.rs:611-656`
  (union-find inside `cyclomatic_number()`); `crates/bebop/src/vault.rs` (XChaCha20-Poly1305 + Argon2id +
  ML-KEM-768⊕X25519, exact match); `discovery.rs:82` (`evict_revoked`, exact line match).
- **Fresh (not merely trusted) re-verification of two headline claims, run live this pass:**
  `grep -rliE "dijkstra|union.find|kruskal|\bmst\b" bebop2/ --include="*.rs"` → **zero** hits (M7 heal
  layer confirmed still 0% at current HEAD, independent of R1-B's original grep). `grep -n "iroh"
  ARCHITECTURE.md` → line 33 still reads *"iroh-QUIC primary, quinn fallback via DECART"* (D2-vs-reality
  inversion confirmed still live in canon, unamended). **Also freshly checked: Phase 4's kernel
  graph-math exports (Dijkstra/A\*/DSU/MST) do NOT yet exist in `dowiz/kernel/src/*.rs`**
  (`grep "fn dijkstra\|struct.*DSU\|pub fn mst"` → zero) — the blueprint's own Wave-0 gate #2 ("do not
  start §2 until Phase 4 exports these") is, as of this pass, **still red**. This is the load-bearing
  "depends on" claim spot-verified per the task instruction: real and currently unmet, not decorative.

### 12.2 — DECART

**No DECART owed.** Every concrete mechanism this blueprint commits to text is a reuse of an
already-existing primitive or already-canonical choice: ML-KEM-768 (Phase 3's canonical KEM),
XChaCha20-Poly1305 (already in `vault.rs`), HRW/rendezvous hashing (already in `matcher.rs`),
Dijkstra/A\*/DSU/MST (Phase 4's library, consumed not invented), SHA3/SHAKE KDF (`bebop2-core`,
zero-dep). The one real choice — D2's iroh-vs-quinn direction (§3) — is explicitly **not decided by
this blueprint**; it is Phase 2's ruling (O5), which Phase 9 only *executes* as a written DECART
artifact at build time. That is correct posture: pre-empting an un-ruled operator decision would itself
be an Anu violation.

One **under-specified future choice flagged for build-time, not decided here**: §7's stdio/HTTP
transport additions do not name a concrete implementation. stdio is trivially std-only; HTTP is not —
if the builder reaches for a crate beyond the already-present `http` (types-only) dependency, **that
pick owes its own DECART at that time**, named here so it isn't silently skipped.

### 12.3 — 2-question doubt audit (per-blueprint)

**Q1 — least confident about, concrete:**
1. I spot-checked ~12 of ~30 file:line citations, not all — the F23/E38 revocation-gossip, F25
   replay-persistence, and F18 batching citations were checked only for the cited struct/fn *existing*,
   not diffed line-by-range the way the wss_transport.rs claim that turned out stale was diffed. More
   comments-left-behind-after-a-fix could exist in the 3 commits (`c4edbf1`, `4f3553f`, `f9c14ea`) I did
   not open in full diff.
2. The wss_transport.rs staleness calls into question how the *original* R1-A gap analysis was produced
   — if it trusted a stale comment once, the same failure mode could recur elsewhere in R1-A/R1-B's
   other 19-anchor claims for this phase; I did not re-audit all of them for the identical pattern.
3. I confirmed Phase 4's kernel exports are absent today, but did not check whether Phase 4's own
   blueprint (a sibling document, not read this pass) specifies the exact export shape (`csr.rs`
   graph-ingestion port) P09 §2.2 assumes — a shape mismatch would break the "hard dependency, not a
   soft copy" compile-fail guarantee in §2.3.
4. §6's "one AEAD across the system... distinct nonce domains" is asserted as a design property; I did
   not verify a concrete nonce-domain-separation mechanism exists anywhere in the cited code today.
5. The E35/O6-conditional item (§9.19) — I did not check whether O6 has since been ruled; if so, this
   blueprint's "conditional" framing may already be resolvable more concretely.
6. I did not check whether dowiz's own recent native-Rust-port commits (`cc3d5c916`, `4519bd7ff`) touch
   Phase 8's telemetry sink that §5/§2 cross-reference — if that shape changed, P09's telemetry pointers
   may need refreshing.

**Q2 — biggest thing this pass might be missing:** the wss_transport.rs finding (§12.1) is a genuine
instance of the exact failure mode this protocol exists to catch — a stale artifact trusted at face
value. I found it only because I happened to open that exact function for an unrelated reason (verifying
MESH-10 via the Cargo feature it gates), not via a systematic staleness sweep. A blueprint this size (20
anchors, ~30 citations) genuinely needs either a scripted citation-freshness check or acceptance that
some fraction of its evidence will drift silently between planning and build — the same "no re-audit
cadence" gap `HERMETIC-REMEDIATION-PLAN.md` §6.Q2 already named as this whole roadmap's structural weak
point.

### 12.4 — Anu (logic) & Ananke (organization) check

**Anu.** The phase's central claims are derivable, re-checked fresh in §12.1: M7 = 0% built (re-grepped,
holds), D2 canon-inversion (re-grepped `ARCHITECTURE.md`, holds), Phase-4 dependency unmet (re-grepped
`kernel/`, holds). Where the blueprint cannot derive a decision, it defers correctly (D2/O5 §3, F25/O11
§4.2, E35/O6 item 19) rather than asserting past its evidence. The one Anu violation found was in the
**inherited source evidence, not this blueprint's own reasoning**: R1-A asserted "readable by a passive
on-path observer" from a comment the code five lines above it already contradicted — an evidence-
gathering failure this blueprint inherited, now corrected in §12.1.

**Ananke.** The numbered acceptance criteria (§10) are genuinely falsifiable — a packet-capture
ciphertext check, a `grep encaps|decaps` non-zero check, an HRW-convergence assertion, all checkable
cold. §8's regression-guard is a strong Ananke instance: it names the exact existing test suites (F11
fuzz/reject, F12 island/custody) that must stay green *at every step*, not just at exit. **What does not
survive on structure alone:** the cross-phase gates (§9 items 1-2) are *stated* as hard gates but nothing
mechanically blocks work from starting early — a builder who skips the checklist line would only be
caught by the compile-fail-if-absent property in §2.3 criterion 4, a real but *late* backstop (fails at
build time, not planning-gate time). Recorded as a known, bounded gap rather than silently assumed away.

---

*Blueprint P09 complete. 20 anchors (M3, M6, M7, D2, E31, E32, E33, E34, E38, F11, F12, F13, F15, F16,
F18, F20, F22, F23, F25, F30) each mapped to a sub-section with current-state evidence, target design,
and acceptance criteria; F11/F12 explicitly regression-guarded. Sources: `R1-A-mesh-crypto-gap-analysis.md`
§Phase-B + per-anchor table, `R1-B-hub-autonomy-agent-infra-gap-analysis.md` §M7, `R2-MERGED-PHASE-ROADMAP.md`
row P9 + §1 major-merges + §4 operator decisions, `ARCHITECTURE.md` §0/§1/§6/§8. All file:line citations
inherited from R1 (bebop-repo tip `b87b7e2`) — re-verify against the live tree at build time. This document
plans; it writes no code.*

> **Cross-link (added 2026-07-17, Layer-I consolidation, L2):** the transport bake-off *rationale*
> behind this phase's BPv7/`Transport`-trait design — DTN/dtn7-rs vs QUIC/TCPCLv4 vs Zenoh vs
> Reticulum vs SpaceWire/SpaceFibre, with **libp2p explicitly rejected** and BIBE custody
> verification — is `docs/transport-research-2026-07-12.md` (restored from git blob `94e257fe9`).
> The decision is already embodied in code; that doc is where the comparison lives.
