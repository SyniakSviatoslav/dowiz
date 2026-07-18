# ROUND-2 — MMU/Isolation Platform Answer + `BebopHeader` Reconciliation (2026-07-17)

> **Status: design blueprint, no code, no commits.** Adjudicates `00-SOURCE-DIALOGUE.md` Part 3
> (`:152-217` — the `BebopHeader` C++ struct, MMU/MPU "Master Table" isolation, the
> Poisoned-Adapter scenario) against the two already-landed round-2 blueprints:
> `BLUEPRINT-SELF-CERTIFYING-BRIDGE-CONTAINMENT.md` (Fable-B: WASM deny-by-default import gate is
> the only real containment boundary today) and
> `BLUEPRINT-CONFIDENCE-WEIGHTED-RECONCILIATION-AND-PRIORITY-PRECISION.md` (Fable-C: sender-set
> confidence = REJECT-AS-CARRIER / ADOPT-AS-LOCAL-STATE), plus R1 §2 (CRC verdict) and the FEC
> blueprint (Fable-A, 41-byte shard header).
>
> **Operator ruling on Part 3 (verbatim):** "згідний з цим, також врахуй наступне, дуже важливо"
> — explicit agreement, stronger than "consider." The dialogue's own closing question is treated
> as the load-bearing one and is answered first, honestly, per real platform: *"чи є у вашій
> цільовій системі MMU, чи вам доведеться імітувати ізоляцію програмно?"*
> Agreement with an architecture binds its **intent and guarantees**; it cannot bind physics or
> OS policy. Where Part 3's *letter* (raw page-table manipulation by an app; a sender-set
> Confidence byte) collides with platform reality or an already-ratified rejection, this document
> says so plainly and delivers the closest design that keeps every guarantee Part 3 actually asks
> for.

---

## 0. TL;DR — five verdicts

1. **The MMU answer (§1):** on courier phones (unprivileged Android/iOS app) **raw page-table
   management of the app's own adapter threads is not available on any target, full stop** — all
   threads of one process share one address space and one page table; `mprotect` is process-wide
   and callable by any thread, so a "kernel thread" cannot durably revoke a sibling thread's
   access. True hardware isolation *between processes* exists on Android (constrained: manifest-
   declared `android:process` / `isolatedProcess` services + Binder/`SharedMemory` IPC, freezer/
   LMK lifecycle) and is **effectively unavailable on iOS** (no fork/exec; app extensions are
   OS-managed, single-purpose). On the owner-hub (operator Linux) real OS-process MMU isolation
   is standard and free — and the codebase has **already chosen its hub-native form**: the
   fail-closed KVM microVM tier (`kernel/src/isolation/microvm.rs`), which is strictly stronger
   than intra-OS page grants. **So: phones = software-simulated isolation (WASM SFI, already
   built); hub = real MMU isolation (microVM tier when the VMM lands).** That is the honest,
   complete answer to the dialogue's question.
2. **Part 3 vs Fable-B (§2): ADOPT-EQUIVALENT.** The MMU pseudocode is a bare-metal-vocabulary
   description of what the existing WASM containment already does: "Master Table" = the
   deny-by-default import grant table (`wasm-host/src/lib.rs:139-211`); "exactly two regions" =
   copy-in input + host-read output channel around a private linear memory; "out-of-bounds →
   hardware trap" = the WASM bounds trap (which wasmtime on 64-bit JIT hosts literally implements
   with MMU guard pages + SIGSEGV); "clear the Page Table Entry" = drop the instance/store and
   disable intake. Part 3 introduces **no new mechanism requirement**; it independently
   re-derives the mechanism Fable-B certified.
3. **`Confidence` (§3): REMOVED from the wire entirely.** Fable-C's §5.2 corollary is explicit —
   "do not put the field on the wire at all — not even as advisory" — and it pre-refutes the
   rename-to-provenance-fact escape (the HDOP test: any sender-settable quality datum that a
   fusion step consumes lets the sender steer the Kalman gain). The header byte is not renamed;
   it does not exist. Receiver-local `trace(P)` / `last_surprise` carry the idea (already
   ADOPT-NOW per Fable-C §5.3). A decode law pins the removal: reserved bytes must be zero.
