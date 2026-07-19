# BLUEPRINT P92 — Mesh hot-stream fast-path (verify-once + channel-bound PQ session MAC) (2026-07-18)

> **Standalone PROTOCOL blueprint (bebop2 `proto-wire`/`proto-cap`).** One coherent, independently
> buildable unit against the 20-point contract in `CORE-ROADMAP-STANDARD-2026-07-17.md` §2. Research
> source: `docs/research/OPUS-HANDSHAKE-ONCE-VS-PERMESSAGE-2026-07-18.md` §5b (the one legitimate
> narrowly-scoped optimisation the handshake-vs-per-message study found). Format precedent:
> `BLUEPRINT-P59-capability-cert-chain.md`. Grounding tree: `/root/bebop-repo/bebop2` at HEAD,
> read live this pass.
>
> **One sentence:** for the single traffic class of *both-endpoints-continuously-online, same-scope,
> high-frequency live streams* (courier↔hub `Presence` pings), verify the hybrid identity **once**
> at session establishment, then protect subsequent presence frames with a **channel-bound,
> PQ-agreed symmetric AEAD + per-direction counter** — while **every** store-and-forwardable,
> gossiped, delegated, control, or breach frame keeps its full per-frame Ed25519⊕ML-DSA-65 signature.

---

## VERDICT (stated up front, per session research discipline)

**GO-WITH-CONDITIONS — and measure-first.** The security design is sound and *reduces* blast radius
versus the status quo (§7.3), but it is worth building **only** if two things hold, and it is a
**NO-GO** if either fails:

1. **Hard prerequisite C1 — the real RFC-5705/9266 exporter must be wired first, as its own landed
   unit.** Today the channel binding is a *simulated literal* (`wss_transport.rs:1324`
   `b"channel-A-handshake-transcript"`; `bpv7.rs:460` `Some([0xAAu8; 32])`) and the receiver **never
   compares** a frame's binding against the live channel (red-team F3/M1, `B3-wire-transport.md:57`,
   **STILL OPEN**). Shipping the fast-path on a simulated binding is a **MITM/relay session-splice
   downgrade**, not an optimisation (§6, §7.4). C1 is independently valuable (it also closes F3/M1
   for the *full-signed* path), so land + independently-review it **before** any fast-path code.

2. **Measure-first gate (NO-GO if it fails).** The fast-path saves ≈`2×Ed25519 + 1×ML-DSA-65` verify
   per presence ping **on continuously-online same-scope streams only** — nothing for the mesh's
   dominant delay-tolerant/gossip traffic (§4/§10). If the measured live presence-ping rate on real
   hardware does not clear `FASTPATH_BENEFIT_THRESHOLD` (§10.3), **do not build it** — the per-message
   model is already correct and simpler. This blueprint is a *targeted* optimisation, not a general win.

Given C1 lands and the volume clears the bar, conditions **C2–C5** (§7 findings, folded into the DoD
§9) make the design safe: scope-locked to `Presence/Send` by a compile-time allow-list + a distinct
`FrameKind` + a separate ephemeral sink (defeats scope-creep, §7.2); ephemeral **ML-KEM-768** per
session + hybrid-signed offer/accept (PQ-safe, §7.3); bounded lifetime ≤ `300 s` + revocation re-check
≤ `30 s` + merge-triggered teardown (bounds stale-trust, §5.6); and the **mandatory independent
adversarial-review gate** (§8, the B4/SSR-2020 lesson). The one honestly-stated residual is a
**≤ 30 s stale-trust window on presence-only, non-authoritative data** after a mid-session revocation
(§5.6) — acceptable because presence pings carry no money/authority/non-repudiation and are discarded
on any dispute.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

> "Ground truth is non-discussible." Every claim below was read from source **this pass**
> (`/root/bebop-repo/bebop2`, HEAD), not inherited from the research sketch. Two corrections to the
> research doc's shorthand are made here because a correct blueprint requires them.

### 0.1 What the per-frame gate actually costs (the avoidable work)

`HybridGate::check` (`proto-cap/src/hybrid_gate.rs:124-209`) runs, **per frame**:

| Step | Cite | Cost |
|---|---|---|
| `capability.is_fresh(now)` | `:134` | cheap (integer compare) |
| `verify_chain(roster, chain, cap, now)` | `:142` | **≥1 Ed25519 verify per delegation link** (`roster.rs` walk) |
| red-line gate (armed) | `:150-154` | cheap enum compare |
| revocation lookups | `:159-168` | cheap hash-set |
| `frame.verify_classical()` | `:171` | **1 Ed25519 verify** |
| `frame.verify_pq()` under `RequireBoth` | `:181` | **1 ML-DSA-65 verify** (FIPS-204, ~3.3 KB sig, the heaviest op) |
| verify-then-record nonce insert | `:193-206` | cheap |

For a same-scope hot stream on one live connection, steps `verify_chain`/`verify_classical`/`verify_pq`
repeat **identical** asymmetric work every frame. That is the avoidable cost this blueprint targets —
and **only** that. `RequireBoth` is the only production policy (`iroh_transport.rs:275,339`,
`wss_transport.rs` accept/connect), so the ML-DSA-65 verify is always paid today.

### 0.2 The channel binding is present as a *primitive* but simulated + unenforced (the C1 gap)

| Element | Cite | State |
|---|---|---|
| `channel_binding_hash(transcript)` = SHA3-256 of transcript | `handshake.rs:29-31` | **real primitive**, tested (`:54-66`). Its own ponytail note (`:26-28`) says the transcript **MUST** be the *authenticated* handshake bytes, else "a MITM that reselects ciphersuites could collide the binding." |
| `sign_frame_bound(frame, seed, handshake_transcript)` | `lib.rs:120-140` | takes an **arbitrary caller-supplied** `handshake_transcript: &[u8]` — it does **not** read the live TLS session. |
| `SignedFrame.channel_binding: Option<[u8;32]>` + `with_binding` | `signed_frame.rs:83`, TLV field `0x03` (`wire_codec.rs:36,211,269`) | wire slot exists; `None` = legacy/unbound (`signed_frame.rs:19`). |
| `TransportPolicy.require_tls_channel_binding` | `transport_policy.rs:66-69,98-102` | defaults **`false`** (`:81`); when true, `admit()` only rejects a **`None`** binding — it **never compares** the binding to the live channel. |
| the transcript actually fed in production carriers | `iroh_transport.rs::send` (`:345-358`), `wss_transport.rs::send` | **`sign_frame_bound` is never called on the production send path** — `send` just `encode_frame` + write; `channel_binding` stays `None`. |
| the transcript fed in tests | `wss_transport.rs:1324,1388`; `bpv7.rs:460` | a **literal** `b"channel-A-handshake-transcript"` / `Some([0xAAu8;32])` — the simulation the research flagged. |
| receiver-side comparison of binding vs live channel | grep: **absent** | red-team F3/M1 (`B3-wire-transport.md:57-61`, `B2-protocol-authz.md:93-94`) **STILL OPEN**: `recv` never derives a channel transcript, never compares, never rejects a mismatched or `None` binding. The green test only proves rejection when an attacker *mutates the binding field but keeps the old signature* (a signature mismatch), not a real exporter check. |

**Consequence:** the fast-path's MITM/relay-splice defense (§7.4) depends entirely on a binding value
that (a) is derived from the *real* authenticated TLS exporter and (b) is *compared on receive*.
Neither is true today. This is exactly why the VERDICT gates on C1.

### 0.3 The TLS stack that *can* provide a real exporter (verified deps)

`proto-wire/Cargo.toml`: **quinn `0.11.11`** (`:31`, feature `ring`), **rustls `0.23.42`** (`:33`),
**tokio-rustls `0.26`** (`:38`), rcgen `0.14.8` (`:32`). Both expose the RFC-5705 exporter:

- **QUIC path:** `quinn::Connection::export_keying_material(&self, output, label, context)` (quinn 0.11)
  → the TLS-1.3 exporter (RFC 5705, as used by RFC 9266 `tls-exporter` channel binding). The
  `QuicTransport::connect`/`accept` methods hold `conn` (`iroh_transport.rs:260-269,324-333`) **before**
  it is consumed into `(send, recv)` — the exporter must be captured there.
- **WSS path:** `tokio_rustls::TlsStream::get_ref().1.export_keying_material(...)` (rustls 0.23
  `ConnectionCommon::export_keying_material`).

`QuicTransport` struct (`iroh_transport.rs:72-84`) holds `endpoint, send, recv, buf, gate, roster,
revocations` — **no exporter field today**; M1 adds one.

### 0.4 The primitives the fast-path reuses are ALL already in-tree, zero new deps (standard §2 item 19)

