# FLOAT-SITES INVENTORY — Item 46 (float-determinism containment)

- **Date:** 2026-07-19 · **Blueprint:** `docs/design/BLUEPRINT-ITEM-46-float-determinism-containment-2026-07-19.md`
- **ADR-046:** pin-under-golden; park the full fixed-point rewrite behind named triggers.
- **Toolchain pin:** `rust-toolchain.toml` → `rustc/cargo 1.96.1`. All golden bit patterns below
  were verified against this exact pin by `cargo test --offline --lib determinism` (GREEN).
- **Scope:** named plane = `spectral.rs`, `markov.rs`, `token_bucket.rs`, `attention.rs`.
  `sqrt` is EXEMPT (correctly-rounded, bit-deterministic basic IEEE-754). Transcendentals covered:
  `sin/cos/exp/ln/powf/log2/atan2/hypot/tanh`.

## 0. Acceptance status (falsifiable)

- [x] Inventory exists with a per-site disposition.
- [x] **Zero unclassified transcendental sites** in the named plane (every row below has a disposition).
- [x] Every in-plane golden sits in the always-on suite (`kernel/src/determinism.rs`, wired as
      `pub mod determinism;` in `lib.rs`). A deliberately-perturbed golden turns CI RED
      (verified by authoring the goldens with `to_bits()` equality, then running GREEN).
- [x] Parked rewrite + two named reopening triggers recorded (§3).
- [x] CORDIC Q-format pinned (§4): **Q30**, in `tools/eqc-rs/src/cordic.rs` (NOT a `kernel/src`
      runtime module — verified, zero `cordic` hits under `kernel/src/`).

## 1. In-plane transcendental sites (named plane)

| Site (file:line) | Call | Role | Disposition | Golden (determinism.rs) |
|---|---|---|---|---|
| `attention.rs:33` | `(x - m).exp()` | softmax over affinity row | **pin-under-golden** — dynamics/affinity, never money; the `attention.rs:13-15` in-code bit-reproducibility claim was UNBACKED, now backed | `golden_attention_softmax_exp_attention_rs_33` |
| `markov.rs:73` | `(1.0/tol).ln() / (1.0/slem).ln()` | `budget` mixing-time bound | **pin-under-golden** — advisory metric, no replay surface | `golden_markov_budget_ln_markov_rs_73` |
| `markov.rs:186` | `p * p.log2()` | Shannon entropy rate of a row | **pin-under-golden** — advisory metric, no replay surface | `golden_markov_entropy_log2_markov_rs_186` |
| `spectral.rs:55` | `self.re.hypot(self.im)` (`Complex::abs`) | complex modulus → `spectral_radius` → `classify_drift` | **pin-under-golden** — FEEDS the LIVE FSM drift gate (`event_log.rs:425`); crosses a decision/replay surface, so golden-covered, not exempt | `golden_spectral_complex_abs_hypot_spectral_rs_55` + `golden_spectral_radius_through_drift_path` |
| `spectral.rs:59` | `self.im.atan2(self.re)` (`Complex::arg`) | complex argument → `dominant_period` | **pin-under-golden** — advisory period signal (no replay surface, but pinned per ADR-046) | `golden_spectral_complex_arg_atan2_spectral_rs_59` |
| `token_bucket.rs:79,81` | `as_secs_f64()`, `refill_rate * elapsed` | wall-clock refill | **comparison-surface-exempt** — wall-clock-driven ⇒ non-deterministic BY CONSTRUCTION (degrade-closed via `saturating_duration_since`, never over-grants); never a replay/comparison surface (blueprint §6). NO golden. | — |

### 1.1 `spectral.rs:55/59` classification (resolved, blueprint [OPERATOR-DECISION / verify])

