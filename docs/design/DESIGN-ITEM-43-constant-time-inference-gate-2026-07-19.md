# DESIGN — Item 43: Constant-Time Inference Gate (plane-classified; dudect design owed)

- **Date:** 2026-07-19 · **Arc:** §H · **Status:** DESIGN v1 — design record artifact, **no code shipped**.
- **Companion:** `BLUEPRINT-ITEM-43-constant-time-inference-gate-2026-07-19.md` (planning, NO code). This file is the *design owed by* that blueprint: it records the plane classification (binding) and fully specifies the mandatory dudect gate so a deferred real-product pilot inherits a ready gate instead of re-deriving one.
- **Substrate (NOT reinvented):** `kernel/src/ct_gate.rs` (roadmap item 6). The dudect machinery — `measure_leakage`, `welch_t`, `T_THRESHOLD = 4.5`, interleaved + `black_box`'d timing, and the `naive_eq` planted-leak self-test — is the substrate. Item 43 designs *what the inference gate wraps*, it does **not** add a new harness.
- **Governing ruling (arc-wide):** *"безпека і передбачуваність понад швидкість"* — execution time must not leak input. For a *public-plane* pilot the leak has no adversary, so the gate is **cheap-but-optional**; the mandatory design is specified now so a future secret-adjacent pilot never bolts safety on late.

---

## 1. Binding note — plane classification for the toy pilot (recorded, not gated)

**Classification: PUBLIC / SYNTHETIC BY CONSTRUCTION.**

- Item 34's ruling (roadmap lines 524–528) classifies the toy pilot's inputs as public/synthetic **by construction**: no capability/crypto/secret-adjacent inputs, no PII, no product data. This is **already settled** for this pilot and pre-empts item 43's scope decision (blueprint §1, dependency gate "scope decided by input-plane classification first — which item 34's ruling has already settled").
- **Consequence → cheap-but-optional branch.** The item-39 kernels and item-38 workspace are already data-oblivious in *memory access* (fixed offsets, fixed lane order). The only data-dependent branch is ReLU (and the numerically-stable `sigmoid` split in `kernel/src/online.rs:144-153`, which exists for numerical-stability, **not** class leakage). On a public plane with no adversary, a timing leak conveys nothing of value.
- **Therefore the toy pilot is NOT gated** (over-design guard satisfied: "Do NOT make the toy pilot pay for a gate it does not need"). No production code, no gated `ct_gate` row, no `ct-gate` activation in the shipping build.

**Reopening trigger (named verbatim — the operator-dispatch point):**
> ANY new **secret-adjacent** consumer — i.e. the deferred real-product pilot fed from capability / crypto / PII surfaces — flips the mandatory branch ON.

This is recorded as a *binding, falsifiable condition*, not a vague "later". When it fires, §3 below becomes binding (not optional). No answer is invented for whether it will fire — only the trigger and its consequence are recorded.

---

## 2. Mandatory branch — full design (ready, not run for this pilot)

All of §2 is specified now so the deferred pilot inherits a ready gate. Substrate calls reference `kernel::ct_gate::*` (defined in `ct_gate.rs`). **SPEC snippets below are design, not compiled into the toy pilot; they become the deferred pilot's production + test form.**

### 2.1 What is measured (scope)

Wrap the **inference forward pass** — the activation-bearing call(s) of the pilot's model — and feed it through `ct_gate::measure_leakage`. The harness already:
- interleaves the two classes and flips interleave order every round (environmental drift cancels out of Δ-mean),
- `black_box`es inputs so the optimizer cannot constant-fold a fixed-input comparator,
- returns `|Welch t|` with accept threshold `T_THRESHOLD = 4.5` (the `ntt_ct_gate` standard).

The inference gate reuses `measure_leakage` exactly as `ct_eq`/`naive_eq` do — it does **not** define its own timing loop.

### 2.2 Input-class plan (blueprint §3B.3)

Construct fixed input classes that maximally exercise the data-dependent activation:

- **Class A — all-positive activations:** every ReLU/activation input ≥ 0, so the clamp branch is *never* taken.
- **Class B — all-negative activations:** every activation input < 0, so the clamp branch is *always* taken.
- **Boundary vs interior:** a third class straddling the activation threshold (inputs at/near 0) to catch threshold-proximate leakage (branch-predictor artifacts, NaN-handling branches).

**Hard gate (binding acceptance for the deferred pilot, blueprint §3B.3 / §5.4):**
```
|measure_leakage(class_a, class_b, infer, rounds, batch)| < T_THRESHOLD   // 4.5
```
i.e. a constant-time inference path keeps `|t|` bounded under the noise floor.

### 2.3 Branchless activation form (blueprint §3B.5)

Replace the data-dependent `if x > 0 { x } else { 0 }` branch with a **sign-mask** so the model runs the same instruction count regardless of input. The blueprint gives the i32 form `x & !(x >> 31)`; the kernel's activations are `f64` (see `online.rs`), so the ready form is the f64 analog:

