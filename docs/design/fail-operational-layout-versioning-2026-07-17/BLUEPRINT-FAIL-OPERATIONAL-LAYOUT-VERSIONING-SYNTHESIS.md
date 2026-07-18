# BLUEPRINT — Fail-Operational / Layout-Versioning Synthesis: One Verdict Ledger, Two Real Findings, Zero Silent Drops (2026-07-17)

> **Status: consolidation of the four grounding passes (R1–R4) over `00-SOURCE-DIALOGUE.md` into
> one plan. Extension work, not greenfield** — per the operator's "враховуючи наявні плани та
> роадмапи" framing: every verdict below is grounded against the already-adopted corpus
> (CORE-ROADMAP-STANDARD, the 185-item mesh-masterwork V2 ledger, this session's V-series
> verification findings). No code, no commits. Every claim traces to one of the four R-docs in
> this directory; `file:line` cites inside those docs were live-verified by their own passes.
>
> **Sources (read in full for this synthesis):**
> - `R1-layout-versioning-bridges-grounding.md` (R1)
> - `R2-fail-operational-vs-degrade-closed-grounding.md` (R2)
> - `R3-heuristic-arbitration-vs-courier-scoring-grounding.md` (R3)
> - `R4-reed-solomon-fec-fit-grounding.md` (R4)
> - `00-SOURCE-DIALOGUE.md` (the pasted external-collaborator dialogue this cluster vets)
> - `../CORE-ROADMAP-STANDARD-2026-07-17.md` (the 20-point contract, honored in §7)
> - `../bebop2-mesh-tensor-hermetic-2026-07-17/BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS-V2.md`
>   (ledger-format template; register items #4/#9/#14/#15/#18/#33–40/#93/#145/#151/#152/#157/#185
>   cited via the R-docs)
>
> Verdict vocabulary (same as the masterwork V2 ledger, §B): **ADOPT** ·
> **ADOPT-WITH-CONSTRAINT** · **ALREADY-EQUIVALENT** · **EXTEND-EXISTING** ·
> **DEFER-WITH-TRIGGER** · **REJECT-ON-PHYSICS** (physics/correctness/determinism/security) ·
> **REJECT-SAME-CLASS** (structurally identical to an already-rejected register item).

---

## 1. Executive Verdict Ledger

### 1.1 Primary ledger — the fourteen named concepts (zero silent drops)

