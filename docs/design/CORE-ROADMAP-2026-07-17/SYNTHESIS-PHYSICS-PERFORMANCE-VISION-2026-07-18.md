# SYNTHESIS — Physics/Performance Vision: extending the ten-pass audit with the 2026-07-18 physics/GPU research + four operator-directed blueprint units (P85–P89)

> **Planning document — writes no product code.** Companion to and EXTENSION of
> `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` (the ten-pass reconciliation, P75–P83). That
> document's tiers, reconciliations, rejections (§6), and wave plan **all stand** — nothing here
> replaces them. This pass folds in the five newer 2026-07-18 research reports plus two
> integration events (thunderdome landed; NTT implemented-but-hook-bypassed), converts the
> operator's four explicit directions (A–D below) into blueprint units **P85–P89** (P84 stays
> reserved for the D-1 golden-digest ruling), and records one explicitly EXCLUDED scope item.
> Every unit is to be written against the 20-point contract in
> `CORE-ROADMAP-STANDARD-2026-07-17.md`.
>
> **Honesty preserved:** several operator directions in this pass **overrule the source
> reports' own recommendations**. Each override is recorded verbatim in §2 (divergence ledger)
> with the research position it reverses, per this repo's rule that decisions reversed at the
> operator layer are logged, never silently absorbed (`OPUS-PERF-ARENA-DEEPDIVE §6` set the
> precedent). The operator's own framing applies throughout: *"if metrics show I'm wrong I'll
> admit it"* — every directed unit therefore carries the falsifiable test that would prove or
> disprove it.

---

## 0. Inputs (all read in full this pass)

| # | Input | One-line verdict / event |
|---|---|---|
| S1 | `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` | Foundation. Tiers A–E, P75–P83, waves W0–W3 — unchanged and binding. |
| R11 | `OPUS-PERF-NTT-IMPLEMENTATION-2026-07-18.md` | Real, KAT-verified incomplete NTT for ML-KEM-768 in bebop2/core; **exhaustive** 65 536-basis-pair proof, 0 mismatches; NOT wired into the live KEM. **BUT committed via `git commit --no-verify`, bypassing the red-line pre-commit gate incl. mandatory 3-model review — OPEN PROCESS VIOLATION (§3, Tier A4).** Bonus finding: kernel `pq/kem.rs` implements the WRONG ring (cyclic, η1=3) — separate red-line lane. |
| R12 | `OPUS-PERF-ARENA-DEEPDIVE-2026-07-18.md` | Research verdict was "(c) no adoption"; **operator overrode it** (§6 of that doc): thunderdome integrated as `kernel/src/slot_arena.rs` behind off-by-default `slot-arena` feature, commit `a857cd71a` on dowiz `main` (**local, not pushed** — verified `git log origin/main..main` this pass). 712 tests green with feature; default build byte-unchanged. |
| R13 | `OPUS-PERF-RGB-GPU-TEXTURE-PACKING-2026-07-18.md` | GPU-compute angle is genuinely different from the (rejected) CPU angle: RGBA/RG32F packing has real value for multi-channel field state, per-cell coefficient texels, complex spectral `(re,im)`, free hardware bilinear for sub-cell sampling, and is the ONLY compute mechanism on the WebGL2 floor. All gated behind P38 §4.2. |
| R14 | `OPUS-PHYSICS-WAVE-ALGORITHMS-AND-TENSOR-SYNTHESIS-2026-07-18.md` | FDTD/Stable-Fluids/Tessendorf survey + three verdicts: atomicity (field step needs zero atomics; first real site = energy reduction, must be fixed-order/fixed-point), quantization (f16/fixed-point presentation-side only; hard-walled from money/oracle), eigenvectors (**recommended DCT/FFT, NOT `spectral.rs` — operator has since OVERRULED this; see §2 row 3 and P89**). |
| R15 | `OPUS-SPECTRAL-EVERYWHERE-SWEEP-2026-07-18.md` | Broad sweep: no new high-value spectral target beyond the field engine; kernel already spectral everywhere it applies; graph `+(D−A)` vs field `−(D−A)` operator mismatch re-confirmed (same overrule note as R14 applies to its field-adjacent framing). Money = absolute wall for spectral/float. |
| R16 | `OPUS-PINGPONG-SHADOW-COPY-PROPAGATION-2026-07-18.md` | Shadow-then-promote already the codebase's native idiom at 8 independent sites; GPU rule fixed: **one ping-pong pair per independently-evolved texture; correlated scalars packed into channels share that pair's swap** (shared stencil + cadence + stability ⇒ shared pair). |
| R17 | `OPUS-PERF-CONTENTION-BENCH-RESULTS-2026-07-18.md` | **ABSENT at this pass** (verified — file does not exist; bebop-repo currently sits on branch `perf/bus-contention-2026-07-18`, suggesting the sibling pass is in flight there). **Fold-in obligation recorded:** when it lands, its results feed P88's CPU-domain boundary and the E12 (Mutex→CAS bench-first) ruling. Do not treat its absence as evidence in either direction. |

