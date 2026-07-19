# Research A — Physics / Thermo / Eigenvector ↔ Kernel (verification pass)

> Verifies-or-corrects the four load-bearing claims the prior recon pass
> (`EQUATIONS-LIBRARY-2026-07-19.md`) left flagged or asserted. Every claim below
> carries a `file:line` citation read live against the working tree at
> `/root/dowiz-wt-eq-thermo-gpu` (branch `research/equations-thermo-eigenvector-2026-07-19`,
> HEAD `5a97e1f6f`, with `58987d79d` — the P06 fix — confirmed an ancestor of HEAD).
> Verdicts are one of: **CONFIRMED** · **CORRECTED** · **GENUINE GAP** · **NOT APPLICABLE**.
>
> Headline: the prior pass's shallow grep produced **two real false-negatives**
> (softmax and cosine both DO exist in the kernel) and **one real false-positive**
> (`geo::observed` does NOT return `None`). Everything else it flagged holds up,
> often with a sharper meaning than it stated.

---

## §1 — The retrieval / softmax / cross-entropy / euclidean gap (Task 1)

The operator's closing ask: *"where data should … use information calculation gain
with entropy as mismatch on internal digital euclidean distance stabilized by
softmax equation."* The prior pass flagged Softmax, Cross-Entropy, and Euclidean
distance as "not found anywhere in the current kernel." Verified term-by-term:

### 1.1 Softmax — **CORRECTED** (prior pass wrong: it IS implemented)

Softmax is a first-class, tested kernel primitive:
- `kernel/src/attention.rs:22-36` — `pub fn softmax(xs: &[f64]) -> Vec<f64>`,
  numerically-stable (subtracts row-max before `exp`, `attention.rs:27-33`),
  fixed summation order, documented bit-reproducible native/wasm (`attention.rs:13-15`).
- `kernel/src/simd.rs:36` `softmax_scalar`, `simd.rs:66` `softmax_lane4`,
  `simd.rs:164` `softmax_batch_lane` — a SoA SIMD batch lane, asserted
  bit-identical to the scalar path (`simd.rs:437-442`).
- Referenced as a shipped organ in `kernel/src/lib.rs:147-149`.

Crucially, `attention.rs:4-11` already draws the *exact* lens the operator is
reaching for: `softmax(QKᵀ/√d)·V` is "**one step of diffusion over a LEARNED
affinity matrix** `A = softmax(QKᵀ/√d)` — exactly the `f(L)` family the kernel
already runs as fixed, multi-step diffusion in `markov` … and PPR. Attention =
one step, learned affinity; PPR = many steps, fixed affinity. Same operator
family." So "softmax-stabilized retrieval" is **already named and implemented**;
it simply is not wired into the production retrieval path (see 1.5).

### 1.2 Cosine similarity — **CORRECTED** (prior pass wrong: it IS implemented)

- `kernel/src/leak_gate.rs:28-47` — `LeakGate::cosine(a,b)`: full dot/‖a‖‖b‖ with
  a `clamp(-1,1)` and an empty/mismatch → `0.0` guard (`leak_gate.rs:29-31,43-45`).
- Consumed as the semantic near-duplicate gate at threshold `0.9`
  (`leak_gate.rs:13-14,50-54`), with tests at `evals.rs:1379-1384` and the doc
  contract at `lib.rs:128`.

Note the *role*: cosine here is a **dedup / anti-contamination** metric (reject a
minted instance too close to a held one), **not** a retrieval ranker. That is a
real distinction the synthesis pass must not blur.

### 1.3 Euclidean / L2 distance — **NOT APPLICABLE** (no named primitive; not needed)

No named euclidean-distance function exists. L2 *norms* appear inline where a
vector must be normalized or a residual measured — `spectral.rs:302,452`
(eigenvector normalization), `householder.rs:142`, `field_eigenmodes.rs:184`,
`kalman.rs:232` (`y_norm` innovation magnitude) — but none is a nearest-neighbour
distance metric, and none is in a retrieval path. The retrieval layer is declared
**"vectorless, deterministic"** at `kernel/src/retrieval/mod.rs:1`, so a
`√Σ(xᵢ−yᵢ)²` metric over an embedding space is *architecturally absent by design*,
not by omission. Verdict: **NOT APPLICABLE** — the kernel deliberately does not do
embedding-kNN retrieval, so it has no euclidean distance to stabilize.