| Need | In-tree primitive | Cite |
|---|---|---|
| PQ key agreement | **ML-KEM-768 (FIPS-203)**, from-scratch zero-dep | `core/src/pq_kem.rs:1-11`; `keygen_from_entropy` (`rng.rs:816`) |
| classical KEM leg (hybrid) | **X25519 (RFC 7748)** | `core/src/x25519.rs:3-8,420-423` |
| symmetric authenticator + (free) confidentiality | **XChaCha20-Poly1305 (RFC 8439 / draft-irtf-cfrg-xchacha-03)**, zero-dep, KAT-verified | `core/src/aead.rs:1`; already the earmarked `PayloadEnc` impl (`transport_policy.rs:107-113` "ML-KEM-768 → XChaCha20-Poly1305") |
| key derivation | in-tree **KDF** | `core/src/kdf.rs` |
| channel-binding hash | **SHA3-256** | `core/src/hash.rs`; `handshake.rs:29` |
| hybrid identity signatures | Ed25519 (`sign.rs`) + ML-DSA-65 (`pq_dsa.rs`), `derive_pq_seed` (`pq_dsa.rs:1135`) | `lib.rs:134-138` |
| eligible-scope tag | `Resource::Presence = 0x04`, `Action::Send` | `proto-cap/src/scope.rs:26,68,179` |
| frame-kind registry (append-only, pinned, fail-closed) | `FrameKind` | `frame_kind.rs` (Data 0x01 / OperatorKill 0x02 / PolicyUpdate 0x03) |

The blueprint therefore **adds no dependency** and **invents no primitive** — it composes existing,
KAT-gated crypto behind the existing `Transport`/`PayloadEnc` seams. The fast-path *is* the concrete
production instantiation of the today-`NoopPayloadEnc` seam (`transport_policy.rs:122-133`), narrowed
to presence frames.

---

## 1. Prior-art map — adopt, don't invent (standard §2 item 19)

Each row is a real, standard construction and exactly how P92 uses it — and what it deliberately does **not** take.

| Prior art | What it is | How P92 uses it — and what it does NOT take |
|---|---|---|
| **QUIC + TLS 1.3 / WireGuard-Noise / Signal-X3DH** | expensive asymmetric handshake once → cheap symmetric AEAD per record after | **Adopt the shape** for one traffic class (continuously-online same-scope streams). **NOT taken:** the generalisation to *all* traffic — store-and-forward/gossip/breach frames structurally cannot ride a session (research §4), so they keep per-frame signing. |
| **RFC 5705 exporter / RFC 9266 `tls-exporter` channel binding** | derive a value bound to the *authenticated* TLS session; a MITM that re-terminates TLS produces a *different* exporter on each side | **Adopt as the hard root** of the session key (§6). Binding the session MAC key to the exporter is what makes a relay/MITM split fail-closed. **NOT taken:** trusting the TLS channel for *identity* — identity is still the one-time hybrid verify; the exporter only *binds* the session to the channel. |
| **Ephemeral (EC)DHE / KEM per session for forward secrecy** | fresh ephemeral key material per session so a leaked session key never exposes other sessions | **Adopt with ML-KEM-768** (not ECDHE — the TLS session key is X25519 = Shor-broken; a PQ session needs a PQ KEM, research §2 H3 row). Ephemeral per session → per-session forward secrecy (§7.3). |
| **IPsec / DTLS 1.3 anti-replay sliding window (RFC 4303 §3.4.3 / RFC 9147)** | monotonic sequence + a bitmap window: accept newer, tolerate bounded reorder, reject too-old/duplicate | **Adopt verbatim** for the per-direction counter (§5.4). Presence pings can arrive slightly out of order over a relay; a hard "strictly increasing" rule would drop benign reorders. |
| **TLS 1.3 downgrade protection / negotiation-transcript binding** | bind the advertised parameter list into the transcript so a MITM can't force a weaker choice | **Adopt** for suite negotiation in the offer/accept (§5.2, §7.1): both advertised MAC/KEM suite lists are bound into the HKDF, so a strip fails the first AEAD tag. Mirrors P59 §6.4. |
| **Signal double-ratchet / MLS group keying** | continuous rekey per message / group epoch trees | **NOT taken.** Over-engineering for ephemeral presence pings (ponytail): no per-message ratchet, no group tree. Bounded-lifetime session + full re-handshake on expiry is sufficient and far simpler; a leaked key is bounded by `not_after`, not by a ratchet. Stated, not shoehorned. |
| **The existing `PayloadEnc` trait (`transport_policy.rs:114`)** | injected ML-KEM-768→XChaCha20 payload encryptor, `NoopPayloadEnc` default | **This is the seam we fill.** P92's session AEAD is the real, presence-scoped `PayloadEnc` impl behind that trait — no new abstraction. |

---

## 2. Scope — what P92 owns vs deliberately does NOT (standard §2 items 11, 18, 19)

### 2.1 P92 OWNS

1. **The RFC-5705/9266 exporter wiring** in both carriers: capture at connect/accept, store on the
   transport, set on outbound frames, and — the currently-missing F3 comparison — **enforce on recv**
   (M1). This is a prerequisite that *also closes* the open full-signed-path F3/M1 finding.
2. **The fast-path session protocol:** promotion handshake (`FastPathOffer`/`FastPathAccept`, full
   hybrid-signed), ephemeral ML-KEM-768 agreement, exporter-bound HKDF key derivation (M3).
3. **The per-frame fast-path:** `FastPathFrame` (distinct `FrameKind`), XChaCha20-Poly1305 authenticator
   with per-direction keys + monotonic counter + anti-replay window (M2, M4).
4. **Session lifetime, rotation, and cap-expiry binding** (M5).
5. **Revocation propagation into live sessions:** re-check tick + merge-triggered teardown (M6).
6. **The scope allow-list + separate ephemeral presence sink + no-non-repudiation invariant + downgrade
   binding** (M7) — the anti-scope-creep machinery.
7. **The mandatory independent adversarial-review gate** as a DoD-blocking checkpoint (§8).

### 2.2 P92 does NOT own (anti-scope — prevents collision & scope-creep)

- **Any frame that can be store-and-forwarded, gossiped, or synced.** `Ledger`, `Order`, `Claim`,
  `DeliveryIntent`, `Sync`, `Backup`, `Menu`, `Analytics`, `Loyalty`, `Customer`, `Corpus` scopes
  **stay full per-frame hybrid-signed.** The BPv7 store-and-forward overlay (`bpv7.rs`) and pull
  anti-entropy (`sync_pull.rs`) are untouched — their authentication must travel *with the frame*, not
  with the channel (research §4).
- **`BreachAlarm/Broadcast`** — the self-signed P2P fail-safe (`iroh_transport.rs:366-389`) bypasses the
  roster/session by design and **must never** be fast-pathed.
- **`OperatorKill` / `PolicyUpdate`** control frames (`frame_kind.rs`) — anchor-signed, non-repudiable,
  never eligible.
- **Any red-line scope** — `Auth`, `Secret`, `Migration`, and anything money/settlement
  (`SettlementRecorded`, `ClaimReleased`) — hard-excluded by the allow-list (M7), independent of the
  red-line gate.
- **The mesh-scoped cross-node nonce ledger** (research §2 C3). Fast-path frames never leave the
  session, so cross-node replay does not apply to them; P92 does not touch the node-scoped `seen`
  ledger design.
- **Confidentiality of the transport as a whole** — provided by QUIC/TLS already; the fast-path AEAD's
  *purpose* is authenticity (the Poly1305 tag), with confidentiality a free defense-in-depth byproduct
  past a semi-trusted relay (`transport_policy.rs:107-113`).
- **iroh DHT/relay deployment** (`iroh_transport.rs:23-25`, out of scope there) — P92 assumes the
  carrier the host wires; it only requires the exporter API, which quinn already exposes.

### 2.3 Dependencies (named by artifact — standard §2 item 7)

**Hard inputs (in tree):** `HybridGate::check` + `verify_chain` (`hybrid_gate.rs`, `roster.rs`) for the
one-time verify; `SignedFrame`/`Capability`/`Scope` (`proto-cap`); `pq_kem`/`x25519`/`aead`/`kdf`/`hash`
(`bebop2-core`, §0.4); `channel_binding_hash`/`sign_frame_bound` (`handshake.rs`/`lib.rs`); the `quinn`
/`rustls` exporter API (§0.3); the `PayloadEnc` seam (`transport_policy.rs`).

**Consumers:** the `mesh-node` presence/liveness surface (courier↔hub live location), which today would
receive `Presence/Send` frames through the full gate; after P92 it receives them through the fast-path
sink (§5.5) with identical *semantics* (liveness hint) but lower per-frame cost.

### 2.4 Honest reconciliation with the per-message model (standard §2 item 6)