---

## 1. Executive summary

One sentence: **today's research confirms the field engine's physics evolution is well-mapped and
mostly future-gated — the one live emergency is procedural, not technical: a red-line crypto
commit that bypassed the very peer-review gate built after the last crypto false-green.**

1. **Tier A gains one item (A4):** the NTT work is technically excellent (exhaustive proof) but
   process-RED — `--no-verify` skipped five pre-commit gates including the mandatory 3-model
   review on a crypto surface. Remediation (P85) is blocking: no wire-in, no building on top,
   until the skipped checks are re-run or retroactive sign-off is recorded (§3).
2. **Four operator directions become four blueprint units** (P86–P89): slot-arena × GPU-texture
   lifecycle fusion; minimal-bit-depth ping-pong companion state; atomicity-by-default in the
   physics/GPU domain; field eigenmodes via kernel `spectral.rs`. Each is a real decision, not an
   open question — but each carries the falsifiable test the operator's own standard demands, and
   §2 records where each diverges from the source research.
3. **One scope item is explicitly EXCLUDED** (§5): quantization inside `kernel/src/money.rs` and
   the CPU determinism/crypto oracle — declined as a safety/correctness red-line, with a named
   safe alternative available on request.
4. **The P75–P83 plan is untouched.** New units slot in as wave W0 (P85 joins immediately) and a
   new wave W4/W5 (§6). The GPU-side units remain gated behind P38 §4.2, which stays
   operator-owned; nothing in this pass silently takes that decision.

---

## 2. Divergence ledger — operator decisions vs. research recommendations (record, don't re-litigate)

These are **decisions, not open questions**. They are logged here so the research record and the
build record never silently contradict each other, and so each bet's falsifiable exit is named.

| # | Topic | Research position (source) | Operator decision | Falsifiable exit |
|---|---|---|---|---|
| 1 | **thunderdome / SlotArena** | "(c) no adoption now" — six-signal sweep found zero current need; hand-roll parked (R12 §0/§5). Earlier: Tier E3 "REJECTED" (S1 §6). | **Adopt now** as forward-looking infrastructure; landed behind off-by-default `slot-arena` feature (`a857cd71a`, local). **E3's rejection is superseded at the operator layer**; its *analysis* (no current consumer) stands. | P86 names the first real consumer (GPU texture/channel lifecycle). If P86's trigger never materializes, the feature stays dormant at zero default-build cost — the bet costs nothing to lose. |
| 2 | **RGBA packing** | CPU angle REJECTED (R9/E1 — stands untouched); GPU angle conditionally positive (R13). | Pursue the GPU angle concretely, **fused with slot-arena** (item A → P86). | R13's own gates: value activates only with multi-channel state / sub-cell sampling / WebGL2 GPU compute. P86's DoD binds to those. |
| 3 | **Field eigenmodes** | DCT/FFT, explicitly **NOT** `spectral.rs` — "for a regular Neumann grid the eigenmodes ARE the DCT basis; `spectral.rs` eigensolves graph `+(D−A)`, opposite sign, different domain" (R14 B3; R15 concurs). | **Use kernel `spectral.rs`**, rejecting the FFT/DCT spectral-ocean approach (item D → P89). Operator: *"if metrics show I'm wrong I'll admit it."* | **P89's T1–T3 reconciliation tests + the head-to-head bench (§4.4)** — the sign/domain mismatch is the FIRST thing the tests must resolve; the modal-vs-DCT cost/accuracy comparison is the named admit-or-confirm metric. |
| 4 | **Atomicity in GPU/physics** | "Field step needs ZERO atomics — adding them would be a bug"; atomics only at reductions/scatters, evidence-gated (R14 B1; Performance Standing Rule). | **Atomicity treated as a DEFAULT requirement in the physics/GPU domain**, not gated on measured contention (item C → P88). CPU-kernel domain stays under the evidence-gated standing rule (E4/E12 stand) unless the operator says otherwise. | P88 §4.3 reconciles: default = atomic; exemption ONLY by structural single-writer proof (which ping-pong provides for the stencil). The absent R17 contention bench folds into the CPU boundary when it lands. |
| 5 | **Ping-pong bit-depth** | Research assumed full-float `RG32F`/`RGBA32F` state (R13 §5, R16 §4.2). | **Realize the ping-pong pattern at minimal bit-depth (2-bit), not 32-bit float** (item B → P87). | P87 §4.2 gives the technically sound reading (2-bit state-mask plane riding beside float physics — NOT 2-bit physics) + a measured-win DoD. If the mask shows no measured win, the plane is dropped and the record says so. |
| 6 | **Quantizing money/determinism oracle** | Never proposed by research; R14 B2 and R15 §3 both name it a hard wall. | Operator asked for it; **DECLINED by the lead session** as a red-line (§5). Alternative offered. | N/A — excluded, not bet. |