### 1.4 Cross-entropy — **GENUINE GAP** (but almost certainly out-of-scope)

`grep -riE "cross.?entropy|nll|log_softmax"` over `kernel/src/` returns **nothing**.
This is a true absence. But it is a *learning-loss* primitive; the kernel is
explicitly non-AI ("the kernel stays non-AI (deterministic pure functions);
learning lives in `online`/`micrograd` at the edge if ever needed",
`attention.rs:17-20`). So it is a **GENUINE GAP only in the trivial sense that the
kernel chose not to be a trainer**. There is no consumer for a cross-entropy loss
in a retrieval or dynamics path. Do not import it speculatively.

### 1.5 The actual retrieval ranking path — structurally does NOT need softmax

This is the decisive finding for Task 1. The production retrieval ranker uses
**none** of softmax / euclidean / cross-entropy. It is two fused deterministic
signals:
- **BM25 + trigram lexical fusion** (`retrieval/recall.rs:133-193`,
  `recall_at_k` `:230-245`; recall@5 = 1.0 over a 12-query oracle,
  `recall.rs:350-390`) — TF-IDF scoring, no normalization-to-distribution step.
- **Personalized-PageRank diffusion** (`retrieval/ppr.rs:42-67`,
  `retrieval/diffusion.rs:133-139`) — ranks by steady-state diffusion mass.

PPR **already produces a probability distribution without a softmax**: mass is
conserved by the `(1−α)`-diffusion + `α`-restart, `Σπ = (1−α)·1 + α·1 = 1`, so the
per-step `÷ sum` normalization is *dropped on purpose* to keep the result bit-exact
(`ppr.rs:15-16`, proven by `diffusion.rs:244-251` `green_ppr_mass_conserved` and
`csr.rs:527-535` final-normalize). **A softmax over PPR scores would be redundant
and would break the bitwise-determinism guarantee** the layer is built around.

**Verdict for the operator's phrase:** the "euclidean-distance-with-softmax"
retrieval architecture describes an **embedding-vector / kNN retriever** — a
*different* retriever than the one this kernel ships. The kernel deliberately
chose vectorless PPR + BM25 (bit-reproducible, zero-dep, no float nondeterminism)
over softmax-over-distances. The softmax primitive that *does* exist
(`attention.rs`) is the "one learned-affinity diffusion step" cousin of PPR, kept
as a tested organ but **not** on the retrieval hot path. So this is **NOT a gap to
fill in the retriever** — forcing a softmax step into PPR retrieval would be a
regression, not an improvement. The only defensible "wire" (see Implications) is
exposing `attention::softmax` as an *optional* learned-affinity re-ranker layered
on top, never replacing the deterministic core.

### 1.6 Information gain — **GENUINE GAP** (entropy exists; gain does not)

Shannon entropy `H=−Σp·log₂p` is implemented twice, as the prior pass correctly
found: `markov.rs:13,179-189` (tool-outcome entropy rate `H=Σᵢπᵢ·(−Σⱼpᵢⱼlog₂pᵢⱼ)`)
and the DOF/entropy under-determination check `intake.rs:406-443`. But
**"information gain" (`H(prior) − H(posterior)`) is not implemented as a named
primitive** — the string "information gain" appears only as fixture text
(`recall.rs:69`, `living_knowledge.rs:265`), never as a computed quantity. Verdict:
**GENUINE GAP**, and this one is the most plausibly *useful* of the gaps — an
entropy-reduction score is a natural, cheap, deterministic retrieval/active-learning
signal built entirely from the entropy the kernel already computes. Flag for the
blueprint pass as the single info-theoretic primitive worth *considering* (not
softmax, not euclidean).

---

## §2 — The `Option<T>` characterization (Task 2)

Prior claim: `Option<T>` on this math surface means "this decomposition/derivation
may not converge or is structurally undefined for this input," not generic
nullability. Read the actual bodies:

### 2.1 `spectral::charpoly_in` — **CONFIRMED** (with a refinement)

