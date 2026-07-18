# BLUEPRINT — Round-2 Master Synthesis: Fail-Operational / Layout-Versioning (2026-07-17)

> **Status: PLANNING consolidation. No code, no execution, no commits** (standing "поки жодних
> комітів"). Consolidates the five round-2 blueprints in this directory into one document:
>
> - `BLUEPRINT-REED-SOLOMON-FEC.md` — **Fable-A** (FEC adopted; placed on loss-visible lanes only)
> - `BLUEPRINT-SELF-CERTIFYING-BRIDGE-CONTAINMENT.md` — **Fable-B** (Pattern C′ / CSC-LAW)
> - `BLUEPRINT-CONFIDENCE-WEIGHTED-RECONCILIATION-AND-PRIORITY-PRECISION.md` — **Fable-C**
>   (CWR, critical-tier resolution, ConfidenceLevel adjudication)
> - `BLUEPRINT-MMU-ISOLATION-HEADER-STRUCT-RECONCILIATION.md` — **Fable-D**
>   (per-platform MMU answer, `LaneFrameHeader` final design)
> - `BLUEPRINT-DELTA-KERNEL-DIFF-TOPOLOGY.md` — **Fable-E** (source-dialogue Parts 4–5:
>   delta-kernel / adapter-as-diff-generator — ADOPT-EQUIVALENT with the zero-cost caveat named)
>
> against the round-1 base `../BLUEPRINT-FAIL-OPERATIONAL-LAYOUT-VERSIONING-SYNTHESIS.md`
> ("R1-synthesis" below; its own sources R1–R4 + `00-SOURCE-DIALOGUE.md`) and the 20-point
> contract `../../CORE-ROADMAP-STANDARD-2026-07-17.md` (Layer A–I altitude axis, §3 there).
> Every claim below traces to one of those documents; nothing is asserted from memory.

---

## 1. What Changed From Round 1 — the four operator rulings

Round 1 ended with a 14-row primary ledger + 12-row supplementary ledger (R1-synthesis §1),
two action findings (§2), and four DEFER-WITH-TRIGGER items (§4). Four operator rulings then
drove round 2:

### Ruling 1 — *"reed-solomon will be used, add FEC too"*

Flipped R1-synthesis row 13 / §4.2 (**DEFER-WITH-TRIGGER**) to **ADOPT-NOW**. Fable-A's honest
reconciliation (§0): *the decision changed; the physics did not.* Both live carriers (WSS/TCP,
iroh/QUIC streams) still do ARQ, so FEC stays OUT of the reliable-stream lanes where it is
physically inert (Fable-A §0.2). Adoption means building FEC where loss IS app-visible: **L1**
the new QUIC unreliable-datagram lane (RFC 9221, quinn 0.11 already a dependency) for
latency-critical supersedable telemetry — quantified: an 8-datagram-class ML-DSA-65 signed
frame at 5% loss goes from ~18.5% (k=4) / ~33.7% (k=8) unreconstructable to ~0.22% with the
CellularDefault parity rule (~84× for k=4, m=2) (Fable-A §0.1, §4.1); **L2** BPv7 bundle
sharding across couriers/paths (RAID-across-couriers, the only lane where FEC buys delivery
probability under partition); **L3** future non-ARQ carriers, pre-hardened. R4's trigger is
inverted from "adopt when" to "already built for" (Fable-A §0.1). Crate:
`reed-solomon-simd = "3.1"` (v3.1.0, MIT AND BSD-3-Clause, DECART in Fable-A §1). Doctrine
unchanged: FEC is a reliability control, never authenticity; FEC-decode sits BELOW crypto-verify
(Fable-A header + §2.2).

### Ruling 2 — *"adapter self-certifies its own work — because in terms of failure the adapter bridge is failing & not poisoning the other parts; fail-operational can be used but not for red-line items like money"*

Commissioned **Pattern C′ / CSC-LAW** (Fable-B) — a precise, honestly-bounded third pattern
between the rejected A′ (trusted proxy) and adopted B′ (untrusted translator), **not** a blanket
trust relaxation. The kernel is *indifferent* to the bridge's self-certification (zero authority
delta); safety comes from three containment layers: **spatial** (WASM deny-by-default import
gate — the only real boundary today; microVM tier pending VMM), **structural** (sealed
`BridgeResult<T>` — outcome universe exactly `{Translated(canonical), Failed(loud)}`, no
data-carrying third state), **authority** (red-line resources un-nameable in any bridge scope +
un-typeable at commit via the sealed `RedLineAdmissible` trait) (Fable-B §2). The honest split:
**RC-2-narrow is fully closed by construction; RC-2-broad carries an information-theoretic
residual** — a well-formed translation of wrong content is structurally undetectable — pinned by
executable test T4, not swept away (Fable-B §3–§4, restated in §5 below). The operator's clause
3 ("not for red-line items") is the *load-bearing premise*, not a carve-out: delete it and
clause 2 becomes false (Fable-B §4). B′ stays mandatory for red-line data (Fable-B §5.1).

### Ruling 3 — *"готувати інфраструктуру наперед як основа всього — не чекати поки розвалиться"* (build ahead, don't wait for it to break)

Flipped the **CWR boundary** from the round-1 interpolator deferral (row 10 / §4.4) to
**ADOPT-NOW** — the same inversion shape as FEC's (Fable-C §5.3, which names the FEC precedent
explicitly). Scope of the flip is the *infrastructure half only*: the five-clause type boundary
(`AdmittedFrame<S>`/`Predicted<S>`/`Fused<S>`, tier compile-fail, `trace(P)` staleness bound)
plus the day-one `ema_next` re-expression as first consumer; dead-reckoning *feature* consumers
beyond that stay demand-driven, per the ruling's own text ("інфраструктуру", not features), and
the ruling itself includes "не силентно фліпати кожен DEFER" — build-ahead has no force on
inadmissible mechanisms (Fable-C §5.3).

### Ruling 4 — *"ядро має працювати з дельтами змін - а не станом... я переконаний"* (the kernel must work with deltas of change, not state — I am convinced)