**Standing-rule citation drift (housekeeping, noted honestly):** S1 and R14 cite the Performance
Standing Rule at `.claude/CLAUDE.md:182-195`; the current `.claude/CLAUDE.md` no longer contains
that text (the file was rewritten under the 2026-07-15 suspension directives). The rule's
substance — bench-gated rewrites, no blanket application, criterion bench per claimed win — is
preserved in S1's preamble and remains the CPU-domain governing text for this synthesis. Row 4
above is the operator's explicit, informed, domain-scoped exception to it.

---

## 3. Tier A extension — A4: the NTT hook-bypass (BLOCKING process violation, not a completed item)

**This is the one live emergency of the pass.** The NTT itself is the strongest technical
artifact of the day; its *process* state is RED, and the two must not be conflated.

**Facts (all re-verified live this pass, not taken from the report):**

- Commit `986646a` (`feat(pq_kem): re-derive correct ML-KEM-768 NTT alongside schoolbook (NOT
  wired), exhaustively proven bit-identical`) sits at HEAD of `/root/bebop-repo` branch
  `perf/bus-contention-2026-07-18`, **no upstream set (unpushed)**.
- bebop-repo's `.git/hooks/pre-commit` is live and enforces FIVE gates: doc-claim verification,
  falsifiable-proof guardrail, logic-gate, law-hooks, and **`scripts/three-model-review.sh` —
  the mandatory builder ≠ reviewer ≠ overlap peer review, an operator standing rule
  (2026-07-11) created precisely because a single agent self-certifying crypto produced the
  §A.3.1 Poly1305 false-green**.
- `.review/` contains **no attestation for `986646a`** — the newest findings files are dated
  2026-07-11 (Ed25519/sign work). The commit was made with `git commit --no-verify`, bypassing
  all five gates.

**Why this is Tier A ("we think we're protected but we're not"):** A1/A2 in S1 were gates that
*silently failed to run*. A4 is worse in kind — a gate that was *deliberately bypassed* on the
exact surface class (red-line crypto) it exists to protect. The exhaustive 0/65 536 proof is
strong evidence the code is right, but "the builder's own tests are green" is precisely the
false-green pattern the 3-model rule answers: reviewer independence, not test volume, is the
control. The crypto-safe-first precedent (`crypto-safe-first-pass-2026-07-14`, B4's genuine
independent review finding a real SSR-2020 forgery the builder missed) shows the gate has
non-hypothetical value on this codebase.

**Status labels (binding for all citations of R11):**
- **Technical status: GREEN** — exhaustive bit-identity proof, non-wired, schoolbook path
  unchanged, 11/11 tests pass.
- **Process status: RED / OPEN VIOLATION** — until P85 closes it, R11 is a *blocked* item, never
  a *completed* one. The S1 Tier D-9 wire-in trigger gains a third precondition: P82 bench
  evidence AND operator sign-off AND **P85 remediation complete**.

**Remediation = blueprint P85 (§4.1).** Quarantine until then: no wiring `poly_mul_ntt` into
`keygen`/`encaps`/`decaps`, no Montgomery layer, no dependent work on `986646a`.

---

## 4. New blueprint units P85–P89

Numbering continues from S1's P83. **P84 remains reserved** for the D-1 golden state-digest gate
(S1 §4, operator-gated — not proposed until ruled). All five units below are written to be
independently buildable against the 20-point standard.

### 4.1 P85 — NTT red-line process remediation (bebop-repo, BLOCKING)