`spectral.rs:145`. `None` is propagated by `matmul_contig_in(&am_mat,&m,arena)?`
— i.e. `None` ⟺ **arena exhaustion**, on which the caller falls back to the heap
`charpoly` (same bytes). Degenerate input `n==0` returns `Some(vec![1.0])` (the
trivially-defined empty-matrix charpoly), *not* `None`. So the precise meaning is
**"a bump-arena couldn't serve the transient matrices; caller must fall back"** —
a *resource-exhaustion / structured-fallback* signal, not numerical failure and not
nullability. Prior pass's line (`EQUATIONS-LIBRARY:39`) already stated this
correctly.

### 2.2 `spectral::dominant_period` — **CONFIRMED**

`spectral.rs:769-782`. `None` is returned when **no eigenvalue clears the
magnitude+argument thresholds** (`PERIOD_MAG=0.85`, `PERIOD_ARG=0.6`) — i.e. the
operator is non-oscillatory, so a "dominant period" is *structurally undefined*.
This is the purest instance of the claimed meaning: **`None` = "this quantity does
not exist for this input,"** not "value missing."

### 2.3 `Csr::personalized_pagerank_in` — **CONFIRMED**

`csr.rs:467-537`. `None` is propagated by `arena.alloc_slice(n)?`
(`csr.rs:476-478`) — again **arena exhaustion**, with a heap fallback documented at
`csr.rs:462-466`. PPR itself always converges for `α<1`, so `None` is *never*
numerical. Same structured-fallback meaning as 2.1. Prior pass correct.

### 2.4 `geo::…::observed` — **CORRECTED** (prior pass's example is wrong)

Prior pass (`EQUATIONS-LIBRARY:42`) claimed `geo::LiveSpeed::observed() -> Option`
returns `None` "before the first accepted ping … the closest analogue to
thermodynamic state undefined until observed." **This is false.** The actual method
is `CourierSpeedEma::observed(&self)` at `geo.rs:261-263`, and it returns
**`Some((self.v_hat, self.pings))` unconditionally** — a cold smoother returns
`Some((0.0, 0))`, never `None`. There is no type named `LiveSpeed`. The `Option`
the prior pass saw is a *different* construct: the struct field
`live_speed_mps: Option<(f64,u32)>` at `geo.rs:213`, which is not the return of
`observed()`. Verdict: **CORRECTED** — `observed()` uses a *saturating sentinel*
`(0.0, 0)`, the opposite of the claimed "undefined-until-observed `None`." (It is,
ironically, the same sentinel philosophy as the Wilson `n=0` case in 2.5 — so the
prior pass filed it under exactly the wrong category.) The "thermodynamic state
undefined until observed" analogy has **no** supporting example in `geo.rs`; drop it.

### 2.5 Wilson interval deliberately avoids `Option` at `n=0` — **CONFIRMED** (framing refined)

`stats.rs:100-103`: `if n == 0 { return (0.0, 1.0); }` — the maximally-uncertain
interval — documented at `stats.rs:99` and tested at
`stats.rs:284-286` (`wilson_zero_n_is_maximally_uncertain`). The sentinel is
genuinely deliberate, documented, and regression-pinned. **CONFIRMED.**

*Refinement:* the prior pass's stronger wording — that this is "the one place the
codebase explicitly *rejected* `Option<T>`" and "a documented explicit design
choice to prefer a saturating sentinel over `None`" — is an **inference, not the
code's own words**. The docstring documents *what* it returns (`(0.0,1.0)`), and
the surrounding module philosophy (a point estimate must carry a check it can't
dodge, `stats.rs:1-19`) *supports* the reading, but nowhere does the source
explicitly weigh `Option` and reject it. So: the deliberateness is real; the
"explicitly rejected `Option`" claim should be softened to "the codebase's
consistent sentinel convention" when cited.

### 2.6 Consolidated `Option<T>` finding

The characterization is **CONFIRMED but bimodal** — there are two distinct
structured meanings, not one:
1. **Structurally-undefined** (`dominant_period`): `None` ⟺ the quantity does not
   exist for this input.
2. **Resource-exhaustion fallback** (`charpoly_in`, `personalized_pagerank_in`):
   `None` ⟺ the bump-arena couldn't serve the transients; caller degrades to heap.

Neither is generic nullability, and the `?` operator threads both cleanly. The
*sentinel* convention (Wilson `n=0`, `CourierSpeedEma::observed`) is the
**deliberate counter-pattern**: where a caller can't cheaply branch on `None`, the
kernel returns a saturating "maximally-uncertain / cold" value instead. That
sentinel-vs-`Option` tension is the actually-interesting design axis to carry into
the synthesis pass — and it is a *codebase* pattern, not, as the prior pass framed
it, a one-off in `stats.rs`.