The research verdict is binding and P92 does not overturn it: **per-message signing is load-bearing and
correct for the mesh's real workload** (research §5a). P92 is a *strict subset layer underneath* it, not
a replacement. The reconciliation rule, enforced structurally (M2/M7): a frame is fast-path-eligible
**iff** it is (i) `Presence/Send` scope, (ii) on a live bidirectional session whose both endpoints
passed the full hybrid verify this session, and (iii) never destined for storage/gossip/broadcast.
**Anything failing any of the three falls through to the full per-frame gate, unchanged.** The default
is per-message; the fast-path is an opt-in, auto-fallback overlay.

---

## 3. Predefined types & constants — named BEFORE implementation (standard §2 item 4)

All new types live in a **new module `proto-wire/src/fastpath.rs`** (keeps `hybrid_gate.rs`/`lib.rs`
from growing a new responsibility; imports `proto-cap` frame types + `bebop2-core` crypto). Constants
are named, never magic. Session state lives on the carrier struct (`QuicTransport`/`WssTransport`).

```rust
// proto-wire/src/fastpath.rs  (NEW)

/// A channel binding derived from the LIVE TLS/QUIC exporter (RFC 5705 / RFC 9266 tls-exporter).
/// This is the value the whole design roots on; it MUST come from `export_keying_material`, never
/// a literal (§0.2, §6). 32 bytes = SHA3-256 output width of the existing `channel_binding_hash`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelBinding(pub [u8; 32]);

/// The RFC-5705 exporter label + context, pinned as part of the wire contract. Distinct from any
/// other exporter use so a value exported for one purpose can never be replayed for another.
pub const EXPORTER_LABEL: &[u8]   = b"EXPORTER-bebop2-fastpath-v1";
pub const EXPORTER_CONTEXT: &[u8] = b"bebop2.mesh.fastpath.channel-binding";

/// Fast-path negotiated suite (single code point today; append-only like FrameKind/AlgSuite).
/// v1 = ML-KEM-768 key agreement + XChaCha20-Poly1305 authenticator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum FastPathSuite {
    MlKem768XChaChaPoly = 0x0001,
    // v2+ reserved (e.g. ML-KEM-1024 / a future PQ MAC) — new code point, never a wire fork.
    // Unknown code points are REJECTED (fail-closed), never best-effort.
}

/// The ONLY scopes a fast-path session may ever cover. Compile-time allow-list — the single place
/// eligibility is defined. Adding a scope here is a red-line-review event (§7.2, §8).
pub const FASTPATH_ELIGIBLE_SCOPES: &[(Resource, Action)] = &[(Resource::Presence, Action::Send)];

/// Bounded session policy (all cheap, all per-session). Values are engineering decisions (§5.7).
pub const FASTPATH_MAX_SESSION_SECS:    u64   = 300;         // hard wall-clock lifetime, then re-handshake
pub const FASTPATH_REVOCATION_TICK_SECS: u64  = 30;          // revocation re-check cadence (== stale-trust bound)
pub const FASTPATH_MAX_FRAMES:          u64   = 1 << 32;     // rekey before counter exhaustion (nonce safety)
pub const FASTPATH_REPLAY_WINDOW:       u64   = 64;          // anti-replay bitmap width (IPsec/DTLS-style)
pub const FASTPATH_MIN_FULLSIGNED_FRAMES: u32 = 1;           // full-signed frames required before promotion
pub const FASTPATH_SESSION_KEY_LEN:     usize = 32;          // 256-bit AEAD key
pub const FASTPATH_MLKEM_SUITE: FastPathSuite = FastPathSuite::MlKem768XChaChaPoly;

/// Promotion offer — carried as the PAYLOAD of a FULL hybrid-signed SignedFrame (FrameKind::FastPathOffer).
/// Its authenticity is the one-time hybrid signature; its contents bootstrap the cheap session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastPathOffer {
    pub epoch: [u8; 16],                 // random session id (offerer-chosen)
    pub scope: (Resource, Action),       // MUST be in FASTPATH_ELIGIBLE_SCOPES; checked both ends
    pub suites: Vec<FastPathSuite>,      // advertised, strongest-first (downgrade-bound, §5.2)
    pub mlkem_ephemeral_ek: Vec<u8>,     // offerer's EPHEMERAL ML-KEM-768 encapsulation key (per session)
    pub x25519_ephemeral_pk: [u8; 32],   // classical hybrid leg (X25519 ⊕ ML-KEM per §0.4)
    pub channel_binding: ChannelBinding, // offerer's LOCAL live-exporter value — responder compares to its own
    pub cap_expiry: u64,                 // the Presence cap's expiry — session not_after is bounded by it (§5.5)
}

/// Promotion accept — PAYLOAD of a FULL hybrid-signed SignedFrame (FrameKind::FastPathAccept).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastPathAccept {
    pub epoch: [u8; 16],                 // echoes the offer's epoch
    pub chosen: FastPathSuite,           // strongest common suite
    pub mlkem_ciphertext: Vec<u8>,       // encapsulation to the offerer's ephemeral ek
    pub x25519_ephemeral_pk: [u8; 32],   // responder's classical leg
    pub channel_binding: ChannelBinding, // responder's LOCAL live-exporter value — offerer compares to its own
}

/// A per-frame fast-path record. NOT a SignedFrame: it carries NO capability, NO delegation chain,
/// NO hybrid signature — it CANNOT be decoded as a SignedFrame and therefore CANNOT enter
/// HybridGate::check / verify_chain (§7.2 defense (a)). Distinct FrameKind::FastPathData.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastPathFrame {
    pub epoch: [u8; 16],                 // session id — no session, no key, no acceptance
    pub direction: u8,                   // 0 = initiator→responder, 1 = responder→initiator (key + reflection guard)
    pub counter: u64,                    // monotonic per (epoch, direction)
    pub aead_ct: Vec<u8>,                // XChaCha20-Poly1305 over the PresenceRecord; tag included
    // nonce = epoch[0..12] ‖ direction ‖ counter_le  (96-bit nonce space in XChaCha's 192-bit room — no reuse)
    // aad   = epoch ‖ direction ‖ counter ‖ (Presence,Send) discriminants  (binds scope + ordering into the tag)
}

/// The ONLY thing a FastPathFrame can decode to — a fixed-shape, ephemeral liveness hint. It has no
/// path to the event log / ledger / capability store (§7.2 defense (b)); it is a "local liveness
/// hint," explicitly NOT third-party-provable evidence (§7.2 defense (d)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceRecord {
    pub node_id: [u8; 32],               // the peer whose liveness this asserts (== session peer; cross-checked)
    pub seq: u64,                        // == FastPathFrame.counter (bound in aad; monotone)
    pub position: Option<[i32; 2]>,      // optional coarse lat/lon e7 — ephemeral, non-authoritative
    pub heartbeat_tick: u64,             // sender's monotonic tick
}

/// Live session state, held on the carrier. Keys are per-direction (no reflection).
#[derive(Debug)]
pub struct FastPathSession {
    pub epoch: [u8; 16],
    pub peer_node_id: [u8; 32],
    pub scope: (Resource, Action),       // pinned == the offer scope; in FASTPATH_ELIGIBLE_SCOPES
    pub suite: FastPathSuite,
    pub key_send: [u8; FASTPATH_SESSION_KEY_LEN],  // HKDF label "i2r"/"r2i" by role
    pub key_recv: [u8; FASTPATH_SESSION_KEY_LEN],
    pub established_at: u64,
    pub not_after: u64,                  // min(established_at + FASTPATH_MAX_SESSION_SECS, cap_expiry)
    pub next_revcheck: u64,              // established_at + FASTPATH_REVOCATION_TICK_SECS, then rolling
    pub send_counter: u64,
    pub recv_highest: u64,               // anti-replay: highest accepted recv counter
    pub recv_window: u64,                // 64-bit bitmap of accepted counters below recv_highest
}
```

**FrameKind additions (append-only to the pinned registry, `frame_kind.rs`):**

```rust
FrameKind::FastPathOffer  => 0x04,   // full hybrid-signed SignedFrame carrying a FastPathOffer
FrameKind::FastPathAccept => 0x05,   // full hybrid-signed SignedFrame carrying a FastPathAccept
FrameKind::FastPathData   => 0x06,   // a FastPathFrame (NOT a SignedFrame) — routed to the presence sink ONLY
FrameKind::FastPathClose  => 0x07,   // teardown (full hybrid-signed; also implied by connection drop)
```

---

## 4. Build items — spec → RED test → code, each with adversarial cases (standard §2 items 2, 3, 5)

Each item: **spec first, a test that goes RED before the change, code, then GREEN.** State transitions
are modelled as events; tests assert on the sequence (item 3). RED tests are named for the suite.

### 4.1 M1 — real RFC-5705 exporter: capture + store + enforce (the C1 prerequisite; also closes F3/M1)