**Scope.** Close the A4 violation by restoring exactly the protection that was bypassed:

1. **Re-run the skipped deterministic gates** against the `986646a` tree: `verify-doc-claims`,
   `guardrail-falsifiable-proof`, `logic-gate`, `law-hooks` — all must pass (or their failures
   fixed in a follow-up commit that itself goes through the hooks).
2. **Execute the 3-model review for real** (not pro-forma): `three-model-review.sh prepare`
   with the builder identity (Opus), then a genuinely independent reviewer attestation and a
   third overlap attestation over the NTT diff, dropped into `.review/` per the script's
   contract. The B4 precedent sets the bar: the reviewer should actively try to break it
   (e.g., adversarial inputs to `basemul_kem`, ζ-table edge indices, i64 bound re-derivation),
   not restate the builder's tests.
3. **OR, failing 1–2:** escalate to the operator for explicit retroactive sign-off, recorded in
   `docs/design/ESCALATIONS.md` — the only other sanctioned exit for a red-line gate.
4. Record the resolution in the bebop regression/escalation ledger either way, so the bypass has
   a permanent, findable paper trail.

**Also in scope (carried flag, not fixed here):** R11 §1.2's kernel finding — dowiz
`kernel/src/pq/kem.rs` implements a cyclic (not negacyclic) ring with η1=3 while claiming
ML-KEM-768. That is its own red-line lane needing its own review + sign-off; P85 files it as a
flagged successor item (P-number when ruled), it does NOT ride this remediation.

**DoD.** `.review/` contains valid reviewer + overlap attestations for the NTT diff (or a
recorded operator sign-off); all four deterministic gates green on the tree; the D-9 quarantine
lifts. **Depends on:** nothing. **Blocks:** D-9 wire-in, any Montgomery follow-up.

### 4.2 P86 — SlotArena × texture-channel lifecycle fusion (operator item A)

**The direction.** The operator states thunderdome's slot-arena logic is "fully applicable" to
the RGB/texture-packing work. This unit makes that concrete rather than asserted — and it is a
genuine fit, not a forced one: **GPU resource pools are the textbook home of generational
indices** (wgpu-core itself keys resources by index+epoch ids internally), and R13's multi-channel
future is exactly a pool of live entries with per-element removal and cross-references — the
first real instance of R12 §5's named trigger class.

**Concrete synthesis — what the fusion IS:**

1. **A CPU-side `ChannelLease` registry in the engine.** When field state grows multi-channel
   (R13 §5: `(vx,vy,p,ρ)` in `RGBA32F`, coefficient texels `(Γ,c²,M,S)`, complex `(re,im)` in
   `RG32F`), each logical scalar field leases a *(texture, channel)* slot:
   `SlotArena<ChannelAlloc>` with `ChannelAlloc { texture: TextureId, channel: u8, pair: PairId,
   format }`, handle = the existing 8-byte generational `Handle`. Freeing a field's channel and
   re-leasing it to a new field is precisely the ABA hazard slot_arena defeats: a stale system
   still holding the old handle gets a safe `None`, never a silent sample of someone else's
   scalar riding the recycled channel.
2. **Ping-pong pair lifecycle through the same arena.** Texture pairs (R16's rule: one pair per
   independently-evolved texture) are expensive to create; a `SlotArena<PingPongPair>` pool
   recycles them. Generation bump on release makes use-after-release of a recycled texture pair
   unrepresentable, and a generation mismatch is the exact signal that dependent bind groups
   must be rebuilt (stale-descriptor invalidation by construction).
3. **Handles never cross into WGSL.** Shaders receive concrete bindings resolved at
   encode time; the arena is CPU-side bookkeeping only. This keeps determinism (allocation
   order is deterministic CPU code) and honors the "engine consumes, never re-derives" contract
   style (FE-07).

**Honesty note.** R12's analysis ("no current consumer") stands — this unit is the consumer
being designed. It is gated behind the same P38 §4.2 operator decision as all GPU field-state
work; until multi-channel GPU state exists, P86 is a written design + the `slot-arena` feature
staying dormant.

**DoD.** Registry module (feature-gated `slot-arena` + `gpu`); tests: stale channel lease →
`None`; recycled pair generation-bump invalidates old handles (ABA); lease/free churn
microbench (safety infrastructure — the bench documents cost, it does not justify existence);
R16's shared-vs-separate-pair rule encoded as the pair-allocation API's type distinction.
**Depends on:** P38 §4.2 (build), R13/R16 rules (design), nothing for the writing.