---

## §3 — E3 Self-Harness Phase-B unblock status (Task 3)

### 3.1 P06 `key_V` HybridSigner is CLOSED in code — **CONFIRMED**

Not memory-trusted — verified in the tree:
- HEAD `5a97e1f6f` (merge "took main's CLOSED HybridSigner"); `58987d79d` (the P06
  fix commit) is an ancestor of HEAD.
- The split-identity signed-verdict verifier exists **in code**, not just docs:
  `tools/ci-truth/src/v1.rs:520` `HybridSigner`, `:664` `evaluate_gate`,
  `:806` `v1_verify`. It **enforces split identity** — rejects
  `key_K == key_V (verdict signed by author key)` at `v1.rs:688` and a cross-role
  anchor at `v1.rs:756` — and is **fail-closed**: missing verdict → RED
  (`v1.rs:675`), no signature → RED (`v1.rs:783`), signature verify failure → RED
  (`v1.rs:795`). Separate `role:'K'` / `role:'V'` signers at `v1.rs:760,781`.
- Corroborated by `docs/design/ROADMAP-LIVE-STATUS-2026-07-18.md:8,45,69`: "P06
  HybridSigner COMPLETE — commit `58987d79d`"; the previously `#[ignore]`d e2e
  `real_hybrid_sig_roundtrip_and_corruption_rejected` is GREEN (signed notes verify
  via real CLI; 1-bit-flipped sig → fail-closed RED); "it is now CLOSED and unblocks
  E3-Phase-B / Layer C / G / P30."

So the memory note is accurate at the substrate level: the key_V split-identity
signed-verify path exists, is tested green, and is fail-closed.

### 3.2 The E3 blueprint's *stated* gate condition — read verbatim

`docs/design/spectral-energy-flow-evolution-2026-07-16/BLUEPRINT-E3-self-harness-loop-for-llm-harness.md`
states the gate five times; the operative DoD is `E3:282-288`:

> "Phase B cannot be STARTED … until `key_V` exists. … The DoD for *beginning*
> Phase B design is a single external precondition: `BLUEPRINT-P06`'s `key_V`
> independent re-execution path exists **in code** (fresh worktree, `key_K ≠ key_V`,
> signed verdict) …"

and `E3:165`: "Until `key_V` lands **and the harness validation is routed through
it**, Phase B stays unbuilt."

So the gate has three cryptographic conditions — (a) in-code key_V path, (b)
`key_K ≠ key_V`, (c) signed verdict — plus a **wiring** condition — (d) the harness
validation is actually routed through it. Optionally (e) "independent
re-execution … fresh worktree."

### 3.3 Verdict: Phase-B is UNBLOCKED, not DONE — **CONFIRMED (unblockable), with a scope caveat**

- Conditions (a)/(b)/(c) are **met** by the landed P06 (§3.1): an in-code
  `HybridSigner`/`evaluate_gate` that enforces `key_K ≠ key_V` and verifies a
  signed verdict, fail-closed. The blueprint's *external precondition for
  beginning Phase-B* is therefore satisfied — Phase-B is **genuinely unblockable**,
  matching the roadmap's own "unblocks E3-Phase-B" line.
- Conditions (d) and (e) are **Phase-B's own build work, not delivered by P06
  closing.** P06 gives the signing/verification *substrate*; routing the E3 harness
  validation *through* that substrate — and providing the fresh-worktree
  re-execution — is exactly what E3-Phase-B must build. "Unblocked" ≠ "done."
- Note the substrate lives in `tools/ci-truth/src/v1.rs`, a **dev-time** tool,
  consistent with the blueprint's "canonical-repo DEV-TIME scope (M5/M6)"
  (`E3:278`). It is not (yet) a kernel-resident verifier the harness calls.

### 3.4 A sharper, independently-verified nuance the memory note misses

