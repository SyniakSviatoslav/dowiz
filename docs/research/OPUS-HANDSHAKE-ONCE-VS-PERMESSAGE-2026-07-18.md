# Handshake-once + cheap-symmetric-after vs. per-message hybrid signing — is bebop2's mesh spending asymmetric crypto it could amortize?

> Research-only. Zero code written, no branches touched, no files modified except this
> doc. Every claim is grounded in a `file:line` read of the live tree at HEAD
> (`/root/bebop-repo/bebop2`, `/root/dowiz/docs`) on 2026-07-18, or a cited red-team
> artifact. Companion to `OPUS-TRUST-BOUNDARY-CLOSED-CHANNEL-SCAN-2026-07-18.md`.

---

## 0. The operator's question, taken seriously

> *"Do the expensive asymmetric handshake ONCE at connection time (hybrid_gate's
> Ed25519⊕ML-DSA), establish an authenticated session, then protect every message
> after with cheap symmetric MAC + sequence counters — the QUIC+TLS1.3 / WireGuard-Noise
> / Signal-X3DH pattern (expensive handshake once, cheap AEAD per-message after)."*

This is a real, standard, correct production pattern **for a live, stateful, point-to-point
session between two continuously-online endpoints.** The finding of this report: the
instinct is right that bebop2 pays per-message asymmetric cost, and there is a **legitimate,
narrowly-scoped fast-path improvement** for one specific traffic class — but the pattern
**cannot replace** the per-message model in general, because bebop2's mesh is deliberately a
**delay-tolerant, store-and-forward, relayed, gossip/anti-entropy** network where the
authentication must survive being *detached from any channel*. The reasons are concrete and
load-bearing, not hand-wavy. Details below.

---

## 1. What actually happens per-message today (cited)

### 1a. QUIC's TLS 1.3 handshake authenticates *nothing about identity* here

`iroh_transport.rs` builds a real `quinn`/`rustls` QUIC session, but:

- **Server cert is throwaway self-signed:** `server_crypto()` calls
  `rcgen::generate_simple_self_signed(vec!["localhost"])` (`iroh_transport.rs:128`) and
  `with_no_client_auth()` (`:132`) — the client is **never** authenticated by TLS.
- **Client accepts ANY server cert** in the default build: `InsecureAcceptAny`
  (`:156-197`) returns `ServerCertVerified::assertion()` unconditionally, wired via the
  default `insecure-tls` feature (`client_rustls_config`, `:216-224`). Hardened mode
  (`:225-234`) checks Mozilla webpki roots — but that authenticates a DNS name to a CA,
  **not** the node's bebop2 hybrid identity, and still never authenticates the client.
- The code says so explicitly, twice: *"Wire authenticity comes from the signed-frame
  hybrid gate, NOT from x509"* (`:151-154`), *"the real auth boundary is the signed-frame
  envelope verified on every recv"* (`:194-196`, `:199-207`).

**Consequence:** QUIC's TLS 1.3 handshake here gives you a **confidential, integrity-protected
byte pipe to *some* endpoint** — i.e. the cheap per-record symmetric AEAD *already exists at
the transport layer* — but it says **nothing about who** or **what they're authorized to do.**
So the per-message hybrid signing is **not redundant with the QUIC handshake**: they do
disjoint jobs. TLS = confidentiality; hybrid gate = identity + authorization + PQ + non-repudiation.

### 1b. The hybrid gate re-verifies FULL asymmetric crypto on EVERY frame

`recv()` calls `self.gate.check(...)` for every decoded frame (`iroh_transport.rs:395-401`;
identically `wss_transport.rs:612-621`). Inside `HybridGate::check` (`hybrid_gate.rs:124-209`),
per frame:

1. freshness `is_fresh(now)` — cheap (`:134`);
2. **`verify_chain(roster, chain, cap, now)`** — walks the UCAN-subset delegation chain, each
   link an **Ed25519 verify** (`:142`);
