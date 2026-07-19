# SYNTHESIS — Equations Knowledge Base, 2026-07-19

> **What this is.** The single reconciling pass over the four prior docs in this directory —
> `EQUATIONS-LIBRARY-2026-07-19.md` (equation lookup), `TOPICS-INDEX-2026-07-19.md` (theme
> enumeration), and the three independent Opus research passes `RESEARCH-A` (physics / thermo /
> eigenvector / `Option<T>`), `RESEARCH-B` (topic-fit triage across ~53 reference-doc concepts), and
> `RESEARCH-C` (living-interface GPU-neural-field arc status + roadmap wiring). It exists so the
> downstream blueprint-writing pass reads **only this file**, not all four inputs.
>
> **Operator standing instruction honored:** *"I'll review carefully everything from you — so any
> misalignment or missing is better not to try and do exactly as I said."* This pass therefore
> surfaces misalignment rather than smoothing it (§4), does not manufacture gaps (§3), and cites
> everything to `file:line` read live against the working tree of this worktree
> (`research/equations-thermo-eigenvector-2026-07-19`, off `main`).
>
> **Headline.** Three independent passes converged on the same conclusion, which also matches the
> same-day `ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-19.md`: **there is no large hidden gap backlog.** The
> reference corpus is overwhelmingly *already implemented* or *deliberately out of scope*. The real
> output is **4 small, honest, actionable items** (§2), a set of corrections to prior docs (§1, now
> applied), and one carefully-bounded verdict on the operator's physics framing (§4).

---

## §1 — Corrections to prior docs (APPLIED this pass, not just listed)

I applied these directly to `EQUATIONS-LIBRARY-2026-07-19.md` with `Edit` (judged cleaner than a
patch spec: that file bills itself as *"a lookup, not a re-derivation"*, and a lookup table carrying
false "not found" rows is an active hazard for the blueprint pass it feeds). Each correction is
tagged `CORRECTED 2026-07-19` in-place. Recorded here verbatim so this synthesis is self-documenting.

### 1.1 §8 false negatives — softmax and cross-entropy ARE implemented

The prior pass grepped only `retrieval/*.rs` and declared softmax / cross-entropy / euclidean "not
found anywhere in the current kernel." Two of the three are false negatives (Research A §1.1/§1.4,
Research B §D5):

- **Softmax** — IMPLEMENTED: `kernel/src/attention.rs:22-36` (`pub fn softmax`, row-max-stable, fixed
  summation order, documented bit-reproducible native/wasm), plus SIMD batch at `kernel/src/simd.rs:36,164`
  (asserted bit-identical to scalar). It is a shipped organ (`lib.rs:147-149`).
- **Cross-entropy** — IMPLEMENTED as the log-loss of `kernel/src/online.rs`'s `NaturalLogistic`
  learner: sigmoid at `online.rs:144`, "gradient of the log-loss is the prediction error `(y−p)`" at
  `online.rs:167-177`.
- **Cosine similarity** — also IMPLEMENTED (`kernel/src/leak_gate.rs:28-47`), not flagged by the
  prior pass at all. Role matters: it is a **dedup / anti-contamination gate** (reject a minted
  instance too close to a held one, threshold `0.9`), **not** a retrieval ranker. Do not blur this.
- **Euclidean / L2 distance** — the one genuine absence, and **intentional**: the retrieval layer is
  declared "vectorless, deterministic" (`retrieval/mod.rs:1`); ranking is PPR-diffusion (hop-distance)
  + BM25/trigram fusion (`retrieval/diffusion.rs:274`, `retrieval/index.rs:80`). Adding an L2 metric —
  or a softmax over it — would break the layer's bitwise-determinism guarantee (PPR mass already sums
  to 1 without normalization, `ppr.rs:15-16`). This resolves EQ-LIB §8's open "(a) gap or (b)
  intentionally-absent?" question to **(b) intentionally absent**.

### 1.2 §2 false positive — `geo::…::observed()` does NOT return `None`

The prior pass cited `geo::LiveSpeed::observed()` returning `None` "before the first accepted ping"
as the closest analogue to "thermodynamic state undefined until observed." **This is wrong on every
particular** (Research A §2.4, re-verified against `geo.rs:205-263` this pass):

- There is **no `LiveSpeed` type**. The method is `CourierSpeedEma::observed(&self)` at
  `geo.rs:261-263`, and it returns `Some((self.v_hat, self.pings))` **unconditionally** — a cold
  smoother returns `Some((0.0, 0))`, never `None`. It is a **saturating sentinel**, the exact
  *opposite* of "undefined-until-observed," and belongs in the same category as the Wilson `n=0` case.
