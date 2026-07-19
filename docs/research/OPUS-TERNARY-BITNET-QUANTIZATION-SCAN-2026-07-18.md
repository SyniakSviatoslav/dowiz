# Ternary / 1.58-bit (BitNet b1.58) Quantization — Codebase Landing-Site Scan

> **Type:** Research-only, blueprint-ready. **Zero code written, no branches touched.**
> **Date:** 2026-07-18 · **Scope:** `dowiz` kernel + engine, `bebop-repo/bebop2`.
> **Verdict up front:** **No genuine landing site for BitNet b1.58 exists in this codebase.**
> The technique needs trained neural-network weight *matrices* consumed by matmul-heavy
> inference; the repo, by explicit standing policy, hosts none. The nearest-adjacent
> surfaces (PPR/spectral ranking, retrieval fusion) are ranking/filtering, but they are
> deterministic-by-contract and far too small for BitNet's memory-bandwidth argument to
> bite — and any bucketing there would be *plain quantization, not BitNet*.

---

## 0. Method + honest environment note

Investigated with Grep/Read across `/root/dowiz/kernel/src`, `/root/dowiz/engine/src`,
`/root/bebop-repo/bebop2/*/src`, and `/root/dowiz/docs/design/`. Primary source (the arXiv
abstract page for the paper) fetched live via WebFetch.

**Environment constraint (reported, not worked around):** the session's WebSearch budget was
already exhausted (200/200) before this task ran, so the broader literature sweep (XNOR-Net,
TWN, BNN survey) could not be issued as live searches. The BitNet b1.58 primary-source claims
below were confirmed by direct WebFetch of `arxiv.org/abs/2402.17764`; the surrounding
low-bit-quantization context is stated from established knowledge (the paper and its lineage
predate the Jan-2026 model cutoff) and is flagged as such where it is not source-verified.

---

## 1. BitNet b1.58 — what the technique actually is (verified)

From the arXiv abstract (`2402.17764`, "The Era of 1-bit LLMs: All Large Language Models are
in 1.58 Bits", WebFetch-confirmed):

- **Weight alphabet:** every LLM weight is **ternary `{-1, 0, 1}`** (verbatim: "every single
  parameter (or weight) of the LLM is ternary {-1, 0, 1}"). It is **not** literally 1-bit.
- **1.58 figure:** `log2(3) ≈ 1.58` bits/weight — the information content of a 3-symbol
  alphabet. (Derivation is standard; the abstract asserts the "1.58-bit" label, WebFetch noted
  the derivation itself is not spelled out in the abstract excerpt.)
- **Training-aware, from scratch:** the abstract frames a "new scaling law and recipe for
  **training new generations of LLMs**" and matches the full-precision Transformer "with the
  same model size and training tokens." This is quantization-aware training (STE through a
  round-to-ternary), **not** post-hoc compression of a finished FP artifact.
- **What matmul becomes:** because weights are `{-1,0,1}`, the dominant `W·x` matrix
  multiplication degenerates to **sign-selected additions/subtractions** (no FP multiply) —
  the memory-bandwidth + energy win. (Standard BitNet result; abstract lists the benefit axes
  but not this mechanism verbatim.)
- **Benefits claimed:** "latency, memory, throughput, and energy consumption" improvements at
  **parity** with FP16/BF16 at the same size/token count.

**Load-bearing consequences for this scan:**
1. The benefit is intrinsic to **large, learned, continuous weight matrices** feeding
   **matmul-heavy inference** (transformer FFN/attention projections). Remove the matmul and
   the learned matrix, and the technique has nothing to act on.
2. You **cannot** take an existing exact/analytic quantity and "compress it to 3 states"
   losslessly — BitNet only works because the network is *trained to tolerate* the precision
   loss. Absent training, ternarization is just lossy rounding.

### 1.1 The distinction this whole doc turns on

| | **BitNet b1.58 (true ternary quantization)** | **Deterministic coarse-bucketing (plain quantization)** |
|---|---|---|
| Substrate | learned weight matrix (matmul) | any numeric field |
| Needs training? | **Yes** — QAT from scratch; a fit threshold that the net learns to tolerate | No — a fixed `sign`/threshold rule |
| Where the accuracy comes back | the network *relearns* around the quantizer | it doesn't — you accept the coarser answer, or refine exactly afterward |
| Applies to this repo? | needs a trained matmul weight store (none exists) | technically applies anywhere, but yields value only if the field is (a) bandwidth-bound and (b) tolerant of coarseness |

Throughout the surface scan below, wherever a "ternary" idea is even conceivable it is the
**right column** (simple bucketing), **never** BitNet. Calling such a change "BitNet" would be
a category error worth flagging in any proposal.

---

## 2. Standing policy: this codebase deliberately hosts no trained NN weights

Two prior operator-directed research syntheses already **reject trained ML components**, which
directly constrains where BitNet could land (no trained matrix ⇒ no target):

- `docs/design/GAUSSIAN-SPLATTING-ADDRESS-PICKER-SYNTHESIS-2026-07-16.md`
  - L37 / L398: **"Reranker — NO trained model."** / **"Trained ML reranker (cross-encoder /
    ColBERT / LLM-listwise) now — rejected."** The chosen reranker is an explicit
    *"deterministic hand-tuned fusion of the signals"* — hand-set coefficients, not learned.
  - L41 / L401: **"TimesFM — NO for per-order ETA"** (Kalman/EMA already optimal).