- **Spec:** at `connect`/`accept`, before `conn` is consumed into `(send, recv)`, call
  `conn.export_keying_material(&mut out32, EXPORTER_LABEL, EXPORTER_CONTEXT)` (quinn) /
  `tls.export_keying_material(...)` (rustls) and store `ChannelBinding(out32)` on the carrier. `send`
  sets `frame.channel_binding = Some(binding)` via `with_binding` for **every** frame (full-signed and
  fast-path). `recv` **rejects** any frame whose `channel_binding != Some(self.binding)` (and rejects
  `None` when `require_tls_channel_binding`). This is the comparison that is absent today (§0.2).
- **RED `red_exporter_mismatch_rejected`:** two live channels with different exporters; a frame bound to
  channel A, replayed on channel B, is rejected by B's `recv` (binding≠B's exporter). RED today (no
  comparison), GREEN after. **Closes F3/M1 for the full-signed path too — REGRESSION-LEDGER entry.**
- **RED `red_none_binding_rejected_when_required`:** `require_tls_channel_binding = true`, a `None`-bound
  frame → `InsecureTransport` (already tested for `admit()` at `transport_policy.rs:156`; M1 extends the
  check to the live `recv` path, not just the pure policy predicate).
- **Adversarial `red_mitm_cert_swap_splits_exporter`:** with `insecure-tls` on, a relay that substitutes
  its own cert produces a *different* exporter on each side → the two derived bindings differ → a
  cross-terminated frame fails the M1 comparison. Proves the exporter is doing real MITM detection, not
  a decorative field. (This is the property §7.4 depends on.)

### 4.2 M2 — `FastPathFrame` codec + FrameKind, structurally disjoint from `SignedFrame`

- **Spec:** `wire_codec` gains `encode_fastpath`/`decode_fastpath`; the FrameKind byte dispatches.
  `decode_fastpath` on a `SignedFrame` byte layout fails, and `decode_frame` (SignedFrame) on a
  `FastPathFrame` layout fails — the two are non-interconvertible. Fail-closed on unknown FrameKind
  (mirrors `frame_kind.rs:52`).
- **RED `red_fastpath_frame_not_decodable_as_signed`:** feed a `FastPathData` byte string to
  `wire_codec::decode_frame` → `WireError::Decode`, never a partial `SignedFrame`. Proves defense (a)
  of §7.2 at the codec boundary.
- **RED `red_signed_frame_not_decodable_as_fastpath`:** the inverse. Both directions closed.
- **Adversarial `red_fastpath_kind_confusion`:** a `FastPathData` frame with the FrameKind byte flipped
  to `Data` → the SignedFrame decoder rejects the truncated/mis-shaped bytes; a `Data` frame flipped to
  `FastPathData` → the fastpath decoder rejects. No cross-kind acceptance.

### 4.3 M3 — promotion handshake + ephemeral ML-KEM-768 + exporter-bound HKDF

- **Spec:** after `FASTPATH_MIN_FULLSIGNED_FRAMES` full-signed `Presence/Send` frames have passed the
  gate this session, either peer may send a `FastPathOffer` (full hybrid-signed SignedFrame,
  FrameKind::FastPathOffer). The responder: (1) runs the **full gate** on the offer frame (hybrid +
  `verify_chain` + revocation), (2) checks `offer.scope ∈ FASTPATH_ELIGIBLE_SCOPES`, (3) checks
  `offer.channel_binding == responder's own live exporter` (M1) — **fail-closed reject on mismatch**,
  (4) picks the strongest common suite, ML-KEM-**encapsulates** to `offer.mlkem_ephemeral_ek` +
  X25519-DHs `offer.x25519_ephemeral_pk`, (5) replies `FastPathAccept` (full hybrid-signed). Both derive:
  `session_key_material = KDF( mlkem_ss ‖ x25519_ss ‖ channel_binding ‖ initiator_node_id ‖
  responder_node_id ‖ epoch ‖ scope_discriminants ‖ transcript_of_both_suite_lists )`, then
  `key_i2r = KDF(session_key_material, "i2r")`, `key_r2i = KDF(session_key_material, "r2i")`.
  The offerer confirms `accept.channel_binding == its own live exporter` before installing keys.
- **RED `red_offer_wrong_scope_rejected`:** an offer whose `scope` is `Ledger/Append` → rejected before
  any key derivation. RED (no check exists), GREEN after — the anti-scope-creep gate at establishment.
- **RED `red_offer_exporter_mismatch_rejected`:** `offer.channel_binding` ≠ responder's live exporter →
  rejected (a spliced/relayed offer cannot promote). Depends on M1.
- **RED `red_offer_unverified_identity_rejected`:** a `FastPathOffer` whose hybrid signature or
  `verify_chain` fails → no session. Proves "verify once" is a *real* full verify, not skipped.
- **Adversarial `red_kem_material_tampered`:** flip one byte of `mlkem_ciphertext` in transit → the two
  sides derive different `mlkem_ss` → the first `FastPathFrame` AEAD tag fails → auto-fallback to
  full-signed (no silent accept). Proves the KEM leg is bound into the key, not decorative.
- **Adversarial `red_downgrade_suite_strip`:** MITM removes the strong suite from `offer.suites` → the
  suite-list transcript hash bound into the KDF differs between signer and verifier → first AEAD tag
  fails → fallback. Mirrors TLS downgrade binding (§7.1).

### 4.4 M4 — XChaCha20-Poly1305 authenticator + per-direction counter + anti-replay window

- **Spec:** `send_fastpath(record)`: `counter = session.send_counter; send_counter += 1;`
  `nonce = epoch[0..12] ‖ direction ‖ counter_le`; `aad = epoch ‖ direction ‖ counter ‖
  Presence_disc ‖ Send_disc`; `aead_ct = aead_xchacha20_poly1305_encrypt(key_send, nonce, aad, record)`.
  `recv_fastpath(f)`: recompute nonce/aad, AEAD-decrypt with `key_recv`; on tag fail → reject (no
  fallback for an *installed* session — a bad tag on an active session is an attack or corruption).
  Anti-replay: accept `counter` iff `counter > recv_highest` (advance window, set bit) **or**
  `recv_highest - counter < FASTPATH_REPLAY_WINDOW` and its bit is unset (set it); reject if
  `counter ≤ recv_highest - FASTPATH_REPLAY_WINDOW` (too old) or the bit is already set (replay).
  Cross-check `record.node_id == session.peer_node_id` (a frame can't claim another node's presence).
- **RED `red_replayed_counter_rejected`:** re-send a byte-identical `FastPathFrame` → bit already set →
  `WireError::Replay`. RED (no window today), GREEN after.
- **RED `red_reorder_within_window_accepted`:** counters 5,4,6 (4 arrives after 5) → all accepted (4 is
  within window, bit unset). Proves benign reorder tolerance (the reason a hard-increasing rule is wrong).
- **RED `red_too_old_counter_rejected`:** after `recv_highest = 200`, a counter `100` (window 64) →
  rejected as too-old. Bounds the window.
- **Adversarial `red_reflected_frame_rejected`:** take an i→r frame and replay it on the r→i direction
  (direction byte flipped) → `key_recv` for that direction differs (per-direction keys) and the aad
  direction byte differs → tag fails. Proves no reflection.
- **Adversarial `red_wrong_epoch_rejected`:** a valid-looking frame with an `epoch` for which no session
  exists → dropped (no key). Proves a frame is meaningless without an installed session.

### 4.5 M5 — session lifetime, rotation, cap-expiry binding

- **Spec:** `not_after = min(established_at + FASTPATH_MAX_SESSION_SECS, offer.cap_expiry)`. On
  `now ≥ not_after`, or `send_counter ≥ FASTPATH_MAX_FRAMES`, or connection drop, or `FastPathClose`,
  the session is **torn down**; further presence traffic reverts to full-signed until a new handshake.
  Re-handshake mints a fresh epoch + fresh ephemeral ML-KEM (new forward-secret key).
- **RED `red_session_expires_at_not_after`:** advance `now` past `not_after` → the next `FastPathFrame`
  is rejected and the session removed; a re-offer establishes a *new* epoch. Event sequence asserted:
  `Established(e1) → Expired(e1) → Established(e2)` with `e1 ≠ e2`.
- **RED `red_session_bounded_by_cap_expiry`:** `cap_expiry < established_at + MAX_SESSION_SECS` → session
  dies at `cap_expiry`, not at the longer bound. Keeps the H1 freshness lesson (research §2 H1 row).
- **Adversarial `red_counter_exhaustion_forces_rekey`:** drive `send_counter` to `FASTPATH_MAX_FRAMES` →
  send is refused until re-handshake (no nonce reuse). Proves nonce safety.

### 4.6 M6 — revocation propagation into live sessions (the critical bound)

