# BLUEPRINT P93 — Transcript-Binding + Replay-Window for the Store-and-Forward Path (2026-07-19)

> **Standalone PROTOCOL blueprint (bebop2 `proto-cap` + `mesh-node`).** One coherent, independently
> buildable unit against the 20-point contract in `CORE-ROADMAP-STANDARD-2026-07-17.md` §2. Scope
> source: `SYNTHESIS-MESH-MAJOR-REFACTOR-PLAN-2026-07-19.md` §4 (the P90/P91-style scope stub this
> pass expands to the full contract). Placement law: `docs/research/OPUS-CORE-CONSOLIDATION-AUDIT-2026-07-19.md`
> §3 (transcript → `proto-cap/signed_frame.rs`; replay window → `mesh-node`; core stays clock-free).
> Format precedent + direct sibling: `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`. Grounding
> tree: `/root/bebop-repo/bebop2` at HEAD, read live this pass (§0). Dependency sequence:
> **M1 → P93 → P92 → P94** (this is P93).
>
> **One sentence:** for **every** per-message-signed frame that travels *detached from any live channel*
> (the store-and-forward / gossip / breach default path — the mesh's dominant workload), bind the
> signature to the intended **receiver + freshness context** — `Sign(SK, Hash(ReceiverID ‖ Nonce ‖
> Timestamp ‖ Data))` — and reject at the receiver any nonce already seen for that sender through a
> bounded, expiry-pruned **`mesh-node`-owned replay window**, closing red-team attack class **C3
> (cross-node replay)** on the path M1 and P92 structurally cannot cover.

---

## VERDICT (stated up front, per session research discipline)

**GO — with two operator-decidable forks closed to concrete recommendations, and one hard build gate.**
Unlike P92 (an *optional, measure-first* throughput overlay), P93 is **unconditional correctness
hardening** of the always-on signed path: it closes a *still-open* red-team replay defect (C3,
`B2-protocol-authz.md` row 3, **STILL OPEN** — verified §0.2). It is worth building regardless of
P92, and it supplies the very cross-node nonce ledger P92 §2.2 explicitly disclaims.

Two forks the synthesis surfaced are **resolved here, not re-listed** (§5):

1. **D-93-A (privacy fork — plaintext vs blinded ReceiverID): RECOMMEND BLINDED (ephemeral per-bundle
   tag), fully specified (§5.1, M3). Operator-gated** because the plaintext option is materially
   simpler to debug. Both are specified end-to-end; the blueprint takes **no silent default** — the
   operator picks, and both build items are written so either can land.
2. **D-93-C (broadcast/multicast): CLOSED (§5.2), not deferred.** A three-way split by recipient
   cardinality — unicast (default transcript binding), small-known multicast (`N ≤
   MULTICAST_PER_RECIPIENT_MAX` → per-recipient signed copies), true broadcast (a signed
   **broadcast-sentinel** `ReceiverID` binding `Nonce ‖ Timestamp ‖ Data` but not a recipient, with
   duplicate suppression by the replay window). The **shared-group-key option is explicitly REJECTED**
   (non-repudiation loss — violates the mesh red-line "trust = signed capability, never a shared
   secret").

**Hard build gate (like P92's C1/§8):** because P93 **changes what bytes are signed** on the
authoritative path, it does not ship until (a) the **versioned signing-domain discriminant** (M1,
append-only, fail-closed) is GREEN so old/new frames are unambiguous, and (b) an **independent
adversarial review** (§8, the B4/SSR-2020 rigor) has *built* a cross-node replay that survives the new
transcript, a topology-deanonymization against the blinded tag, and a broadcast-frame replay — or
proved their impossibility. Unit-green is necessary, not sufficient.

The one honestly-stated residual: true-broadcast frames (§5.2 tier 3) get **freshness + authorship**
binding but **not** per-recipient cross-node-replay protection — *by definition*, because a broadcast
is intended for many nodes, so "replay to another node" is partly its purpose. Their anti-replay is the
`Nonce/Timestamp` freshness window + per-node duplicate suppression, not a per-recipient signature.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

> "Ground truth is non-discussible." Every `file:line` below was read from source **this pass**
> (`/root/bebop-repo/bebop2`, HEAD, 2026-07-19), not inherited from the S4/S5 research sketch. Where a
> line differs from the synthesis shorthand by ≤3 lines it is corrected here silently to the live value.

### 0.1 The signed frame today: channel-binding exists; recipient/freshness binding does NOT

| Element | Cite | State |
|---|---|---|
| `SignedFrame.channel_binding: Option<[u8;32]>` | `signed_frame.rs:87` | The **F7 slot** — "SHA3-256 handshake transcript" (`:45`); `None` = legacy/unbound (`:169`). |
| `signing_domain()` = canonical TLV over `(capability ‖ payload [‖ channel_binding])` | `signed_frame.rs:144-162` | The signed byte domain; `channel_binding` appended at `:152` only if `Some`. |
| `binding_signing_domain()` = `signing_domain()` ++ 32-byte binding slot | `signed_frame.rs:172-177` | Both legs sign this: `sign_classical`/`verify_classical` (`:185+`), `sign_pq`/`verify_pq` over `binding_signing_domain()` (`:191+`, header `:36`). |
| **ReceiverID / Timestamp in the signed domain** | grep: **absent** | The domain binds *sender capability + payload + channel*, **never the intended recipient or a wall-clock freshness field**. This is the P93 gap. |
| `NodeId(pub [u8;32])` — the identity that would be the `ReceiverID` | `node_id.rs:42` | The concrete `ReceiverID` type; `subject_key` (sender) is the same 32-byte shape. |

So the transcript-binding construction is a **generalization of the pattern already at
`signed_frame.rs:144-177`** — extend the domain with `ReceiverID` + `Timestamp` as new canonical TLV
fields, exactly where `channel_binding` already lives. The hash primitive is already centralized:
`bebop2_core::hash::sha3_256` (`core/src/hash.rs:344`) — **no core change** (S5 §3.1, re-verified).

### 0.2 The replay ledger today: per-instance, rebuilt per connection — C3 is STILL OPEN

| Element | Cite | State |
|---|---|---|
| `HybridGate.seen: Mutex<HashSet<[u8;8]>>` | `hybrid_gate.rs:67` | The ONE piece of live gate state; **per-gate-instance**, in-process only (`:37-38`). |
| bounded + pruned to `MAX_SEEN_NONCES` | `hybrid_gate.rs:201-204` | Correctly bounded, but **only within one gate**. |
| verify-then-record ordering (H2 fix) | `hybrid_gate.rs:171` (`verify_classical`) → `:192-195` (`seen.insert` *after*) | **Correct today** — the nonce is inserted only after the signature verifies. P93 must **preserve** this ordering when relocating `seen`. |
| **cross-node / cross-instance replay** | red-team `B2-protocol-authz.md` **row 3, STILL OPEN** | Verbatim: "every `connect`/`accept` builds a fresh gate … → empty `seen` on each connection/node. PoC: NODE 2 (fresh gate) accepts identical replayed bytes." The remediation #2 (`B2-protocol-authz.md`): "Move `seen` out of the per-connection gate into a bounded, expiry-pruned window keyed by `(subject_key, nonce)` shared across connections; insert only *after* `verify_classical` succeeds." **This remediation IS P93's replay window.** |

**Consequence:** the same signed bytes captured off one node are accepted verbatim by a *different*
node (or the same node on a fresh connection), because (a) the signature commits only to
`(capability ‖ payload ‖ channel_binding)` and for a detached bundle `channel_binding` is `None`
(there is no shared live channel), and (b) `seen` is empty on every fresh gate. A per-session counter
(P92) cannot see a replay that *leaves* the session — this is the one place a cheap per-session counter
is provably insufficient (S2 §2, C3 row).

### 0.3 The detached paths that need this (the frame population P93 protects)

| Path | Cite | Why it is channel-less |
|---|---|---|
| BPv7 store-and-forward overlay | `proto-wire/src/bpv7.rs` (exists) | Offline-authored bundles reach peers never in any handshake — authentication must travel *with the frame* (S2 §4). |
| Anti-entropy pull gossip | `proto-wire/src/sync_pull.rs` (exists) | Frames propagate node→node→node; no single live channel binds author to final recipient. |
| Breach broadcast (self-signed P2P fail-safe) | `iroh_transport.rs:366-389` | A breach alarm is a **deliberately self-signed, roster-bypassing, widely-accepted** frame — the D-93-C "true broadcast" archetype. |

### 0.4 Every primitive P93 needs is already in-tree — zero new deps (standard §2 item 19)

| Need | In-tree primitive | Cite |
|---|---|---|
| transcript hash | **SHA3-256**, zero-dep | `core/src/hash.rs:344` (`sha3_256`) |
| blinded-tag key agreement (D-93-A blinded) | **ML-KEM-768 (FIPS-203)** encapsulate/decapsulate | `core/src/pq_kem.rs` |
| blinded-tag KDF | in-tree **KDF** | `core/src/kdf.rs` |
| canonical TLV field codec | `proto-cap::tlv` (`DOMAIN_*`, length-prefixed) | `capability.rs:22`, red-team row 6 (**FIXED** — TLV is the canonical signed form) |
| sender/receiver identity | `NodeId([u8;32])` | `node_id.rs:42` |
| replay-window clock source | caller-supplied `now: u64` (mesh-node threads it) | `hybrid_gate.rs` gate already takes `now` |

P93 **adds no dependency, invents no primitive, and does not touch `bebop2-core`** (§6 anti-scope). It
composes existing KAT-gated crypto behind the existing `signing_domain()` / `tlv` seams and adds one
new `mesh-node` state struct.

---

## 1. Prior-art map — adopt, don't invent (standard §2 item 19)

| Prior art | What it is | How P93 uses it — and what it does NOT take |
|---|---|---|
| **DTN Bundle Protocol Security (RFC 9172 BPSec) — integrity block over a canonical bundle** | integrity/auth that travels *inside the bundle*, not with a channel, for delay-tolerant store-and-forward | **Adopt the shape** — authentication binds to bundle-intrinsic fields (recipient, creation timestamp, payload), so a bundle authored offline authenticates at a peer never handshaked with. **NOT taken:** BPSec's block-structure/cipher-suite registry — bebop2 already has its own TLV + AlgSuite discipline; P93 extends *that*, not BPSec's. |
| **IPsec / DTLS 1.3 anti-replay window (RFC 4303 §3.4.3, RFC 9147)** | monotonic sequence + sliding bitmap: accept newer, tolerate bounded reorder, reject too-old/duplicate | **Adopt the *intent*** (reject replays, tolerate benign reorder) but **NOT the fixed-width sequence bitmap** — DTN reorder is *unbounded* (a frame authored hours ago can arrive after newer ones), so a 64-wide bitmap is wrong. P93 uses a **timestamp-expiry-pruned nonce set** (§3, M4) — the correct DTN analog. Stated, not shoehorned (contrast P92, which correctly uses the bitmap *because* its frames are live-channel). |
| **Sealed-sender / HPKE anonymous recipient (Signal sealed-sender, RFC 9180)** | hide *who* a message is for from a relay, while the intended recipient can still recognize it | **Adopt for D-93-A blinded** (§5.1): a per-bundle ML-KEM encapsulation yields a shared secret only the true receiver recovers; the recipient tag `Hash(ss ‖ receiver_id)` is unlinkable to an observer. **NOT taken:** HPKE's full seal/open of the *payload* — P93 blinds only the *recipient tag*, never the signature (non-repudiation preserved: any verifier still checks the signature over the carried tag). |
| **TLS / FrameKind / AlgSuite append-only versioning (fail-closed on unknown)** | a pinned discriminant so old and new formats are unambiguous and unknown code points are rejected | **Adopt verbatim** for the signing-domain version (M1): `SigningDomainVersion` byte, append-only, unknown → reject. Mirrors `frame_kind.rs` discipline (P92 §3). |
| **Domain separation (per-structure `DOMAIN_*` tags)** | a hash/signature over structure A can never be reinterpreted as structure B | **Adopt** — `DOMAIN_SIGNED_FRAME_V2` and `DOMAIN_RECIPIENT_TAG` are distinct pinned tags (red-team row 6 proved the existing per-type tags close the cross-structure reuse class). |
| **A shared group key for multicast** | one symmetric key all group members hold; one MAC covers the group | **REJECTED, not adopted** (§5.2). A group key is forgeable by *any* member → loses non-repudiation → violates the mesh red-line ("trust = signed capability, NEVER a shared secret"; MEMORY `sovereign-event-exchange`). Recorded as an explicit rejection so a future author does not re-propose it. |

---

## 2. Scope — what P93 OWNS vs deliberately does NOT (standard §2 items 11, 18, 19)

### 2.1 P93 OWNS

1. **The versioned signing-domain discriminant** (M1) — `SigningDomainVersion` V1 (existing) / V2 (P93),
   append-only, fail-closed, with a clean migration window.
2. **The transcript binding** (M2) — extend `signing_domain()` to bind `ReceiverID` + `Timestamp` as new
   canonical TLV fields; `Sign(SK, Hash(ReceiverID ‖ Nonce ‖ Timestamp ‖ Data))` on **both** the
   classical and PQ legs. This is the *plaintext* recipient form.
3. **The blinded recipient tag** (M3) — the D-93-A blinded fork, fully specified end-to-end (ephemeral
   per-bundle ML-KEM encapsulation → unlinkable tag). **Landed only if the operator rules "blinded"**;
   otherwise M3 is not built and M2's plaintext form stands.
4. **The `mesh-node` replay window** (M4) — a bounded, expiry-pruned, `(subject_key, nonce)`-keyed set
   shared across connections, relocated out of the per-instance `HybridGate.seen`.
5. **Broadcast / multicast handling** (M5) — the D-93-C resolution: the `MULTICAST_PER_RECIPIENT_MAX`
   split + the `BROADCAST_RECEIVER` sentinel + its duplicate-suppression semantics.
6. **The mandatory independent adversarial-review gate** (§8) as a DoD-blocking checkpoint.

### 2.2 P93 does NOT own (anti-scope — prevents collision & scope-creep)

- **The live-channel fast-path (P92).** P93 and P92 protect **disjoint frame populations** (S3, S2 §4):
  P93 hardens the *channel-less detached* path; P92 optimizes the *live continuously-online* stream.
  P93 supplies the cross-node ledger P92 §2.2 disclaims; it never touches `fastpath.rs`.
- **The RFC-5705 exporter (M1-of-the-cluster / P92 C1).** That is the *channel-bound* analog for the
  path that *has* a channel. P93's transcript is `ReceiverID`-anchored, **not** exporter-anchored — it is
  the store-and-forward analog for the path that has none. P93 edits the same `signed_frame.rs`
  signing-domain surface, so it sequences **after** the exporter fix to keep that surface single-writer
  (§4.7), but it is functionally independent.
- **`bebop2-core`.** The hash/KEM/KDF primitives are already there (§0.4); P93 owns only the *layout* and
  the *stateful window*, both above core (S5 §3/§6). Core stays clock-free by contract.
- **The delegation lattice / revocation / red-line gate** (`roster.rs`, `revocation.rs`, `redline.rs`) —
  authorization policy, untouched. P93 strengthens *what is signed*, not *who may act*.
- **Removing per-frame signing anywhere.** S1/S2 proved it load-bearing; P93 only *adds* bound fields to
  the signed domain (M2/M3) — it never removes a signature (§5.2 tier 3 keeps the full breach signature).

### 2.3 Dependencies (named by artifact — standard §2 item 7)

**Hard inputs (in tree):** `SignedFrame::{signing_domain, binding_signing_domain, sign_classical,
verify_classical, sign_pq, verify_pq}` (`signed_frame.rs`); `proto-cap::tlv` (canonical codec);
`NodeId` (`node_id.rs`); `bebop2_core::{hash::sha3_256, pq_kem, kdf}` (§0.4); the `HybridGate.seen`
ledger being relocated (`hybrid_gate.rs:67,192-204`); the node runtime that will own the window
(`mesh-node/src/node.rs`).

**Soft prerequisite:** the cluster's **M1 exporter fix** (shared `signed_frame.rs` surface; §4.7). Not a
functional block — P93 is `ReceiverID`-anchored — but sequenced after M1 to keep the shared signing
surface single-writer.

**Consumers:** every recv path that ingests a store-and-forward / gossip / breach frame — after P93 they
call the relocated window and verify against the extended (V2) signing domain.

### 2.4 Honest reconciliation (standard §2 item 6)

P93 does **not** overturn any prior verdict. It is the *positive* half of the S2 finding: per-message
signing is structurally required for store-and-forward, and P93 makes that required signature *bind more*
— the receiver and freshness context — so the signature a store-and-forward frame already carries also
defeats cross-node replay. The default remains per-message signing; P93 strengthens it in place.

---

## 3. Predefined types & constants — named BEFORE implementation (standard §2 item 4)

Transcript types extend `proto-cap/src/signed_frame.rs` + `proto-cap/src/tlv.rs`; the replay window is a
**new module `mesh-node/src/replay_window.rs`** (keeps the stateful, clock-relative concern out of the
clock-free crates — S5 §3.2). Constants are named, never magic.

```rust
// proto-cap/src/signed_frame.rs (EXTEND) + proto-cap/src/tlv.rs (EXTEND)

/// Signing-domain version. Append-only; unknown => REJECT (fail-closed), never best-effort.
/// Mirrors FrameKind/AlgSuite discipline. This is the M1 migration discriminant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SigningDomainVersion {
    V1ChannelBound    = 0x01,   // existing: (capability ‖ payload ‖ channel_binding)
    V2TranscriptBound = 0x02,   // P93: adds RecipientBinding + Timestamp to the signed domain
    // V3+ reserved. A verifier that sees an unknown byte returns CapError::UnknownDomainVersion.
}

/// Exactly one recipient binding per V2 frame. The discriminant is bound into the signed domain,
/// so a frame cannot be reinterpreted across binding modes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecipientBinding {
    /// D-93-A PLAINTEXT: the receiver's NodeId in the clear. Simple + debuggable; leaks topology.
    Plain(NodeId),                                   // [u8;32]
    /// D-93-A BLINDED: an unlinkable per-bundle tag + the ML-KEM ciphertext the receiver
    /// decapsulates to recompute it. Hides topology from an observing relay (§5.1).
    Blinded { tag: [u8; 32], kem_ct: Vec<u8> },
    /// D-93-C TRUE-BROADCAST sentinel: binds Nonce‖Timestamp‖Data but NOT a recipient (§5.2 tier 3).
    Broadcast,                                        // wire value == BROADCAST_RECEIVER
}

/// Reserved sentinel NodeId — never a real enrolled node (all-ones is not a valid Ed25519 point id
/// under enrollment; asserted at genesis). Used for the D-93-C true-broadcast binding.
pub const BROADCAST_RECEIVER: [u8; 32] = [0xFF; 32];

/// Domain-separation tags (distinct so a hash over one can never be reused as another).
pub const DOMAIN_SIGNED_FRAME_V2: &[u8] = b"bebop2.p93.signed-frame.v2";
pub const DOMAIN_RECIPIENT_TAG:   &[u8] = b"bebop2.p93.recipient-tag.v1";

/// New canonical TLV field tags (append-only in the proto-cap tlv registry; do NOT reuse 0xFF —
/// that is the existing channel_binding field tag, signed_frame.rs:136).
pub const FIELD_DOMAIN_VERSION: u8 = 0x04;
pub const FIELD_RECIPIENT:      u8 = 0x05;   // NodeId | blinded tag | broadcast sentinel
pub const FIELD_TIMESTAMP:      u8 = 0x06;   // u64 LE creation time (DTN "bundle creation timestamp")
pub const FIELD_KEM_CT:         u8 = 0x07;   // present iff RecipientBinding::Blinded
```

```rust
// mesh-node/src/replay_window.rs (NEW) — node-owned, shared across connections, clock-relative.

/// Bounded, expiry-pruned anti-replay window keyed by (subject_key, nonce). Replaces the
/// per-connection HybridGate.seen (hybrid_gate.rs:67). The node owns ONE of these and passes
/// &mut to the gate; the gate no longer manufactures its own.
pub struct ReplayWindow {
    per_sender: HashMap<[u8; 32], SenderSeen>,       // subject_key -> its seen nonces
    total_nonces: usize,                              // global bound accounting
}

struct SenderSeen {
    seen: HashMap<[u8; 8], u64>,                      // nonce -> creation timestamp
    newest_ts: u64,
}

/// DTN-appropriate freshness + replay policy. Values are engineering decisions (§5.3).
pub const REPLAY_HORIZON_SECS:        u64   = 24 * 3600;   // accept frames whose ts is within 24h of now
pub const REPLAY_CLOCK_SKEW_SECS:     u64   = 300;         // tolerate +5min future ts (sender clock skew)
pub const REPLAY_MAX_SENDERS:         usize = 100_000;     // LRU-evict coldest sender past this
pub const REPLAY_MAX_NONCES_PER_SENDER: usize = 65_536;    // per-sender bound; prune-by-ts first
pub const REPLAY_MAX_TOTAL_NONCES:    usize = 8_000_000;   // global memory ceiling (~ REPLAY horizon * rate)

#[derive(Debug, PartialEq, Eq)]
pub enum ReplayError { Replay, Stale, FutureBeyondSkew }

impl ReplayWindow {
    /// VERIFY-THEN-RECORD: call ONLY after the frame's signature has verified (preserves the H2
    /// ordering, hybrid_gate.rs:171 before :192). `now` is supplied by mesh-node's clock (core is
    /// clock-free; the window is not). Rejects a replayed nonce or a ts outside the freshness horizon.
    pub fn admit(&mut self, subject: &[u8; 32], nonce: [u8; 8], ts: u64, now: u64)
        -> Result<(), ReplayError> { /* §M4 */ unimplemented!() }

    /// Prune entries older than now - REPLAY_HORIZON_SECS; enforce the three bounds. O(evicted).
    fn prune(&mut self, now: u64) { /* §M4 */ }
}

/// D-93-C multicast split point: at or below this recipient count, emit per-recipient signed copies
/// (each a full unicast-bound V2 frame); above it, use the broadcast sentinel (§5.2).
pub const MULTICAST_PER_RECIPIENT_MAX: usize = 16;
```

---

## 4. Build items — spec → RED test → code, each with adversarial cases (standard §2 items 2, 3, 5)

Each item: **spec first, a test that goes RED before the change, code, then GREEN.** State transitions
are modeled as events (author→sign→gossip→recv→verify→record); tests assert on the sequence (item 3).

### 4.1 M1 — versioned signing-domain discriminant (append-only, fail-closed) — the migration gate

- **Spec:** add `FIELD_DOMAIN_VERSION` (0x04) as the **first** canonical TLV field in `signing_domain()`.
  `V1ChannelBound` reproduces today's exact bytes (existing frames unaffected — the field is written but
  a V1 verifier that predates P93 never sees a V2 frame because the version gates first). `V2TranscriptBound`
  selects the extended domain (M2/M3). A verifier reads the version field first; an **unknown** value →
  `CapError::UnknownDomainVersion` (fail-closed), **never** a best-effort parse of the remaining bytes.
- **RED `red_unknown_domain_version_rejected`:** a frame whose version byte is `0x03` (reserved) →
  `UnknownDomainVersion`, no signature check attempted. RED today (no version field), GREEN after.
- **RED `red_v1_bytes_unchanged`:** a **signature-stability KAT** — a fixed V1 frame's `signing_domain()`
  bytes are byte-for-byte identical before and after M1 lands (the version field on a V1 frame must not
  perturb the historical signed bytes; if it does, all existing signatures break). This is the migration
  correctness proof.
- **Adversarial `red_version_downgrade_rejected`:** take a V2 frame, flip its version byte to `V1` while
  keeping the V2 body → the V1 domain reconstruction omits `ReceiverID`/`Timestamp`, so the recomputed
  bytes differ from what was signed → signature fails (a downgrade cannot strip the recipient binding).

### 4.2 M2 — transcript binding (plaintext ReceiverID form) in `signed_frame.rs`

- **Spec:** for a V2 frame, `signing_domain()` appends, after the capability+payload, the canonical TLV
  fields `FIELD_RECIPIENT` (the `NodeId`), `FIELD_TIMESTAMP` (u64 LE), in a fixed order, then the existing
  `channel_binding` slot (which is `None`/absent for detached frames). The signed hash is therefore
  `Hash(DOMAIN_SIGNED_FRAME_V2 ‖ capability_tlv ‖ ReceiverID ‖ Timestamp ‖ payload)` — the operator's
  `Sign(SK, Hash(ReceiverID ‖ Nonce ‖ Timestamp ‖ Data))` with `Nonce` already carried inside
  `capability_tlv` (the existing per-frame nonce, `hybrid_gate.rs` reads `capability.nonce`) and `Data` =
  `payload`. Both `sign_classical`/`verify_classical` and `sign_pq`/`verify_pq` commit to it (they already
  sign `binding_signing_domain()`, which is built on `signing_domain()`).
- **RED `red_cross_node_replay_rejected` (the C3 closer):** author a frame for receiver B; replay the exact
  bytes at node C. C reconstructs `signing_domain()` with **its own** `NodeId` as `ReceiverID` → the
  recomputed hash differs from what the sender signed for B → `verify_classical`/`verify_pq` fail →
  rejected. RED today (C3 STILL OPEN, §0.2), GREEN after. **REGRESSION-LEDGER entry — closes C3 for the
  store-and-forward path.**
- **RED `red_stale_timestamp_rejected_at_recv`:** a frame with `Timestamp` older than `now -
  REPLAY_HORIZON_SECS` is rejected by the M4 window even if the signature is valid (freshness bound). RED
  today (no timestamp binding), GREEN after.
- **Adversarial `red_receiver_field_tamper`:** flip the `FIELD_RECIPIENT` bytes in transit to point at C →
  the recomputed hash differs from the signature → fail. Proves the recipient is *signed*, not advisory.
- **Adversarial `red_timestamp_field_tamper`:** advance `FIELD_TIMESTAMP` to stay "fresh" → signature fails
  (the timestamp is inside the signed domain; a relay cannot refresh a stale frame).

### 4.3 M3 — blinded recipient tag (the D-93-A blinded fork, fully specified) — **conditional on operator ruling**

- **Spec (built ONLY if D-93-A rules "blinded"; §5.1):**
  1. Each node enrolls an **ML-KEM-768 encapsulation key** alongside its Ed25519/ML-DSA anchor identity (a
     genuine new enrollment field — a real, flagged cost; §5.1).
  2. Sender at author time: `(kem_ct, ss) = pq_kem::encapsulate(receiver_mlkem_ek, entropy)`.
  3. `tag = sha3_256(DOMAIN_RECIPIENT_TAG ‖ ss ‖ receiver_anchor_id)`.
  4. `RecipientBinding::Blinded { tag, kem_ct }` replaces `Plain(NodeId)` in the signed domain: the signed
     hash binds `tag` (not the cleartext id): `Hash(DOMAIN_SIGNED_FRAME_V2 ‖ capability_tlv ‖ tag ‖
     Timestamp ‖ payload)`; `kem_ct` is carried in `FIELD_KEM_CT` (bound into the domain so it cannot be
     swapped).
  5. Receiver on candidate frame: `ss' = pq_kem::decapsulate(kem_sk, kem_ct)`; `tag' = sha3_256(
     DOMAIN_RECIPIENT_TAG ‖ ss' ‖ own_anchor_id)`; **accept-as-recipient iff `tag' == tag`**, then verify
     the signature over the carried `tag`. A non-recipient node computes a different `ss'` (ML-KEM implicit
     rejection) → `tag' ≠ tag` → recognizes "not for me" → relays but does not deliver.
- **RED `red_blinded_only_recipient_recognizes`:** three nodes A(sender)→B(recipient)→C(relay). B's `tag'`
  matches and it delivers; C's `tag'` differs and it does not deliver. RED (no blinded path), GREEN after.
- **RED `red_blinded_unlinkable`:** two frames from A to B carry **different** `kem_ct` (ephemeral per
  bundle) → **different** `tag` → an observer cannot tell they share a recipient. Asserts unlinkability
  (the whole point vs the plaintext leak). A *static* blinded tag would fail this test (§5.1 records why
  ephemeral is recommended over static).
- **Adversarial `red_blinded_cross_node_replay_rejected`:** capture A→B's frame, replay at C. C's `tag' ≠
  tag` → not delivered; and if an attacker swaps `kem_ct` to one C can decapsulate to `tag`, the swapped
  `kem_ct` is bound into the signed domain → signature fails. C3 closed under blinding too.
- **Adversarial `red_blinded_signature_still_third_party_verifiable`:** a relay with no knowledge of B's key
  can STILL verify the signature over the carried `tag` (authorship/non-repudiation preserved — only the
  *identity* behind the tag is hidden). Proves the blinded fork keeps the non-repudiation red-line (§5.4).

### 4.4 M4 — `mesh-node` replay window (relocate `seen`; DTN timestamp-expiry-pruned)

- **Spec:** create `ReplayWindow` (§3) owned by the node; the gate takes `&mut ReplayWindow` instead of
  owning `Mutex<HashSet>`. `admit(subject, nonce, ts, now)`: (1) if `ts > now + REPLAY_CLOCK_SKEW_SECS` →
  `FutureBeyondSkew`; (2) if `ts < now - REPLAY_HORIZON_SECS` → `Stale`; (3) if `nonce ∈ seen[subject]` →
  `Replay`; (4) else insert `(nonce → ts)`, `prune(now)`, enforce the three bounds (per-sender, per-total,
  LRU-evict coldest sender). **Called only after `verify_classical` succeeds** (preserve H2 ordering,
  §0.2). Uses a **timestamp-expiry-pruned set**, NOT a fixed sequence bitmap, because DTN reorder is
  unbounded (§1, IPsec row).
- **RED `red_cross_instance_replay_rejected` (the C3 ledger closer):** the red-team PoC — NODE 1 accepts a
  frame; NODE 2 (fresh connection, but the **same node-owned window**) rejects the identical replayed
  bytes. RED today (`seen` is per-instance, §0.2), GREEN after. **REGRESSION-LEDGER entry.**
- **RED `red_delayed_frame_within_horizon_accepted`:** a frame authored 6h ago (well inside the 24h
  horizon) arriving after newer frames is **accepted** (DTN benign delay — the reason a strict
  monotonic-greater rule is wrong). Proves the set-with-expiry over the sequence-bitmap choice.
- **RED `red_too_old_frame_rejected`:** a frame authored 25h ago (past the horizon) → `Stale`. Bounds the
  set's memory.
- **RED `red_future_timestamp_rejected`:** `ts = now + 1h` (beyond the 5-min skew) → `FutureBeyondSkew`
  (prevents a sender from parking a frame "fresh forever" to bloat the window).
- **Adversarial `red_window_memory_bounded`:** flood 10^7 distinct nonces → the window stays ≤
  `REPLAY_MAX_TOTAL_NONCES` (prune + LRU evict); assert no unbounded growth. The bulkhead (§11).
- **Adversarial `red_verify_before_record_preserved`:** a frame that fails `verify_classical` must **not**
  insert its nonce (else an attacker poisons the window with a victim's nonce to DoS the real frame).
  Assert the ordering holds after relocation (the H2 regression guard).

### 4.5 M5 — broadcast / multicast (the D-93-C resolution) — see §5.2 for the ruling

- **Spec:** the sender classifies by recipient cardinality: **unicast** (1 recipient) → M2/M3 as-is;
  **small-known multicast** (`2 ≤ N ≤ MULTICAST_PER_RECIPIENT_MAX`) → emit `N` independent V2 frames, each
  bound to one recipient (per-recipient signed copies); **true broadcast** (`N >
  MULTICAST_PER_RECIPIENT_MAX` or unbounded/anonymous audience, e.g. breach) → one V2 frame with
  `RecipientBinding::Broadcast` (`BROADCAST_RECEIVER`), signature binds `Nonce ‖ Timestamp ‖ Data` but not
  a recipient; replay defense is the M4 freshness window + per-node nonce dedup (a node that already saw
  this `(sender, nonce)` drops the duplicate broadcast).
- **RED `red_multicast_per_recipient_isolated`:** a 3-recipient multicast → 3 frames; a copy for B replayed
  at C is rejected (each copy is unicast-bound, C3-protected). RED today, GREEN after.
- **RED `red_broadcast_sentinel_accepted_by_many`:** a `Broadcast` frame is accepted by B, C, D (its
  *purpose*) — but a **second** copy with the same `(sender, nonce)` at any node is dropped by M4 (dedup).
  Proves broadcast gets freshness + authorship without per-recipient binding.
- **RED `red_broadcast_sentinel_is_reserved`:** a real enrolled node with id `BROADCAST_RECEIVER` is
  rejected at genesis (`red_genesis_rejects_broadcast_sentinel_id`) — the sentinel can never collide with a
  real recipient.
- **Adversarial `red_group_key_option_absent`:** a compile/CI assertion that **no shared-group-key code path
  exists** — a `Broadcast` frame carries a full asymmetric signature, never a symmetric group MAC. Encodes
  the §5.2 rejection as an executable boundary (a future author cannot silently add a group key).

### 4.6 M6 — enforcement wiring (make the new domain actually checked on recv)

- **Spec:** the store-and-forward / gossip / breach recv paths (`bpv7.rs`, `sync_pull.rs`,
  `iroh_transport.rs:366-389`) must: (1) select the signing domain by the frame's `SigningDomainVersion`;
  (2) for V2, reconstruct `ReceiverID`/`Timestamp` and verify; (3) call the node's `ReplayWindow::admit`
  after verify. This closes the red-team observation that today the recv path *has* the binding fields but
  **never checks them** (row 3: "the `recv` path never sets or checks it").
- **RED `red_recv_actually_enforces_v2`:** a V2 frame with a valid signature but a `ReceiverID` for a
  different node is rejected *on the live recv path* (not just in a unit call to `verify_classical`). Proves
  enforcement is wired, not merely available (the exact gap M1-of-cluster closes for the channel path).
- **RED `red_no_regression_on_v1_recv`:** existing V1 frames still verify and admit unchanged (migration
  window). Ties to `red_v1_bytes_unchanged` (M1).

### 4.7 Sequencing note (single-writer on the shared surface)

M2/M3 edit `signed_frame.rs::signing_domain()` — the **same surface** the cluster's M1 exporter fix edits.
Land **M1 (exporter) → then P93** so the shared signing domain has one writer at a time (§2.2). P93 is
functionally independent (ReceiverID-anchored, not exporter-anchored) and could run concurrently by
careful authors, but the default is sequential to avoid a merge on the authoritative signing surface.

---

## 5. The two open forks — RESOLVED (the heart of this blueprint)

### 5.1 D-93-A — privacy fork: plaintext vs blinded ReceiverID → **RECOMMEND BLINDED (ephemeral), operator-gated**

**Both options, stated fully so the operator picks with the real tradeoff in hand:**

| | **Plaintext `ReceiverID`** (M2 only) | **Blinded, ephemeral tag** (M3) |
|---|---|---|
| Construction | `Hash(ReceiverID ‖ Nonce ‖ Timestamp ‖ Data)`; `ReceiverID` is the `NodeId` in the clear. | `Hash(tag ‖ Nonce ‖ Timestamp ‖ Data)`, `tag = SHA3-256(ss ‖ receiver_id)`, `ss` from a per-bundle ML-KEM-768 encapsulation carried as `kem_ct`. |
| C3 (cross-node replay) closed? | **Yes** — replay at C: C's `ReceiverID` ≠ B's → reject. | **Yes** — replay at C: C's `tag' ≠ tag` → not delivered; swapped `kem_ct` breaks the signature. |
| Topology privacy vs an observing relay | **Leaks** — a semi-trusted relay reads *who talks to whom* (a real metadata leak on a sovereignty-first mesh with future `.onion`/anonymity tiers, fold-in L4/P53). | **Hidden** — the relay sees `tag` + `kem_ct` but cannot derive `ss`, so cannot link a frame to any recipient identity. |
| Unlinkability across frames | N/A (identity is in the clear). | **Yes** — ephemeral `kem_ct` per bundle → different `tag` each time; an observer cannot even group frames by shared recipient. (A *static* blinded tag `Hash(receiver_id ‖ pairwise_secret)` would be linkable — the same tag every time — so **ephemeral is recommended over static**.) |
| Cost | Zero extra bytes; trivial to debug (recipient visible in logs). | +~1.1 KB `kem_ct` per bundle; one ML-KEM decaps per recipiency-check at each node; a new **enrolled ML-KEM-768 key** per node. |
| Non-repudiation | Preserved. | **Preserved** — the signature is over the carried `tag`; any third party still verifies authorship, only the *identity behind the tag* is hidden (M3 `red_blinded_signature_still_third_party_verifiable`). |

**Recommendation: adopt the blinded, ephemeral tag.** Topology-privacy is load-bearing for a sovereign
mesh whose whole premise is operator-sovereignty, and the required KDF/KEM are already in-tree (§0.4). The
ephemeral variant additionally gives unlinkability at the cost of one carried ciphertext per bundle —
worth it for a network that aspires to anonymity tiers.

**But this is an explicit operator decision point (OD-12), and the blueprint takes no silent default**
(MEMORY `never-bypass-human-gates`), because: (a) plaintext is materially simpler to debug and operate;
(b) the blinded option adds a **new enrolled ML-KEM-768 key** to every node's identity — a genuine
enrollment/protocol surface the operator may not want yet; (c) the +1.1 KB/bundle bandwidth is non-trivial
in a DTN. **Both M2 (plaintext) and M3 (blinded) are specified end-to-end above; the operator rules, and
exactly one lands.** If "plaintext," M3 is not built and M2 stands; if "blinded," M3 replaces M2's
`Plain` binding with `Blinded` and the enrollment field is added.

### 5.2 D-93-C — broadcast / multicast → **CLOSED (three-way split by cardinality; group key REJECTED)**

Not deferred — resolved within P93's own machinery. A single-`ReceiverID` transcript cannot bind a frame
intended for *multiple* nodes, so P93 splits by recipient cardinality:

1. **Unicast (1 recipient) — the common case.** The default M2/M3 path. Full per-recipient transcript
   binding; C3 fully closed.
2. **Small, known multicast (`2 ≤ N ≤ MULTICAST_PER_RECIPIENT_MAX = 16`).** Option **(i) per-recipient
   signed copies**: the sender emits `N` independent V2 frames, each unicast-bound to one recipient. Cost
   is `O(N)` signatures + `O(N)` bandwidth — accepted **because N is small and the recipient set is
   enumerable** (e.g. "offer this job to these 3 specific couriers"). Each copy gets genuine cross-node
   replay protection. This is the correct answer for bounded fan-out; it is *not* over-engineering because
   the alternative (a group tag) trades away non-repudiation, which the mesh forbids.
3. **True broadcast (`N > 16`, or unbounded / anonymous audience — e.g. a breach alarm).** Neither
   per-recipient (recipients unknown/unbounded) nor a group key (see rejection below). Instead: keep the
   **existing full per-frame hybrid signature** with `RecipientBinding::Broadcast` (`BROADCAST_RECEIVER`
   sentinel), binding `Nonce ‖ Timestamp ‖ Data` (freshness + authorship) but **not** a specific recipient.
   Anti-replay for broadcast is the **M4 freshness window + per-node `(sender, nonce)` dedup** — a node
   that already saw this broadcast nonce drops the duplicate. This **accepts** that broadcast frames get
   freshness + authorship but **not** per-recipient cross-node-replay protection — *by construction*,
   because a broadcast is intended for many nodes, so "replay to another node" is partly its purpose. The
   breach-alarm precedent (`iroh_transport.rs:366-389`) already treats broadcast as a self-signed,
   roster-bypassing, deliberately-widely-accepted frame; forcing per-recipient binding on it would fight
   its design.

**The shared-group-key option is explicitly REJECTED** (recorded so it is never re-proposed): a group key
is forgeable by *any* group member, so it destroys non-repudiation — and "trust = a signed capability,
never a shared secret / never a score" is a first-class mesh red-line (MEMORY `sovereign-event-exchange`).
A group tag would also be replayable *within* the group with no authorship binding. M5's
`red_group_key_option_absent` encodes this rejection as a CI boundary.

**Why this fully closes D-93-C rather than deferring it:** the only genuinely open sub-question was the
split threshold `N`, which is an engineering-decision constant this blueprint **sets**
(`MULTICAST_PER_RECIPIENT_MAX = 16`, operator-tunable via OD-13). Everything else is answered by P93's
existing parts (M2/M3 for unicast/multicast copies, M4 for broadcast dedup). No separate sub-unit is
needed.

### 5.3 Engineering-decision values (blueprint sets; operator need not)

`REPLAY_HORIZON_SECS = 24h` (DTN store-and-forward frames can be delayed for hours; the horizon must
exceed the mesh's max store-and-forward latency, or legitimate delayed frames are dropped as stale),
`REPLAY_CLOCK_SKEW_SECS = 300`, `MULTICAST_PER_RECIPIENT_MAX = 16`, and the three window bounds (§3). All
are named constants with a one-line change surface. The horizon **must** be ≥ the BPv7 overlay's maximum
bundle-lifetime, or the window rejects frames the store-and-forward layer still considers live.

### 5.4 Red-lines / invariants preserved

- **Signature stability (breaking-change discipline):** the versioned discriminant (M1) makes old/new
  frames unambiguous; `red_v1_bytes_unchanged` proves V1 signed bytes are untouched, so existing
  signatures never break. **Never silently change the domain under the same version.**
- **Verify-then-record ordering** (`hybrid_gate.rs:171` before `:192`) preserved when `seen` relocates to
  `mesh-node` (M4 `red_verify_before_record_preserved`).
- **No signing removed:** store-and-forward / gossip / breach frames stay full per-frame hybrid-signed —
  P93 *strengthens* what they sign (S1/S2 floor).
- **No non-repudiation regression:** the blinded fork (M3) blinds only the *recipient tag*, never the
  signature; the frame signature is always over the *same asymmetric identity*
  (`red_blinded_signature_still_third_party_verifiable`).

---

## 6. The store-and-forward lifecycle in full (the design the synthesis asked for — item 2)

```
AUTHOR (offline capable)                        RECV at a peer never handshaked with
─────────────────────────                        ──────────────────────────────────
1. pick RecipientBinding by cardinality (§5.2)   1. read SigningDomainVersion (M1); unknown => reject
2. Plain(NodeId) | Blinded{tag,kem_ct} | Broadcast  2. reconstruct signing_domain() for that version
3. set Timestamp = author-clock now              3. verify_classical + verify_pq over the V2 domain
4. build signing_domain() V2 (M2/M3)                (RecipientID/tag + Timestamp are BOUND — M2/M3)
5. sign_classical + sign_pq                      4. if Blinded: decapsulate kem_ct, recompute tag',
6. hand to BPv7/gossip; it travels node→node        deliver iff tag'==tag (else relay only — M3)
   detached from any live channel               5. AFTER verify: ReplayWindow::admit(subject,nonce,ts,now)
                                                    - Replay | Stale | FutureBeyondSkew => drop (M4)
                                                 6. broadcast: accept widely, but dedup (sender,nonce) (M5)
```

The signature is the *only* thing that travels with the frame; there is no channel to lean on. Everything
P93 adds is bound *into that signature* (M2/M3) or checked *against node-owned state after it verifies*
(M4) — so the design is correct even when author and recipient never share a connection.

---

## 7. Adversarial self-check — real effort to break the design (item 3 — the heart)

### 7.1 Cross-node replay (C3) — the whole point
- **Replay a unicast frame to a different node?** No — the recipient binding (plaintext id or blinded tag)
  differs at the wrong node → signature/tag mismatch → reject (M2/M3). This is the closed defect.
- **Replay to the *same* node on a fresh connection (the exact red-team PoC)?** No — the replay window is
  **node-owned, shared across connections** (M4), so a fresh gate no longer means an empty `seen`.
- **Replay a broadcast frame?** Accepted widely by design (§5.2 tier 3) — but a *duplicate* `(sender,
  nonce)` is dropped by M4 dedup, and a *stale* one is dropped by the freshness horizon. Bounded, not open.

### 7.2 Timestamp / freshness manipulation
- **Park a frame "fresh forever" with a future timestamp?** No — `ts > now + skew` → `FutureBeyondSkew`
  (M4). The window's memory and the freshness guarantee both hold.
- **Refresh a stale frame at a relay?** No — the timestamp is inside the signed domain; changing it breaks
  the signature (M2 `red_timestamp_field_tamper`).

### 7.3 Topology deanonymization (against the blinded fork)
- **Correlate blinded frames to a recipient?** No — ephemeral `kem_ct` per bundle → different `tag` each
  time; deriving the recipient requires the receiver's ML-KEM secret (M3 `red_blinded_unlinkable`).
- **Swap `kem_ct` so a relay can decapsulate to the tag?** No — `kem_ct` is bound into the signed domain;
  swapping it breaks the signature (M3 `red_blinded_cross_node_replay_rejected`).
- **Honest residual:** the blinded tag hides *identity*, not *existence/size/timing* — a relay still sees
  that *a* bundle of size S passed at time T. Traffic-analysis resistance (padding, mixing, cover traffic)
  is out of P93's scope and belongs to the `.onion`/anonymity tier (P53) — stated, not papered over.

### 7.4 Version downgrade / domain confusion
- **Strip the recipient binding by claiming V1?** No — a V2 body verified under the V1 domain omits the
  bound fields → recomputed bytes ≠ signed bytes → fail (M1 `red_version_downgrade_rejected`).
- **Reinterpret a recipient tag as a channel binding (or vice-versa)?** No — distinct `DOMAIN_*` tags and
  distinct TLV field ids (§3); domain separation closes cross-structure reuse (red-team row 6, FIXED).

### 7.5 Replay-window as an attack surface (the relocation's own risk)
- **Poison the window with a victim's nonce to DoS its real frame?** No — verify-then-record: a nonce is
  inserted only after the signature verifies (M4 `red_verify_before_record_preserved`), so an attacker
  cannot pre-insert a nonce it cannot sign.
- **Exhaust memory with distinct nonces?** No — three bounds + prune + LRU sender eviction (M4
  `red_window_memory_bounded`); the window is a bulkhead (§11), degrade-closed.

---

## 8. Mandatory independent adversarial-review gate — DoD-BLOCKING (standard §2 items 5, 6, 14)

**Grounded in a real prior incident** (MEMORY `crypto-safe-first-pass-2026-07-14.md`): the bebop
`verify_batch` shortcut was *forgeable* and passed unit tests; it was caught only because an independent
reviewer **built and ran** an SSR-2020 forgery. P93 changes the signed domain and adds a new stateful
window — it does **not** ship until an independent adversarial review passes.

### 8.1 Reviewer independence
Performed by an actor **not** the implementer (`system-breaker` / `security-sentinel` or a decorrelated
model). Mandate: **produce a working replay/deanonymization or a proof of impossibility**, not read-and-approve.

### 8.2 Attacks the review MUST attempt (each a concrete artifact)
1. **Cross-node replay that survives the new transcript** — build a frame authored for B and get node C to
   *deliver* (not just relay) it.
2. **Topology deanonymization against the blinded tag** — correlate two blinded frames to a shared
   recipient, or recover a recipient identity without its ML-KEM secret.
3. **Broadcast-frame replay** — get a `Broadcast` frame accepted *twice* at one node past the dedup, or a
   stale one accepted past the horizon.
4. **Version downgrade** — strip the recipient/timestamp binding by claiming V1 on a V2 body.
5. **Window poisoning / DoS** — insert a victim's nonce before its real frame; or grow the window unbounded.
6. **Signature-stability break** — find any V1 frame whose signed bytes changed after M1 (would break every
   existing signature).

### 8.3 Gate outcome (falsifiable)
- **PASS** = written attestation that each 8.2 attack was *attempted with a concrete input*, each rejected
  with the expected typed error, any bypass found fixed and re-attempted-and-rejected. Filed under
  `docs/reflections/`, referenced from D-REVIEW below.
- **FAIL** = any bypass accepted, or any attack not genuinely attempted → P93 is RED and does not ship.

---

## 9. DoD — falsifiable, RED→GREEN, machine-checkable (standard §2 item 2)

| # | Done when… | Falsifier (RED test / check) |
|---|---|---|
| D1 | the signing-domain version is bound, append-only, fail-closed; V1 bytes unchanged | `red_unknown_domain_version_rejected`, `red_v1_bytes_unchanged`, `red_version_downgrade_rejected` (M1) — **REGRESSION-LEDGER entry (migration correctness)** |
| D2 | a store-and-forward frame binds `ReceiverID ‖ Nonce ‖ Timestamp ‖ Data`; cross-node replay is rejected | `red_cross_node_replay_rejected`, `red_receiver_field_tamper`, `red_timestamp_field_tamper` (M2) — **REGRESSION-LEDGER entry (closes C3, S&F path)** |
| D3 | (if D-93-A ruled blinded) only the true recipient recognizes a frame; frames are unlinkable; C3 still closed; non-repudiation preserved | `red_blinded_only_recipient_recognizes`, `red_blinded_unlinkable`, `red_blinded_cross_node_replay_rejected`, `red_blinded_signature_still_third_party_verifiable` (M3) |
| D4 | replay state is node-owned, shared across connections, DTN-appropriate; verify-then-record preserved | `red_cross_instance_replay_rejected`, `red_delayed_frame_within_horizon_accepted`, `red_too_old_frame_rejected`, `red_future_timestamp_rejected`, `red_window_memory_bounded`, `red_verify_before_record_preserved` (M4) — **REGRESSION-LEDGER entry (closes C3, cross-instance)** |
| D5 | multicast splits by cardinality; broadcast sentinel is reserved + deduped; group key absent | `red_multicast_per_recipient_isolated`, `red_broadcast_sentinel_accepted_by_many`, `red_broadcast_sentinel_is_reserved`, `red_group_key_option_absent` (M5) |
| D6 | the new domain is actually **enforced on the live recv path**, not just available | `red_recv_actually_enforces_v2`, `red_no_regression_on_v1_recv` (M6) |
| D-REVIEW | **independent adversarial-review attestation exists and PASSES** | §8.3 artifact under `docs/reflections/`; FAIL ⇒ blueprint RED |
| D-BUILD | `proto-cap` + `mesh-node` build & full `cargo test` green incl. all new REDs now GREEN, no dep added | `cargo test -p bebop-proto-cap -p bebop-mesh-node` |
| D-NOREG | existing signed-path + breach tests stay green (V1 frames, `RequireBoth`, breach bypass) | `bound_frame_fails_on_different_channel`, the breach/full-signed wire tests |

---

## 10. Benchmarks + telemetry + measure (standard §2 item 10)

### 10.1 What to measure
| Bench | Measures | Harness |
|---|---|---|
| `bench_signing_domain_v1_vs_v2` | per-frame `signing_domain()` cost delta (extra TLV fields) — must be negligible vs the crypto | `cargo bench` (existing pq bench pattern) |
| `bench_replay_admit` | `ReplayWindow::admit` cost incl. prune, at 10^4 / 10^6 tracked nonces | same |
| `bench_blinded_recipiency_check` | one ML-KEM decaps + hash per candidate frame (the D-93-A blinded cost) | same |

### 10.2 Telemetry
Emit per-node `{replay_rejects, stale_rejects, future_rejects, window_size, blinded_recipiency_misses}`
through the carrier metrics seam so a regression (e.g. a spike in stale-rejects = a clock or horizon bug)
surfaces automatically, not at review time (item 14).

### 10.3 No measure-first NO-GO gate (unlike P92)
P93 is **correctness**, not a throughput bet — it closes a still-open replay defect, so there is no "build
only if the numbers clear a bar." The benches exist to prove the *cost* is negligible (the V2 domain must
not measurably slow the crypto-dominated verify), and to size the window bounds — not to gate the build.

---

## 11. Cross-cutting obligations (standard §2 items 6, 8, 9, 11–16)

- **Hazard-safety as math (item 6):** the unsafe state — *a frame accepted at a node it was not authored
  for* — is made **unrepresentable**: the recipient is a **signed** field (M2/M3), so a wrong-recipient
  frame fails signature reconstruction; there is no "accept anyway" branch. Cross-instance replay is
  unrepresentable because `seen` is node-scoped (M4), not per-gate. Reachability argued from the
  type/flow structure (finite-anchored-authority doctrine), not a prose assurance.
- **Schemas & scaling axis (item 8):** the scaling axis is **distinct senders × nonces-per-horizon**. The
  window is `O(senders × horizon × rate)`, bounded by three named ceilings with LRU sender eviction (§3).
  It changes shape only if a node tracks ≳10^5 simultaneous active senders (then a slab/arena keyed by
  sender, not a nested `HashMap`) — stated, not timeless. The V2 signed domain grows by two fixed TLV
  fields (+ one `kem_ct` under blinding); no per-scale reshaping.
- **Linux discipline (item 9):** **EXTENDS** the existing `signing_domain()` / `tlv` seam (adds fields,
  keeps the codec); **REINFORCES** the fail-closed decode + verify-then-record patterns; **RELOCATES** the
  `seen` ledger per the B2 remediation (a GAP-close, not a new abstraction elsewhere); **DOES-NOT-TRANSFER**
  — no new daemon, no core state.
- **Isolation / bulkhead (item 11):** the `ReplayWindow` is a **bulkhead** — bounded memory, degrade-closed
  (past a bound it evicts the coldest sender, never grows unbounded); a window bug cannot corrupt the
  ledger/capability plane (it only *rejects* frames, never fabricates authority). Its failure mode is
  "reject a legitimate frame" (fail-safe: the sender retransmits), never "accept a replay."
- **Mesh awareness (item 12):** P93 is the **gossip-propagated / store-and-forward** path by definition —
  the opposite of P92's node-local session. Payload budget: +2 fixed TLV fields (≈ 40 bytes) per frame;
  under blinding, +~1.1 KB `kem_ct` per bundle (the D-93-A cost, §5.1). The window is node-local state, not
  gossiped (each node maintains its own; a replay rejected at one node is independent of others).
- **Rollback / self-healing as math (item 13):** **Self-termination** = the freshness horizon (a frame is
  unrepresentable-as-fresh past `REPLAY_HORIZON_SECS`, not a supervisor's choice). **Snapshot re-entry** =
  the window is *derivable* — it can be dropped and rebuilt from the recent event log on restart (a lost
  window only re-admits recently-seen frames, bounded by the horizon; the signature still gates them).
  **Self-healing is NOT claimed** — a dropped frame is retransmitted by the store-and-forward layer; there
  is no error-correcting recovery, and claiming it would be false.
- **Error-propagation / smart index (item 14):** the bug classes P93 could introduce — a domain silently
  changed under the same version, a group key silently added, a nonce recorded before verify — are turned
  into **compile/CI failures**: `red_v1_bytes_unchanged` (signature-stability KAT), `red_group_key_option_absent`
  (CI boundary), `red_verify_before_record_preserved`. Not runtime surprises.
- **Living-memory awareness (item 15):** the replay window is **time-scoped** (horizon) and
  **topology-scoped** (per-sender) — a textbook living-memory access pattern (cross-ref
  `internal-retrieval-living-memory-arc-2026-07-14`): recent nonces are hot, old ones demote-then-expire.
  It is deliberately *not* flat storage; the expiry-prune is the demotion.
- **Tensor/spectral (item 16):** **N/A, honestly** — a nonce set + a hash domain is not a linear-algebra
  kernel; forcing `spectral.rs` here would be over-engineering (ponytail). Stated.

---

## 12. Hermetic principles honored (standard §2 item 20 — load-bearing only)

- **Correspondence ("as above, so below"):** the signature *is* a function of the intended recipient — a
  different recipient is, of necessity, a different signed domain, hence a different valid signature. The
  binding is self-describing, not asserted.
- **Cause & Effect:** every accepted frame has a signed *cause* — an authorship binding rooted in an anchor
  identity; nothing is trusted by channel, correlation, or a shared secret (the group-key rejection, §5.2,
  is this principle applied: no un-caused/forgeable authority).
- **Polarity / no-middle:** a frame is either bound-to-a-recipient (unicast/multicast) or explicitly
  bound-to-no-recipient (broadcast sentinel) — there is **no ambiguous middle** where a frame is "sort of"
  for someone; the `RecipientBinding` enum makes the third state (unspecified) unrepresentable.

---

## 13. Standard-compliance map (all 20 points — standard §2)

| # | Standard item | Where satisfied |
|---|---|---|
| 1 | Ground truth, live `file:line` | §0 (every cite re-verified this pass; the "binding fields exist but recv never checks", "seen is per-instance" corrections) |
| 2 | Falsifiable DoD | §9 (D1–D-NOREG, each a RED→GREEN test or artifact) |
| 3 | Spec→test→code, event-driven | §4 (spec-first per M; author→sign→recv→verify→record event sequence in §6) |
| 4 | Predefined types & constants | §3 (`SigningDomainVersion`/`RecipientBinding`/`ReplayWindow`/constants named before impl) |
| 5 | Adversarial/breaking tests | §4 (every M has RED adversarial cases), §7 (self-attack), §8 (forgery gate) |
| 6 | Hazard-safety from type structure | §11 (wrong-recipient / cross-instance replay unrepresentable), §7.1 |
| 7 | Links to docs & memory | §14 |
| 8 | Schemas with scaling axis | §11 (senders × nonces/horizon; three bounds + LRU) |
| 9 | Linux engineering discipline | §11 (EXTENDS/REINFORCES/RELOCATES/DOES-NOT-TRANSFER verdict) |
| 10 | Benchmarks + telemetry | §10 (cost-proving benches + reject-rate telemetry; no NO-GO gate — it's correctness) |
| 11 | Isolation / bulkhead | §11 (bounded, degrade-closed window; reject-not-fabricate failure mode) |
| 12 | Mesh awareness | §11 (the gossip/store-and-forward path by definition; payload budget incl. blinded cost) |
| 13 | Rollback/self-heal as math | §11 (self-termination = horizon; snapshot re-entry = derivable window; self-healing NOT claimed) |
| 14 | Error-propagation / smart index | §11 (signature-stability KAT, group-key CI boundary, verify-before-record guard) |
| 15 | Living-memory awareness | §11 (time+topology-scoped window; expiry = demotion) |
| 16 | Tensor/spectral where applicable | §11 (N/A, stated honestly) |
| 17 | Regression tracking | §9 D2/D4 (REGRESSION-LEDGER entries for the C3 closures) |
| 18 | Clear worker instructions | §14 |
| 19 | Reuse-first, upgrade-if-needed | §0.4 (all primitives in-tree), §1 (adopt not invent), §2.2 (anti-scope) |
| 20 | Hermetic principles | §12 |

---

## 14. Links to docs & memory + instructions for other agentic workers (standard §2 items 7, 18)

**Depends on / cites:**
- `SYNTHESIS-MESH-MAJOR-REFACTOR-PLAN-2026-07-19.md` §4 (the scope stub this expands), §2.2 (P93-before-P92
  reasoning), §6 (anti-scope).
- `docs/research/OPUS-CORE-CONSOLIDATION-AUDIT-2026-07-19.md` §3 (placement law: transcript→proto-cap,
  window→mesh-node, core clock-free).
- `docs/research/OPUS-HANDSHAKE-ONCE-VS-PERMESSAGE-2026-07-18.md` §2 (C3 row), §4 (store-and-forward forbids
  the pure-session model — the reason auth must travel with the frame).
- `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md` §2.2 (disclaims the cross-node ledger P93 supplies),
  §7.5 (P92 is C3-immune by construction on its disjoint population).
- Red-team `bebop2/docs/red-team/2026-07-13/B2-protocol-authz.md` row 3 (C3 STILL OPEN) + remediation #2
  (the window design), `B3-wire-transport.md` (F3 channel binding, the M1-of-cluster analog).
- `CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (the 20-point contract).
- MEMORY: `crypto-safe-first-pass-2026-07-14.md` (B4/SSR-2020 review precedent — §8),
  `never-bypass-human-gates-2026-06-29.md` (D-93-A/D-93-C are human-gated forks — OD-12/OD-13),
  `mesh-real-arc-2026-07-13.md` (transport hardening), `internal-retrieval-living-memory-arc-2026-07-14.md`
  (window access pattern, item 15).

**Existing code this blueprint edits/extends (exact targets, bebop-repo — NOT dowiz):**
- **EDIT** `bebop2/proto-cap/src/signed_frame.rs` — add `SigningDomainVersion`, extend `signing_domain()`
  with `FIELD_RECIPIENT`/`FIELD_TIMESTAMP` (+ `FIELD_KEM_CT` under blinding); both legs sign the V2 domain.
- **EDIT** `bebop2/proto-cap/src/tlv.rs` — append the new field tags (0x04–0x07) + `DOMAIN_SIGNED_FRAME_V2`
  / `DOMAIN_RECIPIENT_TAG`, fail-closed on unknown.
- **NEW** `bebop2/mesh-node/src/replay_window.rs` — `ReplayWindow` (§3, M4), node-owned, DTN expiry-pruned.
- **EDIT** `bebop2/proto-cap/src/hybrid_gate.rs` — take `&mut ReplayWindow` instead of owning
  `Mutex<HashSet>` (`:67`); preserve the verify-then-record ordering (`:171` before `:192`).
- **EDIT** `bebop2/mesh-node/src/node.rs` — own the single `ReplayWindow`; thread `now` from the node clock.
- **EDIT** `bebop2/proto-wire/src/{bpv7.rs, sync_pull.rs, iroh_transport.rs}` — select the V2 domain by
  version, enforce on recv, call `ReplayWindow::admit` after verify (M6); broadcast classification (M5).
- **REUSE unchanged** `bebop2-core::{hash::sha3_256, pq_kem, kdf}` — **no core change**.
- **DO NOT TOUCH** `bebop2-core` (S5), the delegation lattice / revocation / red-line gate, or P92's
  `fastpath.rs`.

**For the worker with zero session context — exact acceptance path:**
1. **Surface OD-12 (D-93-A plaintext vs blinded) and OD-13 (D-93-C threshold) to the operator FIRST** — do
   not silently default. If "plaintext," skip M3; if "blinded," build M3 + the ML-KEM enrollment field.
2. **Land after the cluster's M1 exporter fix** (shared `signed_frame.rs` surface, §4.7).
3. Write §3 types first (types → tests → code — item 3); implement M1→M6 in order; each M's RED tests fail
   before its code and pass after.
4. Add the D2/D4 (C3 closure) + D1 (migration) regression entries to `docs/regressions/REGRESSION-LEDGER.md`.
5. `cargo test -p bebop-proto-cap -p bebop-mesh-node` fully green; V1/breach/`RequireBoth` tests (D-NOREG)
   stay green.
6. **Do NOT mark P93 done until §8's independent adversarial-review attestation PASSES (D-REVIEW).** A green
   unit suite is necessary and NOT sufficient — the B4 lesson. Route to an independent reviewer whose job is
   to *build a replay/deanonymization*, not to approve.
7. Anti-scope: never remove a signature; never add a shared group key; never move the window into
   `bebop2-core`; keep the versioned discriminant append-only + fail-closed.
