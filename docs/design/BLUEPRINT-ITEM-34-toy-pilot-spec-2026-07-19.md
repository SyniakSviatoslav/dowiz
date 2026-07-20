# BLUEPRINT — Item 34: Synthetic/Toy Pilot — Execution Spec (the `f(x)=y` contract)

- **Date:** 2026-07-19 · **Arc:** §H · **Status:** BLUEPRINT v1 — planning artifact, NO code.
- **Ruling status:** the **scope ruling is RESOLVED** (operator, 2026-07-19, roadmap lines 511–532;
  synthesis §3): **SYNTHETIC/TOY PILOT FIRST**. This blueprint therefore describes **execution**,
  not further scoping — it fixes the "spec half owed on dispatch": the toy classifier's bounded
  input domain **D** and the output-tolerance guarantee the engine must prove.
- **Governing ruling (arc-wide):** *"безпека і передбачуваність понад швидкість."*
- **Sources read this session:** roadmap §H item 34 (lines 511–532, the CLOSED ruling +
  scope-consequence threading to items 35–44 and item 43); synthesis §1.1 (the real, deterministic
  adjacencies — `attention.rs` softmax, `mat.rs` matmul, `simd.rs` AVX2 lane, `arena.rs` BumpArena),
  §3 (candidate real-product surfaces DEFERRED to a follow-on second pilot); the GROUNDED negative:
  **no model, no weights, no ML dep exist in-repo** (synthesis §1.1, exhaustive grep both repos).
- **Dependency gate:** the ruling is made (zero-prereq to start on dispatch). **This spec GATES
  items 35–44** — nothing downstream has a concrete graph to target until D and the contract are
  fixed here.

---

## 1. Scope / goal + non-goals

**Goal.** Produce (a) a one-page spec fixing the toy classifier's architecture, its bounded input
domain **D**, and the pure-function `f(x)=y` output guarantee; and (b) the *built* toy pilot — the
concrete vehicle every downstream determinism item (quantization → arena → SIMD → oracle → golden
checksum → embedded weights) is proven against **end-to-end** BEFORE any real product workload.

**Non-goals (restating the ruling).** NOT a real-product classifier — the three §3 surfaces
(retrieval reranker head, `Verdict`/`DriftClass` anomaly scorer, ETA-adjacent regressor) are
**DEFERRED to a follow-on second pilot gated on this toy pilot landing green**. NOT design-only:
the toy pilot **is built**. NOT an LLM (P-F physics closed that permanently), NOT training, NOT
GPU. **Zero product data, zero PII, zero product risk** — the input plane is public/synthetic
**by construction** (this is the load-bearing consequence: it settles item 43 to its cheap branch).

## 2. Current-state grounding

- **No model exists.** Synthesis §1.1 GROUNDED: no tensor engine, no quantized model, no weight
  artifact, no ML dep anywhere in either repo. The pilot is genuinely greenfield as a *subsystem*.
- **The substrate is real.** `kernel/src/attention.rs` (deterministic scalar softmax),
  `kernel/src/mat.rs:132` (`matmul_contig`, the ONE matmul shape), `kernel/src/simd.rs`
  (AVX2 bit-identity lane), `kernel/src/arena.rs:43` (`BumpArena`). The pilot *composes* these.
- **Training substrate is edge-side and offline.** `kernel/src/micrograd.rs` (reverse-mode
  autodiff) + `kernel/src/online.rs` (`LinearSGD`, `ScalarAdam`) — deliberately outside the
  kernel's decision path. Any fitting the toy weights need happens here, at build time, and its
  output is a frozen artifact (Q3: inference-only, permanently).

## 3. Implementation plan (execution — ruling already made)

1. **Fix the architecture** — a small, fully-specified feed-forward integer classifier. Concrete
   proposed shape (executor may finalize dims within these constraints): input vector of `N` i8
   values → one hidden affine layer `W1: [i8; H×N]` + ReLU → output affine layer `W2: [i8; C×H]`
   → argmax over `C` classes. KB-scale (e.g. `N≤64, H≤32, C≤10` ⇒ ≤ ~2.4 KB weights) so the
   whole thing fits embedded weights (item 41) and bounded-domain proofs.