3. red-line scope check — cheap enum compare (`:150-154`);
4. revocation lookups — cheap hash-set (`:159-168`);
5. **`verify_classical()`** — the frame's own **Ed25519 verify** (`:171`);
6. **`verify_pq()`** — a real **ML-DSA-65 (FIPS-204) verify** under `RequireBoth`
   (`:181`), the heaviest single op (~kilobyte signature, µs-scale but far above a symmetric MAC);
7. verify-then-record nonce insert (`:193-206`).

So each application message costs **≥2 Ed25519 verifies + 1 ML-DSA-65 verify** (more if the
delegation chain carries hybrid links). **The operator's premise that this is real,
avoidable per-message asymmetric cost is correct** — for a high-frequency stream of same-scope
frames on one live connection, steps 2/5/6 repeat identical work every message.

### 1c. Both carriers now run the *strong* policy, per-message

`RequireBoth` on QUIC connect+accept (`iroh_transport.rs:275,339`) and on WSS connect+accept
(`wss_transport.rs:446,528`), each with a **real wall-clock `now`** (`iroh:391-394`,
`wss:612-615`). This matters for §2.

---

## 2. The B2 red-team attacks — where in the lifecycle they land, and reconciliation with the *current* tree

**Critical reconciliation first.** `B2-protocol-authz.md` is dated **2026-07-13** and its PoC
targets `wss_transport.rs` **as it was then**: `gate.check(&frame, 0)` with a hardcoded
`now=0`, `ClassicalUntilPqAudit` policy, and `verify_chain` **never called** on the acceptance
path (`B2 §1`, `§2` rows 1-5,7). **The live tree at HEAD has fixed exactly these:**
`verify_chain` is wired into `HybridGate::check` (`hybrid_gate.rs:142`); the clock is real on
both carriers (`iroh:391`, `wss:612`); both carriers use `RequireBoth` not
`ClassicalUntilPqAudit`; verify-then-record ordering closes H2 (`hybrid_gate.rs:188-206`).
So B2 is a **point-in-time** artifact; I read it for the *attack mechanics and where they
land in the lifecycle*, which is what the operator's question turns on.

| B2 finding | Where in the connection lifecycle it lands | Would "handshake-once + cheap-MAC" have caught it? |
|---|---|---|
| **C1 self-issued cap accepted** (`§3 C1`, PoC NODE1 `Ok(())`) | **At/around the per-frame *authorization* check**, not the handshake and not a post-handshake session-riding bug. Attacker just opens a connection (accept-any TLS lets *anyone* connect) and sends a frame validly signed **under its own key** with **no anchor-rooted chain**. | **Only if the handshake runs the same `verify_chain`.** Moving the check to handshake-time = *relocating* the delegation-chain walk, not removing it. It catches C1 **iff** the session key is then **channel-bound to the verified hybrid identity** (else an attacker rides a legit session). It is not "cheaper" — it's the same asymmetric check done once instead of N times. |
| **C2 scope carried but never enforced** (`§3 C2`) — a `Presence/Send` cap authorizes a `ledger.append` drain | **Per-frame *effect-binding* gap.** The gate never compared the frame's requested effect to the authorized scope. | **Partially, and this is the key caveat.** A handshake that authenticates "peer holds capability C" **still must check every message's requested effect ⊆ C.** If the fast-path naively trusts "authenticated session ⇒ any effect," it **re-opens C2**. The per-frame **scope⊆cap** check (a cheap integer/enum compare, *no crypto*) must survive into any session model. |
| **C3 cross-instance / cross-node replay** (`§3 C3`, PoC NODE2 accepts replayed bytes) | **Per-frame *freshness/replay* gap that spans nodes** — the same bytes replay to a *different* node. | **No — and this is where the operator's own cheap-counter idea is subtly insufficient.** A per-session monotonic sequence counter defeats replay **within one session**, but C3 is replay to a *second node*. Cross-node replay needs **mesh-scoped nonce tracking** (the live fix direction: `MESH-REAL-PLAN §Layer D`, "HybridGate.seen node-scoped-persistent"). A session counter is strictly weaker: it cannot see a replay that leaves the session. |
| **H1 expiry defeated (`now=0`)** (`§3 H1`) | Per-frame freshness bug (now fixed, real clock). | Orthogonal. A long-lived session must still bound its lifetime to the cap's `expiry` and re-check freshness. Cheap. |
| **H3 PQ-leg strip rewarded** (`§3 H3`) | Per-frame crypto-policy bug (deployed `ClassicalUntilPqAudit`; now `RequireBoth`). | **Handshake-once is fine PQ-wise *iff the handshake uses a PQ KEM (ML-KEM).*** Symmetric AEAD after is quantum-safe (Grover, not Shor); but an ECDH session key is Shor-broken. So a PQ session **must** do ML-KEM at handshake — which is exactly `MESH-REAL-PLAN §Layer C` ("ML-KEM-768→XChaCha20-Poly1305 payload-encryption"). |

