# BLUEPRINT — Reed-Solomon FEC for the bebop2 wire line (round-2, ADOPTED by operator ruling)

> **Status: BUILD PLAN.** The operator overruled R4's DEFER-WITH-TRIGGER verdict with an explicit
> ruling: *"reed-solomon will be used, add FEC too."* This document does not re-litigate adoption.
> It reconciles the ruling with the technical facts R4 established (both live carriers already do
> ARQ), places FEC where it adds **real, non-redundant** value, and specifies a concrete, agent-
> executable build with a falsifiable DoD.
>
> Extends: `../R4-reed-solomon-fec-fit-grounding.md` (R4) and
> `../BLUEPRINT-FAIL-OPERATIONAL-LAYOUT-VERSIONING-SYNTHESIS.md` (synthesis rows 13-14, §4.2-4.3).
> Code ground truth re-read fresh for this pass: `bebop2/proto-wire/src/{framing.rs, envelope.rs,
> error.rs, lib.rs, wire_codec.rs, transport_policy.rs, wss_transport.rs:533-660,
> iroh_transport.rs:57-386, bpv7.rs, sync_pull.rs (outline), discovery.rs (outline)}` and
> `proto-wire/Cargo.toml`, all under `/root/bebop2-verify-redteam/bebop2/`.
>
> No product code touched. No commits (standing "поки жодних комітів").
>
> **Standing doctrine carried forward unchanged:** FEC is a **reliability** control for honest-noisy
> channels. It is **NEVER an authenticity/integrity-of-origin control** and must never be documented
> as one (R4 §1, §4). The signature gate (`HybridGate`: Ed25519 + ML-DSA-65) remains the sole
> authenticity authority, and it runs **after** FEC reconstruction, on reconstructed bytes.

---

## 0. Honest reconciliation: the decision changed; the physics did not

The overruled part is the **verdict** ("near-zero fit today ⇒ defer"). The **facts** R4 measured
stand and dictate placement:

- `wss_transport.rs` is WebSocket over TLS over **TCP** — TCP retransmits; the app *never sees* a
  lost or corrupted byte (`framing::decode` reads a reliable stream).
- `iroh_transport.rs` is **QUIC** (quinn 0.11) — per-packet loss detection + retransmission on
  streams, AEAD packet protection.
- Whole-event gaps are already healed by pull anti-entropy (`sync_pull.rs` / `core/anti_entropy.rs`),
  which is ARQ at the application layer.

Therefore **FEC layered on top of a reliable stream is physically inert**: the transport retransmits
regardless, the app cannot observe the loss, and parity bytes are pure overhead. Adopting FEC does
not change that. What adoption means, honestly, is: **build the FEC capability where a loss IS
app-visible** — and the codebase has exactly three such places, two of them real today.

### 0.1 The three lanes where FEC adds non-redundant value

**L1 — QUIC unreliable-datagram lane (real today; the primary build target).**
quinn 0.11 (already a direct dependency) implements RFC 9221 unreliable datagrams
(`Connection::send_datagram` / `read_datagram`, `max_datagram_size`). Datagrams are **not
retransmitted** — loss is app-visible, which is precisely the carrier FEC was designed for. This
lane converts the transport-redundancy objection into the tail-latency win the operator's
latency-elimination thread is after:

- On the courier cellular profile V5 §4 records (200-2000 ms RTT, 1-10 % loss), a stream
  retransmission costs ≥ 1 RTT — i.e. **0.2-2 s of added tail latency per loss event**, worse under
  TCP RTO. On the datagram lane, a lost shard costs **nothing** as long as any `k` of `k+m` shards
  arrive: recovery is local decode (microseconds), not a round trip. FEC is the standard mechanism
  for exactly this trade (QUIC-FEC, IETF adaptive-FEC drafts — sources below); it strictly wins when
  the latency target is `< 2×RTT`, which courier position/dispatch freshness on a 2 s-RTT cell link
  always is.
- **The loss-amplification argument (new in this pass, and the strongest single number):** a signed
  frame is multi-MTU *by construction* — the ML-DSA-65 signature alone is ~3.3 KB, plus the
  delegation chain — so one logical frame spans `k ≈ 4-8` datagrams at a ~1.2 KB MTU. Without FEC,
  frame delivery needs **all** k datagrams: at 5 % i.i.d. loss, `P(frame lost) = 1−(0.95)⁴ ≈ 18.5 %`
  for k=4. With m=2 parity, `P(frame lost) = P(>2 of 6 lost) ≈ 0.22 %` — an **~84× reduction** for
  50 % overhead on this lane only. The PQ-hybrid signature size, which everywhere else is a cost, is
  here the very thing that makes per-frame FEC pay.