- **Spec:** two mechanisms, both cheap: **(tick)** at `now ≥ next_revcheck`, re-run the gate's revocation
  lookups (`is_revoked_key(peer_subject)`, `pq_key_id`, cap `revocation_hash`) against the current
  `RevocationSet`; if revoked → immediate teardown + drop all further fast-path frames from that peer;
  else `next_revcheck += FASTPATH_REVOCATION_TICK_SECS`. **(event)** on any `RevocationSet::merge`
  (`revocation.rs`, gossip), scan live sessions and tear down any whose peer is now revoked — the
  fast path to revocation; the tick is the upper bound.
- **RED `red_revoked_peer_torn_down_on_tick`:** establish a session, revoke the peer key, advance `now`
  past `next_revcheck` → the next `FastPathFrame` is rejected and the session removed. RED (no revoke
  check on the fast path), GREEN after.
- **RED `red_revoked_peer_torn_down_on_merge`:** revoke via `RevocationSet::merge` *before* the tick →
  the session is torn down immediately, not at the next tick. Proves the event path.
- **Adversarial `red_stale_trust_window_is_bounded`:** assert that the maximum interval between a
  peer's revocation and its fast-path frames being rejected is ≤ `FASTPATH_REVOCATION_TICK_SECS`
  (worst case = revocation lands just after a tick and no merge event reaches this node). This is the
  §5.6 stale-trust bound, made a machine-checkable property, not a prose promise.

### 4.7 M7 — scope allow-list + separate presence sink + no-non-repudiation + downgrade binding

- **Spec:** (a) `is_fastpath_eligible(scope) = FASTPATH_ELIGIBLE_SCOPES.contains(scope)` is the ONLY
  eligibility gate, checked at offer time (M3) and asserted at sink time; (b) `recv_fastpath` writes
  **only** to a `PresenceTable` (a separate ephemeral in-memory map) — there is **no** code path from
  `FastPathFrame` to the event log / `commit` / capability store; (c) a compile-time/test assertion that
  no red-line or store-and-forward scope is in the allow-list; (d) the suite-list transcript is bound
  into the KDF (§4.3, downgrade).
- **RED `red_fastpath_never_reaches_ledger`:** a test that greps/asserts the `recv_fastpath` call graph
  reaches only `PresenceTable::upsert`, never `event_log`/`ledger`/`roster`/`capability`. Structural
  proof of §7.2 defense (b). Wire into CI as a call-graph assertion.
- **RED `red_allow_list_excludes_redline_and_sf`:** a compile-time test asserting
  `FASTPATH_ELIGIBLE_SCOPES` contains no `Auth/Secret/Migration/Ledger/Order/Claim/Sync/Backup/...`.
  If a future edit adds one, this fails (the smart-index for the scope-creep bug class, item 14).
- **RED `red_presence_record_not_evidence`:** a doc/type test asserting `PresenceTable` entries carry no
  signature and are never serialised into a settlement/dispute path — the no-non-repudiation invariant
  (§7.2 defense (d)) as an executable boundary.

---

## 5. The fast-path lifecycle in full (the design the research asked for — item 2)

### 5.1 Establishment / promotion

**First frames full-signed, then explicit in-band promotion.** A stream begins on the normal per-frame
gate. Only after ≥ `FASTPATH_MIN_FULLSIGNED_FRAMES` `Presence/Send` frames have passed
`HybridGate::check` this session — i.e. the peer's hybrid identity + anchor-rooted `Presence` capability
are *proven* — may either peer send a `FastPathOffer`. This guarantees the "verify once" is a **real**
full hybrid verify (research §2: relocating, not removing, the check), never skipped. Promotion is
opt-in and either side may decline (stay full-signed) with no penalty.

### 5.2 The ML-KEM exchange payload & downgrade binding

`FastPathOffer` (full hybrid-signed, so its contents are authenticated by the one-time signature)
carries: a fresh **ephemeral** ML-KEM-768 encapsulation key + an ephemeral X25519 public key (the
hybrid KEM, §0.4), the advertised `suites` (strongest-first), the offerer's **live exporter**
`channel_binding`, and the Presence cap's `expiry`. The responder encapsulates to both legs and returns
the ciphertext in a full hybrid-signed `FastPathAccept`. **Both advertised suite lists are hashed into
the KDF transcript** so a MITM stripping the strong suite changes the derived key → the first AEAD tag
fails → fallback (TLS-1.3 downgrade-binding, §7.1).

### 5.3 Key derivation (why BOTH the KEM secret and the exporter are required)

```
session_key_material = KDF( mlkem_ss ‖ x25519_ss              // PQ + classical KEM (forward secrecy, Shor-safe)
                          ‖ channel_binding                    // RFC-5705 exporter (MITM/relay-splice defense)
                          ‖ initiator_id ‖ responder_id ‖ epoch
                          ‖ scope_discriminants                // pins Presence/Send into the key
                          ‖ suite_list_transcript )            // downgrade binding
key_i2r = KDF(session_key_material, "i2r")   // per-direction, no reflection
key_r2i = KDF(session_key_material, "r2i")
```

- **`mlkem_ss` alone** would be MITM-safe only if the KEM public key were channel-authenticated; a
  relay could otherwise splice two independent KEM sessions. **The exporter closes that** — a spliced
  channel has a different exporter on each side, so the derived keys differ (§7.4).
- **`channel_binding` alone** (the current mesh plan) would inherit the TLS session's **classical-only**
  (X25519) security — Shor-broken, harvest-now-decrypt-later. **The ephemeral ML-KEM closes that** —
  the authenticity key is PQ-agreed (research §2 H3 row).
- Both are load-bearing; neither is sufficient alone. This is the design's crux and the reason C1 is a
  hard gate.

### 5.4 Per-frame protection & anti-replay window

Each `FastPathFrame` is `XChaCha20-Poly1305(key_dir, nonce, aad, PresenceRecord)` where
`nonce = epoch[0..12] ‖ direction ‖ counter` (192-bit XChaCha room, counter never repeats per
direction) and `aad = epoch ‖ direction ‖ counter ‖ (Presence,Send) discriminants`. The 128-bit
Poly1305 tag is the authenticator (real MAC, not under-provisioned); confidentiality of the small
presence payload is a free defense-in-depth byproduct past a semi-trusted relay. Replay/reorder: an
IPsec/DTLS-style **64-bit sliding window** over the monotonic per-direction counter — accept newer,
tolerate bounded reorder, reject too-old or already-seen (§4.4).

### 5.5 Eligible frames & the separate sink

**Only `Resource::Presence`/`Action::Send`.** A `FastPathFrame` decodes to a `PresenceRecord` and is
written **only** to an ephemeral `PresenceTable` (liveness hint). There is no code path from it to the
event log, ledger, capability store, roster, or any authority sink (§7.2). A presence record is a
"local liveness hint," never third-party-provable evidence; any consumer needing non-repudiation
(settlement, dispute, audit) reads the **full-signed event log**, never the presence table.

### 5.6 Session lifetime, rotation & the stale-trust bound (revocation)

- **Lifetime:** `not_after = min(established_at + 300 s, cap.expiry)`; also dies on counter exhaustion,
  connection drop, or `FastPathClose`. A dropped connection *must* end the session because the key is
  channel-bound — a new connection has a new exporter, hence a mandatory new handshake.
- **Rotation:** re-handshake mints a fresh epoch + fresh ephemeral ML-KEM → per-session forward secrecy.
- **Revocation (the critical bound):** because the fast path skips `verify_chain` (where revocation is
  checked today), P92 adds an explicit revocation re-check **tick ≤ 30 s** plus a **merge-triggered
  immediate teardown** (§4.6). **Concrete bound:** the worst-case interval in which a just-revoked peer
  can still push presence pings = the local revocation-visibility time + one tick ≤ **`30 s`**.
  This window applies **only to presence pings** — ephemeral, non-authoritative, discarded on any
  dispute. It never touches money, capability, authority, or non-repudiation, all of which remain on
  the full per-frame gate where revocation is immediate. **This bounded stale-trust window is the one
  honestly-stated cost of the optimisation** (VERDICT, §7.2 residual).

### 5.7 Engineering-decision values (blueprint sets; operator need not)

`FASTPATH_MAX_SESSION_SECS = 300`, `FASTPATH_REVOCATION_TICK_SECS = 30`, `FASTPATH_REPLAY_WINDOW = 64`,
`FASTPATH_MAX_FRAMES = 2^32`, `FASTPATH_MIN_FULLSIGNED_FRAMES = 1`. All are named `fastpath.rs`
constants with a one-line change surface. The tick value **must** be ≤ the mesh's revocation-gossip
convergence bound (so the merge path usually beats the tick); `300 s` is short enough that a leaked
session key is bounded and long enough to amortise the one-time verify across a real ping stream.

---

