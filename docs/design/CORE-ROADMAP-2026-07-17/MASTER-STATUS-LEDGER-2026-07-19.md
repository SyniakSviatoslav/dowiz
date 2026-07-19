# MASTER STATUS LEDGER — 2026-07-18/19 research wave, consolidated (2026-07-19)

> **Planning document — writes ZERO product code, touches no branches, pushes nothing.**
> The single consolidation of the entire 2026-07-18→19 research session (~20 dispatched Opus
> investigations + 5 synthesis/blueprint passes) into one status ledger. This is the handoff
> artifact for the follow-up Opus blueprint-writing pass (§4 is its work list) and for the
> operator (§5 is the decision list). Nothing here re-derives a source doc's findings — every
> row cites its source; re-litigating a CLOSED row requires new evidence overturning the cited
> record.

---

## 0. Provenance & file-state note (READ FIRST — some source files were lost from disk and recovered)

**Incident:** 15 of this session's research/design docs were written to
`docs/research/` and `docs/design/CORE-ROADMAP-2026-07-17/` but were **deleted from disk while
untracked** (collateral of the 2026-07-18 22:17–00:47 merge/worktree wave — they were never
committed; e.g. `OPUS-PERF-NTT-IMPLEMENTATION` is visible as `??` in the `a857cd71a` commit log
output, and is gone now). All 15 were **recovered verbatim from the session's subagent
transcripts** this pass.

- **Restored to the repo (this pass, within the planning-write scope):** the four design docs
  `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`,
  `SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md`, `SYNTHESIS-WAVE3-CLOSEOUT-2026-07-18.md`,
  `SYNTHESIS-MESH-MAJOR-REFACTOR-PLAN-2026-07-19.md` — all back in this directory.
- **Recovered but NOT restored (outside this pass's write scope — `docs/research/` restore is an
  operator/lead action):** 11 research docs, verbatim at
  `/tmp/claude-0/-root-dowiz/0928caea-fa31-4657-87ba-e84740a95a85/scratchpad/recovered/`.
  One-command restore: `cp .../scratchpad/recovered/OPUS-*.md /root/dowiz/docs/research/`.
  Until restored, the `docs/research/OPUS-{PERF-NTT-IMPLEMENTATION, KEM-RING-BUG-INVESTIGATION,
  TERNARY-BITNET…, TRUST-BOUNDARY…, ENERGY-RESOURCE…, QKD…, HANDSHAKE-ONCE…, PACKED-FLAGS…,
  CORE-CONSOLIDATION…, FRAUD-RISK…, TRISTATE-STATUS…}` paths cited below do not resolve on disk.
- **On disk already:** `OPUS-PERF-ARENA-DEEPDIVE` (committed with `a857cd71a`),
  `OPUS-BITSLICE-CONSENSUS-BATCH-SCAN`, `OPUS-BATCH-TOKENIZATION-SCAN`,
  `OPUS-BATCH-CALLS-EVENTS-SCAN`, the ten `OPUS-PERF-*` first-wave docs, and
  `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md`. `OPUS-PERF-CONTENTION-BENCH-RESULTS` +
  `bebop-bus-G-C1-fix.patch` exist **only** on local branch `perf/contention-bench-2026-07-18`
  (worktree `/root/dowiz-perf-contention`).
- **Lesson (pattern-ledger material):** untracked research docs do not survive worktree/merge
  churn — commit (or at minimum copy out) research output in the same step that produces it.

Live push-state re-verified this pass (2026-07-19 01:00): dowiz `origin/main` is at `4b30c9b4c`
— **the entire local main line above it (P57–P74 merge wave, incl. `a857cd71a` slot_arena) is
unpushed**; bebop `986646a` (NTT) sits unpushed at HEAD of local `perf/bus-contention-2026-07-18`
(and twin `perf/bus-clean-2026-07-18`); dowiz local branch `perf/contention-bench-2026-07-18`
(`8c865805b`, `8256dbffb`) is unpushed.

---

## 1. The one-glance status table

Status vocabulary (fixed): **FULLY-BLUEPRINTED-NEEDS-REGISTRATION** ·
**SKETCH-ONLY-NEEDS-FULL-BLUEPRINT** · **CLOSED-NO-ACTION-VALIDATED-DESIGN** ·
**CLOSED-NO-ACTION-NO-TARGET** · **DONE-LOCAL-UNPUSHED-CODE** · **NEEDS-OPERATOR-DECISION**.