**Summary of §2:** every demonstrated B2 attack lands **at or around the per-frame
authorization/freshness gate** — none is a "legitimate handshake, then attacker rides the
session" attack (there *was* no cryptographic session to ride; the `Handshake` struct exists
but the real handshake auth event is the per-frame signature, `handshake.rs:1-12,33-47`). So
"do the handshake once" would help C1/H1/H3 **only by relocating the same checks to handshake
time**, would help C2 **only if the cheap scope check is kept per-frame**, and would **not**
help C3's cross-node replay at all.

---

## 3. Can the relay interpose during the handshake? (Is P2P-bypass even coherent?)

**Today, in the shipped quinn carrier: there is no relay at all.** `iroh_transport.rs:23-25`
explicitly: *"iroh DHT hole-punching is OUT of scope here (quinn gives direct QUIC; NAT
traversal is a deployment concern)."* The QUIC carrier dials the peer directly
(`connect` → `endpoint.connect(remote, ...)`, `:256-264`).

**In the planned iroh deployment** (`MESH-REAL-PLAN §Layer C`, `BLUEPRINTS-MESH-REAL:102`:
"iroh 1.0 … ~90%-hole-punch+relay, dial-by-pubkey"), the DERP-style relay is **unavoidably
on-path for NAT traversal** whenever hole-punching fails — which for carrier-grade-NAT'd
courier phones is a large fraction of connections. In iroh's model the relay forwards the
**end-to-end-encrypted QUIC packets** (including the handshake packets), so it is a **genuine
on-path position** (can drop/delay/observe timing/size) but sees only ciphertext and cannot
forge app-layer frames. **Two caveats sharpen the picture:**

- Because the codebase's TLS is **accept-any in the default build** (`§1a`), a *malicious*
  relay that actively substitutes its own cert **is accepted** — i.e. the relay *can* be a
  real MITM for **confidentiality** in dev mode. It still cannot forge signed frames (no
  hybrid keys). This is **precisely why** the design (a) labels the relay **"semi-trusted"**
  and (b) adds **ML-KEM-768→XChaCha20 payload encryption as "defense-in-depth
  past-semi-trusted-relay"** (`MESH-REAL-PLAN:91`, `BLUEPRINTS-MESH-REAL:121`), and (c) roots
  authenticity in the **per-frame hybrid signature, not the TLS channel.**
- Therefore **"authenticate once, direct P2P, bypass the relay for the handshake" is not
  generally a coherent option**: the relay exists *because* direct P2P is often impossible
  (that is its job — connection brokering / NAT traversal). You cannot bypass for the
  handshake the very node you need *to accomplish* the handshake.

The design's response to an on-path semi-trusted relay is the correct one: **do not trust the
channel; authenticate the payload.** That argues **for** per-frame authentication, not against it.