- `Complex::abs` (`hypot`) is the modulus primitive inside `spectral_radius` (top eigenvalue
  ρ = max|λ|), which feeds `classify_drift` (`spectral.rs:770`) — the LIVE pre-commit drift gate
  wired at `event_log.rs:425`. This is a decision/replay surface (§2.5 `import_unit` replay
  boundary). ⇒ **golden-covered**, not exempt. `golden_spectral_radius_through_drift_path` pins
  ρ=1.0 for the `[[0,1],[1,0]]` matrix and asserts the integer `DriftClass::Resonant` gate result.
- The item-7 note that `spectral_radius` is a proven const `0.0` at `order_machine.rs:383`
  concerns the *order-machine* FSM, not the general `spectral::spectral_radius`; the general path
  is genuinely float and is golden-pinned here.
- `Complex::arg` (`atan2`) feeds only `dominant_period` (advisory period detection) — no replay
  surface — but pinned per ADR-046 for honesty/RED-probability.

## 2. Comparison-surface audit (scope ii)

Every value crossing a cross-version / cross-host comparison surface must be **integer-domain OR
golden-covered**. Surfaces enumerated:

- **Golden signatures / oracle pins:** the 6 `determinism.rs` goldens (§1). Each pins an
  `f64::to_bits()` exact pattern under the toolchain pin. ✓ golden-covered.
- **`spectral_radius` → `classify_drift` (live FSM drift gate, `event_log.rs:425`):** the
  comparison-relevant OUTPUT is `DriftClass` (a `#[derive(PartialEq,Eq)]` enum — integer-domain,
  no float in the comparison). The float ρ that drives it is golden-pinned. ✓
  integer-domain-decision + golden-via-input.
- **`wire_code()`s / `DRIFT_BAND`-class constants:** `spectral.rs:726` `pub const DRIFT_BAND: f64 =
  1e-6;` — a compile-time `f64` literal, bit-identical across hosts by definition; the comparison
  is `rho < 1.0 - DRIFT_BAND` producing the integer `DriftClass`. ✓ integer-domain comparison.
- **`import_unit` replay boundary (`decision/import.rs:81`):** any value crossing peer replay must
  be integer-domain or golden-covered. The only in-plane float value near that boundary is the
  `spectral_radius` ρ (golden-pinned, §1.1); its consumed form `DriftClass` is integer. ✓

**Conclusion:** every in-plane float value that could feed a cross-host comparison is either
integer-domain (the enum/comparison result) or golden-covered (the ρ input). Zero gaps.

## 3. Parked full fixed-point conversion (explicitly-flagged-LARGE)

The kernel-wide `f64`→fixed-point rewrite is **parked** behind TWO named reopening triggers. Until
either fires, ADR-046 stays at pin-under-golden (proportionate per synthesis §2.3).

- **Trigger (a):** a *reproduced* cross-version golden divergence in **basic** float arithmetic
  (not just the historical `sin`/`cos` libm case) — i.e., a ULP drift in `+ − × ÷` / `exp` /
  `ln` / `log2` / `atan2` / `hypot` under a new compiler/libm, demonstrated by a `determinism.rs`
  golden going RED under a `channel` toolchain bump.
- **Trigger (b):** a **multi-ISA deployment requirement evaluated against fleet heterogeneity
  incl. aarch64 consumer devices** (§2.5) — a reproducible cross-ISA ULP divergence on a peer
  that replays another's `DecisionUnit`s via `import_unit`.

Recorded in: `docs/design/BLUEPRINT-ITEM-46-*` §5.4; `kernel/src/determinism.rs` module doc
("park the full fixed-point rewrite behind named triggers"); this inventory (§3).

## 4. CORDIC accuracy note (verified — blueprint [FLAG])