- Scope: latency-critical, supersedable payloads — courier position/telemetry, dispatch pings. This
  is the same lane the synthesis §3.1 already carves out as `CapabilityClass::Telemetry`
  (never-self-assigned, #15). Money/order frames stay on the reliable stream lane; nothing here
  re-opens synthesis row 8.

**L2 — BPv7 bundle sharding across couriers/paths (real today; Wave 3).**
`bpv7.rs` is the store-and-forward overlay for partitioned operation: a courier physically carries
bundles between partitions, custody + retry-until-ack, dedup by nonce. Here the loss unit is not a
packet but a **whole path/courier that never reconnects** — per-connection ARQ cannot help by
definition (there is no connection). Erasure-sharding one logical payload into `k+m` bundles handed
to different couriers/paths means the destination reconstructs after **any k arrive**. This is
RAID-across-couriers: genuinely non-redundant with everything that exists, and the only lane where
FEC buys *delivery probability under partition* rather than latency.

**L3 — future non-ARQ carriers (BLE-mesh / LoRa / broadcast; R4's original trigger).**
The R4 trigger condition is now *pre-satisfied by build order* instead of gating adoption: the pure
`fec` module (Wave 1) is carrier-agnostic, so when a datagram/broadcast carrier lands, its FEC is
already hardened, fuzzed, and ratio-tunable. The trigger is inverted from "adopt when" to "already
built for."

### 0.2 Where FEC is deliberately NOT placed (and must stay out)

| Non-placement | Why (physics, not preference) |
|---|---|
| Inside the WSS/TCP stream lane | The app cannot observe loss on a reliable byte stream; TCP retransmits regardless; parity = pure overhead. WebSocket has no datagram mode, so WSS gets **no FEC lane at all**. |
| Inside the QUIC **reliable stream** lane | quinn retransmits stream data; same inertness. The existing `send`/`recv` stream path is untouched. |
| As an authenticity/integrity-of-origin control | R4 §1 stands verbatim: an attacker's bytes are FEC-perfect by construction; FEC helps with **zero** of the 11 V3 HIGH findings. `HybridGate` remains the only authenticity gate. |
| As a replacement for anti-entropy | `sync_pull` re-pull remains the recovery path of record; FEC-group failure **degrades to it** (see §4.3) — FEC failure is never data loss. |

### 0.3 One adjacent gap observed while reading (flag, not this blueprint's fix)

`iroh_transport.rs::recv` (358-385) verifies via the gate but — unlike `wss_transport.rs::recv`
(546-595) — has **no `ReplayLedger` observe and no per-transport `max_frame_bytes` check**. The new
datagram lane specified below includes both from day one (§2.4); the stream-lane omission is
reported as a separate finding for whichever pass owns MESH-10 parity across carriers.

---

## 1. Crate choice — re-verified July 2026: `reed-solomon-simd`, confirmed

Live-checked this pass (crates.io API + GitHub, 2026-07-17):

| Fact | Value |
|---|---|
| Latest version | **3.1.0**, published **2025-10-14** (repo commits same day) — active |
| License | **MIT AND BSD-3-Clause** — AGPL-3.0 compatible (ADR-020 clean), patent-clean clean-room Leopard-RS lineage |
| Downloads | ~1.77 M all-time |
| Algorithm | GF(2¹⁶) Reed-Solomon **erasure** codes, O(n log n) FFT; runtime SIMD select (AVX2/SSSE3/Neon) + pure-Rust scalar fallback |
| Limits | 1-32768 original + 1-32768 recovery shards; shard size must be **even** (2-byte GF symbols); v3.x supports non-multiple-of-64 sizes |
| `unsafe` | only in the SIMD kernels (scalar path is safe Rust) |

Alternatives re-checked and still declined, same grounds as R4 §3b-3c:
- `reed-solomon-erasure` — still "[Looking for new owners/maintainers, see #88]" in the repo title.
  Maintenance-risk; declined.
- `reed-solomon-novelpoly` (Polkadot) — healthy, but no advantage over `-simd` for our shard counts,
  and `-simd` benches faster at small-m. Kept as the named fallback if `-simd` ever goes dark.
- `raptorq` — still **subordinate and gated** (synthesis §4.3): rateless fountain codes only earn
  their complexity on true one-to-many multicast with heterogeneous loss, which no lane here is
  (L1 is unicast-per-connection; L2 is k+m fixed sharding). The Qualcomm patent-history DECART
  remains a precondition; classical RS needs none. **Nothing in this adoption pulls `raptorq` in.**

**DECART mini-report for the new dependency** (per the standing new-dep rule): pure Rust, zero
transitive C, zero network/IO, zero build.rs surprises (verify in lock review), MIT/BSD dual — the
honest falsifiable comparison is the table above vs. the two alternatives; modern-default tiebreak
selects `-simd`. Version pin: `reed-solomon-simd = "3.1"`. The one-time GF table init (< 10 ms) is
paid at first-use behind a `OnceLock`, never on the hot path.

---

## 2. Architecture — where it plugs in

### 2.1 Module layout

```
proto-wire/src/fec.rs        ← NEW. Pure shard codec + bounded group reassembly. Zero carrier knowledge.
proto-wire/src/iroh_transport.rs  ← EXTENDED (Wave 2): retain quinn::Connection; add the datagram lane.
proto-wire/src/bpv7.rs       ← EXTENDED (Wave 3): bundle sharding + reassembly over StoreForward.
proto-wire/src/error.rs      ← EXTENDED: WireError::Fec(FecError).
proto-wire/Cargo.toml        ← + reed-solomon-simd = "3.1"  (+ [features] fec, default-on is fine — pure Rust, offline-clean)
```

`fec.rs` opens with the same two doc-guards every module in this crate carries:
`CI GUARD: NO-COURIER-SCORING` (FEC counts shards, never grades movers) and a new one-liner that CI
can grep for: **`FEC-IS-NOT-AUTH: reliability control only; HybridGate is the sole authenticity
gate and runs after reconstruction.`**

### 2.2 The pipeline (R4 §4 layering, made concrete)

Datagram lane (L1), receive side — order is load-bearing and test-pinned (§6, T3):

```
quinn read_datagram()                      (AEAD-protected by the QUIC connection; off-path injection impossible)
  → FecShard::parse(bytes)                 hostile-input surface #1 — bounded, no-panic (§5)
  → FecGroupBuffer::ingest(shard)          hostile-input surface #2 — bounded memory, eviction (§5)
  → [any k of k+m present] reconstruct()   reed-solomon-simd decode; exact or typed failure — never partial bytes out
  → wire_codec::decode_frame(&bytes)       existing fail-closed canonical codec (magic, version, bounds)
  → replay.observe(frame.capability.nonce) mirrors wss recv:560-575 — record-before-verify, cross-connection ledger
  → policy/max_frame_bytes check           mirrors wss recv:554-556
  → gate.check(frame, roster, chain, revocations, now)   ← SOLE authenticity gate (Ed25519 + ML-DSA-65)
  → Ok(SignedFrame) → apply
```

Send side: `wire_codec::encode_frame(&frame)` → split into `k` data shards of `shard_bytes` (last
shard zero-padded; true length in the group header) → `m` recovery shards via
`ReedSolomonEncoder` → each shard prefixed with the `FecShard` header → one datagram per shard.

**Deliberate envelope decision (documented divergence from the stream lane):** the stream lane wraps
`wire_codec` bytes in the JSON `Envelope` (`envelope.rs`). serde_json serializes `payload: Vec<u8>`
as a JSON **array of integers** (~3.7× inflation) — tolerable on a reliable stream, hostile to a
datagram MTU budget. The datagram lane therefore FEC-encodes the **canonical `wire_codec` frame
bytes directly**, and the `FecShard` header itself carries the envelope's two jobs: a version
discriminant (`FEC_VERSION`, hard-reject on mismatch — same fail-closed law as
`framing.rs:59-61` / `unknown_version_is_rejected_on_decode`) and the 16-byte `trace` id. Nothing
signed changes: signatures commit to the TLV signing domain inside `proto-cap`, untouched. (The
JSON-inflation observation also suggests the *stream* envelope deserves a binary migration one day —
flagged, out of scope.)

### 2.3 Shard wire format (fixed layout, hand-rolled like `wire_codec` / `bpv7::PrimaryBlock`)

```
[ magic  "BFEC"            4 bytes ]
[ version u8               1 byte  ]   FEC_VERSION = 1; != 1 → hard reject (fail-closed)
[ trace   [u8;16]         16 bytes ]   correlation id (diagnostic only — never a score input)
[ group   [u8;8]           8 bytes ]   random per logical frame (sender-local RNG)
[ index   u16 LE           2 bytes ]   0..k-1 = data shard; k..k+m-1 = recovery shard
[ k       u16 LE           2 bytes ]   original-shard count,   1 ≤ k ≤ MAX_FEC_DATA_SHARDS
[ m       u16 LE           2 bytes ]   recovery-shard count,   1 ≤ m ≤ MAX_FEC_RECOVERY_SHARDS
[ orig_len u32 LE          4 bytes ]   true frame-bytes length; > MAX_ENVELOPE_BYTES → reject BEFORE buffering
[ shard_len u16 LE         2 bytes ]   payload bytes that follow; even; ≤ MAX_FEC_SHARD_BYTES
[ shard payload   shard_len bytes ]
```

Header = 41 bytes; with `DEFAULT_FEC_SHARD_BYTES = 1152` a shard datagram is 1193 bytes — under
quinn's typical ~1200-byte initial `max_datagram_size`. The sender MUST consult
`Connection::max_datagram_size()` at group-encode time and shrink `shard_bytes` (even-rounded) to
fit; if the connection reports no datagram support (`None`), the lane **degrades to the reliable
stream** (`send()`), never fails the frame — degrade-closed to ARQ.

Consistency law: within one group, `(k, m, orig_len, shard_len)` are pinned by the **first accepted
shard**; any later shard disagreeing on any of the four → the whole group is dropped with
`FecError::GroupInconsistent` (fail-closed — an attacker mixing parameters gets a typed drop, never
a confused decode). Duplicate `index` → first-wins, silently ignored (idempotent; re-sent datagrams
are legal).

### 2.4 Integration points, exactly

**`QuicTransport` (Wave 2):** add field `conn: quinn::Connection` (today only `_endpoint` + streams
are retained, `iroh_transport.rs:70-83` — `connect`/`accept` already hold the `Connection` before
opening streams; keep it). Add:

- `async fn send_unreliable(&mut self, frame: SignedFrame, ratio: FecRatio) -> WireResult<()>`
- extend `recv()`'s loop: `tokio::select!` over `self.recv.read_chunk(..)` (stream, unchanged) and
  `self.conn.read_datagram()` (new); datagram bytes feed `self.fec_buf.ingest(..)`; a completed
  group joins the SAME post-reconstruction path the stream uses (`wire_codec::decode_frame` → replay
  → policy → `gate.check`). One verification path, two byte sources.
- give `QuicTransport` the `ReplayLedger` + `max_frame_bytes` the wss carrier already has (closes
  the §0.3 gap for this lane from day one).

**`bpv7::StoreForward` (Wave 3):** `shard_bundle(bundle: &Bundle, ratio: FecRatio, now) ->
Vec<Bundle>` — payload split into k+m shard-bundles, each payload = `FecShard` bytes, each with its
**own fresh nonce** (custody/dedup unit = the shard-bundle; the *logical* frame's replay identity is
still the inner frame's capability nonce, checked after reconstruction — two layers, two jobs).
Receiver side: `BundleReassembler` wrapping the same `FecGroupBuffer`, then the same
decode→replay→gate path. Custody semantics unchanged per shard-bundle (ack/retry/expire as today);
the reassembler's group expiry follows the bundles' own `lifetime`, so no second timer authority.

**Not integrated:** `wss_transport.rs` (no datagram mode exists — §0.2) and the reliable stream
paths of both carriers (untouched).

---

## 3. Predefined types & constants (contract §2.4 — named before implementation)

```rust
// proto-wire/src/fec.rs

pub const FEC_MAGIC: [u8; 4] = *b"BFEC";
pub const FEC_VERSION: u8 = 1;
/// Hard caps on attacker-suppliable header fields (§5). k+m ≤ 96 total datagrams per group.
pub const MAX_FEC_DATA_SHARDS: u16 = 64;
pub const MAX_FEC_RECOVERY_SHARDS: u16 = 32;
/// Shard payload cap; even (GF(2^16) symbols are 2 bytes). 1152 + 41-byte header = 1193 ≤ ~1200 MTU.
pub const MAX_FEC_SHARD_BYTES: usize = 1152;
pub const DEFAULT_FEC_SHARD_BYTES: usize = 1152;
const _: () = assert!(DEFAULT_FEC_SHARD_BYTES % 2 == 0);
/// Reconstructed-frame cap — SAME const as the stream lane (single source of truth, framing.rs:22).
pub use crate::framing::MAX_ENVELOPE_BYTES; // orig_len > this ⇒ reject before buffering
/// Bounded reassembly (§5): per-connection group table.
pub const MAX_FEC_GROUPS_BUFFERED: usize = 32;
pub const MAX_FEC_BUFFER_BYTES: usize = 2 * 1024 * 1024;
/// One decode attempt per group per newly-arrived shard once ≥ k are present, capped:
pub const MAX_FEC_DECODE_ATTEMPTS: u8 = 4;

/// Redundancy ratio — the tuning knob (§4). PRE-NETEM DEFAULT; re-tune from V5 §4 measurements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FecRatio { pub k_hint: u16, pub m_per_k: RecoveryRule }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryRule { /// m = max(2, ceil(k/2)), capped at MAX_FEC_RECOVERY_SHARDS
                        CellularDefault,
                        /// explicit m (operator/netem-tuned)
                        Fixed(u16) }
pub const FEC_RATIO_CELLULAR_DEFAULT: FecRatio =
    FecRatio { k_hint: 0 /* k derived from frame len */, m_per_k: RecoveryRule::CellularDefault };

/// 8-byte per-frame group id (sender RNG; uniqueness per connection lifetime is sufficient —
/// datagrams ride inside the QUIC AEAD, so only the peer itself can address our buffer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FecGroupId(pub [u8; 8]);

/// One parsed shard. Parsing is total: any malformed input → Err(FecError), never panic.
#[derive(Debug, Clone)]
pub struct FecShard {
    pub trace: [u8; 16],
    pub group: FecGroupId,
    pub index: u16,
    pub k: u16,
    pub m: u16,
    pub orig_len: u32,
    pub data: Vec<u8>,   // len == shard_len, even, ≤ MAX_FEC_SHARD_BYTES
}
impl FecShard {
    pub fn to_bytes(&self) -> Vec<u8>;                 // fixed layout, §2.3
    pub fn parse(b: &[u8]) -> Result<FecShard, FecError>; // bounds-checked take(), wire_codec style
}

/// Encode one frame-bytes blob into k data + m recovery shards, ready to datagram.
pub fn encode_group(frame_bytes: &[u8], trace: [u8; 16], group: FecGroupId,
                    ratio: FecRatio, max_shard_bytes: usize) -> Result<Vec<Vec<u8>>, FecError>;

/// Bounded, evicting reassembly buffer. One per connection/reassembler. NOT shared across peers.
pub struct FecGroupBuffer { /* HashMap<FecGroupId, GroupState> + insertion-order eviction queue,
                               byte-count accounting, per-group attempt counter */ }
impl FecGroupBuffer {
    pub fn new() -> Self;
    /// Feed one datagram. Ok(Some(bytes)) = a group completed → reconstructed ORIGINAL frame bytes
    /// (exact, orig_len-trimmed). Ok(None) = buffered/ignored. Err = typed reject (shard AND, on
    /// group-level faults, the whole group are dropped). NEVER returns partial/garbage bytes.
    pub fn ingest(&mut self, datagram: &[u8]) -> Result<Option<Vec<u8>>, FecError>;
    pub fn buffered_bytes(&self) -> usize;   // test hook for the §5 memory bound
    pub fn evictions(&self) -> u64;          // observability counter (diagnostic, not a score)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FecError {
    BadMagic, UnknownVersion(u8), Truncated, ShardTooLarge(usize), OddShardLen(usize),
    BadShardIndex { index: u16, k: u16, m: u16 },
    BadShardCounts { k: u16, m: u16 },
    FrameTooLarge(u32),              // orig_len > MAX_ENVELOPE_BYTES, pre-buffer
    GroupInconsistent,               // (k,m,orig_len,shard_len) mismatch vs first shard
    BufferExhausted,                 // ingest refused: MAX_FEC_BUFFER_BYTES would be exceeded
    DecodeFailed,                    // reed-solomon-simd decode error / attempts exhausted
}
// error.rs: WireError::Fec(FecError) + Display arm + From<FecError>.
```

reed-solomon-simd usage inside (cached engines via `std::sync::OnceLock` keyed init; encoder:
`ReedSolomonEncoder::new(k, m, shard_bytes)` → `add_original_shard` ×k → `encode()` →
`recovery_iter()`; decoder: `ReedSolomonDecoder::new(k, m, shard_bytes)` →
`add_original_shard(i, ..)` / `add_recovery_shard(i, ..)` for whatever arrived → `decode()` →
`restored_original_iter()` fills the holes). Every crate error maps to `FecError::DecodeFailed` —
no `unwrap`/`expect` anywhere in `fec.rs` (§5, enforced by test + grep gate).

---

## 4. Redundancy-ratio tuning — bandwidth/battery vs correction, with the actual numbers

### 4.1 The math (i.i.d. loss p; group fails iff > m of k+m shards lost)

| k | m | overhead | P(frame lost), p=1 % | p=5 % | p=10 % | no-FEC baseline `1−(1−p)^k` at p=5 % |
|---|---|---|---|---|---|---|
| 4 | 2 | 50 %   | 2.0e-5 | **2.2e-3** | 1.6e-2 | 18.5 % |
| 8 | 3 | 37.5 % | 1.6e-5 | **1.5e-3** | 1.9e-2 | 33.7 % |
| 8 | 4 | 50 %   | 5e-7   | 2.7e-4 | 4.3e-3 | 33.7 % |

Reading: at the V5 cellular midpoint (5 % loss), the `CellularDefault` rule (m = max(2, ⌈k/2⌉))
turns an 18-34 % frame-loss rate into ~0.2 %, for 37-50 % extra bytes **on the telemetry lane
only**. The residual ~0.2 % is handled by the lane's semantics (telemetry is supersedable — the
next position update replaces the lost one) or, for L2 sync payloads, by the anti-entropy re-pull
that already exists. **FEC-group failure is therefore never data loss — it degrades to the ARQ
path of record.** That is the degrade-closed shape: exact reconstruction or typed failure, never
fabricated data (R4 §5.1 — FEC is deterministic-exact, unlike interpolation).

### 4.2 Honest caveats pinned into the doc and the defaults

- **Cellular loss is bursty, not i.i.d.** — the table is an upper-bound sizing guide, not a promise.
  Groups are small (one frame each, 4-12 datagrams over milliseconds), so a burst kills at most a
  frame or two, and the supersede/re-pull fallback absorbs it. The netem recipes V5 §4 prescribes
  (`delay 300ms 100ms loss 5% reorder 2%` etc.) are the tuning input: run them, append the measured
  residual-loss + overhead numbers to `BENCH_HISTORY.md`, then fix `RecoveryRule::Fixed(m)` per
  lane. V5 stops being an adoption gate (overruled) and becomes the **calibration source** —
  defaults ship now, tuned values replace them when measured.
- **Battery/bandwidth:** encode/decode CPU is negligible (crate does GB/s; our groups are ≤ ~14 KB).
  The real battery cost is radio airtime = the parity overhead, which is why FEC applies **only** to
  the small, latency-critical lane; the bulk `sync_pull` stream lane carries zero overhead. No
  standing background cost: no datagrams, no FEC work.
- **Adaptive ratio (loss-responsive m)** is deliberately NOT in scope: it would need a loss
  estimator per peer — a per-peer float — which walks straight at the NO-COURIER-SCORING /
  no-per-source-weight fence (synthesis §2.2 flags exactly this shape). Ratio is static per lane,
  operator-tuned from netem. If adaptivity is ever wanted, it must be re-adjudicated explicitly.

---

## 5. Hostile-input discipline — the decoder is a pre-auth attack surface (DoD-integrated, not an afterthought)

The FEC parser+reassembler sits **below the signature** — reachable by the peer before any
authenticity check (R4 §4's "real, honest cost"). It therefore meets the V3 parser discipline as
**requirements with tests**, same standing as `framing`/`wire_codec`:

1. **Bounded allocation, before buffering:** `orig_len` checked against `MAX_ENVELOPE_BYTES` and
   `k·shard_len ≥ orig_len` plausibility BEFORE any group state is allocated; total buffer capped at
   `MAX_FEC_BUFFER_BYTES` / `MAX_FEC_GROUPS_BUFFERED` with oldest-group eviction; a shard that would
   exceed the cap → `BufferExhausted`, connection stays up (typed refusal, not amplification).
   No allocation is ever sized by an attacker field alone.
2. **No panics, total parsing:** every read via bounds-checked `take()` (wire_codec pattern); no
   `unwrap`/`expect`/indexing in `fec.rs` (grep gate, §7); all reed-solomon-simd `Result`s mapped.
3. **Fail-closed decode:** `ingest` returns exact original bytes or a typed `FecError` — **never**
   partial/garbage bytes. Decode attempts per group capped (`MAX_FEC_DECODE_ATTEMPTS`) so a peer
   feeding crafted always-failing shard sets cannot buy unbounded CPU; attempts exhausted → group
   dropped.
4. **Parameter-confusion closed:** first-shard-pins-parameters + `GroupInconsistent` drop (§2.3);
   `index < k+m`, `k,m ≥ 1`, caps enforced; duplicate-index idempotent.
5. **Fuzzed:** `cargo-fuzz` target `fuzz_fec_ingest` — arbitrary byte sequences into
   `FecGroupBuffer::ingest` (fresh buffer per iteration + a stateful variant feeding 64 datagrams);
   invariants asserted inside the target: no panic, `buffered_bytes() ≤ MAX_FEC_BUFFER_BYTES`, any
   `Ok(Some(bytes))` has `bytes.len() ≤ MAX_ENVELOPE_BYTES`. CI smoke `-runs=100000`; longer runs
   local.
6. **Injection surface honestly stated:** QUIC datagrams are AEAD-protected inside the connection —
   an off-path attacker cannot inject shards at all; the threat actor for this surface is the
   **authenticated-but-malicious peer** (or a compromised TLS layer), which is exactly who the
   post-reconstruction `gate.check` and the bounds above are for. No constant-time requirement:
   nothing secret is processed below the gate.

---

## 6. DoD — falsifiable RED→GREEN tests (contract §2.2/§2.5/§2.17)

All in `proto-wire` (`fec.rs` unit tests + `tests/` integration). Names are the deliverable;
each lands in `REGRESSION-LEDGER.md`.

**T1 `fec_roundtrip_survives_any_m_erasures` (property).** Random frame bytes (1 B..64 KB), encode
`k+m`, delete every size-≤m subset (exhaustive for small k+m, sampled otherwise), shuffle arrival
order → reconstruct → **byte-identical** to the original. Also pins determinism: same shards in
any order → same bytes (R4 §5.1's exactness claim, now executable).

**T2 `fec_over_capacity_fails_typed_never_partial`.** Delete `m+1` shards → `DecodeFailed` (or the
group simply never completes); assert no `Ok(Some(..))` is ever produced. RED-form twin: hand a
truncated reconstruction to `wire_codec::decode_frame` and confirm IT also rejects — two
independent fail-closed layers.

**T3 — THE ADVERSARIAL ORDERING PROOF: `fec_valid_forgery_still_rejected_by_gate`.** The test the
operator's point 3 asks for, proving FEC reconstruction cannot bypass authentication:
1. Build an honest, anchor-rooted, hybrid-signed `SignedFrame` (reuse `anchored_frame` from
   `wss_transport.rs` tests:683).
2. **Attacker step:** take `wire_codec::encode_frame` bytes, flip one byte inside the payload
   region, then FEC-encode the *tampered* bytes into a **fully valid group** (all parity freshly
   computed over the tampered data — the codeword is FEC-perfect by construction).
3. Receiver: ingest all shards → reconstruction **succeeds** (assert `Ok(Some(bytes))` — the FEC
   layer is satisfied; this asserts the attack premise is real, not a strawman).
4. `wire_codec::decode_frame` parses (structure is valid) → `gate.check(..)` → assert
   `Err(WireError::CapabilityVerify(_))` — the signature over the tampered bytes fails.
   Assert the error is CapabilityVerify, NOT any `Fec*` variant: the forgery must die at the
   **auth** layer, proving the gate runs after — and independently of — FEC.
   RED form (proving the test bites): a deliberately mis-ordered harness that returns the
   reconstructed frame **without** `gate.check` accepts the forgery — demonstrating the ordering,
   not the code's mood, is the load-bearing control (UT-LAW test shape, synthesis §3.2).

**T3b `fec_reconstruction_is_transparent_to_auth` (converse).** Honest signed frame, drop `m`
shards → reconstruct → `gate.check` **passes**. Together with T3: FEC changes delivery, never
authenticity, in either direction.

**T4 malformed-shard suite (one test per row, exact expected error):** bad magic → `BadMagic`;
version 2 → `UnknownVersion(2)` (fail-closed version law, mirrors
`unknown_version_is_rejected_on_decode`); truncated at every header boundary → `Truncated`;
`k=0`/`m=0`/`k>64`/`m>32` → `BadShardCounts`; `index ≥ k+m` → `BadShardIndex`; odd `shard_len` →
`OddShardLen`; `orig_len = MAX_ENVELOPE_BYTES+1` → `FrameTooLarge` **with zero bytes buffered**
(assert `buffered_bytes() == 0`); second shard with different `m` → `GroupInconsistent` + group
dropped; duplicate index → ignored, group still completes.

**T5 `fec_buffer_bounded_under_group_spray`.** Ingest shards for 10 000 distinct groups (never
completing any): assert `buffered_bytes() ≤ MAX_FEC_BUFFER_BYTES` and groups ≤
`MAX_FEC_GROUPS_BUFFERED` throughout, eviction counter rises, and a subsequently-completed fresh
group still reconstructs (the buffer survives the attack functional).

**T6 `fuzz_fec_ingest`** (§5.5) — CI smoke run green, zero panics/OOM.

**T7 `quic_datagram_lane_delivers_with_m_losses` (Wave 2 integration).** Loopback quinn pair
(pattern of `quic_roundtrip_signs_and_verifies`, iroh_transport.rs:465): sender encodes a group but
**skips sending m chosen shards** (simulated loss — deterministic, no netem needed in CI); receiver
`recv()` returns the verified frame. Plus `quic_datagram_lane_replays_rejected`: same frame's
shards ingested twice → second completion → `ReplayDetected` (proves the lane carries the ledger,
§2.4/§0.3).

**T8 `fec_ratio_table_matches_binomial`.** The §4.1 table encoded as a unit test against a
`residual_loss_upper_bound(k, m, p)` helper — the sizing math is executable, not prose (VERIFIED-
BY-MATH).

**Static gates (§2.14):** grep gate — no `unwrap|expect|panic!|\[idx\]`-indexing in `fec.rs`
(allowlist: tests); the `FEC-IS-NOT-AUTH` doc-guard line must exist verbatim in `fec.rs` (doc-claim
gate greps for it); NO-COURIER-SCORING gate unchanged (fec.rs carries no per-source float —
`RecoveryRule` is per-lane, not per-peer).

---

## 7. Build plan — zero-context agent-executable

> Executor prerequisites: repo at `/root/bebop2-verify-redteam/bebop2/` (or the canonical bebop2
> checkout the operator designates — this worktree is read-only-for-product-code during THIS pass;
> the build happens on a feature branch of the live repo, remote `openbebop`, when the operator
> lifts "поки жодних комітів"). Toolchain: stable Rust ≥ 1.75, `cargo`, offline-capable (crate
> vendored or lockfile-resolved once).

**Wave 1 — `fec.rs` (pure, no carrier; mergeable alone).**
1. `proto-wire/Cargo.toml`: add `reed-solomon-simd = "3.1"` under `[dependencies]` with the DECART
   comment block (license MIT AND BSD-3-Clause, pure Rust, §1 table); run `cargo tree -i
   reed-solomon-simd` and record the (empty) transitive-C surface in the commit body.
2. Create `proto-wire/src/fec.rs` with §3's types/consts/signatures verbatim; register
   `pub mod fec;` in `lib.rs`; add `WireError::Fec(FecError)` + Display + From in `error.rs`.
3. Implement: `FecShard::parse/to_bytes` (bounds-checked `take()` copied from wire_codec.rs:55-63
   pattern), `encode_group` (pad-to-even/pad-last-shard, `ReedSolomonEncoder`), `FecGroupBuffer`
   (`HashMap` + `VecDeque` eviction order + byte accounting + attempt counter).
4. Write T1, T2, T4, T5, T8 (T4's RED rows first — each must FAIL against a stubbed
   always-`Ok` parser to prove it bites, then GREEN against the real one).
5. `cargo test -p bebop-proto-wire fec` green; `cargo clippy -p bebop-proto-wire -- -D warnings`.

**Wave 2 — QUIC datagram lane.**
6. `iroh_transport.rs`: retain `conn: Connection` in `QuicTransport` (thread through
   `from_parts`, both `connect`:238 and `accept`:279 already hold it); add fields
   `fec_buf: FecGroupBuffer`, `replay: ReplayLedger`, `max_frame_bytes: usize` (+ builder methods
   mirroring wss `with_replay_ledger`/`with_max_frame_bytes`).
7. Implement `send_unreliable` (§2.2 send side; consult `conn.max_datagram_size()`, degrade to
   `send()` when absent/too small) and extend `recv()` with the `select!` datagram arm feeding
   `fec_buf.ingest` → shared post-reconstruction path (§2.2 order EXACTLY: decode_frame → replay
   observe → size check → gate.check).
8. Write T3, T3b (they need the full gate; use the `anchored_frame` fixture), T7 pair.
9. Full crate test run green, including all existing wss/quic red-team tests untouched.

**Wave 3 — BPv7 bundle sharding (after Wave 1; independent of Wave 2).**
10. `bpv7.rs`: `StoreForward::shard_bundle`, `BundleReassembler` (§2.4); fresh nonce per
    shard-bundle; reassembler lifetime = bundle lifetime.
11. Tests: `sharded_bundle_delivers_with_any_k_of_km_couriers` (mem-transport fixture from
    bpv7.rs:304-370, deliver only k of k+m shard-bundles → logical payload reconstructs → gate
    verifies), `sharded_bundle_expiry_drops_group`, plus T3-shape forgery variant over bundles.

**Wave 4 — fuzz + ledger.**
12. `cargo fuzz init` under `proto-wire/fuzz/`, target `fuzz_fec_ingest` (§5.5); CI smoke job
    `-runs=100000`.
13. Append all test names to `REGRESSION-LEDGER.md`; append the first netem-measured
    residual-loss/overhead row to `BENCH_HISTORY.md` (V5 §4 recipes) and, if it disagrees with
    §4.1's defaults, flip the lane to `RecoveryRule::Fixed(m_measured)`.

Estimated diff: Wave 1 ~450 LOC + tests; Wave 2 ~150 LOC + tests; Wave 3 ~120 LOC + tests. One new
dependency total.

---

## 8. Doctrine compliance (one paragraph, no hedging)

FEC here is **deterministic-exact recovery** (same shards → identical bytes or typed failure) —
degrade-closed, replay-deterministic, zero interpolation; it lives on the opposite side of R4 §5.1's
line from Temporal Interpolation, which stays deferred (synthesis §4.4). It is a **reliability
control on lanes where loss is app-visible**, never an authenticity control (`FEC-IS-NOT-AUTH`
doc-guard + T3 pin this executably). It adds no per-source weight, score, or adaptive float
(NO-COURIER-SCORING intact; adaptive-ratio explicitly out of scope, §4.2). It introduces one
patent-clean, AGPL-compatible, pure-Rust dependency with a DECART record (§1). The signature gate's
authority, the replay ledger, the size caps, and the fail-closed version law all run unchanged on
reconstructed bytes — FEC widens the *delivery* funnel and leaves the *admission* funnel exactly as
red-teamed.

---

### Sources
- reed-solomon-simd 3.1.0 (2025-10-14, MIT AND BSD-3-Clause, ~1.77M downloads) — crates.io API +
  https://github.com/AndersTrier/reed-solomon-simd (commits verified 2026-07-17) ·
  https://docs.rs/reed-solomon-simd/latest/reed_solomon_simd/
- reed-solomon-erasure maintainer-wanted — https://github.com/rust-rse/reed-solomon-erasure (#88)
- QUIC-FEC (Michel et al., FEC vs retransmission tail latency; "improvement when
  target_latency < 2×RTT") — https://arxiv.org/pdf/1904.11326
- IETF adaptive-FEC-for-QUIC drafts —
  https://www.ietf.org/archive/id/draft-dmoskvitin-quic-adaptive-fec-00.html ·
  https://www.ietf.org/archive/id/draft-zheng-quic-fec-extension-00.html
- MoQ on FEC layering — https://moq.dev/blog/forward-error-correction/
- RFC 9221 (QUIC unreliable datagrams; quinn `send_datagram`/`read_datagram`/`max_datagram_size`)
- Local ground truth (fresh reads this pass): `proto-wire/src/framing.rs:22,41-65,102-117`,
  `envelope.rs:41-57`, `wire_codec.rs:1-63`, `wss_transport.rs:533-660`,
  `iroh_transport.rs:57-386`, `bpv7.rs:1-120,304-370`, `transport_policy.rs`, `sync_pull.rs`
  (outline), `proto-wire/Cargo.toml`; prior passes `R4-reed-solomon-fec-fit-grounding.md`,
  `BLUEPRINT-FAIL-OPERATIONAL-LAYOUT-VERSIONING-SYNTHESIS.md`,
  `V5-cross-platform-device-test-matrix.md:260-304` (via R4).