## 6. What is missing to make the exporter real (item 1 — precise)

Today (§0.2): the binding is a *simulated literal* and the receiver *never compares* it. To make it real:

1. **Derive from the live session, not a literal.** In `QuicTransport::connect`/`accept`, after the
   handshake completes and **before** `conn` is consumed into `(send, recv)` (`iroh_transport.rs:260-269,
   324-333`), call `conn.export_keying_material(&mut b, EXPORTER_LABEL, EXPORTER_CONTEXT)` and store
   `ChannelBinding(b)` on the struct. In `WssTransport`, call `export_keying_material` on the
   `tokio_rustls::TlsStream`'s inner `ConnectionCommon` at the same point.
2. **Set it on send** (both full-signed and fast-path): `frame.with_binding(self.binding.0)` before
   encode — replacing the never-called `sign_frame_bound(..., literal)` path with the live value. Feed
   the live exporter bytes to `channel_binding_hash` *or* use the 32-byte exporter directly (it is
   already a KDF output; hashing again is harmless domain separation).
3. **Enforce on recv** (the currently-absent F3 comparison): reject any frame whose `channel_binding !=
   Some(self.binding)`, and reject `None` when `require_tls_channel_binding` — flip that policy default
   to `true` for prod carriers (`transport_policy.rs:81`). This alone closes red-team F3/M1 for the
   full-signed path, independent of the fast-path.
4. **Prove MITM detection** with `red_mitm_cert_swap_splits_exporter` (§4.1): the exporter must differ
   across a re-terminated channel, or the whole design is decorative.

**This (M1) is the standalone prerequisite the VERDICT gates on — land + independently-review it first.**

---

## 7. Adversarial self-check — real effort to break the design (item 3 — the heart)

### 7.1 Downgrade attacks — can an attacker force fast-path where full signing should apply?

- **Force an authority frame onto the fast path?** No. A non-`Presence/Send` frame cannot be MAC-valid
  under the session key (scope discriminants are in the aad *and* the KDF), and the receiver's fast-path
  decoder routes only to the presence sink. An attacker "downgrading" a Ledger append to a fast-path
  frame yields **rejection + fallback to the full-signed gate**, which then demands the hybrid signature.
  There is no path where a MAC-only frame is accepted as authority.
- **Force a weaker suite?** No. Both advertised suite lists are bound into the KDF (§5.2); a strip
  changes the derived key → first AEAD tag fails → fallback. (`red_downgrade_suite_strip`, §4.3.)
- **Reverse downgrade (force *off* fast-path onto full-signed)?** Trivially possible (drop the KEM
  handshake) — but this is **fail-safe**: more security, more cost, never less. Not a vulnerability.
- **Force *premature* promotion?** No — a `FastPathOffer` is full hybrid-signed; an attacker without the
  keys can't forge one, and a compromised offerer can't widen scope (the responder independently checks
  `scope ∈ allow-list` and its own cap). (`red_offer_wrong_scope_rejected`.)

### 7.2 Scope-creep — could MAC-only frames be misrouted as fully-authenticated for something they shouldn't?

This is the **highest-severity** risk; defended in depth (all four required):

- **(a) Structural non-interconvertibility.** A `FastPathFrame` carries no capability, no chain, no
  hybrid sig; it **cannot decode as a `SignedFrame`** and therefore **cannot enter `HybridGate::check`
  / `verify_chain`** (M2, `red_fastpath_frame_not_decodable_as_signed`).
- **(b) Separate sink.** `recv_fastpath` reaches **only** `PresenceTable::upsert` — no code path to the
  event log / `commit` / capability store / roster (M7, `red_fastpath_never_reaches_ledger`, a CI
  call-graph assertion).
- **(c) KDF scope pinning.** Even if (a)/(b) were bypassed, the key is derived over `Presence/Send`; any
  other effect derives a different key and fails the tag.
- **(d) No non-repudiation.** A symmetric MAC is forgeable by either holder — so a presence record is
  **never** third-party evidence. Consumers needing proof use the full-signed log (M7,
  `red_presence_record_not_evidence`).
- **Residual + mitigation:** the real risk is a *future developer* adding an eligible scope carelessly.
  Mitigated by the compile-time allow-list + `red_allow_list_excludes_redline_and_sf` (fails CI if a
  red-line/store-and-forward scope is added) + the §8 review gate. **Honestly, this residual is why the
  allow-list is a single, greppable, test-guarded constant and not a runtime config.**

### 7.3 Key-compromise blast radius — leaked session key vs. the current design

- **Current per-message worst case:** stealing a node's **long-term hybrid signing keys** lets an
  attacker forge **any** frame for that identity, **mesh-wide**, until revocation. Large blast radius.
- **Fast-path worst case:** leaking a **session MAC key** lets an attacker forge/replay **only
  `Presence/Send` frames, only within one session** (one channel, one peer pair, one epoch), **only
  until `not_after` ≤ 300 s**, and **only presence data** — which is ephemeral and non-authoritative.
  The session key **cannot** sign a capability, append to the ledger, or delegate.
- **Net:** the fast-path's worst case is **strictly smaller** than — and **disjoint from** — the
  existing authority surface. Because each session uses a **fresh ephemeral ML-KEM** keypair, a leaked
  session key gives **per-session forward secrecy** (no past/future session exposure). Establishing any
  session still requires the long-term hybrid keys (the KEM material is hybrid-signed), so the fast-path
  adds **no new way to compromise the long-term identity**. This is a genuine *improvement* to state, not
  a downside.

### 7.4 The relay — replay/reorder within the session, and MITM-splice

- **Can the relay forge or read?** No. It forwards end-to-end-encrypted QUIC packets; the fast-path AEAD
  + presence payload are inside the QUIC record layer. No MAC key → no forge; TLS confidentiality → no read.
- **Replay/reorder?** The relay can drop/delay/reorder QUIC packets. Replay of a captured packet → same
  counter → bit already set → rejected (§5.4). Benign reorder within the 64-window → tolerated. Drop of a
  presence ping → the peer looks *less* present (fail-safe — never falsely present). Stale delayed replay
  → caught by the too-old window and by `not_after`.
- **MITM-splice (the load-bearing case).** A malicious relay that substitutes its own TLS cert (possible
  in `insecure-tls` dev mode, §0.2) produces a **different RFC-5705 exporter on each side** → the two
  peers derive **different session keys** at the KDF step → the **first fast-path frame fails its tag** →
  auto-fallback to full-signed. **Without the real exporter (today's simulated literal), both sides would
  compute the same fake binding regardless of the relay, and the splice would succeed silently.** This is
  exactly why shipping the fast-path on the simulated binding is a MITM downgrade, and why C1 is a hard
  gate. (`red_mitm_cert_swap_splits_exporter`, §4.1.)
- **Relay is never a session participant** — it holds no KEM secret and is not in the roster/anchor
  chain, so it cannot inject a valid `FastPathOffer`/`Accept` (both full hybrid-signed).

### 7.5 Cross-node replay (research §2 C3) — does the fast path re-open it?

No — by construction. Fast-path frames never leave the live session (never stored/forwarded/gossiped),
and the session key is unique per node-pair per epoch. A presence ping captured and replayed to a
*different* node has no matching session/key there → dropped (`red_wrong_epoch_rejected`). The fast path
is immune to C3 because it introduces no shared-across-nodes session key.

---

## 8. Mandatory independent adversarial-review gate — DoD-BLOCKING (standard §2 items 5, 6, 14)

**Grounded in a real prior incident.** Memory `crypto-safe-first-pass-2026-07-14.md`: the bebop
`verify_batch` shortcut was **forgeable** — an independent reviewer *built and ran* an Ed25519
mixed-order **SSR-2020** forgery that the pre-fix code wrongly accepted; it *passed the unit tests* and
was caught only because a reviewer built an actual forgery. **The fast-path is new crypto-adjacent code
(session keying, a symmetric authenticator replacing a signature) and does NOT ship until an independent
adversarial review passes.** Unit green is necessary, not sufficient.

### 8.1 Reviewer independence
Performed by an actor **not** the implementer — `system-breaker`/`security-sentinel` or a decorrelated
model. Mandate: **produce a working forgery/bypass or a proof of its impossibility**, not read-and-approve.

### 8.2 Attacks the review MUST attempt (each a concrete artifact, not a checkbox)
1. **Exporter forgery / collision** — make two channels yield the same binding (defeat §7.4). Attempt a
   cert-swap splice that survives the M1 comparison.
2. **Scope-creep** — get a MAC-only frame accepted as *anything* authority-bearing (a capability, a
   ledger append, a settlement). Attempt to reach a non-presence sink from a `FastPathFrame`.
3. **Downgrade** — force fast-path onto a store-and-forward/red-line frame, or force a weaker suite.
4. **Replay/reflection** — bypass the sliding window; reflect an i→r frame as r→i.
5. **Revocation bypass** — keep transacting on the fast path past revocation beyond the ≤30 s bound.
6. **KEM/binding tamper** — get a spliced/tampered KEM exchange to install a working session key.
7. **Key-compromise escalation** — from a leaked session key, reach anything beyond one session's
   presence frames (disprove §7.3).
8. **Timing on the tag comparison** — `recv_fastpath`'s AEAD-decrypt (`core/src/aead.rs:1`, §4.4 M4:
   "AEAD-decrypt with `key_recv`; on tag fail → reject") is currently *assumed* constant-time, not
   verified. Attempt a timing-differential probe between a near-miss and a far-miss forged tag on an
   installed session, and confirm no early-return/short-circuit on partial tag match and no
   secret-dependent branch in the compare (META-GAP-AUDIT-2026-07-19.md G5). This line is part of
   what Q3's `reviewed-by` pointer (`BLUEPRINT-Q-SERIES-VERIFICATION-OBSERVABILITY-2026-07-19.md`
   §Q3) must cite going forward.