`docs/design/ROADMAP-UPDATE-SESSION-SYNTHESIS-2026-07-18.md:159-161` records — and
I confirm against the code — that **the E3 Phase-A/Phase-B boundary was always
documentation-only; there was never a structural code gate.** The built
`SelfAdaptator` (`kernel/src/evals.rs:794-870`) already *auto-applies* to a real
kernel knob (`KalmanFilter::set_q_scaler`, referenced `evals.rs:762`) via
`propose_step`/`apply_step` guarded only by the **noether Σx² Lyapunov check**
(`evals.rs:796-799`), with **no `key_V` precondition anywhere in that code path**.

Implication (and a mild flag): the "Phase-B is blocked" story was a *governance /
DoD discipline* boundary, not a compiler-enforced one. The auto-apply substrate
partially already exists **and is currently unguarded by key_V** — which the
blueprint itself warns is RC-2 self-certification the moment an apply path can fire
(`E3:284-285`). So the real Phase-B work is less "unlock a blocked feature" and more
"route the *already-firing* `SelfAdaptator.apply_step` through the now-existing
key_V verdict instead of leaving it self-certified." That is the honest, concrete
next step — and it is now possible precisely because P06 closed.

---

## §4 — The thermodynamics / `F=−∇U` framing, judged honestly (Task 4)

The operator said "any misalignment is better not to try." So I split the claim by
layer rather than give it a single yes/no, because it is **true at one layer and a
loose lens at another**, and conflating them is the misalignment to avoid.

### 4.1 At the Laplacian / incidence layer — **CONFIRMED: a real correspondence**

`incidence.rs` implements the genuine discrete `−∇` structure:
- `grad` `B x` (node-field → edge-flow, `(Bx)_e = x_head − x_tail`) at
  `incidence.rs:73-88`;
- `div` `Bᵀ flow` (edge-flow → node-field) at `incidence.rs:90-104`;
- `laplacian` `L x = div(W·grad(x)) = (BᵀWB)x` at `incidence.rs:102-…`.

This is not a metaphor. `L = BᵀWB` is *literally* `div(W·grad)`, the discrete
divergence-of-a-weighted-gradient. The Dirichlet energy is `E(x)=½xᵀLx`, its
gradient is `∇E = Lx`, and a diffusion/heat step `x ← x − ηLx` is **exactly
gradient descent on that energy** — i.e. the discrete `F = −∇U` with `U = ½xᵀLx`.
The `field_frame` sign convention `∇²U = −(D−A)U` vs `csr`/`spectral`'s `+(D−A)`
(`incidence.rs:8-13,23-27`) is the same `−∇` bookkeeping, and the two are
parity-pinned by a seam test. **Verdict: the `F=−∇U` / "rolls downhill" narrative
is an accurate description of the Laplacian layer — quote it here without
hedging.**

### 4.2 At the Lyapunov-guard layer — **CORRECTED to "loose but defensible lens," not an implementation of `F=−∇U`**

`noether::lyapunov_nonincreasing` (`noether.rs:82-108`) checks
`V(f(x)) − V(x) ≤ tol` at every step — one-sided, rejects spontaneous growth,
accepts legitimate decay. The `SelfAdaptator` noether guard (`evals.rs:796-799`,
conserved `Σx²`) and `eval_loss = s²+r²` "squared so the Adam gradient points
downhill toward a calmer filter" (`evals.rs:779-790`) are of the same shape.

But mathematically these guards **certify the *consequence* of downhill flow, not
the gradient relation itself**:
- They verify that a *supplied* potential `V` is non-increasing along a *supplied*
  update `f`. There is **no `∇`, and no requirement that `f = −∇V`.** Many
  non-gradient dissipative systems (any contraction) also satisfy "V
  non-increasing" — the test `lyapunov_catches_growth_accepts_decay`
  (`noether.rs:153-191`) accepts a plain `x↦0.9x` contraction, which is not a
  gradient flow of the energy it checks. So "V non-increasing" is a
  **necessary-not-sufficient signature** of `F=−∇U`, not an encoding of it.
- The module says this itself. The **HONESTY NOTE** at `noether.rs:17-24`: "the
  name is a *lens* — this module checks that a supplied invariant `I` is conserved
  along a sampled trajectory. It does NOT derive conservation from symmetry … a
  passing check certifies trajectory-wise invariance, not a neighborhood stability
  proof." That is the codebase pre-emptively refusing the very over-reading the
  prior pass then made.