| # | Dialogue concept | Verdict | One-line reason | Source |
|---|---|---|---|---|
| 1 | Side-car adapters (as a class) | **ADOPT-WITH-CONSTRAINT** | Admissible **only** as the untrusted-translator form (row 3); the class splits on one question — does the kernel trust the side-car's output or re-verify it — and only the re-verify branch survives the no-proxy rule's three-axis test | R1 §4 |
| 2 | Trusted-proxy pattern (kernel commits on the adapter's word) | **REJECT-SAME-CLASS** (watchdog/proxy) | Correctness depends on a separate process being both alive AND honest; adapter death/compromise = silent fail-open; adapter self-validation as the kernel's reason to trust = `key_K` self-certification by a separate party | R1 §4.2 (Pattern A′), §4.4 |
| 3 | Untrusted-translator pattern (side-car output re-verified by the kernel's own inline gate) | **ADOPT** (as a type-law, §3.2) | Supply-path, not control-path: output is judged BY the kernel through the identical gate raw network input passes; side-car grants zero trust it would not compute itself; absence = visibly empty buffer, not a silent gap | R1 §4.2 (Pattern B′), §4.3, §4.5 |
| 4 | Layout-versioning / Header-as-Type (Magic Number + SchemaID + EpochID) | **EXTEND-EXISTING** (mostly ALREADY-EQUIVALENT); cleartext-Magic-Number variant **REJECT-ON-PHYSICS** (security) | The signed envelope already carries a fail-closed version discriminant (`framing.rs:54-60`, unknown-version hard-reject pinned by a red-team test); the cleartext-first-bytes carrier is the exact form register #93/#157 already rejected; only additive sliver = a one-section spec note naming the two-layer self-terminate(parser)/Escalate(policy) split | R1 §1.1–1.3 |
| 5 | CRC32/CRC64 hardware-accelerated integrity | **DEFER-WITH-TRIGGER** (§4.1) | Zero CRC in tree, but the role is already served twice (FNV per-tile content-address + sha3 Merkle root; QUIC per-packet); a standing second checksum authority = the dual-authority hazard (register #23/#47); admissible only as a private *acceleration* of the content-address recompute, never an independent trust signal | R1 §2 |
| 6 | Zero-Copy Shims (inline raw-pointer layout transformers) | **ADOPT-WITH-CONSTRAINT** (§3.3) | Safe extension of the existing zero-copy discipline **iff** output types as `NormalizedTile` (not `&[u8]`) and the transform is rational/fixed-order (no transcendental) — else it reintroduces bridge-gap #1 and breaks cross-node hash convergence | R1 §3.1 |
| 7 | Layout Aliasing (union/struct-overlay + CommonHeader, static dispatch) | **ADOPT-WITH-CONSTRAINT** — the intent via `#[non_exhaustive]` tagged enum; literal C-union **REJECT-ON-PHYSICS** | A mis-tagged union read silently reinterprets bytes — the precise opposite of "invalid state is unrepresentable"; the tagged enum + `match` delivers the same static dispatch with no `unsafe` | R1 §3.2 |
| 8 | Fail-Operational for the critical/money tier ("рятувати будь-якою ціною") | **REJECT-ON-PHYSICS** (already-adjudicated; do not re-open) | Money is a discrete authoritative fact, not a sampled continuous signal — interpolation is category-inapplicable; the money RED-LINE ("discrete integer channel, never interpolated") already forbids it; applied literally to the live G11 forged-`unit_price` vuln it makes the hole strictly WORSE (§5.3) | R2 §1, §3 |
| 9 | Fail-Operational for the non-critical/telemetry tier | **ADOPT-WITH-CONSTRAINT** (tier boundary now — it is free; interpolator deferred, row 10) | Genuine net-new capability beyond #145's last-VALID-readable; safe **only** because the tier flag is capability-scoped (#15 `CapabilityClass`, "never self-assigned"), which the dialogue's raw 2-bit-header scheme lacks | R2 §0, §4, §5 |
| 10 | Temporal Interpolation (the interpolator itself) | **DEFER-WITH-TRIGGER** (§4.4) | No consumer needs it yet (courier dead-reckoning is the plausible first); building it now violates the over-engineering discipline (#138); on the critical tier it is row 8's rejection | R2 §5 |
| 11 | Gradient State (Binary → ідеально/з втратами/на критичному мінімумі) | **ALREADY-EQUIVALENT** | It is register #14 (tiered consistency = the built `event_log` vs `sync_pull` split) + #145 (Survival-Mode soft-refuse + last-valid-readable) restated; the only genuine delta is forward extrapolation = rows 9/10; the dialogue's closing determinism worry is answered by R4: exact/deterministic recovery (FEC) is safe, lossy guessing (interpolation) is the real flag | R2 §2; R4 §5.1 |
| 12 | Heuristic Arbitration (`Selection = TrustWeight × IntegrityScore`) | **REJECT-SAME-CLASS-AS-#33** (do not soften) | "IntegrityScore" IS the "Data Integrity" column and "TrustWeight" IS the "Trust" axis of the exact Peer Trust Matrix register #33 already stamped REJECT-ON-PHYSICS; near-verbatim #34/#37; the concrete attack trace shows a forger's crafted stream **WINS arbitration outright** (§5.1); constructive carve-out: boolean per-stream gate + source-independent tie-break | R3 §0–§4 |
| 13 | Reed-Solomon FEC at the tensor-protocol level | **DEFER-WITH-TRIGGER** (§4.2); **never a security control** | Helps with ZERO of the 11 adversarial HIGH findings (reliability mechanism for honest-noisy channels, not authenticity); both live carriers (WSS/TCP+TLS, iroh/QUIC) already do ARQ + integrity below the app; today it would only ADD a hostile-input parsing surface below the signature | R4 §1, §2, §4 |
| 14 | Fountain codes / RaptorQ | **DEFER-WITH-TRIGGER** (§4.3, subordinate to row 13) | Technically superior only for true one-to-many broadcast with heterogeneous loss; carries a Qualcomm patent history (expiring ~2025-2027) that collides with the AGPLv3 ADR-020 goal — classical patent-clean RS (`reed-solomon-simd`) is the default if the FEC trigger ever fires; RaptorQ needs an explicit licensing DECART first | R4 §3b–3c |

### 1.2 Supplementary ledger — sibling concepts from the same dialogue, verdicted inside the R-passes

Per this session's no-omission discipline, every remaining concept in `00-SOURCE-DIALOGUE.md`
gets a line. These were adjudicated *within* the R-docs (not separately researched):

| Dialogue concept | Verdict | Source |
|---|---|---|
| EpochID in the header | **ADOPT** (already in the register: monotone logical epoch, max-merge, inside the signed envelope — #21/#32/#156/#159) | R1 §1.1 |
| "Layout as Law / Assertion by Panic" (type-enforced physics) | **ALREADY-EQUIVALENT** — it is the `NormalizedTile`/`TileAddress` newtype invariant; an unnormalized tile is unrepresentable | R1 §1.1 |
| Immutable Pinned snapshot + atomic pointer swap | **PARTIAL-EXISTING** — `RetainedBase` admission + arena snapshot-then-drop; the durable half is the P12 gap (#151) | R1 §1.1 |
| Hash(Snapshot_N) heartbeat compare → rollback | **ADOPT-the-structural-form** as a degrade-closed *inline* consequence (content-address compare → re-derive), never a polling corrector (#152) | R1 §1.1 |
| Exit criteria as hard value-bounds on physically possible values | **ADOPT** — this is the concept the NaN-masking fix concretely validates (§2.1); the law already exists in two places in the kernel and is missing at the third | R1 §5.2 |
| Data Containment / MPU sandbox / Fault Domain Partitioning | **ADOPT** as fail-operational bulkheading (the P-C bulkhead class, #11) — orthogonal to, and never a substitute for, the trust argument | R1 §4.5 |
| Header-Based Priority (2-bit tier in the header) | **ALREADY-EQUIVALENT-WITH-CORRECTION** — maps directly onto the #15 `CapabilityClass` envelope selector, which is checked against capability scope, **never self-assigned**; the raw "2 bits decide everything" scheme is exactly what #15 exists to prevent | R2 §4.1 |
| Hash-chaining / per-stream integrity self-check | **ALREADY-EQUIVALENT** — verify-before-persist / drift-gate (`event_log.rs:419-445`); kept as a **boolean** admission gate, never a score (§5.1 carve-out) | R3 §0, §4.1 |
| `DATA_DEGRADED`/`ADAPTER_WARNING` graceful-degradation flags | **ALREADY-EQUIVALENT** (partly) — #145's soft-refuse + last-valid-readable is the adopted form; the mark-degraded-then-escalate path is the R3 carve-out's third branch | R2 §2; R3 §4.2(c) |
| Zero-copy networking (general principle) | **ALREADY-EQUIVALENT** — content-addressed opaque payload + zero-dep fixed-layout TLV envelope (#55 rejected FlatBuffers-as-dep for this reason) | R1 §3.1 |
| Kernel as version-agnostic "Диспетчер потоків" (ARINC 653 framing) | **ADOPT-the-posture, with the R1 correction** — "adapter death → kernel disables that adapter" is the observable, fail-operational degraded state Pattern B′ produces; but the kernel is never version-*blind* at the gate: every stream still passes the same inline verify | R1 §4.2 |
| Lock-free structures · FPGA/ASIC offloading · in-band telemetry headers · State-Keepers vs Compute-Units · immutable-state (general) | **NOT-ADJUDICATED-THIS-PASS** — outside all four research clusters; no R-doc makes a claim about them, so this synthesis makes none either (honest gap, not a silent drop); they remain whatever the masterwork ledger already says about their siblings | — |

**Tally (primary 14):** ADOPT / ADOPT-WITH-CONSTRAINT = **5** (rows 1, 3, 6, 7, 9) ·
ALREADY-EQUIVALENT = **1** (row 11) · EXTEND-EXISTING = **1** (row 4) · DEFER-WITH-TRIGGER =
**4** (rows 5, 10, 13, 14) · REJECT = **3** (rows 2, 8, 12). 14/14 verdicted, zero silent drops;
supplementary concepts all carried or honestly marked.

---

## 2. The Two Real Findings That Need Action (not just verdicts)

### 2.1 The NaN-masking fail-open at the drift-measurement point — **fourth independent confirmation today; highest-priority action in this document**

**This is NOT a new finding.** It is the **fourth independent confirmation of the same bug
pattern across today's research** — previously surfaced in the dowiz-kernel line of work and the
spectral-evolution arc, and now independently re-derived by R1's grounding pass, which named it
explicitly without being asked to look for it (R1 §5.2). Four independent arrivals at the same
defect in one day is the strongest replication signal any finding in this cluster carries. It
should be prioritized accordingly.

**The exact defect (R1 §5.2, live-read):**
`dowiz-spectral-evolution/kernel/src/spectral.rs:217-218` —
`spectral_radius(a) = eigenvalues(a).map(|e| e.abs()).fold(0.0, f64::max)`. Because `f64::max`
returns the non-NaN operand, the fold **silently masks a NaN** eigenvalue-modulus. A
garbage/degenerate tile whose spectrum is NaN reports ρ≈0 → `classify_drift` → **Damped** →
admitted as *healthy* through the very gate (`RetainedBase::admit`, drift-gate) built to reject
it. That is fail-OPEN masking, RC-2 vacuity shape.

**Why it is a one-predicate-class fix, not a redesign (R1 §5.2):** the kernel *already applies
this exact law in two other places* — `spectral_cache.rs:117,131-135,149` canonicalizes NaN on
the hash path, and `hydra.rs`/`integrity_check` guards `rho.is_finite()` on the hydra path. The
law exists; it is simply absent at the third point, the `spectral_radius` measurement feeding the
drift gate. The fix: `spectral_radius` (or its `admit`/`classify_drift` caller) rejects
non-finite ρ as `UnstableSpectrum`/self-terminate, mirroring `hydra.rs`.

**DoD (falsifiable, RED-first):** a NaN-bearing-tile test that today passes admission as
`Damped` (RED — proving the hole live) and after the predicate is rejected as
`UnstableSpectrum` (GREEN). Named regression test → `REGRESSION-LEDGER.md` per contract §2.17.

**Doctrinal note:** this concretely validates the source dialogue's *value-bound* concept ("exit
criteria as hard mathematical bounds on physically possible values") — while confirming the
dialogue's *layout-header* concept is orthogonal to it: a correctly-typed, correctly-versioned
tile can still carry NaN; only the value-bound law catches it (R1 §5.2).

### 2.2 The CI enforcement-gate gap — `ci-no-courier-scoring.sh` misses `trust_weight`

**The finding (R3 §1.2, live-read):** `/root/bebop-repo/scripts/ci-no-courier-scoring.sh` is a
field-NAME grep over
`\b(score|rating|reputation|rank|trust_score|trust_level|courier_score|agent_rating)\b`.
Consequence, exactly: a field `integrity_score: f32` trips `score` → gate RED; a field
`trust_weight: f32` matches **nothing** — `trust_weight` is not a listed token (only
`trust_score`/`trust_level` are) → gate stays GREEN. The executable gate would half-catch the
row-12 proposal and is trivially renamed around (`integrity_score` → `sel_coeff` evades the
other half too). The thing that actually forbids the pattern is the doctrine
(`SOVEREIGN-EVENT-EXCHANGE-BLUEPRINT-2026-07-14.md:18-42`, "never by trusting, ranking, or
blacklisting a source"), which the grep only partially operationalizes (R3 §1).

**The concrete fix (tiny, low-risk, high-value; R3 §4.3):**
1. Minimum: extend the token list with `trust_weight`, `integrity_score`, and a
   `weight`-on-source pattern (e.g. `\b\w*(trust|peer|source|stream)_?weight\w*\b`).
2. Better (structural, since name-grep is inherently renameable-around): add a second check that
   flags any `f32`/`f64` field on a per-peer/per-source keyed struct in `bebop2/` (excl.
   `core/`) for human review — floats keyed by source identity are the shape of every rejected
   register item #33–#37, whatever they are named.
3. Note R3's own primary recommendation stands above both: keep arbitration-by-weight **out of
   the codebase entirely** per the row-12 verdict, so the gap never matters — the gate fix is
   defense in depth, not the load-bearing control.

**DoD (falsifiable, RED-first):** a fixture file containing `trust_weight: f32` on a per-peer
struct that today passes the gate (RED — proving the evasion) and fails it after the fix
(GREEN). The `sel_coeff` rename evasion documented as a known residual of any name-based lint,
answered by fix (2).

### 2.3 Carried-forward note (not this cluster's to fix, not silently dropped)

- **bebop2 `core/rng.rs` Android/iOS cfg-gap (R1 §5.1):** real portability gap for the
  phone-mesh (`target_os = "android"/"ios"` aarch64 selects no entropy arm → compile error by
  design), but it is a `cfg`-dispatch-completeness finding, **not** a layout-law miss — R1
  explicitly refused to force the connection. Route to whichever phase owns crypto portability
  (Layer E, §6).
- **G11 forged-`unit_price` remediation (R2 §5):** the live money exposure's fix is the exact
  *opposite* of Fail-Operational — server recompute via `compute_order_total` + hard-reject +
  T2 dual-authority collapse. Already tracked (T2 flip pending); flagged here only because the
  dialogue's principle, applied to it, would have deepened the hole (§5.3).

---

## 3. What Genuinely Extends Today's Work (new, safe capability)

### 3.1 Telemetry-tier temporal interpolation — the scope law (boundary adopted now; interpolator deferred, §4.4)

**Attachment point (R2 §4, no new machinery):** the interpolatable lane attaches to the
already-adopted register #15 Priority composition — `CapabilityClass::Telemetry` (or
`::Optional`) envelopes in the nested `TokenBucket` map
`BTreeMap<(PeerId, CapabilityClass), TokenBucket>`. Three existing mechanisms compose to enforce
the boundary with zero additions:

1. **#15's load-bearing property:** the tier flag is an envelope selector **checked against
   capability scope, never self-assigned** — a stream cannot declare "I'm telemetry, interpolate
   me." This single property is what the dialogue's raw 2-bit header lacks and what makes the
   extension safe (R2 §4.1).
2. **#14's non-commutative path:** money/order transitions structurally live on the `event_log`
   side, never CRDT-merged — interpolation on that path is already unrepresentable (R2 §4.2).
3. **The money RED-LINE type:** `money_guard.rs` — a fractional/interpolated money value is
   un-typeable, not detected-and-corrected (R2 §4.3).

**Predefined shape (contract §2.4 — named before implementation):** a future
`interpolate_stale()` exists **only** on the telemetry-class envelope type; `Critical` envelopes
have no such method. The boundary is type-level absence, not a runtime branch.

**DoD / falsifiable "never-critical" test (contract §2.2/§2.5):**
- **Compile-level:** a trybuild-style compile-fail test invoking `interpolate_stale()` on a
  `Critical`/money envelope — must fail to compile, permanently.
- **Runtime-adversarial:** a chaos test in which a stream self-assigns the telemetry tier flag
  without matching capability scope — must be rejected at admission (the #15 never-self-assigned
  guard), proving a forged-total message cannot smuggle itself into the interpolatable lane.
- **Determinism boundary stated honestly (R4 §5.1):** interpolation is lossy approximation —
  genuinely non-deterministic across nodes — which is precisely why it is confined to the
  eventual/adaptive-drop lane where cross-node bit-convergence is not a law, and why it can
  never touch the content-addressed command path.

### 3.2 The Untrusted-Translator Law (UT-LAW) — the side-car pattern as a type-law

**Name:** **UT-LAW** (Untrusted-Translator Law), the codified Pattern B′ of R1 §4.

**Statement (R1 §4.5):** a side-car translator's output type is *raw untrusted bytes* (an
`UntrustedBytes`-class newtype, same standing as network input), and the only path from there
into a committed/`RetainedBase` tile is the kernel's own inline gate —
`NormalizedTile::canonicalize → classify_drift → content_address` — **identical to raw network
input**. The adapter MAY self-validate and self-terminate (good: degrade-closed, one fewer bad
tile emitted), but adapter self-validation is **never** the kernel's reason to trust — that
would be `key_K` self-certification and the rejected Pattern A′ (R1 §4.4).

**Why it passes the no-proxy rule (R1 §4.3, the three doc-19 axes):** the translator sits on
the *supply* path being judged, never the *control* path doing the judging; it grants zero trust
the kernel would not compute itself (it can only cause a would-be-rejected tile to also be
rejected, never a would-be-rejected tile to be accepted); and its absence is a visibly empty
input buffer → disabled adapter (fail-operational degrade), not a silent correctness gap.

**Placement recommendation (R1 §4.5 — bulkhead, not supervision):**
- **Default: in-process** — the §3.3 Zero-Copy Shim or tagged-enum static dispatch — whenever
  the translation is cheap and the legacy codec is trusted.
- **Separate process ONLY for isolating an untrusted/crashy legacy codec**, and then the
  isolation rationale is Data Containment / MPU-style memory sandbox (the P-C bulkhead class,
  #11) — fail-operational bulkheading, never supervision. The "adapter only reads old, never
  writes old" property is kept as the orthogonal blast-radius bound (R1 §4.5).

**DoD (falsifiable, adversarial per contract §2.5):** a **lying-adapter chaos test** — a
translator that self-reports valid but emits a drift-violating / non-canonical tile; the kernel
gate must reject it pre-persist with the adapter's self-validation bypassed entirely. RED form:
temporarily route side-car output around the inline gate → the bad tile commits (proving the
gate is load-bearing); GREEN: with UT-LAW typing in place, that route does not compile.

### 3.3 Zero-Copy Shims + tagged-enum Layout Aliasing — small, real, safe

**Zero-Copy Shims (R1 §3.1) — DoD:**
- Shim signature returns `NormalizedTile`, never `&[u8]` — the bridge-gap-#1 invariant is
  structural, not reviewed-for.
- Transform body is rational/fixed-order only (no transcendental/`exp`/softmax) — the same
  determinism boundary (`rng.rs:22-28`) that killed the RGB-seed codec.
- Falsifiable check: a property test that `shim(legacy_bytes)` on two nodes yields the identical
  content-address (cross-node hash convergence); an intentionally-transcendental shim variant is
  the designed-to-fail case.

**Tagged-enum Layout Aliasing (R1 §3.2) — DoD:**
- A `#[non_exhaustive]` enum tagged by the version discriminant + `match` — static dispatch on a
  CommonHeader with zero `unsafe`; the literal C-union overlay is rejected (row 7).
- CI check: `unsafe`/`union` forbidden in the versioning module (`#![forbid(unsafe_code)]` at
  module scope or a grep gate) — the bug class becomes compile-time (contract §2.14).
- Falsifiable check: unknown discriminant → typed decode error, mirroring the existing
  `unknown_version_is_rejected_on_decode` red-team test (`framing.rs:102-114`).

**Envelope spec note (row 4's only additive sliver, R1 §1.3):** one section folded into the
envelope doc naming the two-layer split — unknown version at the **wire-decode layer** =
hard-reject/self-terminate (already built); stale/out-of-context at the **DecisionUnit/semantic
layer** = Escalate/re-pull, not death (#20/#22). Both verdicts already exist; the note only
names the split so no future reader flattens it the way the dialogue did.

---

## 4. Deferred-With-Trigger Register

Exact trigger conditions, not "if needed later":

| Item | Trigger (exact) | If/when built, the constraint | Source |
|---|---|---|---|
| **4.1 CRC32/64 HW integrity** | A measured maintenance-pass profile shows the FNV content-address recompute is hot — **the number appended to `BENCH_HISTORY.md` first**, then build | A private *acceleration of* the content-address recompute (same verdict bytes); **never** an independent trust signal — a standing second checksum authority is the #23/#47 dual-authority hazard | R1 §2.2 |
| **4.2 Reed-Solomon FEC** | The mesh actually adds a **raw datagram / broadcast / BLE-mesh / LoRa carrier with no transport-level ARQ** (today both carriers — WSS/TCP+TLS and iroh/QUIC — retransmit and integrity-check below the app; app-level gaps are handled by pull-based anti-entropy, which is ARQ) | Erasure-mode (not bit-error-mode) via patent-clean `reed-solomon-simd`; **FEC-decode BELOW crypto-verify** (`carrier bytes → FEC reconstruct → framing::decode → SignedFrame::verify → apply`); the decoder is a new hostile-input surface below the signature and must meet the V3 parser discipline (bounded alloc, no panic on adversarial input, fail-closed, fuzzed); redundancy ratio set from V5 §4 netem measurements, not guessed. **Documented as a V5 reliability control, never a security control** | R4 §2–§5 |
| **4.3 RaptorQ / fountain codes** | 4.2's trigger fires **AND** the carrier is genuine one-to-many multicast with heterogeneous unknown loss | Explicit patent/licensing **DECART** first (Qualcomm patent history vs the AGPLv3 ADR-020 goal); otherwise classical RS stands | R4 §3c |
| **4.4 The temporal interpolator** | A **real telemetry consumer needs dead-reckoning** — concretely, the courier-position/ETA smoothing feature is actually built (the plausible first consumer) | Confined to the §3.1 scope law (`CapabilityClass::Telemetry` only, compile-fail on `Critical`); the tier *boundary* is adopted now at zero cost because #15 already carries it | R2 §5 |

---

## 5. Rejected, With The Precise Reason (not softened)

### 5.1 Heuristic Arbitration / `TrustWeight × IntegrityScore` — REJECT-SAME-CLASS-AS-#33

The structural equivalence is **exact, not loose** (R3 §0, §4 — all `[PRIOR-ART-ADJUDICATED]`,
masterwork V2 `:354-358`):

| Dialogue term | Already-rejected register item | Why identical |
|---|---|---|
| `IntegrityScore` (per-source, comparative) | #33 — the "Data Integrity" **column** of the Peer Trust Matrix | it *is* that column, used to rank sources |
| `TrustWeight` | #33 "Trust" axis / #37 Trust-Weighted Dispatcher | per-source accrued weight = reputation |
| the product → pick a winner | #34 TrustScore-weighted vote | weighted selection between conflicting sources |
| continuous-float weight | #36 `AtomicF32` TrustScore (second, independent ground) | float arbitration is replay-nondeterministic; collides with deterministic replay and `money_guard.rs` |

#33's own ground note pre-answers the scope defense: *"'Local' does not exempt it"* — and by
identical logic **"it's a data stream, not a courier" does not exempt it**, because every stream
is sourced by an adapter/node; arbitrating between two streams IS ranking two sources (R3 §1).
The doctrine covers it completely regardless of field names: sovereignty is achieved by
first-principles verification, *"never by trusting, ranking, or blacklisting a source."*

**The concrete attack trace (R3 §3 — the forgery WINS, it does not merely tie):** built on this
session's own confirmed capability (`apply_event_trusts_forged_totals`, HIGH — a node already
controls the content and framing of its own stream). IntegrityScore rewards
*well-formedness* — which the forger fully controls: the attacker authors internally-perfect
bytes (valid hash, CRC, RS parity, known SchemaID, in-bounds forged `total = 1`) →
`IntegrityScore = 1.0`; the honest stream, crossing a real lossy channel, carries genuine noise
→ scores lower. Arbitration selects the forgery. TrustWeight compounds it (Cheng–Friedman
symmetric-reputation impossibility + Friedman–Resnick whitewashing: a fresh Sybil farms clean
traffic, then forges once its weight is high). **This is strictly worse than no arbitration**:
without it, the forged stream is caught by first-principles recomputation; with it, the forger
is handed a weighting mechanism to *out-rank* the honest recomputation. The attack does not
require breaking integrity — it requires *satisfying* it perfectly, which the author of the
bytes always can (R3 §3).

**The constructive carve-out (the compliant alternative — this half IS adopted doctrine, R3 §0,
§4):** (1) per-stream first-principles integrity as a **boolean admission gate**, applied to
each stream independently — a failing stream **self-terminates**; it is not "scored low," it is
refused (verify-before-persist / drift-gate, `event_log.rs:419-445`). (2) A genuine tie (both
integrity-valid, disagreeing) breaks on a **source-independent checkable fact**: capability
authorization (`verify_chain` — authorization, not scoring), or higher *validated* anchored
EpochID / hash-chain height; (3) if neither resolves it — **degrade-closed refusal**. No weight,
no score, no new machinery. Cheng–Friedman does not reach the per-payload predicate; it reaches
exactly and only the comparative weighting step — that seam is the adopt/reject line (R3 §2).

### 5.2 Trusted-proxy-shaped side-cars (Pattern A′) — REJECT

Why specifically (R1 §4.2, §4.4): the kernel commits on the adapter's word, so a translation
bug commits bad state directly; correctness depends on a separate process being **both alive
and honest**; adapter death or compromise is a **silent fail-open** (the kernel keeps trusting
a dead or lying source); and adapter self-validation as the kernel's reason to trust is `key_K`
— self-certification by a separate party. This is the watchdog/proxy class on axis 1 (absence =
silent gap), which is precisely the process the no-proxy rule removes. The dialogue's own spec
("валідація зосереджена на самому мосту") is necessary but **not sufficient** — admissibility
requires the kernel's inline re-verify regardless (R1 §4.4), which converts the pattern into
the accepted B′ form (§3.2).

### 5.3 Critical-tier Fail-Operational / interpolation — REJECT-ON-PHYSICS (already adjudicated; not re-opened)

Why specifically (R2 §1, §3): money is a **discrete authoritative fact computed from inputs**,
not a sampled continuous physical quantity — there is no meaningful "between two totals" to
interpolate; the technique is category-inapplicable, not merely risky. The sharpest test: applied
literally to the live G11 forged-`unit_price` vulnerability (the kernel's most critical tier →
the dialogue's own Priority-`00` → "рятувати будь-якою ціною" via interpolation), it converts a
trust-the-client hole into a **fabricate-and-accept** hole and destroys the one correct escape
(hard reject → server recompute via `compute_order_total`) — i.e. it makes a real, live
vulnerability strictly worse. The dialogue is also internally self-contradictory on this exact
point (critical data "ігноруються" at lines 66-69 vs "рятувати будь-якою ціною" at 72-83, never
reconciled — R2 §3). The corpus already decided the money case in the only safe direction —
unrepresentable → refuse (money RED-LINE; register #4 Speculative-Consensus REJECT: "degrade-OPEN
in a degrade-closed arch") — so this is a closed adjudication restated, not a new decision.

---

## 6. Closing: How This Connects Back to CORE-ROADMAP

Mapping every actionable onto the CORE-ROADMAP-STANDARD Layer A–I altitude axis (§3 of that
doc; "Layer" naming per the Wave-3 ratification), so this document has owners and does not
orphan:

| Actionable | Owning Layer | Note |
|---|---|---|
| **NaN `is_finite` fix at the drift-measurement point (§2.1)** | **Layer B** (state/consistency — the drift-gate admission path), applying **Layer C**'s value-bound/self-termination law | R1 §5.2 itself routes it "P-B/P-C"; RED-first test → `REGRESSION-LEDGER.md` (Layer H's ledger) |
| **`ci-no-courier-scoring.sh` keyword/structural fix (§2.2)** | **Layer H** (ops/CI/regression tooling) enforcing **Layer D**'s NO-COURIER-SCORING doctrine | tiny, standalone; RED-first fixture proves the `trust_weight` evasion before the fix |
| **UT-LAW side-car type-law (§3.2)** | **Layer B** (the `NormalizedTile` canonicalize→drift→content-address gate is the law's substrate); the separate-process variant's isolation rationale belongs to **Layer C**'s bulkhead class | in-process default; process form only for untrusted legacy codecs |
| **Zero-Copy Shims + tagged-enum aliasing (§3.3)** | **Layer A/B** boundary (memory-layout primitives producing Layer-B canonical types) | the envelope two-layer spec note lands in **Layer E**'s proto-wire framing doc |
| **Telemetry-tier boundary (§3.1, adopt now)** | **Layer D** (capability/`CapabilityClass` envelopes own the never-self-assigned tier law) | free — #15 already carries it |
| **Interpolator DoD (§4.4, deferred)** | **Layer G** (product-level consumer: courier dead-reckoning/ETA), under Layer D's boundary law | trigger = the consumer feature actually gets built |
| **CRC defer (§4.1)** | **Layer H** owns the trigger (bench discipline / `BENCH_HISTORY.md`); if built, lives in **Layer B** as content-address-recompute acceleration | never a second authority |
| **FEC / RaptorQ defer (§4.2–4.3)** | **Layer E** (network/carrier); decoder hardening per V3 parser discipline; ratio from V5 netem | never documented as a security control; RaptorQ gated on a licensing DECART |
| **rng.rs Android/iOS cfg-gap (§2.3)** | **Layer E** (crypto portability for the phone-mesh) | separate finding, honestly not a layout-law miss |
| **G11 remediation (§2.3)** | **Layer G** (money dual-authority flip, T2 — already explicitly gated there) | this cluster only reconfirms its direction: the opposite of Fail-Operational |

---

## 7. Contract compliance note (CORE-ROADMAP-STANDARD §2, adapted for extension work)

This is a verdict-and-plan synthesis, not a phase blueprint, so the 20-point contract applies in
its extension-work form: **§2.1 ground truth** — every claim cites an R-doc whose own pass
live-verified the `file:line`; nothing inherited from memory. **§2.2 DoD** — every adopted item
(§2.1, §2.2, §3.1–3.3) carries a falsifiable RED-first check. **§2.4 predefined types** —
`NormalizedTile`-typed shim output, `UntrustedBytes`-class translator output, telemetry-only
`interpolate_stale()`, tagged version enum. **§2.5 adversarial cases** — the lying-adapter
test, the self-assigned-tier test, the NaN-bearing-tile test, the `trust_weight` fixture, the
designed-to-fail transcendental shim. **§2.6 safety-from-structure** — every rejection argues
reachability (attack trace, silent-fail-open, category-inapplicability), never policy prose.
**§2.11/§2.13** — bulkhead vs supervision named explicitly (UT-LAW placement); self-termination
used in its hard-invariant sense throughout. **§2.14** — each adopted item names its
compile-time/CI gate. **§2.17** — both §2 findings ship named regression tests. **§2.19
reuse-first** — every extension names the existing pattern it extends (#15 envelopes, the
inline gate, the envelope version field) and no new machinery is proposed where the corpus
already carries the capability. No code was written; no commits made, per the operator's
standing instruction this session.