**Different in kind from rulings 1–3, and the difference is worth naming.** Rulings 1 and 3
*flipped verdicts* (DEFER→ADOPT-NOW); ruling 2 *commissioned a new mechanism* (C′). Ruling 4 is
a third outcome class: **an operator conviction that live code re-read proves is ALREADY the
built architecture** — a validation, not a change request. Fable-E's three-way split (§1–2
there): **K1** the kernel's own state-transition substrate is already delta-native
(`event_log.rs` — content-addressed `MeshEvent` intents, `commit_after_decide`
decide-before-persist, hash-chained log, idempotent replay, `verify_chain`); **K2** cross-node
sync already ships deltas, not snapshots (`anti_entropy.rs::diff`, masterwork #59
re-verified live); **K3** the adapter topology ("read-only view in, diff out, atomic
integration") IS CSC-LAW Layer 1 + the `BridgeResult` ingest discipline already adopted this
round — and the built WASM form is *stronger* than the dialogue asked for (the adapter cannot
even READ kernel memory by default, let alone write). Verdict: **ADOPT-EQUIVALENT**, with a
small genuinely-new residue (`DeltaPatch`/`PatchOp` + ABSOLUTE-OP LAW, deferred
`KernelStateView`, one conditional fault arm, the DELTA-DETERMINISM LAW — §2.3) and one
**zero-cost honesty correction**: the write-side air-gap is genuinely free (already banked via
WASM); the read-side is NOT free (copy-in / host-read-imports / hub-only RO mapping — real,
unbenchmarked cost); and content validation at patch-apply is NEVER removable regardless of
topology — Fable-E caught the dialogue conflating "free spatial bounds-check" with "mandatory
content check" (Fable-E §3–4).

### Two named outcome classes, one reusable heuristic

Round 2 therefore produced **two distinct outcome classes for operator rulings**, and future
passes should name which one they are in rather than treating every ruling as a build order:

- **Verdict-flip rulings** (1, 3): the operator overrules a DEFER; the pass reconciles the
  ruling with unchanged physics and builds where the ruling actually lands value.
- **Validated-by-what's-built rulings** (4, and the load-bearing half of 2): the operator's
  conviction describes shipped or already-adopted architecture; the pass's job is *verification
  against live code + naming the small residue*, and the honest deliverable is "no rebuild"
  plus the corrections the conviction's framing needs (here: the zero-cost pricing).

Conflating the two classes is how a correct conviction turns into redundant work (Fable-E §1).

### The reusable decision heuristic for the verdict-flip class

Both build-ahead inversions share one justification structure, which is hereby named the
**Build-Ahead Inversion Test** for future DEFER-vs-ADOPT-NOW calls. Flip a DEFER to ADOPT-NOW
iff **all three** hold:

1. **The substrate already exists in-tree** (FEC: quinn 0.11 with datagram API already a direct
   dependency; CWR: `kalman.rs` full KF built + tested, `geo.rs::ema_next` in production —
   Fable-A §0.1, Fable-C §5.3).
2. **The absence of the boundary IS the future break** — the failure mode being prevented is
   precisely what happens if the capability is later wired in a hurry without it (FEC: a future
   datagram carrier shipped without a hardened decoder; CWR: a rushed naive extrapolator with no
   admission typing, no tier gate, no staleness bound — Fable-C §5.3).
3. **A real, non-hypothetical consumer exists on day one** (FEC: courier position/dispatch on
   the cellular profile, L1 is real today; CWR: the existing `ema_next` smoothing path,
   proven-equivalent by the in-tree test — "born wired, not shelved").

A DEFER failing any prong stays deferred (that is why RaptorQ, the CRC accelerator, and the
dead-reckoning consumers did NOT flip — see §5.2). REJECTs are never in scope for this test.

---

## 2. Unified Verdict Ledger

All round-1 concepts (14 primary + 12 supplementary, R1-synthesis §1.1–1.2) plus all round-2
material, one line each. **Δ** column marks what round 2 did: `—` untouched · `CONFIRMED`
re-verified/strengthened · `REFINED` verdict kept, precision added · `SUPERSEDED` verdict
changed (delta shown explicitly).

### 2.1 Round-1 primary ledger, carried forward

| # | Concept | Round-1 verdict | Δ round-2 | Source |
|---|---|---|---|---|
| 1 | Side-car adapters (class) | ADOPT-WITH-CONSTRAINT (untrusted-translator form only) | REFINED: class now splits three ways — A′ REJECT / B′ ADOPT / **C′ ADOPT for fail-operational lanes only** (Fable-B §1) | R1 §4; Fable-B |
| 2 | Trusted-proxy Pattern A′ | REJECT-SAME-CLASS (watchdog/proxy) | CONFIRMED — C′ is explicitly NOT a reversal; C′ without the red-line exclusion "is A′ with extra steps" (Fable-B §0, §4) | R1 §4.2; Fable-B §4 |
| 3 | Untrusted-translator Pattern B′ / UT-LAW | ADOPT (type-law) | CONFIRMED + sharpened: B′ never claimed translation faithfulness either — its exact semantic ceiling is now stated and pinned (Fable-B §5.3) | R1 §4.2–4.5; Fable-B §5 |
| 4 | Layout-versioning / Header-as-Type; cleartext-Magic carrier | EXTEND-EXISTING; cleartext variant REJECT-ON-PHYSICS | CONFIRMED — `LaneFrameHeader` is lane-boundary only, NEVER a network preamble; magic admissible only because of that placement (Fable-D §4.4, §5) | R1 §1; Fable-D |
| 5 | CRC32/64 HW integrity | DEFER-WITH-TRIGGER (never a second authority) | CONFIRMED — Part 3's Checksum field does NOT appear; need subsumed by FNV content-address recompute (lane) + FEC/AEAD/signature stack (wire); HW-CRC trigger untouched (Fable-D §4.3) | R1 §2; Fable-D §4.3 |
| 6 | Zero-Copy Shims | ADOPT-WITH-CONSTRAINT | — | R1 §3.1 |
| 7 | Layout Aliasing (tagged enum yes, C-union no) | ADOPT-WITH-CONSTRAINT / REJECT-ON-PHYSICS | — | R1 §3.2 |
| 8 | Fail-Operational for critical/money tier | REJECT-ON-PHYSICS | CONFIRMED by operator ruling verbatim ("not for red-line items like money") + refined into the precise §3.4 behavior (Fable-C §3; row below) | R2; Fable-C §3 |
| 9 | Fail-Operational telemetry tier | ADOPT-WITH-CONSTRAINT (tier boundary now) | CONFIRMED; CWR is its concrete estimation mechanism (Fable-C §2) | R2 §4; Fable-C |
| 10 | Temporal Interpolation (the interpolator) | DEFER-WITH-TRIGGER | **SUPERSEDED (split): boundary/infrastructure half → ADOPT-NOW** (operator build-ahead ruling, Fable-C §5.3); consumer half stays demand-driven; build shape pinned — reuse `kalman.rs` predict/update, no bespoke extrapolator (Fable-C §2.6) | R2 §5; Fable-C §2.6, §5.3 |
| 11 | Gradient State | ALREADY-EQUIVALENT | — | R2 §2 |
| 12 | Heuristic Arbitration (`TrustWeight × IntegrityScore`) | REJECT-SAME-CLASS-AS-#33 | CONFIRMED-STRONGER: rejection is robust under **all three** readings of the undefined `TrustWeight` (accrued / static / vacuous) — forger wins outright in every branch; and the most charitable (b)-reading needs no IntegrityScore at all (dissolution, Fable-C §1.3, §2.5) | R3; Fable-C §1 |
| 13 | Reed-Solomon FEC | DEFER-WITH-TRIGGER | **SUPERSEDED: ADOPT-NOW, operator ruling** — on the L1 datagram / L2 BPv7-shard / L3 future-carrier lanes ONLY; reliable-stream lanes and the authenticity role remain non-placements (Fable-A §0) | R4; Fable-A |
| 14 | RaptorQ / fountain codes | DEFER-WITH-TRIGGER (subordinate, licensing DECART first) | CONFIRMED — "Nothing in this adoption pulls `raptorq` in" (Fable-A §1) | R4 §3b–3c; Fable-A §1 |

### 2.2 Round-1 supplementary ledger, carried forward

| Concept | Round-1 verdict | Δ round-2 | Source |
|---|---|---|---|
| EpochID in header | ADOPT (#21) | CONFIRMED unchanged — Part 3's `uint64_t EpochID` matches exactly; timeliness use = the epoch-tick disable path (Fable-D §4.1) | R1 §1.1; Fable-D §4.1 |
| Layout as Law / Assertion by Panic | ALREADY-EQUIVALENT | — | R1 §1.1 |
| Immutable Pinned snapshot + pointer swap | PARTIAL-EXISTING (#151 durable half = P12 gap) | REFINED: Part 5's Snapshot-Checkpointing = **EXTENSION of #151/#152/#153** — the one new detail (snapshot delivered INTO adapter memory) is realized by existing pieces (`KernelStateView` refresh or adapter reinstantiation); degenerates to nothing under the stateless-absolute default; #44/#107/#110/#154 defers untouched, triggers unfired (Fable-E §2.1, §10.4). Durable half still open, §5.2 | R1 §1.1; Fable-E §10.4 |
| Hash(Snapshot) heartbeat → rollback | ADOPT-structural-form only | CONFIRMED-STRONGER: **retained as the independent falsifier** under the DELTA-DETERMINISM LAW — delta-log equality proves state equality only under canonical encoding + bit-deterministic fold, and a concrete counterexample exists (`householder.rs` FMA-vs-scalar runtime dispatch: identical logs could fold to divergent states, which delta comparison would HIDE and state-hash comparison catches). Must not be retired as redundant (Fable-E §7) | R1 §1.1; Fable-E §7 |
| Exit criteria as hard value-bounds | ADOPT | CONFIRMED — the `trace(P)` staleness bound is a new instance of the same class ("value-bound on uncertainty", Fable-C §2.5) | R1 §5.2; Fable-C §2.5 |
| Data Containment / MPU / Fault Domain Partitioning | ADOPT as bulkheading (#11) | REFINED: formalized as C′'s three containment layers (Fable-B §2); Part 3's MMU scheme ruled **ADOPT-EQUIVALENT** — phones = WASM SFI (built), hub = microVM tier (probe built, VMM follow-up); raw page-table work item DISSOLVED on physics (Fable-D §1–2) | R1 §4.5; Fable-B §2; Fable-D §1–2 |
| Header-Based Priority (2-bit tier) | ALREADY-EQUIVALENT-WITH-CORRECTION (#15, never self-assigned) | REFINED twice: unassigned `11` codepoint → typed decode reject (Fable-C §4.2); storage widened to u8 with `3..=255` → decode reject, 0-255 *continuous* priority REJECTED, decoded tier must equal the grant — grant is the authority (Fable-D §4.2) | R2 §4.1; Fable-C §4.2; Fable-D §4.2 |
| Hash-chaining / per-stream self-check | ALREADY-EQUIVALENT (boolean gate, never score) | — | R3 §0, §4.1 |
| `DATA_DEGRADED`/`ADAPTER_WARNING` flags | ALREADY-EQUIVALENT (partly) | REFINED: **self-incrimination rule** for any future sender-settable flag bit — admissible only if setting it can worsen, never improve, the sender's own lane treatment; `IsDegraded` passes (still deferred, no consumer), `NeedsInterpolation` fails, `Confidence` fails maximally; no flag bits assigned (Fable-D §3.4) | R2 §2; Fable-D §3.4 |
| Zero-copy networking (general) | ALREADY-EQUIVALENT | — | R1 §3.1 |
| Kernel as version-agnostic dispatcher | ADOPT-posture with correction | CONFIRMED — Part 3's "kernel checks only WHERE and WHEN" restates CSC-LAW's premise (Fable-D §2 row 5) | R1 §4.2; Fable-D §2 |
| Lock-free · FPGA/ASIC · in-band telemetry headers · State-Keepers/Compute-Units · immutable-state (general) | NOT-ADJUDICATED | PARTIALLY CLOSED: `Health<Threshold` self-termination adjudicated — self-check against a fixed bound on own observables = admissible value-bound class; the same quantity exported as a trust input or compared across units = #33-class, never (Fable-C §4.3). Rest remains honestly unadjudicated (§5.2) | R1-synthesis §1.2 row 67; Fable-C §4.3 |

### 2.3 New round-2 concepts

| Concept | Verdict | One-line reason | Source |
|---|---|---|---|
| FEC on QUIC unreliable-datagram lane (L1) + BPv7 sharding (L2) + future carriers (L3) | **ADOPT-NOW** | Loss is app-visible on exactly these lanes; ~84× frame-loss reduction at k=4/m=2/5%; FEC-group failure degrades to ARQ-of-record, never data loss | Fable-A §0, §4 |
| `reed-solomon-simd` v3.1.0 dependency | **ADOPT** (DECART done) | Pure Rust, MIT AND BSD-3-Clause (ADR-020 clean), active, O(n log n), SIMD + scalar fallback; alternatives re-checked and declined | Fable-A §1 |
| FEC on reliable streams / FEC as authenticity control | **REJECT (non-placement)** | Physically inert on ARQ streams; attacker bytes are FEC-perfect by construction — `FEC-IS-NOT-AUTH` doc-guard + T3 pin it executably | Fable-A §0.2, §5 |
| Pattern C′ / CSC-LAW (contained self-certifying bridge) | **ADOPT-WITH-HONEST-BOUNDARY** | Self-certification admissible exactly where containment makes the certifier's strongest lie land in a lane that is red-line-free by construction and consumed only fail-operationally; zero authority delta; fail-operational lanes only, never red-line | Fable-B §1–2, §6 |
| Round-trip witness (pinned legacy encoder, memcmp) | **DEFER-WITH-TRIGGER** | The one construction that genuinely closes the semantic gap for *bijective* translations; trigger: a bijective bridge lane feeds any consumer above pure telemetry | Fable-B §3 |
| N-version translation (two independent translators, byte-compare) | **DEFER** (same trigger; lossy-case fallback) | Boolean and admissible but 2× cost + a second codebase | Fable-B §3 |
| CWR — Confidence-Weighted Reconciliation (boundary + local-confidence state) | **ADOPT-NOW** | Kalman fusion of ONE admitted stream with the kernel's own prediction; already in-tree (`geo.rs::ema_next` = scalar steady-state KF); five type-level clauses (single-stream key, post-admission-only, identity-blind variance weights, fusion-never-selection, telemetry-tier only); operator build-ahead ruling flips the boundary half | Fable-C §2.2, §5.3 |
| `ConfidenceLevel` as a wire/header field (any spelling, incl. "SampleQuality"/HDOP-as-R) | **REJECT-AS-CARRIER, ADOPT-AS-LOCAL-STATE** | Self-reported metric priced at zero for a forger; #15 self-assigned-quality threat; `key_K` self-certification; the legitimate idea already lives receiver-side as `trace(P)` + `last_surprise`, never transmitted; not even "advisory" — every wire field finds a consumer; rename escape closed | Fable-C §5.2; Fable-D §3 |
| Critical-tier failure behavior | **ADOPT (the precise resolution)** | refuse-and-escalate + typed observable flag + last-VALID served read-only + recompute-from-truth where a derivation path exists (`compute_order_total`) — never proceed-without, never interpolate; "ignored-as-silently-proceed" rejected as fail-open; interpolability keys on data-KIND (signal vs fact) ∧ tier, both type-level | Fable-C §3.2–3.4 |
| Eviction predicate for alive-but-garbage adapters | **ADOPT** (deterministic circuit-breaker) | Integer counter of *consecutive* boolean gate refusals, fixed bound N → intake disabled; TokenBucket-class, not #33-class; health scores and innovation-magnitude triggers explicitly rejected for this slot | Fable-C §4.1 |
| MMU "Master Table" / `BebopHeader` (source-dialogue Part 3) | **ADOPT-EQUIVALENT** | The pseudocode re-derives what the WASM deny-by-default gate already does (wasmtime uses real MMU guard pages internally on JIT hosts); hub form = existing microVM tier, no new bare-process tier; raw intra-process PTE manipulation impossible on every target (process, not thread, owns the address space) | Fable-D §1–2 |
| Per-adapter OS processes on phones / new bare-process hub tier | **NOT ADOPTED** | Android `isolatedProcess` documented as a future single-helper DECART option only; iOS has no process option; hub already has the stronger microVM tier | Fable-D §1.2–1.4 |
| `LaneFrameHeader` (32-byte lane-boundary header) | **ADOPT — the concrete build artifact** | Final reconciled `BebopHeader`: EpochID unchanged, tier = 2-bit semantics in u8 storage, no Confidence, no CRC, reserved-must-be-zero, FNV content-address with recompute-as-sole-authority; enters only through the `BridgeResult` gate | Fable-D §5 |
| Bonus finding: `quinn::Connection` dropped in `QuicTransport` | **FIX-IN-WAVE-2** | Only `_endpoint` + streams retained today; the datagram lane needs the `Connection` — retained as part of the FEC build | Fable-A §2.4, §7 step 6 |
| Bonus finding: iroh stream-lane `recv` missing `ReplayLedger` + `max_frame_bytes` | **FLAGGED (separate finding)** | wss `recv` has both, iroh stream lane has neither; the new datagram lane includes both day one; stream-lane omission routed to whichever pass owns MESH-10 carrier parity | Fable-A §0.3 |
| Delta-kernel / adapter-as-diff-generator topology (Part 4) | **ADOPT-EQUIVALENT** | K1/K2 already built (`event_log.rs` delta-native substrate; `anti_entropy.rs::diff` delta sync); K3 = CSC Layer 1 + `BridgeResult` ingest, already adopted — the conviction describes shipped code; no rebuild | Fable-E §0–§3 |
| The "zero-cost air-gap" claim | **CORRECTED, not adopted** | Write side genuinely free (banked via WASM: no pointer exists at all); read side NOT free — M1 copy-in (default, O(view), unbenchmarked) / M2 host-read imports / M3 RO-mapping hub-only post-VMM; content validation at ingest never removable — spatial bounds-check ≠ content check | Fable-E §4 |
| `DeltaPatch`/`PatchOp` + **ABSOLUTE-OP LAW** | **NEW-small (ADOPT)** | Op-list output against named targets: per-op scope checking + untouched-state guarantee; frame-degenerate floor (one `Put` of one frame = today's design); ops are absolute `Put`/`Remove` ONLY — increments are non-idempotent (P07 money-bug shape one level up) and #53's rejected soft-merge entry ramp; `MAX_PATCH_OPS` hard budget, never an anomaly score | Fable-E §5, §8 |
| `KernelStateView` (read-side contract) | **DEFER-WITH-TRIGGER** | Reading is not ambient in WASM — the view must be constructed (a copy BY DESIGN: epoch-stamped, torn-read-proof); read scope is an authority grant symmetric to write-lane scope; trigger = first adapter whose output is a function of kernel state, not just its input | Fable-E §4, §8 |
| `BridgeFault::LaneScopeReject { op_index }` | **CONDITIONAL-NEW** (exists only if `DeltaPatch` lands) | The single proposed addition to Fable-B's seven arms — per-op lane-scope violation, distinct from `DecodeReject` and instantiation-time `ScopeViolation`; flagged explicitly because Fable-D promised no new variants (see §5.3 item 3) | Fable-E §2.3 N3 |
| **DELTA-DETERMINISM LAW** | **ADOPT (law)** | Delta-equality ⇒ state-equality iff (i) canonical content-addressed encoding AND (ii) bit-deterministic fold (integer/fixed-order per #52; no runtime ISA dispatch on cross-node-compared fold paths); where (ii) unproven, the state-hash heartbeat is the retained falsifier | Fable-E §7 |
| Stateful vs Stateless adapter (Part 5 Q1) | **STATELESS-ABSOLUTE is the law**; sparse-absolute-stateful **DEFER-WITH-TRIGGER** | The kernel's own event model is already stateless-absolute (chain-independent content-ids, pull-side divergence handling); loss profile (1–10%) is toxic to base-tracking; supersedability — FEC's own loss-recovery property — only holds for absolute values; trigger = ≫frame-scale payloads + netem-measured airtime pain in `BENCH_HISTORY.md`. Increments never, under any trigger | Fable-E §10.1 |
| Dynamic Squashing (Part 5 Q2) | **SPLIT** | Telemetry tier: ALREADY-EQUIVALENT — squashing done optimally IS CWR's Kalman `update()` (same mechanism, buffer angle); raw patch layer: DEFER-WITH-TRIGGER (no commit-rate pressure at ~0.2–1 Hz courier telemetry) under the binding **LAW-LANE UNSQUASHABILITY** rule — lanes whose `decide` validates transitions (order FSM, money, capability) must see every delta; gate-before-squash; seam = existing `BoundedDrainer` | Fable-E §10.2 |
| Squash-validate vs Atomic-Rollback (Part 5 Q3) | **FALSE DICHOTOMY — ALREADY-EQUIVALENT** | The squashed patch is always gated (mandatory floor, not the expensive option); on failure: discard whole batch, last-good state, typed fault — degrade-closed restated, not a new decision; the dialogue's own resync-storm worry is bounded by the existing consecutive-failure circuit-breaker | Fable-E §10.3 |
| Snapshot-Checkpointing (Part 5 Q4) | **EXTENSION of #151/#152/#153** | See the refined supplementary row above; heartbeat + #152 checksum + checkpoint = one loop, not three mechanisms | Fable-E §10.4 |
| "Git for real-time memory" analogy | **REPLACED** (not kept) | Structurally wrong three ways (branches persist/diverge; kernel never merges; reset direction inverted). Accurate framing: *an untrusted contributor emailing a single patch against a pinned base, applied-or-rejected whole — no branches, no merges, and the maintainer wipes and re-clones a contributor whose patches stop applying* | Fable-E §10.5 |

**Supersession tally:** exactly two round-1 verdicts changed (row 13 FEC: DEFER→ADOPT-NOW; row
10 interpolation: DEFER→split ADOPT-NOW/demand-driven), both by explicit operator ruling under
the §1 heuristic. Fable-E adds **zero** supersessions — its ADOPT-EQUIVALENT confirms existing
verdicts and code (the §1 "validated-by-what's-built" class). Zero REJECTs were weakened; two
were strengthened (rows 2, 12). Everything else is confirmation or added precision.

---

## 3. The Concrete Build Artifact: `LaneFrameHeader` + Supporting Types

### 3.1 Type inventory (consolidated reference — defined in full in the source blueprints)

| Layer | Types / consts | Defined in |
|---|---|---|
| FEC (wire, datagram lanes) | `FEC_MAGIC "BFEC"`, `FEC_VERSION=1`, `MAX_FEC_DATA_SHARDS=64`, `MAX_FEC_RECOVERY_SHARDS=32`, `MAX_FEC_SHARD_BYTES=1152`, `MAX_FEC_GROUPS_BUFFERED=32`, `MAX_FEC_BUFFER_BYTES=2MiB`, `MAX_FEC_DECODE_ATTEMPTS=4`, `FecRatio`/`RecoveryRule`, `FecGroupId`, `FecShard` (41-byte header: magic/version/trace/group/index/k/m/orig_len/shard_len), `encode_group`, `FecGroupBuffer`, `FecError` (11 arms), `WireError::Fec` | Fable-A §2.3, §3 |
| Lane boundary (adapter→kernel) | `LANE_MAGIC 0xBEB0_0BEE`, `LANE_SCHEMA_V1=1`, `LANE_HEADER_BYTES=32`, `TIER_CRITICAL=0`/`TIER_TELEMETRY=1`/`TIER_OPTIONAL=2` (`3..=255` reject), `LaneFrameHeader { epoch_id: u64, payload_len: u32, tier: u8, content_address: u64 }` — magic/schema/flags/reserved validated at decode, not retained; flags and reserved MUST be zero | Fable-D §5 |
| Bridge gate (CSC/UT-LAW substrate) | `BridgeResult<T>` (sealed: `Translated(Provenanced<T>)` \| `Failed(BridgeFault)`), `BridgeFault` (7 loud arms: Trapped, FuelExhausted, ScopeViolation, DecodeReject, NonCanonical, BoundsReject, SelfTerminated), `Provenanced<T>` + `BridgeOrigin` (unstrippable), `BRIDGE_GRANTABLE_RESOURCES` (const array excluding Ledger/Auth/Secret/Migration), sealed trait `RedLineAdmissible` (never implemented for `Provenanced<T>`), `SandboxTier` (existing) | Fable-B §2.2–2.3, §6 |
| CWR (post-admission estimation) | `AdmittedFrame<S>` (sole constructor = the stream's inline boolean gate), `Predicted<S>`, `Fused<S>` (phantom stream key `S` — cross-stream fusion is a compile error), `trace(P)` staleness bound, `last_surprise` (existing `kalman.rs`), the deterministic circuit-breaker counter | Fable-C §2.2, §2.5, §4.1 |
| Delta/patch (adapter output vocabulary) | `DeltaPatch { base_epoch, ops }` (canonical op order, decode-enforced), `PatchOp::{Put, Remove}` (ABSOLUTE-OP LAW: no increment arm exists or may be added), `MAX_PATCH_OPS=256` (hard boolean budget, TokenBucket shape), conditional `BridgeFault::LaneScopeReject { op_index }`; deferred: `KernelStateView { epoch_id, lane, bytes }` + `MAX_STATE_VIEW_BYTES` (M1 copy-in, bench at trigger); a `DeltaPatch` is a lane payload UNDER `LaneFrameHeader` — no new admission surface | Fable-E §8 |

`LaneFrameHeader`'s decode-failure vocabulary is REUSED from Fable-B (`BridgeFault::DecodeReject`
etc.) — zero new admission surfaces, zero new result types (Fable-D §5); Fable-E's
`LaneScopeReject` is the one *conditional* eighth `BridgeFault` arm, existing only if the
`DeltaPatch` vocabulary lands (Fable-E §2.3 N3).

### 3.2 The pipeline — layering order is load-bearing and test-pinned

```
                        NETWORK PATH (wire bytes)
  quinn read_datagram()            (AEAD-protected; off-path injection impossible)
    │
    ▼
  [A1] FecShard::parse             hostile-input surface #1 — bounded, no-panic
    ▼
  [A2] FecGroupBuffer::ingest      hostile-input surface #2 — bounded memory, eviction
    ▼
  [A3] reconstruct (any k of k+m)  exact bytes or typed FecError — NEVER partial
    ▼                              ── FEC-DECODE IS BELOW CRYPTO-VERIFY (Fable-A §2.2; T3) ──
  [A4] wire_codec::decode_frame    fail-closed canonical codec (magic, version, bounds)
    ▼
  [A5] replay.observe(nonce)       cross-connection ledger (mirrors wss recv)
    ▼
  [A6] max_frame_bytes check
    ▼
  [A7] gate.check (HybridGate: Ed25519 + ML-DSA-65)   ← SOLE AUTHENTICITY GATE
    │
    ▼  Ok(SignedFrame)
  ┌─────────────────────────────────────────────────────────────────────┐
  │ [B] ONLY IF the frame needs a legacy-layout adapter:                │
  │     sandbox invocation (WASM tier / microVM tier — SandboxTier)     │
  │       ▼  raw output bytes, read only after a clean sandbox return   │
  │ [B1] LaneFrameHeader decode    magic/schema/tier/flags==0/          │
  │                                reserved==0/payload_len cap/         │
  │                                content-address RECOMPUTED           │
  │       ▼                                                             │
  │ [B2] BridgeResult ingest gate  strict TLV → canonicalize →          │
  │                                value-bounds → per-op lane-scope     │
  │                                check when the payload is a          │
  │                                DeltaPatch (LaneScopeReject{op_index},│
  │                                all-or-nothing — zero ops applied on │
  │                                any reject) → Translated(Provenanced<T>)│
  │                                any deviation → Failed(BridgeFault)  │
  │     ── GATE OUTCOMES ARE THE ONLY TWO; Translated ctor is sealed ── │
  └─────────────────────────────────────────────────────────────────────┘
    │
    ▼  tier dispatch (tier byte must equal the GRANTED CapabilityClass — grant is authority)
  ┌───────────────────┬──────────────────────────────┬──────────────────┐
  │ TIER_CRITICAL     │ TIER_TELEMETRY               │ TIER_OPTIONAL    │
  │ B′ mandatory:     │ [C] CWR (gate-BEFORE-fusion, │ drop; observable │
  │ inline gate +     │  Fable-C §2.2 clause 2):     │ flag only        │
  │ refuse-escalate/  │  AdmittedFrame<S> ⊕          │                  │
  │ last-valid-RO/    │  Predicted<S> → Fused<S>,    │                  │
  │ recompute-from-   │  trace(P) staleness bound    │                  │
  │ truth; Provenanced│  → refuse past bound         │                  │
  │ un-typeable at    │  (squashing at this tier IS  │                  │
  │ commit (sealed    │   the Kalman update — no     │                  │
  │ RedLineAdmissible)│   second mechanism)          │                  │
  └───────────────────┴──────────────────────────────┴──────────────────┘
    │
    ▼
  [E] commit_after_decide           ── BUILT (event_log.rs:366-449, Fable-E §5):
    │                                  dedup by content-id (replay = structural
    │                                  no-op, P07 discipline) → decide Law →
    │                                  drift gate where wired → durability
    │                                  barrier → tip advance; rejection persists
    │                                  NOTHING (typed CommitError)
    ▼
  fold                              ── derived state; DELTA-DETERMINISM LAW
                                       (Fable-E §7): cross-node-compared lanes
                                       fold bit-deterministically, else the
                                       state-hash heartbeat is the falsifier
```

The delta-patch application step slots into the existing pipeline **cleanly, with no new
stage**: a `DeltaPatch` is a lane payload under `LaneFrameHeader`, its per-op scope check runs
*inside* the existing [B2] ingest gate, and its atomic integration IS the already-built
`commit_after_decide` — Fable-E's own §5 pipeline is byte-for-byte this diagram's
[B]→[E] segment. The only visible additions are the per-op check inside [B2] and the [E]/fold
stage now drawn explicitly (it always existed — the round-1 diagram's "commit" — but Fable-E's
K1 finding makes it worth showing that this stage is BUILT, not planned).

Ordering laws, each pinned by a named test (§4): **FEC below crypto-verify** (Fable-A T3: a
tampered-then-validly-FEC-encoded frame reconstructs fine and dies at `gate.check`, error must
be `CapabilityVerify`, not `Fec*`); **gate before CWR** (Fable-C clause 2: `AdmittedFrame<S>`'s
only constructor is the gate — a frame that fails admission is unrepresentable at `update()`);
**LaneFrameHeader before BridgeResult** with recompute-as-sole-authority (Fable-D T7); **grant
over tier byte** (Fable-D T5); **gate-before-squash** — if raw-layer squashing is ever
triggered, each delta passes the structural gate before entering the squash buffer, and
Law-lanes (order FSM / money / capability) are unsquashable entirely (Fable-E §10.2, E-T10);
**all-or-nothing patch application** (Fable-E E-T2). Placement laws: `LaneFrameHeader` is NEVER
a network preamble (Fable-D §5 placement law — cleartext-first-bytes carrier remains
REJECT-ON-PHYSICS, R1-synthesis row 4); the direct path (native-layout frames from healthy
peers) skips block [B] entirely; nothing signed changes on the FEC lane — signatures commit to
the TLV signing domain, untouched (Fable-A §2.2).

---

## 4. Consolidated DoD / Test Inventory

One inventory for the future build pass. **Overlaps flagged and consolidated — do not build
twice.** All names land in `REGRESSION-LEDGER.md` (contract §2.17).

### 4.1 FEC suite (Fable-A §6)

| Test | Proves |
|---|---|
| `fec_roundtrip_survives_any_m_erasures` (T1, property) | any ≤m erasures, any order → byte-identical reconstruction; determinism pinned |
| `fec_over_capacity_fails_typed_never_partial` (T2) | >m losses → typed failure; never partial bytes; wire_codec is the independent second fail-closed layer |
| **`fec_valid_forgery_still_rejected_by_gate` (T3 — THE adversarial ordering proof)** | tampered bytes, freshly-computed valid parity → reconstruction succeeds, `gate.check` rejects with `CapabilityVerify` (not `Fec*`); RED form: harness without `gate.check` accepts — ordering is the control |
| `fec_reconstruction_is_transparent_to_auth` (T3b, converse) | honest frame, m losses → reconstruct → gate PASSES |
| Malformed-shard suite (T4, one test per row) | exact typed error per malformation; `FrameTooLarge` with zero bytes buffered; `GroupInconsistent` drop; duplicate-index idempotent |
| `fec_buffer_bounded_under_group_spray` (T5) | 10k-group spray: memory ≤ caps, eviction works, buffer stays functional |
| `fuzz_fec_ingest` (T6) | no panic / no OOM / output ≤ MAX_ENVELOPE_BYTES under arbitrary input |
| `quic_datagram_lane_delivers_with_m_losses` + `quic_datagram_lane_replays_rejected` (T7) | integration: lane delivers under loss AND carries the replay ledger |
| `fec_ratio_table_matches_binomial` (T8) | the §4.1 sizing math is executable (VERIFIED-BY-MATH) |
| Wave 3: `sharded_bundle_delivers_with_any_k_of_km_couriers`, `sharded_bundle_expiry_drops_group`, T3-shape forgery over bundles | BPv7 sharding semantics + same auth-ordering law on the bundle lane |
| Static gates | no `unwrap`/`expect`/`panic!`/indexing in `fec.rs`; `FEC-IS-NOT-AUTH` doc-guard grep |

### 4.2 Containment suite (Fable-B §7 + Fable-D §7 — **merged: one adversary harness**)

Fable-D §7 states its tests "all reuse the Fable-B adversary harness (malicious WASM adapter)."
Build ONE harness; the overlapping pairs below are one test each, not two:

| Consolidated test | Merges | Proves |
|---|---|---|
| `evil_adapter_ungranted_import_refused` (B-T1) | — | Layer-1 escape dies at instantiation (extends existing `notify_scope_allows_only_notify_import`) |
| `bridge_fault_is_loud_never_partial` (B-T2) **+** `poisoned_adapter_trap_isolate_serve_telemetry` (D-T1) | **OVERLAP — same injection harness.** B-T2 is the exhaustive no-third-state property (outcome ∈ {Translated, Failed} for every injection); D-T1 is the end-to-end drill continuing into circuit-breaker disable + CWR `predict()` degradation + staleness-bound refusal. One harness, two assertion phases — the fuzz property and the drill are phases, not separate builds. This also subsumes Fable-C's staleness-bound refusal test at integration level (keep the unit-level bound test inside the CWR suite). |
| `red_line_unreachable_from_bridge_provenance` (B-T3) **+** `poisoned_adapter_critical_lane_never_interpolates` (D-T2) **+** R1-synthesis §3.1 trybuild `interpolate_stale()` compile-fail | **OVERLAP — one trybuild suite, three distinct compile-fail cases:** (i) `Provenanced<T>` into the money/command commit path does not compile (sealed `RedLineAdmissible`); (ii) no interpolation/CWR API exists on the `Critical` type; (iii) cross-stream fusion (`AdmittedFrame<S1>` with `Predicted<S2>`) does not compile (Fable-C clause 1). Plus D-T2's runtime half: last-VALID readable + typed refusal on mutating consume under poisoning. |
| **`residual_semantic_gap_pinned` (B-T4 — the honest one, never skip)** | — | the well-formed WRONG tile IS accepted into its own lane; the test asserts the gap exists; closing it later flips the test and forces doc revision |
| `round_trip_witness_closes_bijective_case` (B-T5) | — | activates with the §5.2 witness trigger; RED: witness off → wrong tile in-lane; GREEN: memcmp mismatch → `Failed` |
| `csc_never_granted_to_inprocess_tier` (B-T6) | — | only `SandboxTier::WasmComponent` (and post-VMM KVM) qualify for C′ |
| `missed_epoch_deadline_disables_lane` (D-T3) | — | D1 detection path: silence past the epoch tick → lane disabled, observable; re-admission only via cool-down/operator |

### 4.3 Header/lane suite (Fable-D §7)

| Test | Proves |
|---|---|
| **`reserved_and_flags_must_be_zero` (D-T4)** | the executable pin of the Confidence removal — any nonzero flag/reserved byte → `DecodeReject`; re-adding any byte flips it |
| `tier_discriminants_and_grant_authority` (D-T5) | tier `3..=255` → reject; tier ≠ granted `CapabilityClass` → gate reject. **OVERLAP: this IS the round-1 §3.1 "self-assigned tier chaos test" and Fable-C's #15 chaos test — one test, three prior descriptions. Build once.** |
| `lane_header_layout_pinned` (D-T6) | const-assert size==32 + offsets; golden-bytes round-trip; no-transmute grep |
| `content_address_recompute_is_sole_authority` (D-T7) | flip payload byte OR header address → same reject; wire value can never override the recompute |
| `unknown_lane_schema_rejected` (D-T8) | unknown schema version → typed reject. **PATTERN-SIBLING (not the same test): the `unknown_version_is_rejected_on_decode` law appears at three decode surfaces — envelope/framing (exists), FEC shard version (A-T4 row), lane schema (D-T8). Three surfaces, three tests, one shared law.** |

### 4.4 CWR suite (Fable-C §2.2/§5.3, DoD carried from R1-synthesis §3.1)

Trybuild cases folded into §4.2's consolidated suite. Remaining unit-level: staleness-bound
refusal at exactly `trace(P)` = bound (RED-first); `ema_next`-equivalence of the re-expressed
first consumer (extends the existing `scalar_kf_equival_ema_next` pin); circuit-breaker fires at
exactly N consecutive boolean refusals and never on innovation magnitude (Fable-C §4.1).

### 4.5 Delta/patch suite (Fable-E §9, §10.6 — overlaps flagged)

| Test | Proves |
|---|---|
| `delta_untouched_state_is_byte_identical` (E-T1) | the footprint guarantee — keys not named by a patch are byte-identical pre/post; a frame-replacement implementation cannot pass this shape (the executable diff-vs-frame difference) |
| `patch_op_out_of_lane_scope_rejected` (E-T2) | one out-of-lane op → `Failed(LaneScopeReject{op_index})`, ZERO ops applied (all-or-nothing) |
| `patch_replay_is_structural_noop` (E-T3) | **EXTENDS an existing built test, not new machinery**: same `DeltaPatch` committed twice → `Duplicate`, decide not re-run (extends `commit_after_decide_replay_on_nonempty_log_is_true_duplicate`, `event_log.rs:679-751`) |
| `wrong_content_patch_accepted_in_lane` (E-T4) | **SAME PIN AS B-T4, do not double-count.** This is `residual_semantic_gap_pinned` carried to the patch surface — one law-pin, two payload forms (frame, patch), maintained as one test family; if a later mechanism flips it, Fable-B §4 AND Fable-E §6 must be revised together |
| **`delta_equality_does_not_imply_state_equality_under_float_fold` (E-T5 — never skip)** | the DELTA-DETERMINISM LAW's RED form: identical delta logs + two f64 summation orders → divergent state hashes (no hardware dependency needed); GREEN: integer/fixed-order fold → identical. Pins WHY the state-hash heartbeat survives |
| `state_view_is_epoch_stamped_copy` (E-T6) | conditional on the `KernelStateView` trigger: view mutation after handoff never affects kernel state; torn-read-proof |
| `patch_op_budget_is_boolean_not_score` (E-T7) | ops > `MAX_PATCH_OPS` → typed reject; grep-guard: no "typical patch size" / anomaly metric anywhere (NO-SCORING) |
| `absolute_op_law_pinned` (E-T8) | match-exhaustiveness pin: `PatchOp` has no increment arm; adding one breaks the test and forces revisiting #53 |
| `stale_base_patch_rejected` (E-T9) | `base_epoch` outside the lane's declared window → typed reject, observable |
| `law_lane_unsquashable` (E-T10) | **FOLDS INTO the §4.2 consolidated trybuild suite** as its fourth compile-fail case: compose not implemented for Law-lane delta types; plus the runtime property half (telemetry compose is associative and equals sequential apply) |
| `squash_failure_discards_whole_batch_and_bounds_resync` (E-T11) | **REUSES Fable-C §4.1's circuit-breaker, not a new bound**: commit-time failure → last-good state + typed fault; N consecutive → intake disabled (the resync-storm bound IS the existing breaker) |
| `checkpoint_resets_stateful_base` (E-T12) | conditional on the sparse-stateful trigger ever firing: desynced base → heartbeat mismatch → golden-snapshot view push or reinstantiation → next patch correct |

Consolidation summary for this suite: E-T4 = B-T4 (one pin, two surfaces — count once);
E-T10's compile-fail half joins the single trybuild suite of §4.2; E-T3 and E-T11 extend/reuse
built or already-planned machinery rather than adding parallel mechanisms; E-T6 and E-T12 are
trigger-conditional and are not owed until their DEFERs fire.

### 4.6 Round-1 action items' tests (unchanged by round 2, still owed)

NaN-bearing-tile RED→GREEN at `spectral_radius` (R1-synthesis §2.1); `trust_weight: f32`
fixture RED→GREEN against `ci-no-courier-scoring.sh` (§2.2); lying-adapter UT-LAW chaos test
(§3.2 — note: Fable-A T3 is its wire-lane sibling, same UT-LAW shape, different surface — keep
both); shim cross-node hash-convergence property + designed-to-fail transcendental variant
(§3.3).

---

## 5. What Remains Honestly Open (not papered over)

### 5.1 The RC-2-broad residual gap — NOT closed, and never will be by this design

Stated in Fable-B's own words and kept at full strength: a semantically-plausible-wrong
translation — byte-perfect, canonical, in-bounds, drift-passing encoding of the *wrong content*
— **cannot be eliminated by any type or structural mechanism**. The kernel by design does not
know the legacy layout; information it cannot independently derive it cannot verify — only
bound. Any complete in-kernel checker would be a re-implementation of the translation itself
(Fable-B §3). Containment converts "kernel poisoning" into "bounded in-lane wrongness" — but
in-lane wrongness is still wrongness; consumers of that lane see wrong telemetry until the
witness, staleness, or cross-checks surface it (Fable-B §4). This design **bounds and makes
visible** the gap (Layer 3 reach-limitation + unstrippable provenance + blast-radius-in-time
query) and **pins it executably** (B-T4, which passes by asserting the gap exists). The only
genuine closure mechanisms are the round-trip witness (bijective case) and N-version translation
(lossy fallback), both DEFER-WITH-TRIGGER (Fable-B §3). Doctrine line: *verify where possible;
contain where impossible; refuse where neither holds* (Fable-B §4).

**Fable-E independently re-verified this under the delta topology and CONFIRMED it: not
closed.** Memory topology constrains *where* bytes can be written, never *whether* they are
semantically correct — a canonical, in-scope, in-bounds `DeltaPatch` with wrong values is
integrated exactly as a wrong frame was (Fable-E §6, walking Fable-B's theorem with a diff
substituted). Two honest narrowings, precisely scoped so they are not mistaken for closure:
deltas name their write targets, so unnamed state is provably untouched — the residual gap's
*area* shrinks (key-level blast radius, cheaper finer-grained provenance queries), its
*existence* does not; and canonical content-addressed diffs make the existing closure
mechanisms cheaper to run (N-version compare becomes a 32-byte id memcmp), adding zero
verification power of their own. B-T4/E-T4 (one pin, §4.5) must never be retired on the
strength of "the delta log matched."

### 5.2 Round-1 items round 2 did NOT touch (still open, unchanged)

- **DEFER 4.1 — CRC32/64 HW acceleration.** Trigger unchanged: a measured hot content-address
  recompute, number in `BENCH_HISTORY.md` first (R1-synthesis §4.1; re-confirmed Fable-D §4.3).
- **DEFER 4.3 — RaptorQ.** Trigger + licensing-DECART precondition unchanged (Fable-A §1).
- **DEFER 4.4 (residual) — dead-reckoning/fusion consumers** beyond the day-one `ema_next`
  re-expression: demand-driven, trigger = the courier-position/ETA feature is actually built
  (Fable-C §5.3).
- **Action §2.1 — the NaN `is_finite` fix** at `spectral_radius`: verdicted, RED-first DoD
  written, NOT landed (no commits this round).
- **Action §2.2 — the `ci-no-courier-scoring.sh` `trust_weight` gap fix**: same status.
- **#151 durable-snapshot half** (PARTIAL-EXISTING, P12 gap) — untouched.
- **NOT-ADJUDICATED remainder**: lock-free structures, FPGA/ASIC offloading, in-band telemetry
  headers, immutable-state (general), and the non-Health parts of State-Keepers/Compute-Units
  (only the `Health<Threshold` half was closed, Fable-C §4.3).
- **Carried notes**: `rng.rs` Android/iOS cfg-gap (Layer E); G11 remediation direction / T2
  dual-authority flip (Layer G) (R1-synthesis §2.3).

New open items round 2 itself created: **VMM launch** for the microVM tier (probe only today —
C′ cannot certify the native tier until it lands, Fable-B §2.1); **fuel constant** is an explicit
placeholder pending the B4 bench (Fable-B §2.1); **round-trip witness / N-version** triggers
(§5.1); **`IsDegraded` flag bit** — passes the self-incrimination rule but deferred until a
consumer exists (Fable-D §3.4); **iroh stream-lane replay/size-cap parity** (Fable-A §0.3 →
MESH-10 owner); **stream-envelope JSON→binary migration** — flagged, out of scope (Fable-A
§2.2); **netem calibration** of FEC ratios — defaults ship now, V5 recipes are the calibration
source, measured rows go to `BENCH_HISTORY.md` (Fable-A §4.2); **Android single-helper-process
option** — documented for a future DECART only (Fable-D §1.2).

Fable-E adds four more, all trigger-gated: **the read-side M1 copy-in cost is real and
UNBENCHMARKED** — `KernelStateView` construction is an O(view) memcpy per adapter per epoch
(order-of-magnitude estimate sub-10 µs at frame scale, explicitly flagged as an estimate, not a
measurement); the number is owed to `BENCH_HISTORY.md` when the first state-reading adapter
lands, and the zero-cost correction is *falsified* if that measurement reaches the 5–10% Part 4
feared (Fable-E §4, §9 falsification criteria); **`KernelStateView` itself** —
DEFER-WITH-TRIGGER on that same first state-reading adapter; **sparse-absolute-stateful
adapters** — DEFER on netem-measured airtime pain (never increments, under any trigger,
Fable-E §10.1); **raw-layer Dynamic Squashing** — DEFER on measured commit-rate pressure on a
squashable non-Law lane; seam pre-named (`BoundedDrainer`), law pre-bound (LAW-LANE
UNSQUASHABILITY) (Fable-E §10.2).

### 5.3 Cross-blueprint tensions found on cross-read (three, all stated honestly)

1. **The "Pattern C′ does not exist" statement in Fable-C §4 vs. Fable-B defining it.**
   Fable-C §4 opens with a naming correction — "No Pattern C′ exists in the corpus" — written
   against the corpus as it stood during that pass; Fable-B (landed in the same round) defines
   C′, and Fable-D §2 treats it as real and reconciles against it. **Resolved by chronology,
   not a live doctrinal conflict:** the substance Fable-C routed to "UT-LAW + Data Containment"
   is exactly what Fable-B's C′ formalizes (C′ = B′'s substrate with the impossible semantic
   step honestly absent plus lane confinement — Fable-B §5.2), and both agree B′ stays mandatory
   for red-line. Fable-C's §4 sentence is superseded by Fable-B; noted here so no future reader
   mistakes it for a dispute.
2. **Adaptive FEC ratio (Fable-A §4.2) vs. the channel-physics seam (Fable-C §5.1) — a real but
   mild tension in emphasis, with an agreed operational outcome.** Fable-A holds adaptive
   (loss-responsive) `m` out of scope because a per-peer loss estimator "walks straight at the
   NO-COURIER-SCORING / no-per-source-weight fence" and requires explicit re-adjudication.
   Fable-C's seam would classify a link-loss estimator deciding *transport parameters* as the
   admissible TCP-congestion-control class (channel physics, never claim-authority).
   **Operationally they agree**: adaptive ratio stays OUT until explicitly re-adjudicated
   (Fable-A's own condition). The reconciliation, recorded now so the future adjudication has
   its vocabulary: if adaptivity is ever wanted, Fable-C §5.1's seam is the test it must pass —
   the estimator may key on *channel/link state* feeding transport parameters only, must never
   feed admission/arbitration, and must never become a per-identity behavioral model. Until that
   DECART happens, `RecoveryRule::Fixed(m)` from netem measurements is the only tuning path.
3. **Fable-E's `LaneScopeReject` vs. Fable-D's "adds no variants" promise — a managed,
   self-flagged delta, not a silent conflict.** Fable-D §8 stated its blueprint "adds no
   variants" to Fable-B's seven-arm `BridgeFault`; Fable-E proposes an eighth arm and **flags
   the collision itself** ("flagged explicitly because Fable-D's blueprint promised no new
   variants and this one is new — it exists only if N1 is built," Fable-E §2.3). Resolution
   recorded: the arm is CONDITIONAL on the `DeltaPatch` vocabulary landing; until then the
   seven-arm enum stands exactly as Fable-B/D specified, and if N1 lands, `BridgeFault` becomes
   eight arms with this ledger row as the authorization trail.

No other tensions found: FEC-shard-header-carries-no-checksum vs LaneFrameHeader-carries-FNV is
deliberate layering, cross-cited by both docs (Fable-A §2.3, Fable-D §4.3); Fable-D §4.2
generalizes (does not contradict) Fable-C §4.2's codepoint law; the Confidence removal is stated
identically in Fable-C §5.2 and Fable-D §3; Fable-E's tier-split of Dynamic Squashing lands
exactly on Fable-C's CWR (same mechanism, restated — Fable-E §10.2 cites it as
ALREADY-EQUIVALENT rather than proposing a rival), and its Part-5 stack table routes every
point to a home already adjudicated by Fable-A/C/D or the R1-synthesis (Fable-E §10.5).

---

## 6. Mapping to CORE-ROADMAP Layers (A–I)

Per CORE-ROADMAP-STANDARD §3 (Layer = altitude axis, never renumbering P01–P30) and consistent
with R1-synthesis §6's routing:

| Round-2 artifact | Owning Layer | Note |
|---|---|---|
| `fec.rs` + QUIC datagram lane + BPv7 sharding + `reed-solomon-simd` dep | **Layer E** (network/carrier) | Same owner R1-synthesis §6 gave the FEC defer; now a build. Decoder hardening per V3 parser discipline is part of the deliverable, not a separate item |
| FEC netem calibration, fuzz CI job, `BENCH_HISTORY.md` rows, `REGRESSION-LEDGER.md` entries | **Layer H** (ops/bench/regression) | V5 recipes = calibration source (Fable-A §4.2) |
| `BridgeResult`/`BridgeFault`/`Provenanced`/ingest gate | **Layer B** (state/consistency — the gate substrate) | Fable-B §8's own routing "Layer B (gate substrate) + Layer D (scope law)" |
| `BRIDGE_GRANTABLE_RESOURCES`, sealed `RedLineAdmissible`, CSC-LAW scope law | **Layer D** (capability/trust) | Red-line un-nameability lives with the grant machinery |
| Sandbox tiers: WASM gate (built), microVM VMM follow-up, C′ tier restriction | **Layer C** (safety/isolation/bulkhead) | `isolation/microvm.rs` is Layer-C substrate; the bulkhead class #11 (R1-synthesis §6 UT-LAW row) |
| `LaneFrameHeader` + decode laws (reserved-zero, schema reject, content-address recompute) | **Layer B** | Lane-boundary decode feeding the Layer-B gate; the tier↔grant cross-check leans on Layer D |
| CWR boundary types + `trace(P)` bound + `ema_next` re-expression | **Layer B** substrate under **Layer D**'s tier law | `kalman.rs`/`geo.rs` are Layer-A primitives; the boundary wraps admission (B); tier law is D (R1-synthesis §6 telemetry-boundary row) |
| CWR dead-reckoning consumers (deferred) | **Layer G** (product) | Unchanged from R1-synthesis §6 interpolator row |
| Critical-tier refuse-and-escalate / recompute-from-truth behavior | **Layer G** consumers under **Layer B/D** laws | Reconfirms the G11 direction (server recompute), already gated in Layer G |
| Circuit-breaker eviction predicate (Fable-C §4.1) | **Layer C** | P-C scope explicitly names circuit breakers |
| ConfidenceLevel wire-rejection + self-incrimination rule for flag bits | **Layer E** doctrine (what may exist on the wire) + **Layer B** decode law (reserved-zero pin) | |
| Consolidated test inventory (§4) + the T4 residual pin | **Layer H** | The merged adversary harness is the Layer-H chaos-harness item (idea #143) getting its first concrete population |
| Heuristic-Arbitration robustness proof, Health-half adjudication | **Layer D** doctrine record | Strengthens NO-COURIER-SCORING enforcement rationale; pairs with the §2.2 CI-gate fix (Layer H) |
| `DeltaPatch`/`PatchOp` + ABSOLUTE-OP LAW + `MAX_PATCH_OPS` + conditional `LaneScopeReject` | **Layer B** | Extends the ingest-gate/`event_log` substrate (`commit_after_decide` is Layer B's built machinery); the per-op scope half leans on Layer D's grant law |
| `KernelStateView` (deferred) + read-scope grants | **Layer B** contract under **Layer D** scope law | Read authority is a grant symmetric to write-lane scope (Fable-E §4); M3 zero-copy hub form rides Layer C's VMM follow-up + masterwork #154's trigger |
| DELTA-DETERMINISM LAW + retained state-hash heartbeat | **Layer B** (fold/consistency law) enforced via **Layer A** discipline (masterwork #52 fixed-order/integer contract) | The `householder.rs` FMA-dispatch example is a Layer-A determinism finding; E-T5 and the M1 bench land in **Layer H** |
| LAW-LANE UNSQUASHABILITY + gate-before-squash (deferred squash mechanism) | **Layer B** (`decide`-law boundary; seam = built `BoundedDrainer`) | Telemetry-tier squashing needs no owner — it IS Layer B/D's CWR, already mapped above |
| Snapshot-Checkpointing extension (view-refresh / reinstantiation delivery) | **Layer B** (#151/#152/#153 owners) with the reinstantiation half in **Layer C** (isolation teardown) | Degenerates to nothing under the stateless-absolute default; activates only with §5.2's sparse-stateful trigger |

No orphaned artifact: every §2.3 row above maps to a Layer, and the two supersessions (§2.1 rows
10/13) keep their round-1 owners (Layer E, Layer B/D/G respectively) with the verdict updated.
Fable-E's artifacts concentrate in Layer B by design — its whole finding is that the delta
substrate is Layer B's already-built `event_log` core, with only a thin adapter-boundary
vocabulary added at its edge.

---

## 7. Contract compliance note (CORE-ROADMAP-STANDARD §2)

Extension-work form, same as R1-synthesis §7: **§2.1** every claim cites a round-2 blueprint or
R1-synthesis section whose own pass live-verified the underlying `file:line`. **§2.2/§2.5** the
consolidated §4 inventory is entirely falsifiable RED-first tests, including five designed-to-
prove-a-hole forms (A-T3's RED harness, B-T3's RED impl, B-T4/E-T4's asserted gap, D-T4's fuzz
pin, E-T5's float-divergence RED form). **§2.4** all types named before implementation (§3.1).
**§2.6** every rejection argues reachability (forger-wins trace, fail-open trace,
information-theoretic bound), never prose. **§2.11/§2.13** bulkhead vs supervision and the
three self-* terms used in their ratified senses throughout (containment = bulkhead;
self-termination = invariant boundary; FEC = the self-healing/redundant-math class). **§2.14**
static gates named (grep gates, trybuild, const-asserts, match-exhaustiveness pins). **§2.17**
every test routes to `REGRESSION-LEDGER.md`. **§2.19** reuse-first honored: the merged harness
(§4.2), the reused `BridgeFault` vocabulary in `LaneFrameHeader`, CWR reusing `kalman.rs`,
zero new mechanisms in the Poisoned-Adapter drill, and Fable-E's whole shape (delta topology =
built `event_log` + adopted gate, no rebuild; squash seam = built `BoundedDrainer`; rollback =
degrade-closed restated) are this document's own consolidation deltas. **§2.10** the one
outstanding measurement obligation is named, not waived: the M1 copy-in bench at
`KernelStateView`'s trigger (§5.2). No code written; no commits; output confined to the
round-2 directory as instructed.