| ID | Title | Status | One-line reason | Source doc(s) |
|---|---|---|---|---|
| **M1** | Real RFC-5705 exporter binding: capture + set-on-send + enforce-on-recv | **FULLY-BLUEPRINTED-NEEDS-REGISTRATION** | Full spec + RED tests already inside P92 §4.1/§6/§14 (no separate file needed); independent prerequisite bug (red-team F3/M1 STILL OPEN); gate-0 of the mesh cluster; 0 code | BLUEPRINT-P92 §4.1/§6; SYNTHESIS-MESH-MAJOR-REFACTOR §3 |
| **P75** | CI bench-regression gate re-architecture (same-runner criterion A/B) | **SKETCH-ONLY-NEEDS-FULL-BLUEPRINT** | Scoped as a wave-W0 unit in the synthesis only — no `BLUEPRINT-P75` file exists; owns the bench-id/baseline schema P80–P82 cite | SYNTHESIS-PERFORMANCE-AUDIT §3.1-A1, §5 |
| **P76** | bebop hidden-tests un-gate + bus-lock fix | **SKETCH-ONLY-NEEDS-FULL-BLUEPRINT** | Scoped unit only; NOTE: the bus fix is now DONE+VERIFIED as `bebop-bus-G-C1-fix.patch` (commit-blocked on C3) — the blueprint must ABSORB the patch, not re-implement | SYNTHESIS-PERFORMANCE-AUDIT §3.1-A2/A3, §5; SYNTHESIS-WAVE3-CLOSEOUT §2 |
| **P77** | Kernel complexity fixes: `spool.rs` O(N²) drain + `spine.rs` O(R²) dedup | **SKETCH-ONLY-NEEDS-FULL-BLUEPRINT** | Scoped unit only (B1/B2), behavior-preserving fixes with in-repo patterns named | SYNTHESIS-PERFORMANCE-AUDIT §3.2, §5 |
| **P78** | bebop complexity fixes: `MerkleDigest::add` + `hub_ring::ranked` | **SKETCH-ONLY-NEEDS-FULL-BLUEPRINT** | Scoped unit only (B3/B4), ~5/~10-line fixes + benches | SYNTHESIS-PERFORMANCE-AUDIT §3.2, §5 |
| **P79** | Kernel data-layout ports: causal flat `Samples` + spectral evec flatten | **SKETCH-ONLY-NEEDS-FULL-BLUEPRINT** | Scoped unit only (B5/B6); P89 prerequisite (evec k·n buffer) | SYNTHESIS-PERFORMANCE-AUDIT §3.2, §5 |
| **P80** | Kernel bench expansion (money tripwire, PQ lane, spectral, ppr/absorbing, contended locks) | **SKETCH-ONLY-NEEDS-FULL-BLUEPRINT** | Scoped unit only (C1); NOTE: the contended-lock sub-item is ALREADY DONE on `perf/contention-bench-2026-07-18` — blueprint must cross-reference, not re-specify | SYNTHESIS-PERFORMANCE-AUDIT §3.3-C1, §5; SYNTHESIS-WAVE3-CLOSEOUT §2 |
| **P81** | Engine bench harness (crate has ZERO benches, runs every frame) | **SKETCH-ONLY-NEEDS-FULL-BLUEPRINT** | Scoped unit only (C2); substrate for P87/P89 DoDs | SYNTHESIS-PERFORMANCE-AUDIT §3.3-C2, §5 |
| **P82** | bebop bench expansion (sign/KEM/AEAD; `HybridGate::check` headline) | **SKETCH-ONLY-NEEDS-FULL-BLUEPRINT** | Scoped unit only (C3); its KEM bench gates the D-9 NTT wire-in decision; natural substrate for P92's D-BENCH measurement | SYNTHESIS-PERFORMANCE-AUDIT §3.3-C3, §5 |
| **P83** | Kernel span metrics (`SpanMetricsLayer`→`metric.jsonl`) + breach-triggered `perf record` | **SKETCH-ONLY-NEEDS-FULL-BLUEPRINT** | Scoped unit only (C4); zero new deps; parallel-safe | SYNTHESIS-PERFORMANCE-AUDIT §3.3-C4, §5 |
| **P84** | Golden state-digest regression gate (reserved number) | **NEEDS-OPERATOR-DECISION** | Reserved, NOT proposed until D-1 is ruled — touches money/FSM red-line surfaces | SYNTHESIS-PERFORMANCE-AUDIT §4 D-1; SYNTHESIS-PHYSICS-PERFORMANCE-VISION §4 |
| **P85** | NTT red-line process remediation (the `--no-verify` A4 violation) | **SKETCH-ONLY-NEEDS-FULL-BLUEPRINT** (scope judged ADEQUATE — near-executable as written) | S2 §4.1 recipe is complete (re-run 4 skipped gates + real 3-model review OR recorded retroactive sign-off); S3 §4 confirms scope unchanged; BLOCKING: quarantines D-9 wire-in, P91.1 port, and (with C3) all bebop hook-respecting commits | SYNTHESIS-PHYSICS-PERFORMANCE-VISION §3/§4.1; SYNTHESIS-WAVE3-CLOSEOUT §4; OPUS-PERF-NTT-IMPLEMENTATION |
| **P86** | SlotArena × GPU texture-channel lifecycle fusion (operator item A) | **SKETCH-ONLY-NEEDS-FULL-BLUEPRINT** | S2 §4.2 design sketch; build gated on P38 §4.2 (operator-owned GPU decision); designs the first real slot-arena consumer | SYNTHESIS-PHYSICS-PERFORMANCE-VISION §4.2; OPUS-PERF-ARENA-DEEPDIVE §6 |
| **P87** | Minimal-bit-depth (2-bit) ping-pong companion state-mask plane (operator item B) | **SKETCH-ONLY-NEEDS-FULL-BLUEPRINT** | S2 §4.3 sketch; 2-bit = lifecycle flags NOT physics (that literal reading is REJECTED); falsifiable DoD (cut if no measured win); depends P81/P86/P38 §4.2 | SYNTHESIS-PHYSICS-PERFORMANCE-VISION §4.3 |
| **P88** | Atomicity-by-default policy in the physics/GPU domain (operator item C) | **SKETCH-ONLY-NEEDS-FULL-BLUEPRINT** | S2 §4.4 sketch; policy+WGSL checklist; WRITE FIRST among physics units (constrains P86/P87 shader design); CPU domain stays evidence-gated (E4/E12 stand) | SYNTHESIS-PHYSICS-PERFORMANCE-VISION §4.4 |
| **P89** | Field eigenmodes via kernel `spectral.rs` (operator item D — the falsifiable bet) | **SKETCH-ONLY-NEEDS-FULL-BLUEPRINT** | S2 §4.5 sketch; T1–T3 sign/domain reconciliation tests + 3-path head-to-head bench make the operator-vs-research call; CPU-only, NOT gated on P38 §4.2; depends P79/P81/P75 | SYNTHESIS-PHYSICS-PERFORMANCE-VISION §4.5 |
| **P90** | Contention-bench verified fixes — registration + open ends | **SKETCH-ONLY-NEEDS-FULL-BLUEPRINT** + **DONE-LOCAL-UNPUSHED-CODE** (the fixes) + carries 3 **NEEDS-OPERATOR-DECISION** items | Budget CAS (2.0×) + token_bucket clock hoist (+6–18%) + contention harness DONE on local branch; GCRA benched-not-shipped (operator-gated); blueprint = register results + carry W3-1/2/3 | SYNTHESIS-WAVE3-CLOSEOUT §2; OPUS-PERF-CONTENTION-BENCH-RESULTS (branch-only) |
| **P91** | Kernel `pq/kem.rs` ring correction (cyclic→negacyclic, η1=3→2, ct 1536→1088) | **SKETCH-ONLY-NEEDS-FULL-BLUEPRINT** (+ P91.0 sub-item **NEEDS-OPERATOR-DECISION**) | Confirmed NOT FIPS-203; un-wired/feature-gated = fix-before-wiring, not an incident; false compliance header is the trap (P91.0 comment-only defusal separable); P91.1 port gated on P85 | OPUS-KEM-RING-BUG-INVESTIGATION; SYNTHESIS-WAVE3-CLOSEOUT §3 |
| **P92** | Mesh hot-stream fast-path (verify-once + channel-bound PQ session MAC) | **FULLY-BLUEPRINTED-NEEDS-REGISTRATION** | Full 20-point blueprint on disk (restored §0); VERDICT GO-WITH-CONDITIONS + measure-first D-BENCH NO-GO gate; hard-prereq M1; independent adversarial-review gate mandatory; 0 code | BLUEPRINT-P92; OPUS-HANDSHAKE-ONCE-VS-PERMESSAGE §5b |
| **P93** | Transcript-binding + replay-window for the store-and-forward path | **SKETCH-ONLY-NEEDS-FULL-BLUEPRINT** | Scope sketch in mesh plan §4; closes cross-node replay C3 on the DOMINANT default path; blueprint pass MUST close D-93-C (broadcast) and the blinded-tag derivation if D-93-A rules blinded; homes settled (transcript→`proto-cap/signed_frame.rs`, window→`mesh-node`) | SYNTHESIS-MESH-MAJOR-REFACTOR §4; OPUS-CORE-CONSOLIDATION-AUDIT §3 |
| **P94** | Scope/Effect in-memory bitmask (`ScopeMask([u32;18])`, Copy, branchless subset) | **SKETCH-ONLY-NEEDS-FULL-BLUEPRINT** | Scope sketch in mesh plan §5; in-memory ONLY — TLV signed wire form unchanged (signature-stability KAT is the DoD spine); value-depends on P92 (sequence after) | SYNTHESIS-MESH-MAJOR-REFACTOR §5; OPUS-PACKED-FLAGS-AUTH-LAYER-SCAN |
| **P95** | Living-memory BM25 index persistence + incremental update (vs. full rebuild every call) | **FULLY-BLUEPRINTED-NEEDS-REGISTRATION** | Explicit VERDICT = HOLD/NO-GO absent a real repeated-write/-query caller (`gov_recall`'s only driver is dead code); latent hazard only — primary fix is persistence+incremental, SIMD explicitly secondary; property-test plan (P1-P5) proves incremental ≡ full-rebuild | OPUS-TOKENIZATION-LIVINGMEMORY-RECHECK; BLUEPRINT-P95 |
| **P96** | Wire live Kalman/EMA courier speed into ETA (replaces discarded-signal static baseline) | **FULLY-BLUEPRINTED-NEEDS-REGISTRATION** | Small, isolated, non-red-line — explicitly does NOT need the heavy review process the mesh/crypto blueprints need; new `eta_seconds_adaptive` falls back byte-for-byte to the existing static estimate when cold/out-of-band, bounded-degradation guaranteed by construction; respects (does not reopen) the standing TimesFM-for-ETA rejection | OPUS-HIGHERABSTRACTION-PRODUCT-SCAN; BLUEPRINT-P96 |
| **I1/NTT** | bebop2 ML-KEM-768 incomplete NTT (`986646a`), exhaustively proven, NOT wired | **DONE-LOCAL-UNPUSHED-CODE · PROCESS-RED** | Technical GREEN (0/65,536 mismatches) but committed `--no-verify` past 5 gates incl. mandatory 3-model review — a *blocked* item, never a *completed* one, until P85 closes | OPUS-PERF-NTT-IMPLEMENTATION; SYNTHESIS-PHYSICS-PERFORMANCE-VISION §3 |
| **I2/Arena** | thunderdome→`kernel/src/slot_arena.rs` behind off-default `slot-arena` feature (`a857cd71a`) | **DONE-LOCAL-UNPUSHED-CODE** | Operator override of the research "no adoption" verdict, logged in the divergence ledger; zero default-build cost; P86 designs the first consumer; push decision open (W3-4) | OPUS-PERF-ARENA-DEEPDIVE §6; SYNTHESIS-PHYSICS-PERFORMANCE-VISION §2 row 1 |
| **I3/Contention** | Contended benches + budget CAS + clock hoist (`8c865805b`+`8256dbffb`, branch-only) | **DONE-LOCAL-UNPUSHED-CODE** | 637 kernel tests green on branch; discharges the R17 fold-in; registered by P90; merge/push decision open (W3-2) | OPUS-PERF-CONTENTION-BENCH-RESULTS (branch); SYNTHESIS-WAVE3-CLOSEOUT §2 |
| **R-TB** | Trust-boundary / closed-channel crypto-removal scan | **CLOSED-NO-ACTION-VALIDATED-DESIGN** | The principle is ALREADY applied everywhere valid (event_log = ordering-only; app↔pgrust = loopback+RLS, zero signing); the one signed wire (mesh) provably needs it (semi-trusted relay + live forgery PoCs); watch-item only: future same-host UDS IPC | OPUS-TRUST-BOUNDARY-CLOSED-CHANNEL-SCAN; SYNTHESIS-WAVE3-CLOSEOUT §5.2 |
| **R-HS** | Handshake-once vs per-message signing | **CLOSED-NO-ACTION-VALIDATED-DESIGN** (spawned M1 + P92) | Per-message signing is structurally required for store-and-forward/gossip/breach (auth travels WITH the frame); the ONE legitimate narrow opportunity became P92; the exporter gap became M1 | OPUS-HANDSHAKE-ONCE-VS-PERMESSAGE |
| **R-TRI** | Tri-state status modeling audit (money/payment/verification) | **CLOSED-NO-ACTION-VALIDATED-DESIGN** | No bool-collapse exists; `PaymentStatus::NoneYet`/`Option`/`Result<_,enum>` model "not-yet-known" correctly everywhere; leaf crypto `bool` is exactly right; positive validation record | OPUS-TRISTATE-STATUS-AUDIT |
| **R-CC** | bebop2 core-consolidation audit ("more calculation into core?") | **CLOSED-NO-ACTION-VALIDATED-DESIGN** (+ binding placement rulings feeding P93) | Crypto primitives already centralized in zero-dep core; W/A/H split + license boundary deliberate — keep; ruled: transcript layout→proto-cap, replay window→mesh-node NEVER core (no-clock contract) | OPUS-CORE-CONSOLIDATION-AUDIT |
| **R-BCE** | Batch calls / RPC coalescing + event-tick batching scan | **CLOSED-NO-ACTION-VALIDATED-DESIGN** | Already batch-oriented where batching matters (WASM array ingest; Merkle anti-entropy ships ONE batched frame); deliberately single-item where determinism demands (per-event commit, saga legs); volume ~3 orders of magnitude below any batching threshold | OPUS-BATCH-CALLS-EVENTS-SCAN |
| **R-BTOK** | Batch/SIMD tokenization scan (auth tokens + BM25 corpus) | **CLOSED-NO-ACTION** — auth angle **NO-TARGET**, corpus angle **VALIDATED-DESIGN** | Auth: one-chain/one-frame by construction, rate-limited, no HTTP surface, sig axis settled (B4); corpus: the hundreds-scale batch exists but is once-per-process/`OnceLock` cold-path — the right shape already; conditional trigger recorded (live re-index hot path) | OPUS-BATCH-TOKENIZATION-SCAN |
| **R-PF** | Packed-flags / bitmask scan of the auth decision layer | **CLOSED-NO-ACTION-VALIDATED-DESIGN** (spawned P94) | ~90% of the layer already minimal (Result/enum/niche); the ONE genuine candidate (Scope/Effect Vec→mask) became P94, correctly scoped as a P92 enabler, in-memory only | OPUS-PACKED-FLAGS-AUTH-LAYER-SCAN |
| **R-BIT** | BitNet b1.58 / ternary weight quantization | **CLOSED-NO-ACTION-NO-TARGET** | No trained NN weight matrix exists anywhere, by explicit standing policy (kernel = deterministic pure functions; rerankers/TimesFM already rejected); only reopening trigger = operator commissions an in-repo embedding model | OPUS-TERNARY-BITNET-QUANTIZATION-SCAN; SYNTHESIS-WAVE3-CLOSEOUT §5.1 |
| **R-QKD** | QKD / quantum physical-layer security deep-dive | **CLOSED-NO-ACTION-NO-TARGET** | QKD needs two owned immobile endpoints + dedicated fiber — dowiz has zero such links (single Hetzner box, mobile clients, public-internet paths, third-party backup); PQC (already in-tree, hybrid ML-KEM/ML-DSA) is the real answer; QKD provides no auth anyway | OPUS-QKD-QUANTUM-SECURITY-DEEPDIVE |
| **R-EN** | Energy/compute-expenditure as internal currency / anti-Sybil unit | **CLOSED-NO-ACTION-NO-TARGET** | Already realized in better form where it applies (Wasmtime fuel = gas; `budget.rs` = degrade-closed spend meter) and PoW explicitly REJECTED for Sybil (Batch-7, regressive); one flagged-not-recommended residual: client-puzzle on the pre-crypto admission surface, telemetry-triggered only. Side observation carried: no customer-API rate-limiter exists (a rate-limit gap, not an energy gap — out of that scan's scope) | OPUS-ENERGY-RESOURCE-CURRENCY-SCAN |
| **R-FR** | Fraud / risk / anomaly scoring surface scan | **CLOSED-NO-ACTION-NO-TARGET** | No un-delegated product problem: paid-order fraud → PSP (Stripe Radar/Adyen); spam → TokenBucket+Turnstile; scoring red-lined (NO-COURIER-SCORING, never-reputation); the typed escalate-only `FraudAuth` slot + `voice.rs` template exist as the documented reservation — do not build speculatively | OPUS-FRAUD-RISK-SURFACE-SCAN |
| **R-BS** | Bit-slicing / SWAR batch state-classification scan | **CLOSED-NO-ACTION-NO-TARGET** | No batch classify step exists (the only real batch is dominated by per-item sig verify — Amdahl); the state it would classify in bulk (multi-valued trust) is architecturally FORBIDDEN (NO-COURIER-SCORING); positive control: repo already bit-slices Keccak (keccak_x4_avx2) where a batch genuinely dominates | OPUS-BITSLICE-CONSENSUS-BATCH-SCAN |

Standing prior context (unchanged by this ledger): S1's Tier-D deferred items D1–D10 and
rejection log E1–E18, and S2's divergence ledger + updated override log, remain binding —
cite, don't re-derive.

---

## 2. The two kinds of "closed" — make the distinction legible

Both kinds mean "no code needed," for **opposite** reasons. Conflating them causes exactly two
failure modes this section exists to prevent: re-litigating a NO-TARGET item as if it were
"never investigated," and failing to count the VALIDATED items as the positive evidence they are.

### 2.1 CLOSED because the EXISTING design was investigated and found already correct (positive validation record)

These five scans went looking for a defect/gap and found the codebase already holds the correct
design — each is an adversarial pass the architecture *survived*:

| Item | What was validated |
|---|---|
| R-TB trust-boundary | Cheap ordering already used on every genuinely-closed path; heavyweight crypto spent ONLY where ordering provably cannot substitute (adversarial wire; insider non-repudiation) |
| R-TRI tristate-status | Money/payment/verification state modeling: `NoneYet`, `Option`, `Result<_,enum>`, event-sourced ledger — no bool-collapse anywhere; the P60 payment adapter is the exemplar |
| R-CC core-consolidation | "One authoritative impl per concern, at the lowest layer needing no ambient authority" already in force; W/A/H split + MIT/AGPL boundary deliberate and correct |
| R-BCE batch-calls-events | Already batched where batching pays (WASM ingest, Merkle anti-entropy); deliberately single-item where determinism/atomicity demand it |
| R-BTOK (corpus angle) | Corpus tokenization already once-per-process behind `OnceLock` — the correct cold-path shape; nothing on a latency budget |

(R-HS and R-PF are also validations of the existing per-message model / decision-layer
minimality — they additionally spawned P92/M1 and P94 respectively, so they carry action rows.)

### 2.2 CLOSED because NO target/precondition for the technique exists here (do NOT re-scan without the named trigger)

These five investigated a real, established technique and found the codebase has **nothing for
it to attach to** — often because a standing policy deliberately forbids the precondition:

| Item | Why no target | Named reopening trigger |
|---|---|---|
| R-BIT BitNet | No trained weight matrices exist, by policy | Operator commissions an in-repo embedding model |
| R-QKD | No two owned fixed endpoints with owned fiber | dowiz owns two metro DCs + dark fiber (several pivots away) |
| R-FR fraud-risk | Fraud surface deliberately PSP-delegated / mechanical / reputation-free | A PSP-invisible, rate-limit-immune, reputation-free signal is identified |
| R-BS bitslice | No batch classify step; bulk trust-state forbidden (NO-COURIER-SCORING) | Thousands-node mesh AND a periodic mechanical-lifecycle roster sweep |
| R-EN energy-currency (as NEW mechanism) | Compute-as-unit already exists in better form; PoW rejected with citations | Telemetry shows a real spoofed-source pre-crypto flood (client-puzzle residual only) |
| R-BTOK (auth-token angle) | Validation is one-frame by construction + rate-limited; no HTTP auth surface in-tree | A real production batch caller appears |

---

## 3. Dependency-ordered execution sequence (everything that DOES eventually need code)

Reasoned, not concatenated. Three findings govern the merge of the mesh sequence with P75–P91:

1. **The two repos are largely independent lanes** — dowiz-kernel/engine perf work (P75, P77,
   P79–P81, P83, P87–P89, P90-merge, P91) never touches bebop2; the mesh cluster + bebop perf
   work (P85, P76, P78, P82, M1, P93, P92, P94, D-9) never touches the dowiz kernel. They
   parallelize fully across lanes.
2. **The bebop lane has a real shared gate-0 the mesh plan alone doesn't show:** the C3
   ungated-keygen HARD-law red state + the unremediated `986646a` base currently freeze ALL
   hook-respecting commits on the bebop working branch. Until C3 is resolved (operator-gated)
   and P85 closes, nothing bebop-side — including M1/P93/P92/P94 implementation — can land
   through the hooks. So **P85 + C3 resolution precede the entire bebop lane**, not just the
   NTT wire-in.
3. **Only two genuine cross-lane soft edges exist:** P75's bench schema is cited by every new
   bench including P82's; and P82's `HybridGate::check` bench lane is the natural substrate for
   P92's measure-first D-BENCH gate (P92 §10 can self-measure, but doing it inside P75's
   schema/P82's lane avoids a second measurement convention). Neither is a hard block.

**The sequence (∥ = parallel within a wave):**

| Wave | dowiz lane | bebop lane | Gate |
|---|---|---|---|
| **0 — protection machinery + gate-0** | **P75** (CI gate re-architecture — everything benches into its schema) ∥ P90 rulings (W3-1/2/3, operator) | **P85** (NTT remediation) + **C3 resolution** (operator) — unfreezes all bebop commits | none |
| **1 — correctness fixes** | **P77** ∥ **P79** (disjoint kernel files); **P88** policy text written (constrains all later WGSL; no build dep) | **P76** (un-gate tests + ABSORB the bus patch) → **M1** (exporter fix, own reviewed commit + independent adversarial review) | P76/M1 need wave-0 bebop gate |
| **2 — bench coverage + mesh hardening** | **P80** (after P75 hard + P90 merge ruling so contended benches aren't re-specified) ∥ **P81** ∥ **P83** (anytime) | **P78** → **P82** (bench expansion; feeds D-9 + P92 D-BENCH) ∥ **P93** (transcript+replay — after M1 on the shared `signed_frame.rs` surface) | P80–P82 need P75 |
| **3 — the falsifiable bets** | **P89** (after P79+P81; delivers the modal-vs-DCT verdict data) ∥ **P91.1** (kernel KEM fix — after P85, ports bebop2's proven code; P91.0 header defusal earlier on operator OK) | **P92** (after M1 HARD + P93 by priority; run D-BENCH — NO-GO if presence volume doesn't clear the threshold) | P92 gated on M1 + D-BENCH + review gate |
| **4 — value-dependent + GPU-gated** | **P86 → P87** (GATED on the P38 §4.2 operator GPU decision; inherit P88's rule) | **P94** (after P92 — its value is created by the fast-path) ∥ **D-9 NTT wire-in** (only if P82 bench proves it hot + P85 closed + operator sign-off) | P38 §4.2; P92 landed; D-9 triple-gate |

Priority *within* the merged picture: the highest-leverage single items are **P75** (dowiz — the
perf gate everything else writes baselines into) and **P85+C3** (bebop — the freeze-breaker),
then **M1** (an open red-team correctness item on the live path). P92/P94 are opt-in,
gated optimizations and correctly sit late; P86/P87 wait on an operator decision that has not
been taken.

---

## 4. Exactly which items are SKETCH-ONLY-NEEDS-FULL-BLUEPRINT (the follow-up Opus pass's work list)

Confirmed against the real docs (P75–P83 were checked in SYNTHESIS-PERFORMANCE-AUDIT §5: they
are wave-table scoped units, never individual blueprint files — no `BLUEPRINT-P7x`/`P8x`/`P9x`
file exists in this directory except P92):

**Eighteen blueprints to write** (each against `CORE-ROADMAP-STANDARD-2026-07-17.md` §2's
20-point contract):

1. **P75** — must own the bench-id/baseline schema + gate semantics (P80–P82 cite, never redefine).
2. **P76** — must absorb `bebop-bus-G-C1-fix.patch` as landed-work, not re-implementation; carries the D-2 `reputation.rs` flag without blocking on it.
3. **P77**
4. **P78**
5. **P79** — coordinate B6 with the Phase-28 single-eigen-surface ruling.
6. **P80** — must cross-reference P90 (contended benches already exist on-branch) and code R1 §4's ppr sweep as written.
7. **P81**
8. **P82**
9. **P83**
10. **P85** — scope already adequate per S3 §4 (a remediation recipe more than a build unit); the writing pass may be light — do not expand scope, just formalize + track the C3 precondition.
11. **P86** — encode R16's shared-vs-separate-pair rule as a type distinction; note P38 §4.2 gating.
12. **P87** — precision ladder (f32 authority / f16 presentation / 2-bit flags) is its single-owner contract.
13. **P88** — write FIRST among the physics units; the atomicity-exemption proof format + WGSL checklist are its single-owner contract.
14. **P89** — T1–T3 first; the three-path bench table is the verdict mechanism, no other doc may pre-empt it.
15. **P90** — registration + the three open ends (GCRA, merge, C3); DoD is rulings-recorded, not code.
16. **P91** — P91.0/P91.1/P91.2 structure as sketched; ship-RED ACVP gate; same 3-model rigor as P85 enforces.
17. **P93** — MUST close D-93-C (broadcast/multicast) and fully specify the blinded-tag shared-secret derivation if D-93-A is ruled "blinded"; versioned signing-domain discriminant is non-negotiable.
18. **P94** — signature-stability KAT + full 17×24-matrix equivalence RED test are the DoD spine; in-memory only, wire form untouched.

(18 P-numbers total: P75–P83 = 9 · P85–P91 = 7 (P84 excluded — operator-gated, unproposed) ·
P93–P94 = 2.)

**NOT on the list:** **M1** (P92 §4.1 suffices — needs an implementation+review pass, not a
blueprint) · **P92** (fully blueprinted — needs roadmap registration + its gates run) · **P84**
(reserved until D-1 ruled) · every §2 closed item.

---

## 5. Operator decisions outstanding (consolidated — one line each, nothing lost)

| # | Decision | Blocks / affects | Default if unruled | Source |
|---|---|---|---|---|
| OD-1 | GCRA lock-free swap on `token_bucket` (3.6× @8t benched; security primitive; low real contention) | P90 | NOT shipped; Mutex+clock-hoist stands | S3 §6 W3-1 |
| OD-2 | Push/merge `perf/contention-bench-2026-07-18` to remote/main | P90, P80 (contended benches) | Stays local — against the push-after-milestone precedent; rule promptly | S3 §6 W3-2 |
| OD-3 | Resolve bebop C3 ungated-keygen red state (or explicit `--no-verify` ruling for the bus patch) | ENTIRE bebop lane (commit freeze), P85, P90 | Bus patch stays a file; branch commit-frozen | S3 §6 W3-3 |
| OD-4 | Push `a857cd71a` (slot_arena) — and, wider, the whole unpushed local main line above `4b30c9b4c` (P57–P74 wave) | I2; general repo safety | Stays local — same push-precedent concern | S3 §6 W3-4; §0 above |
| OD-5 | Execute P91.0 (comment-only false-FIPS-claim removal in `kem.rs` header) ahead of P91.1 | P91 | Header keeps falsely claiming FIPS-203 — the trap stays armed | S3 §6 W3-5 |
| OD-6 | P85 closure path: real 3-model review vs recorded retroactive sign-off | P85, D-9, P91.1 | Quarantine holds (no wire-in, no Montgomery, no dependent work) | S3 §6 W3-6 |
| OD-7 | D-1 golden state-digest regression gate (would become P84; touches money/FSM red-line) | P84 | Not proposed | S1 §4 D-1 |
| OD-8 | D-2 `reputation.rs` — delete or event-source (courier-scoring red-line divergence) | P76 scope note | Undecided; fix trivial either way once ruled | S1 §4 D-2 |
| OD-9 | D-3/D-9 `pq_kem` NTT wire-in (triple-gated: P82 bench evidence AND P85 complete AND sign-off) | D-9 | Not wired | S1 §4 D-3; S2 §7 D9′ |
| OD-10 | D-4 PPR determinism relaxation — standing default REJECTED; recorded so it is never adopted silently | nothing | Rejection stands | S1 §4 D-4 |
| OD-11 | P38 §4.2 GPU field-state decision (operator-owned) | P86, P87 (build), all W5 GPU work | Nothing starts | S2 §4.2/§6 |
| OD-12 | D-93-A privacy fork: plaintext `ReceiverID` vs blinded recipient tag (recommendation: blinded; human-gated fork) | P93 | Blueprint records BOTH; no default taken | Mesh plan §4.3 |
| OD-13 | D-93-C broadcast/multicast strategy: per-recipient signed copies vs wildcard-sentinel defer (provisional: sentinel-defer) | P93 | Blueprint pass must resolve or defer-with-reason; no silent default | Mesh plan §4.3 |
| OD-14 | P92 proceed ruling after the D-BENCH measure-first gate runs (+ arrange the mandatory independent adversarial review for M1 and the fast-path) | P92, M1 | NO-GO if the bench doesn't clear; review gate is DoD-blocking | P92 VERDICT/§8/§10.3 |
| OD-15 | Restore + commit the 11 recovered `docs/research/` files from the scratchpad `recovered/` dir (§0) — this session's research record is otherwise disk-lost | Source integrity of this whole ledger | Files remain only in scratchpad + transcripts | §0 above |

(For completeness: S2 §2's divergence-ledger rows are *decisions already taken* — slot-arena
adoption, GPU-RGBA pursuit, spectral.rs-over-DCT, atomicity-by-default, 2-bit-as-flags,
money-quantization DECLINED — each with a named falsifiable exit. They are not re-raised here.)

---

## 6. Closing assessment (honest, one paragraph)

Across the whole day — roughly twenty dispatched investigations plus five synthesis passes — the
dominant result is that **dowiz's core architecture held up under sustained, genuinely
adversarial scrutiny**: money and payment state modeling, the crypto trust topology (per-message
hybrid signing over a semi-trusted mesh, capability-not-reputation, ordering-where-closed), core
crate layering, and batch/throughput posture were each attacked by a dedicated pass and each came
back "already correct," while five speculative technique-applications (BitNet, QKD, fraud
scoring, bit-slicing, energy-currency) came back "no target exists here, often by deliberate
policy" — honest negatives that are themselves load-bearing deliverables. The genuinely
actionable work is concentrated and finite: the mesh-auth cluster (M1's real exporter binding —
the one open red-team correctness item — then P93's store-and-forward hardening, then the
opt-in P92 fast-path and its P94 enabler) and the performance-infrastructure tiers (P75's broken
CI bench gate first, then the small algorithmic fixes and the large mechanical bench-coverage
expansion, P85/P91 restoring crypto process-integrity and spec-compliance). Two process findings
matter as much as any technical one: a red-line crypto commit bypassed the review gate built for
exactly that surface (P85 exists to close it), and a day of research output nearly vanished
because untracked docs don't survive worktree churn (§0). The system is not short on sound
design; it is short on landed wiring, pushed branches, and a handful of operator rulings — §5 is
the actual critical path.

---

*Cross-references: `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` (S1) ·
`SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md` (S2) · `SYNTHESIS-WAVE3-CLOSEOUT-2026-07-18.md`
(S3) · `SYNTHESIS-MESH-MAJOR-REFACTOR-PLAN-2026-07-19.md` · `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`
· `CORE-ROADMAP-STANDARD-2026-07-17.md` (the 20-point contract for §4's writing pass) ·
`docs/research/OPUS-*-2026-07-1{8,9}.md` (see §0 for on-disk vs recovered state) · memory:
`crypto-safe-first-pass-2026-07-14.md`, `worktree-remote-push-collision-avoidance-2026-07-18.md`,
`performance-priority-over-minimal-change-2026-07-17.md`, `never-bypass-human-gates-2026-06-29.md`.*