2. **Author or offline-fit the weights** at KB scale using the edge substrate (`micrograd.rs` /
   `online.rs`) or hand-authored, then **freeze**. Quantize to i8-symmetric per item 35. Zero
   product data anywhere.
3. **Define D and its plane classification.** D is **synthetic and enumerable-or-tightly-bounded**
   (§5 decision). Classify the plane: **public/synthetic by construction** — records into item 43
   that this pilot takes the cheap-but-optional constant-time branch, and names the reopening
   trigger (a real-product / secret-adjacent pilot).
4. **Define the golden reference behavior.** The frozen model + D feed the item-37 reference oracle
   (the `f(x)=y` ground truth) and the item-40 per-layer golden checksums.
5. **Write the one-page spec** pinning: architecture, weight provenance (hand/offline-fit + frozen),
   D, and the tolerance guarantee (§4).

## 4. The `f(x)=y` contract — tolerance is ZERO (bit-exact), not epsilon

Because the whole engine is **integer-domain** (i8 in, i32 accumulate, i8 requantize, argmax out),
the correctness contract is **bit-exact equality**, not a float epsilon:

> For every `x ∈ D`, the engine's output `y = f(x)` MUST equal the item-37 reference oracle's
> output **exactly**, and MUST be identical across repeated runs and across native/wasm32 targets.

This is the source dialogue's part-3 "pure function `f(x)=y`" made concrete: zero internal
mutable state (Q3), bounded input domain D, and a *bit-exact* (not "within tolerance") guarantee —
the strongest form the ruling permits. The engine is `sin(x)`-class deterministic, not
probabilistic.

## 5. Falsifiable acceptance criteria

1. A one-page spec doc exists fixing the architecture, D, and the bit-exact `f(x)=y` contract.
2. The frozen toy weights exist at **KB scale** with **zero product/PII data** (provenance recorded:
   hand-authored or offline-fit via `micrograd`/`online`, then frozen).
3. D is recorded as **synthetic** and **enumerable or tightly bounded** (the §5-decision form).
4. The input plane is recorded as **public/synthetic by construction**, with the item-43
   cheap-branch consequence and the named reopening trigger threaded through.
5. The spec is the concrete target items 35–44 build against — i.e. it fixes every layer shape whose
   overflow bound (item 35) and workspace offset (item 38) the downstream items must prove.

## 6. Dependency gate + operator-decision-needed

- **Gate:** ruling RESOLVED; spec-half owed on dispatch. This spec **gates items 35–44**.
- **Operator-decision-needed — FLAGGED, not invented:** the *form* of D is a genuine design fork
  with a determinism-proof consequence, and it is the one point worth an explicit ruling at spec
  time:
  - **(A) Fully-enumerable hand-authored D** (e.g. classify points on a bounded integer grid into
    regions; D is a finite, enumerable set). Consequence: the **entire `f(x)=y` contract is
    provable by exhaustion end-to-end** — the strongest possible expression of "safety and
    predictability over speed."
  - **(B) Toy-MNIST-style D** (a tiny embedded synthetic digit corpus + bounded input range).
    Consequence: end-to-end proof is a **bounded corpus**, not exhaustive.
  - **Architect recommendation (not a decision):** prefer **(A) fully-enumerable** — it lets item
    37's oracle and item 42's end-to-end test be *exhaustive* over D, which the ruling favors. This
    is a design recommendation; the operator/spec-author sets the final form. It is **not** a
    downstream blocker (both forms feed items 35–44 identically); it only changes whether the
    end-to-end acceptance is "exhaustive over D" or "bounded corpus over D."
- The exact `N/H/C` dims are executor engineering within the KB-scale constraint — **not** an
  operator gate.