- `docs/design/SYSTEMS-GPU-ML-KERNEL-SYNTHESIS-2026-07-16.md`
  - L51–56 (verbatim, translated): *"'Using ML/GPU approaches' today means ML-**like math**,
    not ML and not GPU. There is no GPU in the stack … no trained weights."*

In-code, the boundary is enforced at the source:

- `kernel/src/attention.rs:17` — *"Scope: a reference scalar implementation (**no learned
  weights**). The trained-attention path (learning Q/K/V projections) is deliberately NOT
  here — the kernel stays non-AI (deterministic pure functions)."*
- `kernel/src/retrieval/recall.rs` (header) — *"The semantic (ONNX) signal is intentionally
  **out of scope** here — it is a **build-time neural model, not a kernel primitive**."*

So the kernel is, by design, a set of deterministic pure functions. BitNet's substrate
(learned matmul weight matrices) is exactly the thing the architecture keeps *out*.

---

## 3. Surface-by-surface scan (real files, honest verdicts)

| Surface | File:line | Current representation | BitNet applies? | Why |
|---|---|---|---|---|
| Attention weights | `kernel/src/attention.rs:17` | reference softmax, **no learned weights** | **No** | Nothing is trained; row-stochastic mixing, not a weight matrix. Learned Q/K/V explicitly excluded. |
| Spectral top-k | `kernel/src/spectral.rs:264–320` (`topk_symmetric`, `_in`) | f64 power-iteration + Hotelling deflation, **n ≤ 32** | **No** | Not a learned matrix; and n≤32 means zero memory-bandwidth pressure. Result feeds exact eigenmodes; ternary would wreck the fixed-iteration determinism for no win. |
| Personalized-PageRank | `kernel/src/retrieval/ppr.rs` | dense row-stochastic `W`, **bit-exact fixed-K** power iteration | **No** | Contract is *bit-identical reproducibility* (mirrors `markov.rs` summation order). Ternarizing `W` destroys that and saves nothing at this scale. Not learned. |
| Retrieval fusion / reranker | `kernel/src/retrieval/recall.rs`, `bm25.rs`, `index.rs` | **hand-tuned** deterministic fusion of trigram + BM25 (+PPR) signals | **No** | Coefficients are hand-set, not learned (policy §2). No weight matrix. Trigram index is exact set-membership (0 false positives). |
| Router edge weights | `kernel/src/router.rs:60` | `weight: f64` = **metric road distance** (Contraction Hierarchies) | **No** | Shortest-path correctness needs exact ordering; bucketing breaks admissibility of the A*/Eikonal heuristic. Not learned, not bandwidth-bound. |
| TokenBucket rate/burst | `kernel/src/token_bucket.rs`, `budget.rs` | two f64 config scalars (`capacity`, `refill_rate`) | **No** | Two scalars, not a matrix; over-grant invariant needs exact arithmetic. Nothing to quantize. |
| Agent admission priority | `kernel/src/ports/agent/admission.rs:269–516` | per-shard `TokenBucket`s, f64 refill | **No** | Same as above — rate config, no learned matmul. |
| Lyapunov / spectral weights | `bebop2/core/src/lyapunov.rs:48–98` | PSD form weights `wᵢ`, fail-closed on non-finite/negative | **No** | Correctness-gated stability certificate; a ternary `wᵢ` is meaningless (would fail the PSD gate). Not learned. |
| Kalman / causal / markov | `kernel/src/{kalman,causal,markov}.rs` | exact f64 statistical/spectral state | **No** | Analytic filters, not trained weights; determinism + numeric fidelity required. |

**Explicitly OUT OF SCOPE (per task, not re-litigated):** `money.rs`, ledger sums,
CPU-determinism oracle, `pq/*` KEM/signature byte representations — all require exact,
lossless, auditable arithmetic or full crypto security. Not evaluated. Correctly excluded.

---

## 4. The one genuine learning surface — and why BitNet still misses it

`kernel/src/online.rs` + `kernel/src/micrograd.rs` are the **only** place in the repo that fits
learned parameters:

- `micrograd.rs` — a scalar reverse-mode autodiff engine (`Value = Rc<RefCell<…>>`),
  dependency-free, deterministic.
- `online.rs` — `LinearSGD` (`y ≈ w·x + b`, ridge-regularized, one SGD step/sample) and
  `ScalarAdam`, both fed only local samples. Growth-substrate for capture-field / SIREN /
  2D-Gaussian-splat fits and self-improvement metric tracking.

**Why BitNet b1.58 does not apply here anyway:**
1. **No matmul, no matrix.** `LinearSGD` has parameters `w, b` — a scalar (or, for splat fits,
   a small vector). BitNet's win is amortized `W·x` over large `d×d` projections. At scalar /
   tiny-vector scale there is no matmul to turn into add/sub and no memory bandwidth to save.