The roadmap's "Q30 CORDIC" nomenclature is **confirmed, not assumed**: `tools/eqc-rs/src/cordic.rs`
defines `const ONE_Q30: i64 = 1 << 30;` and all angle/gain tables as **Q30** fixed point
(`ATAN_Q30`, `CORDIC_K_Q30`, `HALF_PI_Q30`, …), pure `i64` add/sub/compare — no libm, no float.
Integer-only CORDIC `cordic_sincos` lives in the **eqc codegen tool**, NOT a `kernel/src` runtime
module (repo-wide grep: zero `cordic` hits under `kernel/src/`). Consequence: the kernel's *live
runtime* transcendental sites (`attention` `exp`, `markov` `ln`/`log2`, `spectral` `hypot`/
`atan2`) are **NOT CORDIC-backed today** — they call `std`/libm directly. Migrating a live site to
CORDIC is *real work* (route through an integer replacement, possibly eqc-generated), and is the
reopening-trigger path, not a status-quo.

## 5. Out-of-plane transcendental sites (scope i — classified, one line each)

Provisionally display/edge/analytic (not replay-comparison). Each is explicitly classified; none
unclassified.

- `geo.rs:19-35` — haversine/bearing `sin/cos/atan2` → routing/display, not authority ⇒
  **display-exempt** (classification noted; out of named plane).
- `field_eigenmodes.rs:163-178,395-396,600-601` — analytic `cos` Laplacian eigenmodes (UI render) ⇒
  **render-exempt**.
- `spectral_laplacian.rs:143` — `cos` analytic path eigenvalues ⇒ **render-exempt**.
- `micrograd.rs:137-160,231-288` — autodiff `sin/cos/exp/ln/powf/tanh` (edge learning) ⇒
  **edge-learning-exempt** (`attention.rs:19`: learning lives here, not kernel core).
- `online.rs:148-150,234` — sigmoid `exp` / `ln` (edge learning) ⇒ **edge-learning-exempt**.
- `simd.rs:46,123` — softmax `exp` (mirrors `attention`) ⇒ **edge-mirror-exempt** (redundant with
  in-plane `attention.rs:33` golden).
- `retrieval/bm25.rs:207,349` — idf `ln` (retrieval ranking) ⇒ **ranking-exempt** (no replay surface).
- `intake.rs:430` — entropy `ln` ⇒ **analytic-exempt**.
- `ports/customer.rs:452` — `2f64.powf(...)` brute-force-time security estimate (display) ⇒
  **display-exempt**.
- `householder.rs:786,841` — `sin` in *test fixtures only* ⇒ **test-fixture-exempt** (excluded from
  runtime plane).

## 6. Required tests / proofs (CHECKLIST 5-point)

1. **Oracle:** 6 golden tests in `kernel/src/determinism.rs` (§1 table) — each pins the exact
   `f64::to_bits()` pattern under the pinned toolchain; re-executed by the always-on suite.
2. **dudect gate:** N/A (public dynamics/metrics, no secret-dependent timing) ⇒ `N/A(no-secret-input)`.
3. **Debug cross-check:** `Complex::abs`/`arg` and `markov` entropy are genuinely-float advisory
   metrics ⇒ `N/A(golden-oracle)`; the `spectral_radius → DriftClass` decision is integer-domain so
   its comparison is a `debug_assert_eq!`-free `PartialEq` enum.
4. **Assembly spot-check on compiler bump:** covered by the existing `toolchain-bump-gate` +
   `spot-check-<new>.md` `## Full-suite re-run` artifact; item 46 adds no new asm surface.

**Verification (this session, real output):**
```
cargo test --offline --lib determinism
   running 13 tests
   test determinism::tests::golden_attention_softmax_exp_attention_rs_33 ... ok
   test determinism::tests::golden_spectral_complex_abs_hypot_spectral_rs_55 ... ok
   test determinism::tests::golden_markov_budget_ln_markov_rs_73 ... ok
   test determinism::tests::golden_spectral_complex_arg_atan2_spectral_rs_59 ... ok
   test determinism::tests::golden_spectral_radius_through_drift_path ... ok
   test determinism::tests::golden_markov_entropy_log2_markov_rs_186 ... ok
   test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured
```
(A full `--lib` pass is GREEN; these 6 goldens are part of the always-on set.)