```rust
// SPEC — branchless f64 ReLU, data-oblivious. NOT compiled into the toy pilot.
// Same number of ops for any x; no branch to mispredict or time-leak.
fn relu_branchless(x: f64) -> f64 {
    let sign = (x.to_bits() >> 63) & 1;          // 1 if x < 0, else 0
    let keep = (sign as u64).wrapping_neg();       // 0 if x<0, all-ones if x>=0
    let mask = f64::from_bits(keep);              // 0.0 or 1.0
    x * mask
}
```

Any other data-dependent activation (e.g. the `sigmoid` split in `online.rs:144-153`) is likewise **mask/cmov'd** on the secret plane — the split exists for numeric stability, so the deferred pilot replaces it with a branchless stable form (`1.0/(1.0+(-t).abs().exp() + ...)` via sign-mask selection) rather than an `if t >= 0` branch. The contract: *the model runs the same instruction count regardless of input.*

### 2.4 Planted-leak self-test (blueprint §3B.4 — the load-bearing half)

The gate's credibility rests on it being able to **reject**, not just pass. A deliberately leaky ReLU-with-early-return MUST be caught by the *same* `measure_leakage`/`welch_t` machinery (|t| ≥ 4.5), or the whole gate is RED (SYNTHESIS §10/P7: "the verifier the author cannot forge").

```rust
// SPEC — planted leak, mirrors ct_gate::naive_eq. MUST be detected by the same harness.
// SPEC — the leaking variant: early-return on first clamped lane (structural timing channel).
fn relu_leaky(xs: &[f64]) -> f64 {
    for &x in xs {            // early exit the moment one lane clamps
        if x < 0.0 { return 0.0; }
    }
    xs.iter().copied().fold(0.0, |a, x| a + x)
}

// SPEC — the dudect self-test (timing; run in release by hardening-gate.sh step E, same
// invocation as the gate). Mirrors ct_gate::dudect_gate_detects_planted_leak_and_passes_ct_eq:
//   (1) planted relu_leaky  |t| >= T_THRESHOLD  (else gate is blind)
//   (2) relu_branchless     |t| <  T_THRESHOLD  (best-of-N)
//   (3) leak_t >= 3.0 * ct_t  (separation proof — holds regardless of runner noise floor)
```

This is the verbatim analog of `ct_gate::dudect_gate_detects_planted_leak_and_passes_ct_eq` (ct_gate.rs:221-276), swapped from `ct_eq`/`naive_eq` to `relu_branchless`/`relu_leaky`. **No new harness** — same `measure_leakage`, same threshold, same self-test shape.

### 2.5 Containment & CI (blueprint §3B.6)

- **CI-time, not linked into release:** the inference gate module is `#[cfg(any(test, feature = "ct-gate"))]`, identical containment to `ct_gate` (Cargo.toml:45-52). A shipping binary carries none of the timing harness.
- **Self-test runs in the same invocation as the gate** — `scripts/hardening-gate.sh` step E in release: `cargo test --release --lib ct_gate -- --ignored` (ct_gate.rs:221, lib.rs:196-203). The inference self-test is added to the same ignored test set; it is **never presence-checked** in the default suite.

---

## 3. Falsifiable acceptance (binding for the deferred pilot)

1. Plane classification recorded with reasoning → **DONE in §1** (public/synthetic by construction).
2. Toy pilot NOT gated; reopening trigger named → **DONE in §1** (any secret-adjacent consumer).
3. Mandatory-branch design complete & ready → **DONE in §2** (input-class plan §2.2, planted-leak self-test §2.4, branchless mask/cmov ReLU §2.3).
4. **IF trigger fires:** Welch `|t| < 4.5` across input classes **AND** planted leak demonstrably caught (§2.2 + §2.4) — stated now as the binding acceptance for that reopening.
5. When active, gate is CI-time only (`ct-gate` containment) and its self-test runs in the same invocation (§2.5).

---

## 4. HOT-PATHS.tsv disposition (audit trail)

- The `ct_gate` row **already exists** at `docs/audits/hardening/HOT-PATHS.tsv:49`:
  `kernel/src/ct_gate.rs  -  ct_gate  1  dudect  2  dudect-harness+planted-leak-selftest(item2);ct_eq-primitive`
- **No new gated row is added for the toy pilot** — this is the deliberate over-design guard (§1): the toy pilot is not gated, so it must not register a gated hot-path row. The mandatory branch, when it fires for the deferred pilot, extends *this same row* (the inference self-test joins the `ct_gate` ignored test set), not a separate zone.
- The `@ZONE kernel/src/ct_gate.rs` already declared (HOT-PATHS.tsv:35).

---

## 5. Operator-decision-needed (flagged, not answered)

The named reopening trigger (§1) **IS** the operator-dispatch point. When the deferred second (real-product) pilot is chosen, its input plane MUST be re-classified; if secret-adjacent (fed from capability/crypto/PII surfaces), the mandatory dudect branch + branchless mask/cmov activations (§2) become **required before that pilot ships**. This is settled for the *toy* pilot and explicitly recorded for the *second* pilot — never silently skipped. No answer is invented.