4. **EpochID / Priority / Checksum (§4):** EpochID `u64` = adopted item #21 unchanged, no
   redesign. Priority: **semantic width is 2 bits** (Part 1's tiering, = #15 `CapabilityClass`,
   never self-assigned); Part 3's `uint8_t` is storage width only — discriminants `3..=255` are
   decode-rejects (Fable-C §4.2 law generalized), and a 0-255 *continuous* priority axis is
   rejected (a ranking axis + a self-assigned wire claim, both already-rejected classes).
   Checksum: **no CRC32 field** — R1 §2's DEFER-WITH-TRIGGER stands; the slot's purpose (payload
   integrity in one cheap pass) is served by the existing FNV-1a-64 content-address recomputed at
   the ingest gate, and transport-loss integrity is Fable-A's FEC layer + QUIC AEAD. Part 3's
   inclusion does not change the verdict; it re-raises the need, which was already met twice.
5. **Final deliverable (§5-§7):** `LaneFrameHeader` — 32 bytes, Rust, decoded field-by-field
   (no transmute, `forbid(unsafe_code)`-compatible), positioned at the **adapter→kernel lane
   boundary** (not as a cleartext network preamble — that carrier is synthesis row 4's
   REJECT-ON-PHYSICS; the network keeps the signed envelope). Poisoned-Adapter DoD maps the
   dialogue's detect→isolate→replace triad 1:1 onto Fable-B's `BridgeResult`/disable path and
   Fable-C's CWR/circuit-breaker — **zero new parallel mechanisms**.

---

## 1. The MMU question, answered per real target platform

### 1.1 The physics common to every target (why Part 3's pseudocode cannot run as written anywhere)

Part 3's model (`:181-189`): the kernel *thread* owns a "Master Table" of page-table entries;
each adapter *thread* is granted exactly `Read_Buffer` + `Write_Buffer`; on poisoning, the
kernel "скидає сторінки в Read-Only або очищає Page Table Entry."

On every OS we target (Linux, Android's Linux, iOS's XNU), the unit of address-space ownership
is the **process**, not the thread. All threads of one process share one `mm`/page-table
hierarchy. The consequences are not implementation details; they are the whole question:

- **`mprotect` is process-wide and unprivileged-symmetric.** Any thread may call it on any
  range of the shared address space. A "kernel thread" that marks an adapter thread's buffer
  read-only has *advised*, not *revoked*: the (by assumption compromised) adapter thread can
  call `mprotect` right back, or simply write through any other mapping it can name. Within one
  process there is no asymmetry of memory authority between threads — so an intra-process
  "Master Table" is **not a security boundary on any target**, courier or hub.
- **Per-thread permission views exist only as research/x86 niches.** x86 PKU (`pkey_mprotect`)
  gives per-thread views, but `WRPKRU` is an unprivileged instruction — a sandboxed thread
  rewrites its own permission register unless *code* is confined (i.e., unless you already have
  SFI, at which point SFI is the boundary). ARM's equivalent (POE/permission overlays) is too
  new to exist on the courier fleet and is not app-exposed on Android/iOS. ARM MTE (Pixel 8+
  class devices) is a memory-*safety hardening* feature with probabilistic tags, not a
  grant/revoke isolation API. None of these changes the verdict.
- Therefore the "Master Table owner" must be one of exactly two things: **the OS** (separate
  processes — real MMU isolation, granted *between* address spaces) or **a software runtime**
  (SFI — WASM linear memory + import gating). There is no third door. This is the precise
  content of the dialogue's own either/or question, and the split below answers it per target.

### 1.2 Courier phones — Android (unprivileged app)

- **Raw page-table access: no.** Covered by §1.1; additionally no `/dev/kvm` for apps, no
  privileged mm syscalls under the `untrusted_app` SELinux domain.