### 8.3 Gate outcome (falsifiable)
- **PASS** = written attestation that each attack in 8.2 was *attempted with a concrete input*, each was
  rejected with the expected typed error, and any bypass found was fixed and re-attempted-and-rejected.
  Filed under `docs/reflections/`, referenced from this blueprint's DoD (D-REVIEW).
- **FAIL** = any bypass accepted, OR any attack not genuinely attempted → the fast-path is RED and does
  not ship (exactly as B4 walked back a shipped optimisation when a forgery was found).

---

## 9. DoD — falsifiable, RED→GREEN, machine-checkable (standard §2 item 2)

| # | Done when… | Falsifier (RED test / check) |
|---|---|---|
| D1 | the live RFC-5705 exporter is captured, set on send, and **compared on recv** | `red_exporter_mismatch_rejected`, `red_none_binding_rejected_when_required`, `red_mitm_cert_swap_splits_exporter` (M1) — **REGRESSION-LEDGER entry (closes F3/M1)** |
| D2 | a `FastPathFrame` cannot be decoded/handled as a `SignedFrame` (or vice-versa) | `red_fastpath_frame_not_decodable_as_signed`, `red_signed_frame_not_decodable_as_fastpath` (M2) |
| D3 | promotion requires a real full hybrid verify + eligible scope + exporter match | `red_offer_unverified_identity_rejected`, `red_offer_wrong_scope_rejected`, `red_offer_exporter_mismatch_rejected` (M3) |
| D4 | the session key binds BOTH the ephemeral ML-KEM secret AND the exporter (tamper→fallback) | `red_kem_material_tampered`, `red_downgrade_suite_strip` (M3) |
| D5 | per-frame AEAD + counter: replay rejected, bounded reorder accepted, too-old rejected, no reflection | `red_replayed_counter_rejected`, `red_reorder_within_window_accepted`, `red_too_old_counter_rejected`, `red_reflected_frame_rejected` (M4) |
| D6 | session lifetime bounded by `min(300 s, cap.expiry)`; counter exhaustion forces rekey | `red_session_expires_at_not_after`, `red_session_bounded_by_cap_expiry`, `red_counter_exhaustion_forces_rekey` (M5) |
| D7 | a revoked peer is torn down within ≤ `FASTPATH_REVOCATION_TICK_SECS` (tick) and immediately on merge | `red_revoked_peer_torn_down_on_tick`, `red_revoked_peer_torn_down_on_merge`, `red_stale_trust_window_is_bounded` (M6) |
| D8 | fast-path reaches ONLY the presence sink; allow-list excludes all red-line/store-and-forward scopes | `red_fastpath_never_reaches_ledger`, `red_allow_list_excludes_redline_and_sf`, `red_presence_record_not_evidence` (M7) |
| D9 | `BreachAlarm`/`OperatorKill`/`PolicyUpdate`/store-and-forward frames are NEVER fast-pathed | assertion tests: each such scope/kind bypasses promotion and stays on the full gate |
| D-REVIEW | **independent adversarial-review attestation exists and PASSES**, including §8.2 item 8 (constant-time AEAD tag comparison in `recv_fastpath`/`core/src/aead.rs`) | §8.3 artifact under `docs/reflections/`; FAIL ⇒ blueprint RED; an attestation silent on item 8 is incomplete |
| D-BENCH | measured presence-ping saving clears `FASTPATH_BENEFIT_THRESHOLD`, else the phase is NO-GO | §10.3 benchmark output |
| D-BUILD | both crates build & full `cargo test` green incl. all new REDs now GREEN, no dep added | `cargo test -p bebop-proto-wire -p bebop-proto-cap` |
| D-NOREG | the existing full-signed path + breach bypass tests stay green (no regression) | `quic_p2p_breach_no_hub_no_roster`, `wss_rejects_cross_channel_replay`, `RequireBoth` wire tests |

---

## 10. Benchmarks + telemetry + the measure-first gate (standard §2 item 10)

### 10.1 What to measure
| Bench | Measures | Harness |
|---|---|---|
| `bench_presence_full_signed` | `HybridGate::check` cost per `Presence/Send` frame (verify_chain + Ed25519 + ML-DSA-65) — the baseline | `cargo bench` (existing pq bench pattern) |
| `bench_presence_fastpath` | XChaCha20-Poly1305 verify + counter/window per frame — the fast path | same |
| `bench_fastpath_handshake` | one-time cost: 2× full hybrid verify + ML-KEM encaps/decaps + HKDF (amortised over N pings) | same |
| `bench_exporter_export` | `export_keying_material` cost at connect/accept | same |

### 10.2 Telemetry
Emit per-session `{pings, full_signed_cost, fastpath_cost, handshake_cost, fallbacks}` through the
existing carrier metrics seam so a regression (e.g. spurious fallbacks = a binding bug) surfaces
automatically, not at review time (item 14).

### 10.3 The measure-first NO-GO gate (D-BENCH)
Amortised saving per ping ≈ `bench_presence_full_signed − bench_presence_fastpath`, minus
`bench_fastpath_handshake / N` where N = pings per session. **Define `FASTPATH_BENEFIT_THRESHOLD` = the
break-even N below which the handshake cost dominates.** If real courier↔hub presence streams do not
sustain `N ≫ FASTPATH_BENEFIT_THRESHOLD`, **the phase is NO-GO** — the per-message model is already
correct and this adds only complexity. Measure on real hardware **before** building M2–M7; M1 (the
exporter) is worth landing regardless (it closes F3/M1).

---

## 11. Cross-cutting obligations (standard §2 items 6, 8, 9, 11–16)

- **Hazard-safety as math (item 6):** the unsafe states are made **unrepresentable**: (i) a MAC-only
  frame carrying authority — a `FastPathFrame` has no capability/chain/sig fields and no decoder into
  `SignedFrame` (M2), and no sink but `PresenceTable` (M7); (ii) a session surviving a channel change —
  the key is derived from the exporter, so a new channel *is* a new key (§5.6); (iii) a revoked peer
  transacting indefinitely — bounded by the tick (M6). Reachability argued from type/flow structure, per
  the finite-anchored-authority doctrine.
- **Schemas & scaling axis (item 8):** scaling axis = **live sessions per node** (one per hot peer) and
  **pings/sec per session**. `FastPathSession` is O(1) state per peer; the anti-replay window is a fixed
  64-bit word. It changes shape only if a node holds ≳10⁴ simultaneous hot sessions (then a slab/arena
  of sessions, not a hashmap) — stated, not timeless. Chain-length/gossip are untouched (fast-path
  frames never gossip).
- **Isolation / bulkhead (item 11):** the fast path is a **bulkhead** — its failure mode is *fallback to
  the full-signed gate*, which always exists. A fast-path bug cannot corrupt the ledger/capability plane
  because there is no code path from it to those sinks (M7). The `PresenceTable` is a throwaway sink;
  losing it loses only a liveness hint.
- **Mesh awareness (item 12):** fast-path frames are **strictly node-local to one live session** — never
  gossip-propagated, never store-and-forwarded (the whole point). Promotion frames are point-to-point,
  full-signed. Payload budget: a `FastPathFrame` is `16 + 1 + 8 + |PresenceRecord|+16` bytes ≈ tens of
  bytes — far below a mesh frame; the `FastPathOffer`/`Accept` carry a one-time ML-KEM-768 ek (~1.2 KB) +
  ct (~1.1 KB), amortised over the session.