2. **Ternary would break the fit, not accelerate it.** These learners solve regression/field
   fits where the *value* of `w` is the answer (`w→2, b→1` on `y=2x+1`). Constraining `w` to
   `{-1,0,1}` doesn't compress a large model — it forbids the correct answer.
3. It is a **growth-substrate primitive**, not a matmul-heavy inference hot path.

So even the repo's single learning surface is the wrong *shape* for BitNet.

---

## 5. The only real trained artifact — and it's kept outside the boundary

The **semantic embedding signal** (bge-small, ONNX) referenced in `retrieval/recall.rs` is the
one genuinely-trained neural weight matrix anywhere in this system's orbit. Two facts kill it
as a BitNet target *for this repo*:

1. It is a **build-time / external ONNX artifact injected via `&dyn LlmBackend`** — it is
   **not in-repo** and not a kernel primitive (recall.rs header, `lib.rs:97`). There is no
   source file here to quantize.
2. Even upstream, ternarizing bge-small would require **quantization-aware retraining/fine-
   tuning** of the embedding model to preserve the recall@5 property the fusion depends on —
   exactly BitNet's from-scratch requirement, on an artifact the project has already decided
   the kernel does not own. This is an *upstream-model-vendor* decision, not a dowiz code change.

Honest note: if the operator ever wants a *self-hosted, in-repo* embedding model (reversing the
current "semantic signal is out of scope" stance), **that** future artifact would be the one
legitimate BitNet-b1.58 candidate in the entire system — trained ternary weights, matmul-heavy
inference, memory-bandwidth-bound on commodity CPUs (BitNet's exact sweet spot). That is a
*new-component* proposal, not a retrofit of anything that exists today.

---

## 6. Bottom line

- **BitNet b1.58 (true trained-ternary quantization): no landing site.** The repo hosts no
  trained NN weight matrix consumed by matmul-heavy inference — by explicit, documented policy
  (§2). The one learning surface (`online.rs`) is scalar SGD, the wrong shape (§4). The one
  real trained artifact (bge-small) is external and out of the kernel boundary (§5).
- This is a **legitimate, valuable "no" answer**, consistent with this session's discipline of
  not padding findings. Applying BitNet "wherever it genuinely can" here yields: **nowhere,
  today** — because the architecture deliberately excludes BitNet's substrate.
- The nearest-adjacent surfaces (PPR/spectral ranking, retrieval fusion) are ranking/filtering
  and *could* in principle take a coarse pre-filter, but that is **plain deterministic
  bucketing, not BitNet**, and even that yields no real benefit here (§7).

---

## 7. If forced to find *something*: the least-wrong micro-candidate (NOT BitNet)

Presented only for completeness, clearly labeled as **simple quantization, not BitNet**, and
**not recommended** on current evidence:

**Candidate — coarse-bucketed approximate pre-rank ahead of exact PPR/spectral refinement.**
- **Where:** `retrieval/ppr.rs` (relatedness ranking) or `spectral.rs::topk_symmetric`.
- **Idea:** compute a fast approximate ordering from a low-precision (e.g. `{-1,0,1}`
  sign-bucketed) sketch of the transition/affinity matrix to shortlist top-K candidates, then
  run the existing **exact** power iteration only on the shortlist.
- **Training needed?** **None.** This is a fixed `sign`/threshold rule — deterministic
  bucketing. It is emphatically *not* BitNet (no learned weights, no QAT).
- **Realistic benefit:** **Effectively none, and likely net-negative.** (1) The matrices are
  tiny (spectral n≤32; living-memory PPR is a few hundred notes at most) — the memory-bandwidth
  argument that justifies low-bit representation never engages at this scale. (2) `ppr.rs`'s
  entire contract is **bit-exact reproducibility** (fixed K, fixed summation order mirroring
  `markov.rs`); a two-stage approximate→exact path adds branch complexity and a determinism
  surface for no measured speed win. (3) There is no benchmark today showing ranking is a hot
  path. **Recommendation: do not build.** Revisit only if a profiler ever shows PPR/top-k on a
  large (n ≫ 10³) graph dominating a real request path — which the current architecture
  (small, local-first living memory) does not produce.

---

## Sources
- BitNet b1.58 primary source (WebFetch-confirmed): *The Era of 1-bit LLMs: All Large Language
  Models are in 1.58 Bits*, arXiv 2402.17764 — <https://arxiv.org/abs/2402.17764>
- In-repo policy: `docs/design/GAUSSIAN-SPLATTING-ADDRESS-PICKER-SYNTHESIS-2026-07-16.md`
  (L37/L41/L398/L401); `docs/design/SYSTEMS-GPU-ML-KERNEL-SYNTHESIS-2026-07-16.md` (L51–56).
- In-repo code cited inline (attention.rs, online.rs, micrograd.rs, spectral.rs, retrieval/*,
  router.rs, token_bucket.rs, bebop2/core/lyapunov.rs).

_Note: broader low-bit literature (XNOR-Net, Ternary Weight Networks, BNN survey) not re-fetched
live — session WebSearch budget was exhausted (200/200) before this task; stated from knowledge
where not primary-source-verified, and flagged as such above._