- **Separate OS processes: real but constrained — viable as *a* helper, not as an N-adapter
  fabric.** The supported form is manifest-declared: components with `android:process`, and
  `android:isolatedProcess="true"` services (which run in a near-empty SELinux domain with no
  permissions of their own — genuinely strong, OS-enforced, MMU-backed isolation). Constraints
  that matter for a mesh node: process names are **static manifest declarations** (no dynamic
  per-adapter spawning; `exec()` of binaries from app-writable storage is blocked since
  API 29's W^X rule, and raw `fork()` without exec is unsupported under ART/Binder); IPC is
  Binder/AIDL with `android.os.SharedMemory` (ashmem/memfd) for the ring-buffer analog; child
  processes are subject to the cached-process **freezer** (Android 11+) and LMK kills, so an
  adapter subprocess can be frozen or reaped whenever the app backgrounds — exactly when a
  courier's phone is in a pocket. Verdict: admissible as one static, restartable sandbox
  process if a concrete need ever outgrows WASM; not the general adapter substrate.
- **WASM SFI: available and already the codebase's built mechanism.** JIT is permitted for
  Android apps (anonymous `PROT_EXEC` mappings are allowed in-app — this is how every browser
  and V8-embedding app works), so wasmtime runs at full speed. The Fable-B gate applies as-is.
- **TrustZone/StrongBox: out of scope by design.** Apps get Keystore key custody and
  attestation, not third-party code-in-TEE. Already scoped for capability attestation elsewhere
  this session; it is not an adapter-isolation option and is deliberately not conflated here.

### 1.3 Courier phones — iOS (unprivileged app)

- **Raw page-table access: no** (§1.1, plus the App Sandbox).
- **Separate OS processes: effectively unavailable.** `fork`/`posix_spawn` are denied to
  sandboxed apps; the only multi-process form is Apple-defined **app extensions** — OS-launched,
  single-purpose (widget, share, network-extension...), tightly memory-capped, and not a
  general-purpose "run my adapter" facility. There is no per-adapter process option on iOS.
- **WASM SFI: the only option — with one honest performance note.** iOS forbids
  writable-then-executable pages for apps (no JIT entitlement outside the browser), so the
  runtime must interpret: `wasmi` or wasmtime's Pulley interpreter, not wasmtime's JIT.
  Guarantees are identical (bounds checks are explicit software checks instead of guard-page
  traps — same invariant, different trap mechanism); throughput is interpreter-class. Adapter
  translation workloads (layout shims over frames) are small and per-message, so this is
  acceptable; it is stated here so nobody later "discovers" it as a surprise.
- **Secure Enclave:** same non-conflation note as Android.

### 1.4 Owner-hub (operator-controlled Linux server)

**Here the dialogue's scheme is real, standard, and essentially free** — with the important
correction that the codebase has already chosen its concrete form:

- Unix processes ARE per-adapter MMU isolation: each subprocess has its own page tables; the
  shared ring is a **sealed memfd** (`memfd_create` + `F_SEAL_SHRINK|F_SEAL_GROW`, mapped
  read-only into the adapter for the input side); an out-of-bounds write in the adapter raises
  **SIGSEGV in the adapter process only** — Part 3's "Hardware Trap," delivered exactly where
  the dialogue wants it; the kernel process observes death via `pidfd`/`waitpid`. "Очистити
  Page Table Entry" = unmap/close the memfd and/or kill the subprocess. `seccomp` +
  namespaces tighten ambient authority. All of this is stock Linux; no capability the operator
  does not already have.
- **The codebase's already-scoped native tier is stronger than bare processes:** the KVM microVM
  tier (`SandboxTier::NativeProcessRequiresKvm`, fail-closed probe,
  `kernel/src/isolation/microvm.rs:20-76`) puts the adapter behind *two-stage* translation
  (guest page tables + EPT/NPT) — a hardware Master Table the guest cannot even name. Fable-B
  records it honestly as probe-only today (VMM launch is the follow-up); until the VMM lands,
  native adapters are refused on this host, which is the correct fail-closed posture.
- **Design ruling (anti-duplication):** the hub realization of Part 3 is the **existing microVM
  tier**, not a new bare-process+seccomp tier. Adding an intermediate `SandboxTier` variant is
  admissible only via its own DECART with a concrete workload that needs it (per the
  new-dep/new-tier rule); this blueprint does not add one.

### 1.5 The answer to the dialogue's closing question, in one paragraph

**Courier target: no usable MMU-grant facility — isolation is software-simulated, and the
simulation is already built and tested** (the WASM deny-by-default gate, Fable-B §2.1). **Hub
target: yes — real MMU isolation is available and already scoped as the KVM microVM tier**,
which exceeds Part 3's page-grant scheme. The system is therefore *heterogeneous by necessity*:
one containment **contract** (deny-by-default grants, two-channel data flow, trap-on-violation,
revoke-by-teardown), two **enforcement substrates** (SFI on phones, hardware virtualization on
the hub), selected by `SandboxTier` exactly as the existing code already does. Nothing in
Part 3's guarantees is lost on any platform; only its *letter* (an app editing PTEs) was never
available anywhere, including on the hub, where the OS/hypervisor does it on the kernel
process's behalf.

---

## 2. Reconciliation with Fable-B: Part 3 is ADOPT-EQUIVALENT, not a new requirement

Point-by-point mapping of Part 3's mechanism onto the containment Fable-B live-verified:

| Part 3 (`:178-193`) | Existing mechanism (Fable-B §2.1, live-read) | Delta |
|---|---|---|
| Kernel owns a "Master Table" of memory grants | Deny-by-default `Scope` → import mapping; rejection at instantiation, before the component runs (`wasm-host/src/lib.rs:139-211`, `:111`) | none — same authority shape, table keyed by capability grant instead of page frame |
| Adapter sees exactly `Read_Buffer` + `Write_Buffer` | Private linear memory; host copies input in; output = return bytes read only after clean return | none — WASM is *stricter*: the adapter cannot name kernel memory at all, vs. Part 3 where it merely lacks write permission |
| OOB write → Segmentation Fault / Hardware Trap | WASM bounds trap → `BridgeFault::Trapped`. On 64-bit JIT hosts wasmtime implements this with virtual-memory reservations + guard pages, i.e. **literally an MMU trap under the hood**; on iOS-interpreter it is an explicit software check — same invariant | mechanism detail only, per §1.3 |
| Revoke: reset pages / clear PTE | Drop the `Store`/instance (the linear memory ceases to exist) + disable lane intake | none |
| "Ядро перевіряє лише ДЕ він це робить і ЧИ вкладається в часові рамки" | Exactly Fable-B's C′ thesis: the kernel is *indifferent* to adapter claims; containment bounds reach, the epoch tick bounds time | none — Part 3 restates CSC-LAW's premise |

**Verdict: ADOPT-EQUIVALENT.** Part 3 independently re-derives, in bare-metal vocabulary, the
containment architecture Fable-B already certified for the WASM tier and scoped for the microVM
tier. The operator's "згідний з цим" is therefore *already implemented in its load-bearing
part* — no raw-MMU work item exists. The only genuinely new artifacts Part 3 contributes are
(a) the concrete header struct — reconciled in §3-§5 — and (b) the Poisoned-Adapter scenario as
a named, testable end-to-end drill — adopted as the DoD in §7. One boundary restated to prevent
drift: this equivalence holds for the **WASM and microVM tiers only**; a plain in-process Rust
module remains NOT a containment boundary (Fable-B §2.1 tier 3), and Part 3 gives no reason to
revisit that — a sibling thread is precisely what §1.1 shows cannot be contained.

---

## 3. The `Confidence` field: removed from the wire, not renamed

Part 3 (`:170,177`) puts `uint8_t Confidence` in the wire header, **written by the adapter**
("Адаптер пише сюди рівень впевненості... Ядро читає це за один такт"), and gates interpolation
on it (`:192`). This is byte-for-byte the shape Fable-C §5.2 adjudicated:
**REJECT-AS-CARRIER, ADOPT-AS-LOCAL-STATE**, on three grounds that transfer without
modification — it is a self-reported metric priced at zero for a forger (`Confidence = 100` on
forged frames is free, honest senders on noisy channels honestly report less, so any consumer
selects *for* the adversary); it is the #15 self-assigned-quality threat one field over; and it
is `key_K` self-certification (the kernel must be structurally indifferent to what a sender
claims about its own output — the exact Fable-B C′ inversion).

**The rename escape is closed, deliberately.** The task asks whether a mechanical
"SampleQuality"/provenance fact ("redundant vs primary sensor") could stay on the wire. Fable-C
already tested this exact shape with GPS HDOP-as-claimed-`R` and rejected it: *any*
sender-settable datum that a fusion or rescue step consumes is a steering input to the Kalman
gain (claim pristine quality → your bytes dominate the fuse), regardless of how mechanical its
name sounds. And its §5.2 corollary forbids the "advisory field" compromise: every wire field
eventually finds a consumer. The precise final design is therefore single-option, as required:

1. **No confidence, quality, health, or trust byte exists anywhere in the header** (§5). The
   layout has no reserved-for-confidence slot; reserved bytes are decode-enforced zero, so the
   field cannot be squatted back silently (§7 T4 pins this).
2. **The idea lives receiver-side, where Fable-C already adopted it:** `trace(P)` (staleness of
   the local estimate, grows with every `predict()`-without-`update()`) and `last_surprise`
   (innovation magnitude, advisory-only) — computed locally, never transmitted, never read from
   any wire field. Part 3's "інтерполяція, якщо це дозволяє Confidence" becomes: interpolation
   is permitted iff *the receiver's own* `trace(P)` is under the fixed staleness bound and the
   lane is `Telemetry`-tier (Fable-C §2.5/§3.3) — the same gate, honest owner.
3. **If a genuine provenance fact is ever needed** (e.g., a stream that really is a redundant
   sensor), it belongs in the **capability grant / stream registration** — gate-verified at
   admission, per-stream, cryptographically bound to the grantor — not in a per-packet
   self-claimed byte; and even then it may only select a receiver-side channel-class constant,
   never set `R` directly. This is a statement of where such a fact would live, not a proposal
   to add one now.
4. **Sender-settable header bits in general — the self-incrimination rule (new, one line):** a
   future flag bit is admissible only if setting it can *worsen* the treatment of the sender's
   own lane, never improve it. The dialogue's `IsDegraded` passes this test structurally
   (claiming degraded on good data sabotages only yourself; claiming clean on bad data gains
   nothing the gate doesn't already re-check) but is still deferred until a consumer exists;
   `NeedsInterpolation` **fails** it (an instruction to the receiver — the receiver decides
   rescue from its own state, point 2). `Confidence` fails it maximally. No flag bits are
   assigned in this blueprint.

---

## 4. EpochID, Priority, Checksum — against the adopted ledger

### 4.1 `EpochID: u64` — adopted item #21, no redesign

Confirmed as-is: monotone logical epoch, max-merge, **inside the signed envelope**
(synthesis supplementary row "EpochID in the header," register #21/#32/#156/#159). Part 3's
`uint64_t EpochID` matches the adopted width and semantics exactly. Its *timeliness* use
("адаптер не виставляє EpochID вчасно" as poison detection) is likewise already adopted —
it is the epoch-tick deadline of the disable path (§6 step D1). Nothing to change.

### 4.2 `Priority: u8` — 2-bit semantics in 1-byte storage; the 0-255 axis is rejected

The discrepancy is real: Part 1 specifies 2-bit tiers (`00/01/10`, `:72-74`); Part 3 widens to
`uint8_t Priority (0-255)`. Resolution, from the already-adopted rulings rather than taste:

- **Semantic width is and stays 2 bits** — the tier is #15 `CapabilityClass`
  (Critical / Telemetry-Important / Optional), ALREADY-EQUIVALENT-WITH-CORRECTION per synthesis
  row 62, and the correction is the load-bearing part: the tier is **checked against capability
  scope, never self-assigned**. The enum needs no wider representation — there is no fourth
  tier, and Fable-C §4.2 already ruled the fourth 2-bit codepoint (`11`) a typed decode-reject.
- **A 0-255 priority is rejected as semantics.** A continuous priority scale is a ranking axis
  on the wire — inviting exactly the comparative-arbitration consumer R3 rejected — and as a
  sender-set claim it is the #15 threat again. Part 3 gives no use for the extra values (its
  own scenario uses three tiers); the byte is layout convenience, not a requirement.
- **Adopted form: `tier: u8` as storage, discriminants `0|1|2` valid, `3..=255` → typed decode
  reject** (Fable-C §4.2's law, generalized from 1 undefined codepoint to 253), **and** the
  decoded tier must equal the stream's granted `CapabilityClass` — mismatch → reject at the
  gate. The wire byte is a redundancy/consistency check; **the grant is the authority.** So
  Part 3's byte survives as exactly that: a byte-wide slot whose upper values are unused *and
  rejected*, not unused and available.

### 4.3 `Checksum: u32` (CRC32) — R1's verdict stands; the field does not appear

R1 §2's ruling was precise and remains correct after Part 3: zero CRC in tree, the role served
twice (FNV-1a-64 per-tile content-address, `spectral_cache.rs:103-111`, recomputed-and-compared
on access — item 152; sha3 Merkle root cross-party), a standing second checksum authority = the
dual-authority hazard (#23/#47), DEFER-WITH-TRIGGER as a *private acceleration* of the
content-address recompute only. Part 3 changes none of the premises — it re-raises the **need**
(cheap payload-integrity check at the header boundary), which is met without a new authority:

- **End-to-end integrity at the lane boundary:** the header carries the payload's existing
  **FNV-1a-64 content-address** (same algorithm, same authority as the tree already uses), and
  the ingest gate **recomputes and compares**; mismatch → `BridgeFault::DecodeReject`. The
  header copy is a commitment the sender makes; the recompute is the only authority; the wire
  value is never "trusted" and never corrected-from. One algorithm, one authority — the hazard
  R1 named cannot arise.
- **Transport-corruption integrity is a different layer and is already owned:** Fable-A's FEC
  shard header (41 bytes: magic/version/trace/group/index/k/m/orig_len/shard_len) deliberately
  carries **no checksum** — QUIC AEAD authenticates the datagram above it, RS decode +
  `wire_codec` fail-closed decode + the signature gate catch corruption below it. A CRC in the
  lane header would duplicate that stack at the wrong layer, adding the third authority R1
  refused. Confirmed: Part 3's Checksum need is **subsumed** — by FNV at the lane boundary and
  by the FEC/AEAD/signature stack on the wire.
- The HW-CRC acceleration trigger (measured hot content-address recompute, number appended to
  `BENCH_HISTORY.md` first) is untouched and still available.

### 4.4 `Magic: u32` and `alignas(64)` — two placement corrections

- **Magic is admissible only because this header is NOT a network preamble.** Synthesis row 4
  rejected the cleartext-first-bytes Magic/SchemaID carrier ON-PHYSICS (register #93/#157); the
  network keeps the signed envelope's version discriminant (`framing.rs:54-60`, hard-reject
  law). Part 3's header therefore lives at the **adapter→kernel lane boundary** — the sandbox
  output channel Fable-B's ingest gate already decodes — where a magic word is a cheap
  type-confusion tripwire on host-local bytes, not a security carrier. §5 fixes this placement
  in the type's name and docs so it can never migrate to the wire by copy-paste.
- **`alignas(64)` is an in-memory concern, not a wire law.** The lane layout is a packed
  32-byte little-endian record decoded field-by-field (`u64::from_le_bytes` at const-asserted
  offsets — no transmute, no `unsafe`, no endianness UB; the zero-parse intent of `:176`
  survives as fixed-offset reads). Padding the record to 64 wire bytes buys nothing; if a
  measured profile ever shows the *decoded* struct wants cache-line alignment, that is
  `#[repr(align(64))]` on the in-memory type, taken under the bench discipline, invisible to
  the layout.

---

## 5. The deliverable: `LaneFrameHeader` — final reconciled design (Rust)

```rust
//! Lane-boundary frame header — the reconciled `BebopHeader`.
//!
//! PLACEMENT LAW: this header frames records crossing the ADAPTER→KERNEL lane
//! (the sandbox output channel consumed by the BridgeResult ingest gate,
//! CSC/UT-LAW substrate). It is NEVER a network preamble: on the wire, the
//! signed envelope + framing version discriminant govern (synthesis row 4;
//! cleartext magic REJECT-ON-PHYSICS), and datagram transport is the FEC
//! shard layer (Fable-A). Decoding is field-by-field from little-endian
//! bytes at fixed offsets — no transmute, forbid(unsafe_code)-compatible.

/// Lane-record type tripwire (host-local; not a security carrier).
pub const LANE_MAGIC: u32 = 0xBEB0_0BEE;

/// Lane schema version. Unknown value → `DecodeReject` (framing.rs law).
pub const LANE_SCHEMA_V1: u16 = 1;

/// Header size: exactly 32 bytes. Const-asserted; golden-bytes test T6.
pub const LANE_HEADER_BYTES: usize = 32;

/// Tier discriminants — storage u8, semantics 2-bit (#15 CapabilityClass).
/// 3..=255 → DecodeReject (Fable-C §4.2 law, generalized).
/// The decoded tier MUST equal the stream's granted CapabilityClass;
/// mismatch → reject at the gate. The GRANT is the authority (#15:
/// never self-assigned); this byte is a consistency check only.
pub const TIER_CRITICAL: u8 = 0;   // never interpolated; refuse-and-escalate
pub const TIER_TELEMETRY: u8 = 1;  // CWR-eligible (Fable-C §2.2 five clauses)
pub const TIER_OPTIONAL: u8 = 2;   // droppable

/// Fixed 32-byte layout, little-endian, offsets law-fixed:
///
///   off  size  field
///   0     4    magic            == LANE_MAGIC            else DecodeReject
///   4     2    schema_version   == LANE_SCHEMA_V1        else DecodeReject
///   6     1    tier             in {0,1,2} AND == grant  else DecodeReject
///   7     1    flags            MUST be 0x00 (all bits reserved; any set
///                               bit → DecodeReject; future bits must pass
///                               the §3.4 self-incrimination rule)
///   8     8    epoch_id         adopted item #21 (monotone, max-merge)
///   16    4    payload_len      <= MAX_LANE_PAYLOAD_BYTES, checked BEFORE
///                               any payload read (single source of truth
///                               with the framing cap)
///   20    4    reserved         MUST be 0x0000_0000      else DecodeReject
///                               (pins the ABSENCE of Confidence/CRC slots)
///   24    8    content_address  FNV-1a-64 of payload (the tree's existing
///                               per-tile algorithm). Gate RECOMPUTES and
///                               compares; mismatch → DecodeReject. The
///                               recompute is the sole authority — the wire
///                               value is a sender commitment, never trusted,
///                               never a second algorithm (R1 §2 honored).
///
/// ABSENT BY RULING (§3, §4.3): Confidence (any spelling), CRC32/Checksum,
/// per-packet priority 0-255, sender-set quality/health of any kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaneFrameHeader {
    pub epoch_id: u64,
    pub payload_len: u32,
    pub tier: u8,
    pub content_address: u64,
    // magic / schema_version / flags / reserved are validated at decode and
    // not retained: they carry no post-decode information by construction.
}

/// Decode outcome vocabulary — REUSED from Fable-B, not duplicated:
/// every failure arm above maps into the existing
/// `BridgeFault::DecodeReject { offset }` / `NonCanonical` / `BoundsReject`,
/// and the only two lane outcomes remain
/// `BridgeResult::{Translated(Provenanced<T>), Failed(BridgeFault)}`.
```

Deliberate properties, each traceable to a ruling above: 32 bytes exactly (Part 3's size intent,
`:176`); no Confidence byte and no reserved slot that could become one (§3, pinned by the
must-be-zero law + T4); no CRC (§4.3 — content_address is the same single authority the tree
already has); tier is a cross-check, the grant decides (§4.2); EpochID unchanged (§4.1); decoded
without `unsafe` at fixed offsets (§4.4); enters the kernel **only** through the existing
`BridgeResult` ingest gate — this blueprint adds zero new admission surfaces and zero new result
types.

---

## 6. The Poisoned-Adapter scenario, mapped onto existing mechanisms only

Part 3's triad (`:187-193`) — Detection → Isolation → Replacement — lands 1:1 on machinery
already adopted this round. No third parallel mechanism is introduced; that absence is the
design.

**D — Detection (three signals, three existing homes):**
- **D1 missed EpochID deadline** → the lane is empty at the epoch tick → the existing
  adapter-death disable path (R1 §4.2 "empty buffer → kernel disables that adapter"). Not a new
  `BridgeFault` arm: `BridgeFault` is per-invocation; deadline-miss is per-epoch absence.
- **D2 hardware trap** → WASM trap (or, hub microVM tier, guest fault) →
  `BridgeFault::Trapped` — Fable-B §2.2, unchanged. Alive-but-garbage adapters (every frame
  failing the gate — the case death-detection misses) → Fable-C §4.1's deterministic
  circuit-breaker: integer count of *consecutive* boolean gate refusals, fixed bound N →
  intake disabled. Never a health score, never innovation magnitude.
- **D3 checksum mismatch** → content-address recompute mismatch at the ingest gate →
  `BridgeFault::DecodeReject` (§4.3/§5).

**I — Isolation ("The Kill"):** drop the sandbox instance (WASM `Store` teardown — the linear
memory ceases to exist, §2 row 4; hub: VM/process teardown, memfd unmapped) **and** disable
lane intake, typed and observable. Re-admission is operator action or the deterministic
cool-down (Fable-C §4.1) — never an automatic trust judgment.

**R — Replacement/Degradation, by tier (the corrected `:190-193`):**
- **`TIER_CRITICAL`:** refuse-and-escalate + last-VALID served read-only + recompute-from-truth
  where a derivation path exists (Fable-C §3.4; operator red-line ruling). **Never
  interpolated** — Part 3's "інтерполяція, якщо дозволяє Confidence" is doubly corrected here:
  the tier forbids it, and the gate that would have allowed it is receiver-local `trace(P)`,
  not a wire byte (§3).
- **`TIER_TELEMETRY`:** CWR — `KalmanFilter::predict()` forward (dead-reckoning) under the
  fixed `trace(P)` staleness bound; `update()` fuses when a valid frame resumes (Fable-C
  §2.2/§2.5, ADOPT-NOW boundary). This *is* Part 3's "старий кешований тензор або інтерполяція"
  with the confidence gate re-homed to the receiver.
- **`TIER_OPTIONAL`:** drop; observable flag only.

---

## 7. DoD — falsifiable tests

RED-first where a hole is being proven; all reuse the Fable-B adversary harness (malicious WASM
adapter) and Fable-C's CWR/circuit-breaker types. No new mechanism appears in any expected
outcome.

| # | Test | Proves | Expected |
|---|---|---|---|
| T1 | `poisoned_adapter_trap_isolate_serve_telemetry` | the full Part 3 drill, telemetry lane | inject traps → `Failed(Trapped)` each; after N consecutive → intake disabled (circuit-breaker); consumer receives `predict()` values with strictly growing `trace(P)`; past the staleness bound → typed refusal, not a stale value |
| T2 | `poisoned_adapter_critical_lane_never_interpolates` | tier law under poisoning | same injection on a `TIER_CRITICAL` lane → last-VALID readable + typed refusal on any mutating consume; trybuild compile-fail (synthesis §3.1) that no interpolation API exists on the Critical type |
| T3 | `missed_epoch_deadline_disables_lane` | D1 path | adapter goes silent past the epoch deadline → lane disabled, observable event; last-good served per tier rules; re-admission only via cool-down/operator |
| T4 | `reserved_and_flags_must_be_zero` | §3's removal is permanent, not remembered | fuzz headers with any nonzero flag/reserved byte → `DecodeReject` for every value; **this test is the executable pin of the Confidence removal** — re-adding any byte flips it and forces this document's revision |
| T5 | `tier_discriminants_and_grant_authority` | §4.2 | tier `3..=255` → `DecodeReject`; tier byte ≠ granted `CapabilityClass` → gate reject (never-self-assigned #15, the chaos-test form) |
| T6 | `lane_header_layout_pinned` | §5 layout law | const-assert size == 32 and every offset; golden-bytes encode/decode round-trip; header never transmuted (grep-guard for `transmute` stays clean under `forbid(unsafe_code)`) |
| T7 | `content_address_recompute_is_sole_authority` | §4.3 | flip one payload byte after address computation → `DecodeReject`; flip the header address instead → same reject (the recompute wins in both directions; the wire value can never override it) |
| T8 | `unknown_lane_schema_rejected` | §5 version law | extends the existing `unknown_version_is_rejected_on_decode` red-team pattern (`framing.rs:102-114`) to the lane header |

Falsification criterion: the blueprint's containment claim fails iff any injection in T1-T3
produces kernel-memory effect, silent-partial data, an interpolated Critical value, or an
un-disabled poisoned lane; the reconciliation claims fail iff T4/T5/T7 can pass with a
Confidence byte, a live 0-255 priority, or a second checksum authority present.

---

## 8. Exists-today vs to-build

| Piece | Status | Where |
|---|---|---|
| WASM deny-by-default containment (the "Master Table") | **Built + tested** | `bebop2/wasm-host/src/lib.rs:111,139-211,244-293` (Fable-B §2.1) |
| microVM tier, fail-closed probe (hub's real-MMU form) | **Probe built; VMM = follow-up** | `kernel/src/isolation/microvm.rs:20-76,156` |
| `BridgeResult` / `BridgeFault` / `Provenanced` / ingest gate | **NEW (Fable-B blueprint)** — this doc adds no variants | Fable-B §2.2 |
| CWR boundary + `trace(P)` bound + circuit-breaker | **NEW, ADOPT-NOW (Fable-C §5.3/§4.1)** — reused here unchanged | Fable-C §2.2, §4.1 |
| EpochID, #15 tier law, framing version hard-reject, FNV content-address | **Built / adopted register items** | `framing.rs:54-60`; `spectral_cache.rs:103-111`; synthesis rows 4/62 + EpochID row |
| FEC shard layer (transport integrity, no checksum by design) | **NEW (Fable-A blueprint)** | `BLUEPRINT-REED-SOLOMON-FEC.md` §2.3 |
| `LaneFrameHeader` + constants + decode laws + T1-T8 | **NEW — this blueprint's deliverable** | §5-§7 |
| Raw MMU/page-table work item | **NONE — dissolved by §1-§2** | — |
| Per-adapter OS processes on phones; new bare-process hub tier; wire Confidence/CRC/0-255 priority | **NOT adopted** (Android helper-process option documented §1.2 for future DECART only) | §1-§4 |

## 9. Provenance

Read in full this pass: `00-SOURCE-DIALOGUE.md` (all three parts, Part 3 `:152-230` including
the operator ruling), `BLUEPRINT-SELF-CERTIFYING-BRIDGE-CONTAINMENT.md` (whole),
`BLUEPRINT-CONFIDENCE-WEIGHTED-RECONCILIATION-AND-PRIORITY-PRECISION.md` (whole),
`BLUEPRINT-REED-SOLOMON-FEC.md` §2.2-2.4 (pipeline, shard layout, constants),
`R1-layout-versioning-bridges-grounding.md` §2 + verdict table,
`BLUEPRINT-FAIL-OPERATIONAL-LAYOUT-VERSIONING-SYNTHESIS.md` rows 4/5/9/62 + EpochID row.
Platform facts (§1) are stated from OS-level ground truth (thread/address-space model,
`mprotect` semantics, Android `android:process`/`isolatedProcess`/W^X/freezer, iOS
no-fork/no-JIT/extensions model, PKU/POE/MTE status, wasmtime guard-page bounds checks,
Linux memfd sealing/pidfd) — each stated conservatively and marked where it is a constraint
("no raw PTE access") vs an option ("one static helper process"). No product code touched;
no commits; output confined to the round-2 directory as instructed.