---

## 4. Store-and-forward is a real, load-bearing requirement — and it *kills* the pure-session model for that traffic

This is the decisive constraint. The mesh has **built, tested store-and-forward** and an
**explicit offline-first write model**:

- **`bpv7.rs` — a hand-rolled BPv7 (RFC 9171 DTN) store-and-forward overlay.** Its own header:
  *"A courier (the relay node that physically carries bundles between partitions) MUST deliver
  every bundle exactly once even when the radio drops mid-transfer. The `StoreForward` queue
  survives reconnects: on a fresh channel it drains the still-undelivered bundles oldest-first"*
  (`bpv7.rs:11-18`). The RED property is named `offline_courier_reconnect_delivers_exactly_once`
  (`:9`). Custody is keyed by `Capability.nonce`; the receiver dedupes by `PrimaryBlock.nonce`
  (`:20-26,62-64`).
- **Offline-first writes** (`MESH-REAL-PLAN §Layer E`): *"Local-writes→kernel::decide→commit-
  event-log BEFORE-network (offline = «sync-hasn't-run» never-degraded-write)"*; sync is
  **pull-anti-entropy** where a peer folds in frames after its last `actor_seq`, dup = no-op;
  long-offline catch-up via a Merkle/prolly-tree digest.
- **Reconnect drain** (`MESH-REAL-PLAN:92`): *"offline-reconnect=persist-undelivered-queue
  drain-oldest-first FRESH-channel-binding-each-replay."*

**Why this forbids the pure-session model for that traffic:** a frame authored by node A while
offline may be handed to a **third-party courier C** (a "data mule") that carries it across a
partition and delivers it to B **days later, over a channel where A was never present.**
There was **never an A↔B handshake** to hang a session MAC on; a symmetric session key between
A and B is not merely expensive to establish — it is **impossible**, because A and B are never
simultaneously online. The **per-frame asymmetric signature is the only thing that lets B
verify A's authorship and A's delegated authority** without any live channel to A. The same is
true of gossip/anti-entropy sync: a node validates a content-addressed frame it received from a
peer that is **not** the frame's origin. **Authentication must travel *with* the frame, not
with the channel** — which is exactly what per-message signing gives and what a
handshake-bound session MAC structurally cannot.

Two more per-frame-only cases confirm the model is deliberate:

- **Breach alarm self-signed fail-safe** (`iroh_transport.rs:366-389`): a `BreachAlarm/Broadcast`
  frame **bypasses the roster/session entirely** and is admitted on the strength of its own
  hybrid signature alone — a P2P forge-proof alarm that must work with **no session and no
  shared trust** (tested `quic_p2p_breach_no_hub_no_roster`, `:606`).
- **Delegated, transferable, non-repudiable authority:** the UCAN-subset delegation chain proves
  *an anchor authorized this key for this scope*, third-party-verifiable. A symmetric MAC (shared
  secret) is **forgeable by either holder** and carries **no transferable authority** — it can
  never replace a signature here (this is the §1 table conclusion of the companion scan).

---

## 5. Honest verdict

**5a. The current per-message hybrid signing is NOT redundant and NOT a mistake for the mesh's
real workload.** QUIC's TLS handshake authenticates *no identity* here (`§1a`); the app-layer
signing does identity+authorization+PQ+non-repudiation, and it is deliberately
**connection-independent** so it survives store-and-forward (`bpv7.rs`), gossip/anti-entropy
sync (`Layer E`), semi-trusted relaying (`:91`), and offline authorship. For **any frame that
crosses a store-and-forward boundary, is gossiped/synced, is a breach alarm, or must carry
delegated non-repudiable authority, per-frame signing is load-bearing and correct.** The B2
attacks were authorization/wiring bugs at the per-frame gate — now fixed — and "handshake-once"
would have caught them only by *relocating the same asymmetric checks*, would re-open C2 unless
the cheap scope check is kept per-frame, and would not have caught C3's cross-node replay at all.

**5b. There IS a legitimate, narrowly-scoped improvement — a live-session fast-path, layered
UNDER the existing per-frame model, not replacing it.** For the specific traffic class of
**both-endpoints-continuously-online, high-frequency, same-scope streams** (the clearest real
example: a courier phone streaming `Presence`/position updates to a hub over a stable QUIC
connection), the current code re-runs `verify_chain` + Ed25519 + ML-DSA-65 on *every* frame
(`§1b`) — genuinely avoidable. The QUIC/WireGuard/Signal pattern maps cleanly:

1. **Once at session establishment:** run the full hybrid verify + `verify_chain` (proving the
   peer holds capability `C` rooted in an enrolled anchor) — the same asymmetric work, done once.
2. **Bind a session key to the *verified hybrid identity*** via a **real RFC-5705 TLS/QUIC
   exporter** channel binding, and use a **PQ KEM (ML-KEM-768)** so the session key is
   quantum-safe (`Layer C` already specifies ML-KEM-768).
3. **Per subsequent frame:** symmetric AEAD/MAC + a **monotonic per-session sequence counter**
   (cheap) — **but keep the two cheap non-crypto per-frame checks: `effect ⊆ C.scope` (else
   re-open C2) and freshness ≤ `C.expiry` (else re-open H1).**

**Hard boundaries on that fast-path (why it's a subset optimization, not a replacement):**

- It applies **only** to frames that never leave the live session. Any frame destined for
  store-and-forward, gossip, sync, or breach-broadcast **must** keep its full per-frame
  signature (`§4`).
- The per-session counter does **not** substitute for **mesh-scoped nonce tracking** for
  frames that leave the session (C3 is cross-node; `§2`).
- **Prerequisite gap:** the safe version requires the **real TLS/QUIC exporter binding**, which
  is currently a documented *plan* item, not wired — the channel-binding *primitive*
  (`handshake.rs:29`, `channel_binding_hash` = SHA3-256 of transcript) and `sign_frame_bound`
  exist and are tested, but only against a **simulated** transcript literal
  (`wss_transport.rs:1319-1336`), not the live TLS exporter (`Layer C`: "channel_binding з
  real-TLS/QUIC-exporter (RFC5705)"). **Binding the session key to anything less than the
  authenticated handshake bytes is a MITM hole** (the ponytail note at `handshake.rs:26-28`
  says exactly this).

**5c. Net.** The operator's pattern is correct *and already half-present* — QUIC's record layer
is the cheap symmetric per-message AEAD, done once-after-handshake. What it deliberately does
**not** cover — identity, authorization, PQ, non-repudiation, and *detachability from the
channel* — is what the per-frame hybrid signature provides, and the mesh's store-and-forward /
gossip / semi-trusted-relay reality **requires** that detachability. The right move is **not**
to replace per-message signing, but to **add a live-session fast-path for the online same-scope
hot streams** (verify-once + channel-bound PQ session key + symmetric MAC + counter, keeping the
cheap scope/expiry checks), while leaving every store-and-forwardable / gossiped / delegated /
breach frame on the full per-frame signature. Sequence-order the prerequisite: **wire the real
RFC-5705 exporter binding first** — without it, the fast-path is a MITM downgrade, not an
optimization.

---

*Sources — live tree (`file:line`) cited inline: `iroh_transport.rs`, `hybrid_gate.rs`,
`wss_transport.rs`, `bpv7.rs`, `handshake.rs`; docs `MESH-REAL-PLAN.md`,
`BLUEPRINTS-MESH-REAL.md`; red-team `B2-protocol-authz.md` (2026-07-13, reconciled against
HEAD). Companion: `OPUS-TRUST-BOUNDARY-CLOSED-CHANNEL-SCAN-2026-07-18.md`.*