- **Rollback / self-healing as math (item 13):** **Self-termination** = the session `not_after` /
  counter-exhaustion / channel-drop invariants (a bad session is unrepresentable past its bound, not a
  supervisor's choice). **Snapshot re-entry** = re-handshake regenerates a fresh epoch from the
  last valid identity verify. **Self-healing is NOT claimed** — a dropped ping is simply re-sent or the
  peer looks stale; there is no error-correcting recovery, and claiming it would be false.
- **Error-propagation / smart index (item 14):** the bug classes this introduces (a scope silently
  widened, a binding silently faked, a MAC silently downgraded) are turned into **compile/CI-time**
  failures: the compile-time allow-list + `red_allow_list_excludes_redline_and_sf`, the exporter
  comparison test, the suite-transcript binding, and the `red_fastpath_never_reaches_ledger` call-graph
  assertion. Not runtime surprises.
- **Living-memory awareness (item 15):** presence records are **time-scoped** (session `not_after`,
  monotonic counter) and **topology-scoped** (one peer-pair session) — deliberately *not* persisted; they
  are the opposite of living memory (ephemeral by design). Anything needing durable memory uses the
  full-signed event log.
- **Tensor/spectral (item 16):** **N/A, honestly** — session keying + a counter window is not a
  linear-algebra kernel; forcing `spectral.rs` here would be over-engineering (ponytail). Stated.
- **Linux discipline (item 9):** **EXTENDS** the existing `Transport`/`PayloadEnc` seam (fills the
  `NoopPayloadEnc` slot with a presence-scoped real impl); **REINFORCES** the fail-closed decode +
  verify-then-record patterns; **ALREADY-EQUIVALENT** on domain separation (reuses `channel_binding_hash`,
  scope discriminants); **DOES-NOT-TRANSFER** — no new daemon, no ratchet, no group keying.

---

## 12. Hermetic principles honored (standard §2 item 20 — load-bearing only)

- **Polarity / no-middle:** a frame is either full-signed-authoritative or fast-path-ephemeral — there is
  **no partial-authority MAC-only code point** that could carry a capability. The AEAD tag verifies or it
  does not; there is no degraded accept.
- **Cause & Effect:** every fast-path session has a signed *cause* — a full hybrid-verified `FastPathOffer`
  rooted in an anchor; nothing is trusted by channel or correlation (mesh red-line: capability, never a
  channel, never a score).
- **Correspondence:** the session key *is* a function of the live channel (exporter) — "as above (the
  authenticated TLS session), so below (the session key)"; a different channel is, of necessity, a
  different key. The binding is self-describing, not asserted.

---

## 13. Standard-compliance map (all 20 points — standard §2)

| # | Standard item | Where satisfied |
|---|---|---|
| 1 | Ground truth, live `file:line` | §0 (every cite verified this pass; the "binding is simulated + unenforced" and "exporter API exists" corrections) |
| 2 | Falsifiable DoD | §9 (D1–D-NOREG, each a RED→GREEN test or artifact) |
| 3 | Spec→test→code, event-driven | §4 (spec-first per M; event-sequence asserts in M5/M6) |
| 4 | Predefined types & constants | §3 (`FastPathSession`/`FastPathFrame`/`FastPathOffer`/… named before impl) |
| 5 | Adversarial/breaking tests | §4 (every M has RED adversarial cases), §7 (self-attack), §8 (forgery gate) |
| 6 | Hazard-safety from type structure | §11 (unrepresentable authority-MAC / channel-surviving session), §7.2 |
| 7 | Links to docs & memory | §14 |
| 8 | Schemas with scaling axis | §11 (sessions/node, pings/sec; window is fixed-width) |
| 9 | Linux engineering discipline | §11 (EXTENDS/REINFORCES/… verdict) |
| 10 | Benchmarks + telemetry + measure-first | §10 (incl. the D-BENCH NO-GO gate) |
| 11 | Isolation / bulkhead | §11 (fallback-to-full-signed; no path to authority sinks) |
| 12 | Mesh awareness | §11 (node-local, never gossiped; payload budget) |
| 13 | Rollback/self-heal as math | §11 (self-termination = bound; re-handshake = snapshot re-entry; self-healing NOT claimed) |
| 14 | Error-propagation / smart index | §11 (compile-time allow-list, exporter test, call-graph assertion) |
| 15 | Living-memory awareness | §11 (presence = deliberately ephemeral; durable data uses full-signed log) |
| 16 | Tensor/spectral where applicable | §11 (N/A, stated honestly) |
| 17 | Regression tracking | §9 D1 (REGRESSION-LEDGER entry for the exporter/F3-M1 closure) |
| 18 | Clear worker instructions | §14 |
| 19 | Reuse-first, upgrade-if-needed | §0.4 (all primitives in-tree), §1 (adopt not invent), §2.2 (anti-scope) |
| 20 | Hermetic principles | §12 |

---

## 14. Links to docs & memory + instructions for other agentic workers (standard §2 items 7, 18)

**Depends on / cites:**
- `docs/research/OPUS-HANDSHAKE-ONCE-VS-PERMESSAGE-2026-07-18.md` §5b (the sanctioned fast-path), §4
  (store-and-forward forbids the pure-session model), §2 (C1/C2/C3/H1/H3 lifecycle mapping).
- `docs/research/OPUS-TRUST-BOUNDARY-CLOSED-CHANNEL-SCAN-2026-07-18.md` (companion; MAC≠signature, no
  transferable authority).
- Red-team: `bebop2/docs/red-team/2026-07-13/B3-wire-transport.md` §F3 (channel binding decorative —
  the STILL-OPEN finding M1 closes), `B2-protocol-authz.md` M1.
- `CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (the 20-point contract).
- Format precedent: `BLUEPRINT-P59-capability-cert-chain.md`.
- Memory: `crypto-safe-first-pass-2026-07-14.md` (B4/SSR-2020 forgery precedent — §8), `mesh-real-arc-
  2026-07-13.md` (MESH-10 transport hardening / Layer C ML-KEM-768→XChaCha20 plan).

**Existing code this blueprint edits/extends (exact targets, bebop-repo — NOT dowiz):**
- **NEW** `bebop2/proto-wire/src/fastpath.rs` — all §3 types; promotion; KDF; AEAD; window; revocation
  tick; `PresenceTable`.
- **EDIT** `bebop2/proto-wire/src/iroh_transport.rs` + `wss_transport.rs` — capture the live exporter at
  connect/accept, store on the struct, set on send, **compare on recv** (M1); route `FastPathData` to the
  fast-path handler, everything else to the existing gate.
- **EDIT** `bebop2/proto-wire/src/frame_kind.rs` — append `FastPathOffer/Accept/Data/Close` (0x04–0x07),
  pinned + fail-closed (keep existing discriminants unchanged).
- **EDIT** `bebop2/proto-wire/src/wire_codec.rs` — `encode/decode_fastpath` disjoint from `SignedFrame`.
- **EDIT** `bebop2/proto-wire/src/transport_policy.rs` — the presence-scoped real `PayloadEnc` impl; flip
  `require_tls_channel_binding` default to `true` for prod carriers.
- **REUSE unchanged** `bebop2-core::{pq_kem,x25519,aead,kdf,hash,sign,pq_dsa}`, `proto-cap::{signed_frame,
  hybrid_gate,roster,revocation,scope}`, `handshake::channel_binding_hash`.
- **DO NOT TOUCH** the store-and-forward (`bpv7.rs`), gossip (`sync_pull.rs`), or breach
  (`iroh_transport.rs:366-389`) paths — they must stay full per-frame signed.

**For the worker with zero session context — exact acceptance path:**
1. **Land M1 first (the exporter) as its own reviewed unit** — it closes F3/M1 and is the hard
   prerequisite; do not write any fast-path code until `red_mitm_cert_swap_splits_exporter` is GREEN.
2. **Run the D-BENCH measure-first gate (§10.3).** If real presence volume does not clear
   `FASTPATH_BENEFIT_THRESHOLD`, STOP — report NO-GO; the per-message model stands.
3. Write §3 types in `fastpath.rs` first (types → tests → code — item 3); implement M2→M7 in order; each
   M's RED tests fail before its code and pass after.
4. Add the D1 (exporter/F3-M1) regression entry to `docs/regressions/REGRESSION-LEDGER.md`.
5. `cargo test -p bebop-proto-wire -p bebop-proto-cap` fully green; the breach/full-signed/`RequireBoth`
   tests (D-NOREG) must stay green.
6. **Do NOT mark P92 done until §8's independent adversarial-review attestation PASSES (D-REVIEW).** A
   green unit suite is necessary and NOT sufficient — that is the entire B4 lesson. Route the review to an
   independent reviewer (`system-breaker`/`security-sentinel` or a decorrelated model) whose job is to
   *build a forgery/bypass*, not to approve.
7. Anti-scope: never fast-path a store-and-forward/gossip/breach/control/red-line frame; never add a
   scope to `FASTPATH_ELIGIBLE_SCOPES` without a red-line review; never persist a `PresenceRecord` as
   evidence.