- The `Option<(f64,u32)>` the prior pass actually saw is a *different* construct: the function
  **parameter** `live_speed_mps: Option<(f64,u32)>` of `eta_seconds_adaptive` at `geo.rs:213` (a
  refinement over Research A, which called it a "struct field" — `CourierSpeedEma` has no such field).
- **Drop the "thermodynamic state undefined until observed" analogy entirely** — no code in `geo.rs`
  supports it. The `Option<T>` characterization otherwise holds, but it is **bimodal** (Research A §2.6):
  (1) *structurally-undefined* (`dominant_period` → `None` iff no oscillatory mode exists) and
  (2) *resource-exhaustion fallback* (`charpoly_in`, `personalized_pagerank_in` → `None` iff the
  bump-arena can't serve transients; caller degrades to heap). Neither is generic nullability.

### 1.3 §3 overreach — the Lyapunov guards are NOT an implementation of `F=−∇U`

The prior pass called `noether::lyapunov_nonincreasing` + the `SelfAdaptator` `Σx²` guard + `markov`
"dowiz's actual, already-battle-tested implementation of `F=−∇U` … on THREE surfaces." Research A §4.2
corrected this and I confirm: these guards **certify the downhill *consequence*, not the gradient
relation.** They check a *supplied* potential `V` is non-increasing along a *supplied* update `f`;
there is no `∇` and no requirement that `f = −∇V`. A plain contraction `x↦0.9x` (not a gradient flow
of the energy it checks) also passes the test (`noether.rs:153-191`), so "V non-increasing" is a
**necessary-not-sufficient** signature. The module's own HONESTY NOTE (`noether.rs:17-24`) pre-emptively
refuses exactly this over-reading. The genuine `F=−∇U` correspondence lives at the **Laplacian layer**
only (§4). Also softened: the Wilson `n=0` sentinel is the codebase's *consistent* convention, not
"the one place it explicitly rejected `Option`" (the source documents *what* it returns, not a weighed
rejection of `Option`).

### 1.4 (No edits required to RESEARCH-A/B/C or TOPICS-INDEX)

Those four are internally consistent with each other; the only stale artifact was the EQ-LIB, now
fixed. RESEARCH-C's Task-4 roadmap edits remain *proposals* (see §2 item 4).

---

## §2 — Prioritized actionable list (4 items; the doc-correction is already discharged in §1)

All three passes independently produced a near-empty gap list. After RICE-style filtering
(Research B §G), exactly four items clear the bar for a downstream blueprint, in priority order.
None is invented; each traces to a specific research finding.

### Item 1 — Information-gain primitive `H(prior) − H(posterior)` — *the one genuinely useful missing quantity*
- **Scope (1 line):** add a deterministic entropy-reduction score built entirely from the entropy the
  kernel already computes; expose it as a scoring / active-selection signal, **not** as a
  retrieval-ranker replacement.
- **Touches:** `kernel/src/markov.rs` (entropy rate `H=Σᵢπᵢ·(−Σⱼpᵢⱼlog₂pᵢⱼ)` at `:179-189`) and/or
  `kernel/src/intake.rs` (DOF/entropy under-determination check at `:406-443`); a small new named fn,
  consumer TBD by the blueprint.
- **Why:** Research A §1.6 — entropy exists twice, but "information gain" (the *difference*) appears
  only as fixture text (`recall.rs:69`, `living_knowledge.rs:265`), never computed. This is the honest
  home for the operator's phrase *"information calculation gain with entropy as mismatch"* — cheap,
  deterministic, no new deps, no determinism risk. **Highest-confidence, lowest-risk build.**

### Item 2 — Route `SelfAdaptator.apply_step` through the now-existing `key_V` verdict (realize E3 Self-Harness Phase-B) — *highest leverage, operator-gated*
- **Scope (1 line):** the `SelfAdaptator` auto-apply path already fires against a real kernel knob
  (`KalmanFilter::set_q_scaler`) guarded **only** by the noether `Σx²` Lyapunov check, with *no*
  `key_V` precondition — route that apply path through the P06 signed-verdict verifier so a self-mod
  can't self-certify.
- **Touches:** `kernel/src/evals.rs:794-870` (the auto-firing `apply_step`/`propose_step`, guard at
  `:796-799`) and `tools/ci-truth/src/v1.rs:664,806` (`evaluate_gate`/`v1_verify`, the landed
  split-identity `HybridSigner`, `key_K ≠ key_V`, fail-closed).
- **Why:** Research A §3 — P06 `key_V` HybridSigner is CLOSED in code (`58987d79d`, ancestor of HEAD;
  the previously-`#[ignore]`d e2e is GREEN), so E3 Phase-B is **unblockable**. But §3.4 found the sharp
  nuance: the auto-apply substrate **already exists and is currently self-certified** — the blueprint
  itself warns this is RC-2 self-certification the moment an apply path can fire
  (`BLUEPRINT-E3-…:284-285`). So the real work is not "unlock a blocked feature" but "route an
  *already-firing* apply path through the now-legal verdict." **Highest leverage the session unlocked
  — but it modifies a self-modification path, so it is a propose-only / operator-gated change** (per
  the standing "never bypass human-gated decisions" + H4-proposal-only rule), not an auto-apply.

### Item 3 — Thin `coords_2d` / `coords_3d` eigenvector→layout wrapper (unblock R-LM FE-12) — *small, pure, low-risk*
- **Scope (1 line):** a thin embedding wrapper on top of the already-landed `spectral::eigh` /
  `topk_symmetric` that returns 2-D/3-D node coordinates from a graph Laplacian's low eigenvectors.
- **Touches:** new small helper over `kernel/src/spectral.rs:251,269,424` + `householder.rs:378-476`
  (eigenvectors, sign-fixed, Phase-28 `03ac0fefe`).
- **Why:** Research C §1b — R-LM/FE-12 named a Laplacian-eigenvector solve as "the one net-new
  primitive" the living-memory layout needed, to be vendored from bebop2. That solve **now exists
  natively in-kernel**; "**only the thin `coords_2d/coords_3d` wrapper remains**" (~90% met). No
  bebop2 vendor route needed anymore. Pure deterministic math; no GPU, no network dependency.

### Item 4 — Register the living-interface arc into MASTER-ROADMAP §20 + GROUND-TRUTH — *documentation-only, append-only*
- **Scope (1 line):** two surgical, append-only registration edits (no scope change, no new phase, no
  code) so the arc and its now-*dissolved* blockers appear in the two roadmap docs the operator reads.
- **Touches:** `docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` (append a `## 20.`
  status-ledger section, same pattern as §19) and `docs/design/GROUND-TRUTH-2026-07-17.md` (one bullet).
- **Why:** Research C §3/§4 — the `living-interface-2026-07-16/` arc name + its four new designs are
  **absent** from both docs (0 hits), though its *engine substrate* is already absorbed into DELIVERY
  P38a/P38b (FE-12 explicitly at `:966`) and it is index-rowed in `CORE-ROADMAP-INDEX.md:63`. Two
  blueprint-cited blockers have since **dissolved and must be registered, not re-planned**: (1) the
  **W21 "wgpu uncached" offline ceiling is broken** — the O18a graphics-unlock was granted, `wgpu
  30.0.0` is in `kernel/Cargo.lock`, and `kernel/src/render/gpu.rs` is a real headless bring-up (P38);
  the *engine* still carries the stale empty `gpu = []` stub, so W21 is now "consume the available
  crate in `engine/`", not "wait for network". (2) the **eigenvector solve (FE-12) now exists**
  (Item 3). Research C §4 already drafted both edits ready-to-apply. Note O18a (graphics network-gate)
  and P06 (`key_V` crypto) are **two distinct operator decisions**; both have landed.

### Discharged this pass (was a candidate item, now done)
- **EQUATIONS-LIBRARY §8/§2/§3 corrections** — applied directly in §1 above. No downstream action.

### Explicitly deferred (a real "consider" from Research A §Implications #6, but NOT committed)
- **Optional `attention::softmax` learned-affinity re-ranker** layered *on top of* PPR/BM25, realizing
  the `attention.rs:4-11` "attention = one PPR step, learned affinity" lens as a wired feature. Left
  **deferred, low-priority, and explicitly bounded**: it must never replace the deterministic PPR+BM25
  core (that would regress bit-exactness). Only pursue if a concrete re-ranking need appears; do not
  build speculatively. Recorded so it isn't re-discovered as a "gap."

---

## §3 — Explicit rejections (do NOT build; cited)

These are reference-doc concepts that sound adjacent but have **no target** in this architecture.
Rejecting them explicitly is the point — so a future reader doesn't "add the missing piece."

| Reject | Why (cited) |
|---|---|
| **Group A — embedded / MCU** (Microcontroller, GPIO, ADC, PWM `V_avg=(D/100)·Vcc`, Interrupt/ISR) | Research B §A: **100% decorative.** No firmware / `no_std` / register-map / MCU surface anywhere. Every candidate "device" file checked is not firmware (`apps/courier/src/battery.rs` is a drain *measurement gate*; `pq/codesign.rs` signs an opaque OTA blob, authors no firmware). |
| **Frequency-domain control theory — B1 LTI/convolution, B3 poles-zeros (transfer-function algebra), B5 GM/PM** | Research B §B, EQ-LIB §4: no convolution / Laplace / s-plane / Bode / Nyquist surface. The control the kernel *needs* is already covered by **state-estimation (`kalman.rs`)** and **queueing backpressure** (`token_bucket.rs` saturating P-controller, `impedance.rs` `ρ<1−margin` gate, `WorkerSlots` degrade-closed). `T+S=1` is not derived or checked against; GM/PM has no target. Building it would be for a use case that doesn't exist. |
| **E5 data-warehouse patterns** (Medallion / Lambda / Kappa / Lake / Warehouse / Lakehouse / Mesh / Fabric / Data-Vault / Modern-Streaming) | Research B §E5: presuppose a Spark/batch-stream analytics warehouse dowiz does not run. Only **Event-Driven/Event-Sourcing** maps (it *is* the kernel spine, `event_log.rs`); the rest are decorative. |
| **D7 HPO libraries** (Optuna / Ray Tune / Hyperopt / SMAC3 / …) | Research B §D7: no training loop with tunable hyperparameters; `online.rs` uses a fixed LR **by determinism requirement**. Importing an HPO lib would violate the zero-vendor-ML-runtime house rule. |
| **H1 neuroscience** (nerve-cell taxonomy) | Research B §H1: aesthetic/morphology reference for the Document-4 viz layer that was **already declined** (below). No code, no gap. |
| **C1 literal Navier–Stokes fluid sim** (`ρ(∂v⃗/∂t+(v⃗·∇)v⃗)=…`) | EQ-LIB §1, Research B: no fluid-sim surface. Only the `−∇p` *operator shape* matches the Laplacian's `−∇` pattern (§4); there is no fluid dynamics to compute. |
| **Spiking-neuron ODEs for the viz — Izhikevich / LIF / Hodgkin-Huxley / cable equation** | EQ-LIB §7, Research B §H1, Research C: **already deliberately declined** by the 2026-07-16 R-LM pass. The living-memory viz needs **no neuron-simulation layer** — positions from `L=D−A` low eigenvectors, activity from `personalized_pagerank`, health from `graph_spectrum`. Do not re-import the ODEs the prior pass already refused. |
| **Euclidean-distance / embedding-kNN retrieval** (and softmax-over-distances) | Research A §1.3/§1.5, Research B §D5: retrieval is **vectorless by design** (`retrieval/mod.rs:1`). PPR mass already sums to 1 (`ppr.rs:15-16`); a softmax step is redundant and **would break bit-determinism**. The reference's "euclidean-distance-with-softmax" describes a *different* retriever the kernel consciously rejected on determinism grounds. |

**Already-ruled rejections — do NOT re-litigate** (Research B §E2, gap-audit §3): **CRDT** (declined
for money single-writer correctness, `wallet/mod.rs:18`), **Bloom filter** (declined for
bit-reproducibility, `retrieval/index.rs:80`), **classic 3-state circuit breaker** (replaced by
fail-closed/degrade-closed + `impedance.rs`, deliberate determinism preference), **JWT/session/OAuth
token taxonomy** (replaced by capability-certificates, `capability_cert.rs`, operator ruling
2026-07-18). None is a gap; each is a stated position.

---

## §4 — Honest verdict on the operator's physics framing

> Operator's central ask: *"apply same scalar & thermodynamics equations logic stored in the
> eigenvectors and rust `Option<T>`."*

Split by layer, because it is **literally true at one place and an evocative-but-non-cashing metaphor
elsewhere** — and conflating the two is exactly the misalignment the operator asked to have surfaced.

### 4.1 Where it is LITERALLY TRUE (state without hedging)

**The graph-Laplacian / incidence layer.** `kernel/src/incidence.rs` implements the genuine discrete
`−∇` structure, not an analogy to it:
- `grad` `B x` — node-field → edge-flow, `(Bx)_e = x_head − x_tail` (`incidence.rs:73-88`);
- `div` `Bᵀ flow` — edge-flow → node-field (`incidence.rs:90-104`);
- `laplacian` `L x = div(W·grad(x)) = (BᵀWB)x` (`incidence.rs:102`).

So `L = BᵀWB` **is** `div(W·grad)`. The Dirichlet energy `E(x)=½xᵀLx` has gradient `∇E = Lx`, and a
diffusion/heat step `x ← x − ηLx` is **exactly gradient descent on that energy** — i.e. the discrete
`F=−∇U` with `U=½xᵀLx` (Research A §4.1). The `field_frame` sign convention `∇²U=−(D−A)U` vs
`csr`/`spectral`'s `+(D−A)` (`incidence.rs:8-13`) is the same `−∇` bookkeeping, parity-pinned by a
seam test. **"Rolls downhill / `F=−∇U`" is an accurate description of this layer.**

**"Stored in the eigenvectors" cashes out here too.** The living-memory layout takes node positions
from the *low eigenvectors* of exactly this `L` (Item 3 / R-LM FE-12). Those eigenvectors are the
**standing modes of the Dirichlet-energy operator** — so "the scalar-field logic is stored in the
eigenvectors" is a *true* statement about `spectral::eigh`/`topk_symmetric`: the eigenbasis of `L`
literally is the modal decomposition of the `½xᵀLx` energy the `F=−∇U` picture describes.

**Shannon entropy is a real, byte-identical cross-link** (not eigenvector-resident, but real):
`H=−Σp·log₂p` in the reference doc is the *same formula* already live at `markov.rs:179-189` (entropy
rate of the tool-outcome Markov chain), just applied to tool-call transitions. This is the single
strongest existing correspondence in the whole corpus.

### 4.2 Where it is a METAPHOR that sounds compelling but does NOT cash out (do not ship as an engineering claim)

- **"Thermodynamics equations stored in the eigenvectors" as a *unified* literal claim.** The entropy
  lives in the **Markov layer** (`markov.rs`, a stationary-distribution quantity); the eigenvectors
  live in the **spectral layer** (`spectral.rs`). They are linked only by the loose "both measure
  disorder" analogy. Graph energy `E=Σ|λᵢ|` (`spectral.rs:592-600`) is a spectral quantity but it is
  **not** thermodynamic entropy — treating `Σ|λᵢ|` and `−Σp log₂p` as the same "thermodynamics stored
  in eigenvectors" object conflates two different computations. The eigenvectors carry the *Dirichlet
  energy* structure (§4.1, real); they do **not** carry the entropy.
- **The Lyapunov guards as "`F=−∇U`"** (`noether.rs`, `evals.rs` `Σx²`, `markov` potential) —
  necessary-not-sufficient; certify the downhill *consequence*, not the gradient (see §1.3). The code's
  own HONESTY NOTE refuses this reading.
- **Fourier's law `q=−k∇T` as a thermal surface** — no heat computation exists; only the `−∇` operator
  *shape* matches. Same for **Navier–Stokes `−∇p`** (no fluid sim).
- **"Thermodynamic state undefined until observed" via `Option<T>`** — no code supports it; the cited
  `geo::observed()` returns `Some(...)` always (§1.2). `Option<T>` on this surface means
  *structurally-undefined* (`dominant_period`) or *arena-exhaustion fallback* (`charpoly_in`,
  `personalized_pagerank_in`) — both real, neither thermodynamic.

### 4.3 Net honest judgment (one sentence)

**The operator's "scalar & thermodynamics equations stored in the eigenvectors" framing is literally
and precisely true at exactly one place — the graph Laplacian `L = BᵀWB = div(W·grad)`, whose
eigenvectors are the standing modes of the Dirichlet energy `½xᵀLx` and where `x ← x−ηLx` is
genuinely `F=−∇U` — plus a real byte-identical Shannon-entropy cross-link in `markov.rs`; everywhere
else (Fourier heat, Navier-Stokes pressure, the Lyapunov guards, "state-undefined-until-observed"
`Option`) it is an evocative lens that shares the `−∇` *shape* without corresponding to any
thermodynamic computation the kernel performs, and should not be shipped as an engineering claim.**

---

## Provenance

Written 2026-07-19 in worktree `research/equations-thermo-eigenvector-2026-07-19`
(`/root/dowiz-wt-eq-thermo-gpu`), off `main`. Synthesizes `EQUATIONS-LIBRARY-2026-07-19.md`,
`TOPICS-INDEX-2026-07-19.md`, `RESEARCH-A/B/C-2026-07-19.md`, and `OPERATOR-PROMPT-VERBATIM-2026-07-19.md`
(all this directory). §1 corrections were **applied** to `EQUATIONS-LIBRARY-2026-07-19.md` this pass
(tagged `CORRECTED 2026-07-19`); the §2 items 2 and 4 (E3 routing, roadmap registration) are
propose-only / operator-gated and are **not** applied here. All `file:line` citations verified live
against the working tree, not the Repowise index. Staging deferred to the orchestrating session.