### 4.3 P87 — Minimal-bit-depth ping-pong companion state (operator item B)

**The direction.** The operator wants the ping-pong pattern realized at SMALL bit-depth (2 bits
named) rather than the `RGBA32F`/`RG32F` full-float packing the research assumed.

**The technically sound interpretation (stated explicitly, per the task's demand):** 2-bit
**physics** state is physically meaningless — a wave amplitude at 4 levels cannot carry the PDE
(the CFL/energy math and the bit-deterministic oracle collapse), and no reading of the
direction should force that. The coherent realization is a **2-bit-per-cell STATE-MASK plane
riding alongside the float physics state**: a packed `u32` buffer/`R32Uint` texture (16 cells
per word, 1/16 the float plane's bandwidth) whose 2 bits per cell encode lifecycle flags, e.g.
`{settled, active, source, invalid}`. Three real jobs, each already latent in the design:

1. **Settle-mask → lazy evaluation (FE-14/G5).** R14 B3 showed the near-settle field is
   dominated by a few slow modes; cells marked `settled` can skip stencil work entirely.
   Determinism split: on the GPU presentation path, skip freely (already non-bit-identical by
   canon); on the CPU authority path, skip ONLY cells whose update is provably bit-identical to
   not skipping (conservative mask: cell + neighborhood delta exactly 0.0) — otherwise the
   oracle dies. The blueprint decides which leg ships first; both are named here.
2. **Validity bit for the GPU shadow-frame gate** (R16 §4.3): cells failing the
   energy/Lyapunov tolerance are marked `invalid` in the mask, and an invalid region falls back
   to the CPU-authority values — the shadow-*comparison* pattern at per-cell grain.
3. **Swap-parity bookkeeping.** The "which buffer is current" tag the operator's phrasing also
   admits is real but tiny — a per-pair scalar, not a plane; P86's `PingPongPair` carries it as
   a field. Recorded so the interpretation space is honestly covered.

**Cadence rule (from R16 §4.2, applied):** the mask is updated by the same step at the same
cadence as the float field ⇒ it **shares the float pair's swap** (same stencil + cadence +
stability ⇒ shared pair). It is a channel-plane of the field's ping-pong unit, not an
independently-evolved texture.

**The precision ladder this fixes in canon (reconciling row 5 of §2):** *float (f32) for
physics authority · f16/fixed-point for GPU presentation state (R14 B2, unchanged) · 2-bit for
per-cell lifecycle flags.* The operator's low-bit direction and the research's full-float
assumption are both right — at different layers.

**DoD (falsifiable).** A measured bandwidth/frame-time win from mask-skip on large grids
(P81's engine bench harness is the substrate — grid-swept {128², 256², 512²} with settle
fractions {0%, 50%, 90%}); CPU-leg skip proven bit-identical to no-skip against the `compose()`
oracle (or the CPU leg explicitly dropped with the measurement recorded); mask plane costs
≤ 1/16 the float plane's memory (asserted). **If no measured win, the plane is cut and the
negative result logged in §6-style.** **Depends on:** P81 (bench substrate), P86 (the mask
plane leases a channel slot), P38 §4.2 for the GPU leg.

### 4.4 P88 — Atomicity-by-default in the physics/GPU domain (operator item C)

**The direction.** The operator overrules R14 B1's "field step needs zero atomics" *as a
gating philosophy*: in the physics/GPU domain, atomicity is a DEFAULT requirement, not a
concession granted only after measured contention.

**The reconciliation (this is the load-bearing design move).** Stated as a rule the stencil
can actually live under:

> **Every cross-invocation shared-write site in physics/GPU code is atomic BY DEFAULT.
> Exemption is granted only by a STRUCTURAL PROOF of single-writer disjointness, recorded at
> the site.**

Under this rule the pure-gather stencil step does not get pessimal per-cell atomics — but not
because "no bench showed contention" (the evidence-gated framing the operator rejected here);
rather because ping-pong makes every cell single-writer *by construction*, and that proof
obligation is now explicit and reviewable instead of assumed. R14's "adding atomics to the
stencil would be a bug" survives as the exemption's justification; its evidence-gated default
does not. Concretely:

1. **Reductions are atomic-and-deterministic from day one.** The energy/Lyapunov reduction (the
   first real GPU-atomics site, R14 B1.2) ships as the two-level workgroup pattern with
   **fixed-point `i32` `atomicAdd`** — integer add is associative, so the sum is
   order-independent and reproducible. This is where P87's quantization theme *buys*
   correctness: fixed-point is the mechanism that makes an atomic reduction deterministic.
   Float `atomicAdd` is banned in this domain outright (non-associative + arbitrary order =
   non-deterministic), not merely discouraged.
2. **Scatter passes (future particle→grid, spatial hash) are born atomic** — no "probably
   race-free" first drafts.
3. **A WGSL shared-write checklist** (per-shader review artifact): every `var<storage,
   read_write>` and `var<workgroup>` write is either (a) atomic, (b) barrier-separated, or
   (c) carries a written single-writer proof. This is the domain's standing review gate.

**Domain boundary (explicit, per the operator's scoping):** this default governs the
physics/GPU domain only. The CPU kernel domain — the domain the Performance Standing Rule was
written for — stays evidence-gated: E4 (SeqCst→Relaxed declined) and E12 (Mutex→CAS
bench-first) stand unchanged. **When R17 (contention-bench results) lands, its data folds into
this boundary** — it can move specific CPU sites, it cannot dissolve the GPU-domain default.

**DoD.** The checklist exists and is applied to the first landed WGSL compute shader; the
deterministic fixed-point energy reduction is implemented with a reproducibility test
(N runs, bit-identical sums) the moment the GPU port lands; a contention/throughput microbench
accompanies each atomic site (documenting cost — the standing rule's bench-per-claim survives
as *measurement*, no longer as *permission*). **Depends on:** nothing for the policy text;
P38 §4.2 for the implementation legs. **Should be written FIRST among the physics units — it
constrains P86/P87 shader design.**

### 4.5 P89 — Field eigenmodes via kernel `spectral.rs` (operator item D — the falsifiable bet)

**The direction.** The field engine's modal/eigenvector needs (FE-10 impulse response, G5
settle-region truncation, any future modal solve) are to be realized by calling the EXISTING
kernel `spectral.rs` infrastructure — explicitly REJECTING the FFT/DCT spectral-ocean approach
R14 B3 and R15 converged on. §2 row 3 records the disagreement; this unit builds the
operator's way and names the metric that settles it.

**The FIRST thing the tests must resolve — the sign/domain reconciliation (RED→GREEN before
anything else):** the reports' objection is real: `spectral.rs::laplacian` builds graph
`+(D−A)` (PSD); the field stencil applies `−(D−A)` (negative-definite, TORVALDS-21,
`field_frame.rs:101-129`). But the objection is also *reconcilable in principle*: a regular
Neumann grid IS a lattice graph, so the two operators share **identical eigenvectors** with
**negated eigenvalues** (λ_field = −λ_graph). Whether that reconciliation holds *numerically
and cheaply enough* is exactly the bet. Tests, in order:

- **T1 (eigenvector identity):** build the N×N grid's graph Laplacian via the existing
  `spectral_laplacian.rs`/`Csr` path; run `topk_symmetric` for the first r modes; assert
  |⟨φ_k^spectral, φ_k^DCT⟩| ≥ 1−1e-6 against the analytic separable-cosine (DCT) modes,
  handling degenerate-eigenvalue subspaces by subspace angle, not vector match.
- **T2 (eigenvalue map):** λ_k^graph matches the analytic 2(2−cos(πp/N)−cos(πq/N)) values
  within tolerance, and the field-side modal advance uses −λ_k with the damped-oscillator
  closed form.
- **T3 (evolution equivalence):** an r-mode modal advance of a smooth initial field matches
  the stencil `step()` evolution within the E1 energy-gate tolerance over M steps.

If T1–T3 go green, the operator's path is *correct*; the remaining question is *cost*, which is:

**The head-to-head that proves or disproves the bet (the operator's own exit condition):**
three implementations of the same modal job, benched in P81's harness:

| Path | Precompute | Per-frame (r modes, n cells) | Domain generality |
|---|---|---|---|
| A. `spectral.rs` `topk_symmetric` modes | O(iters·nnz·r) once per grid/domain | O(r·n) reconstruction + O(r) advance | **Any domain** (masked/irregular grids included) |
| B. DCT/FFT (research's pick) | none (basis is analytic) | O(n log n) full-spectrum advance | Rectangular Neumann grids ONLY |
| C. Stencil `step()` (baseline/oracle) | none | O(n) per step, but ×steps | Any domain; the authority |

Named falsifiable outcome: **for the two designed homes — FE-10 sparse-impulse response and G5
near-settle advance, where r ≤ ~16 — path A meeting frame budget and E1 tolerance confirms the
bet for those homes. For full-field evolution (all n modes), B wins by construction
(O(n log n) vs O(r·n) with r→n) and the bench is expected to show it — that expected loss is
recorded up front, and if it materializes the operator's stated exit applies.** The bench
numbers, not this document, make the call.

**The honest steelman of the operator's bet (undersold by the research, recorded because it is
true):** the DCT argument holds ONLY for a perfect rectangular Neumann grid. The moment the
field domain is masked or shaped — SDF-carved regions, widget-graph-coupled fields, obstacles,
all plausible in this UI — the analytic DCT basis is simply wrong, and a numerical eigensolve
of the actual domain's Laplacian becomes the only exact modal method. `spectral.rs` is the
domain-general path. Two further architectural points in its favor: the Phase-28 ruling that
`spectral.rs` stays the SINGLE eigen surface (a DCT module would create a second spectral
authority), and `topk_symmetric`'s fixed-iteration/fixed-seed/fixed-order determinism
discipline, which a hand-rolled FFT would have to re-earn.

**Coordination.** Uses P79-B6's contiguous evec flatten (the k·n buffer feeds the FE-07 bridge
contract "flat f64 array, zero eigen-math in the engine"); CPU-precompute-then-consume per the
DyRT pattern R15 §2a confirmed — no GPU eigensolve, so P89 is NOT gated on P38 §4.2.
**DoD:** T1–T3 green; the three-path bench table filled with measured numbers; a written
verdict paragraph citing the numbers (either direction). **Depends on:** P79 (evec layout),
P81 (bench substrate), P75 (bench schema).

---

## 5. EXCLUDED scope — money/determinism quantization (declined, with reasons)

The operator also asked for quantization applied literally inside `kernel/src/money.rs` and the
CPU determinism/crypto oracle. **This was DECLINED by the lead operator-facing session** (not by
this synthesis) as a genuine safety/correctness red-line, and is recorded here so the exclusion
is explicit and citable:

- **`money.rs` is `i64` exact minor-units arithmetic** — a load-bearing invariant this same
  operator established and repeatedly reinforced this session (and which the type system,
  `money_guard`, the test-integrity red-lines, and R15 §3's "absolute wall" all encode).
  Quantization is by nature lossy/float-adjacent; applied to money it is not a perf tradeoff
  but a correctness violation by construction.
- **The CPU oracle (compose(), FSM/order projections, crypto verify) is a bit-exact determinism
  contract** — the authority every approximate path (GPU presentation, f16 state, modal
  truncation) is *measured against*. Quantizing the oracle destroys the reference that makes
  every other quantization in this document safe to attempt.
- Where quantization DOES belong is already in-plan and unaffected: GPU presentation state
  (f16/fixed-point, R14 B2), the deterministic fixed-point reduction (P88), and the 2-bit mask
  plane (P87) — all strictly on the presentation side of the P38 honesty split.

**Offered alternative (available, not designed here):** an isolated, feature-flagged
experimental quantization branch that never touches the real money/crypto path — parallel
quantized shadow computations compared against the exact authority, for the operator to test
the hypothesis safely. If wanted, it gets its own blueprint number on request; it is
deliberately NOT specified further in this pass.

---

## 6. Wave / dependency order (P85–P89 alongside the standing P75–P83 plan)

S1's waves W0–W3 are unchanged. New units slot in as follows:

| Wave | Units | Rationale |
|---|---|---|
| **W0 (immediate)** | P75, P76 (standing) **+ P85** | P85 is protection-machinery work, cheap, and BLOCKING (quarantines D-9 and any NTT follow-on until closed). bebop-repo lane — sequence with P76/P78 to avoid CI churn overlap, but do not wait for them. |
| **W1–W3 (standing)** | P77/P78/P79 · P80/P81/P82 · P83 | Unchanged. P79-B6 and P81 are named prerequisites of P89; P81 also substrates P87's DoD. |
| **W4 (physics-vision, CPU-side — after P79+P81)** | **P88 (write first)** ∥ **P89** | P88 is policy + spec: written first because it constrains all later WGSL design; its implementation legs wait for §4.2. P89 is pure CPU (DyRT pattern) — buildable now, fully parallel with P88's writing; its T1–T3 + bench deliver the §2-row-3 verdict data. |
| **W5 (GPU port wave — GATED on the P38 §4.2 operator decision)** | **P86 → P87** | P86 (lease registry + pair pool) lands with the first multi-channel GPU field state; P87's mask plane leases through it. Both inherit P88's atomicity rule and R16's pair-sharing rule. Nothing in W5 starts until the operator takes §4.2. |
| **Reserved / pending** | **P84** (D-1 golden digest — awaiting ruling) · **R17 fold-in** (contention bench — feeds P88's CPU boundary + E12 when it lands) | Named so neither is silently dropped. |

**Single-owner contracts (extending S1's list):** the atomicity-exemption proof format and WGSL
checklist → P88; the channel-lease/pair-pool API → P86; the precision ladder
(f32 authority / f16 presentation / 2-bit flags) → P87; the modal-vs-DCT verdict → P89's bench,
no other document may pre-empt it. **Push hygiene:** two verified-local-only commits exist
(`a857cd71a` dowiz main; `986646a` bebop branch) — per the worktree/remote-push precedent,
pushing after milestones is standing guidance; flagged for the operator-facing session, not
performed by this research pass.

---

## 7. Updated rejection/override log (delta to S1 §6 — cite, don't re-derive)

| # | Item | New status | Reason |
|---|---|---|---|
| E1′ | RGBA-packing generalization (CPU) | REJECTION STANDS — **scope clarified** | R9's CPU verdict untouched; R13 shows the GPU-compute domain is a different question with a conditional-positive answer. E1 must not be cited against P86/GPU work. |
| E3′ | thunderdome adoption | **OVERRIDDEN by operator** (R12 §6) | Integrated behind off-by-default `slot-arena` (`a857cd71a`, local). Analysis ("no current need") stands; verdict reversed at the operator layer; P86 designs the first consumer. |
| D9′ | pq_kem NTT wire-in | Trigger UPDATED | Now requires P82 bench evidence AND operator sign-off AND **P85 remediation complete** (§3). Implementation exists and is exhaustively proven — process-quarantined, not technically blocked. |
| NEW | Kernel `pq/kem.rs` wrong ring (cyclic, η1=3) | FLAGGED, own red-line lane | R11 §1.2. Internally self-consistent but not ML-KEM-768/FIPS-203. Needs its own review + sign-off; carried by P85 as a flag, not fixed there. |
| NEW | 2-bit *physics* state | REJECTED (physically meaningless) | §4.3: a 4-level amplitude cannot carry the PDE or the oracle. The direction's sound realization is the 2-bit flag/mask plane; this row exists so no future pass "completes" the literal reading. |
| NEW | Float `atomicAdd` in physics/GPU reductions | BANNED in-domain | §4.4 / R14 B1: non-associative + arbitrary order = non-deterministic; fixed-point `i32` atomics or fixed-order trees only. |
| NEW | Money/oracle quantization | **EXCLUDED (red-line, declined)** | §5. Safe experimental alternative available on request. |

---

*Cross-references: `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` (foundation — tiers, P75–P83,
§6 rejections) · `docs/research/OPUS-PERF-NTT-IMPLEMENTATION-2026-07-18.md` ·
`OPUS-PERF-ARENA-DEEPDIVE-2026-07-18.md` (§6 override record) ·
`OPUS-PERF-RGB-GPU-TEXTURE-PACKING-2026-07-18.md` ·
`OPUS-PHYSICS-WAVE-ALGORITHMS-AND-TENSOR-SYNTHESIS-2026-07-18.md` ·
`OPUS-SPECTRAL-EVERYWHERE-SWEEP-2026-07-18.md` ·
`OPUS-PINGPONG-SHADOW-COPY-PROPAGATION-2026-07-18.md` ·
`OPUS-PERF-CONTENTION-BENCH-RESULTS-2026-07-18.md` (ABSENT — fold-in obligation §0-R17) ·
`BLUEPRINT-P38-webgpu-render-engine.md` (§4.2 operator-owned GPU decision, FE-16 WebGL2 floor)
· `CORE-ROADMAP-STANDARD-2026-07-17.md` (blueprint contract) · bebop-repo
`.git/hooks/pre-commit` + `scripts/three-model-review.sh` (the bypassed gate, §3) · memory:
`performance-priority-over-minimal-change-2026-07-17.md`, `crypto-safe-first-pass-2026-07-14.md`
(B4 independent-review precedent), `worktree-remote-push-collision-avoidance-2026-07-18.md`.*