So the prior pass's line (`EQUATIONS-LIBRARY:51`) — "dowiz's actual, already
battle-tested implementation of `F=−∇U` … enforces as a correctness gate on THREE
surfaces (markov, noether, evals)" — is **over-reaching on the noether/evals/markov
surfaces.** Those enforce *dissipation / non-growth of a chosen Lyapunov quantity*,
which downhill flow satisfies but does not uniquely identify. Verdict for this
layer: **CORRECTED** — call it "a Lyapunov/dissipation guard that certifies the
*downhill consequence*," not "an implementation of `F=−∇U`."

### 4.3 Net honest judgment

The `F=−∇U` mapping is **apt and precise at exactly one place — the graph
Laplacian (`incidence.rs`/`csr`/`spectral`), where `L = div(grad)` is the real
gradient of the Dirichlet energy — and is a *loose, necessary-not-sufficient lens*
at the Lyapunov guards.** State it in those two tiers. Do **not** present the
Lyapunov guards as "the codebase's `F=−∇U`"; the code's own HONESTY NOTE is the
strongest evidence that would be the misalignment the operator asked us to avoid.

---

## Implications for the blueprint pass (what to build vs. leave alone)

**Leave alone (already correct — building would regress):**
1. **Retrieval ranking.** PPR + BM25 fusion is deterministic and correct; PPR mass
   already sums to 1, so a softmax step is redundant and would break bit-exactness
   (`ppr.rs:15-16`). Do not "add softmax to retrieval."
2. **Softmax / cosine.** Both already exist, tested (`attention.rs:22`,
   `leak_gate.rs:28`). Do not re-implement; *cite* them.
3. **Euclidean-kNN retrieval.** Deliberately absent (`retrieval/mod.rs:1`
   "vectorless"). Do not introduce an embedding-distance retriever to match the
   reference doc's shape — that is a different architecture the kernel rejected on
   determinism grounds.
4. **Lyapunov guards.** Correctly implemented and honestly documented
   (`noether.rs`). Do not re-label them as `F=−∇U`.

**Consider building (real, non-forced gaps):**
5. **Information-gain primitive** (`H(prior) − H(posterior)`) — the one genuinely
   useful missing info-theoretic quantity, buildable purely from the entropy the
   kernel already computes (`markov.rs:179-189`, `intake.rs:406-443`). This is the
   honest home for the operator's "information gain with entropy as mismatch"
   phrase — as a *scoring/active-selection* signal, **not** as a retrieval-ranker
   replacement.
6. **Optional learned-affinity re-ranker** exposing `attention::softmax` as a thin
   layer *on top of* PPR/BM25 (never replacing the deterministic core), realizing
   the `attention.rs:4-11` "attention = one PPR step, learned affinity" lens as a
   wired feature rather than a tested-but-unconsumed organ.
7. **E3 Phase-B routing.** P06 `key_V` is CLOSED (§3.1), so Phase-B is unblockable.
   The concrete build is: route `SelfAdaptator.apply_step` (`evals.rs`, currently
   auto-firing under only the noether guard) through the now-existing key_V signed
   verdict (`tools/ci-truth/src/v1.rs:664,806`), closing the RC-2 self-certification
   the blueprint warns about (`E3:284-285`). This is the highest-leverage, now-legal
   wiring the session unlocked.

**Corrections to propagate into the equations library / any downstream doc:**
- Softmax and cosine are IMPLEMENTED (§1.1, §1.2) — the prior "not found" flags are
  false negatives.
- `geo::observed` returns `Some(...)` always (§2.4) — the "None-until-observed"
  example is a false positive; drop the "thermodynamic state undefined until
  observed" analogy (no code supports it).
- The `F=−∇U` correspondence is real at the Laplacian layer only; it is a loose
  lens at the Lyapunov guards (§4).

---

*Provenance: written 2026-07-19 in worktree
`/root/dowiz-wt-eq-thermo-gpu` (branch `research/equations-thermo-eigenvector-2026-07-19`,
HEAD `5a97e1f6f`). All citations read live against the working tree, not the
Repowise index. Companion inputs: `EQUATIONS-LIBRARY-2026-07-19.md`,
`TOPICS-INDEX-2026-07-19.md`, `OPERATOR-PROMPT-VERBATIM-2026-07-19.md`. Not staged
by this agent — staging deferred to the orchestrating session to avoid
concurrent-write races.*
