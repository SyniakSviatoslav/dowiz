# FLOAT-SITES-2026-07-19 — Item 46 float-determinism containment inventory

Companion to `BLUEPRINT-ITEM-46-float-determinism-containment-2026-07-19.md`.
Implements ADR-046: **pin-under-golden now, park the full fixed-point rewrite
behind named triggers.** Pure-std, zero new deps. Money (`kernel/src/money.rs`)
is integer-exact and is **out of the float plane** — untouched.

## Scope (i): in-plane transcendental site inventory

Named plane: `spectral.rs`, `markov.rs`, `token_bucket.rs`, `attention.rs`.
Every libm-transcendental call site classified. Dispositions:
`migrate-to-CORDIC-class` (NOT used — ADR-046 pins instead) /
`pin-under-golden` / `basic-arith-exempt` or `comparison-surface-exempt`.

| Site (file:line) | Call | Role | Disposition | Golden |
|---|---|---|---|---|
| `attention.rs:33` | `(x - m).exp()` | softmax over affinity row | **pin-under-golden** (dynamics/affinity, never money) | `determinism::tests::golden_attention_softmax_exp_attention_rs_33` |
| `markov.rs:73` | `(1.0/tol).ln() / (1.0/slem).ln()` | mixing-time bound `budget` | **pin-under-golden** (advisory metric) | `determinism::tests::golden_markov_budget_ln_markov_rs_73` |
| `markov.rs:186` | `p * p.log2()` | Shannon entropy rate | **pin-under-golden** (advisory metric) | `determinism::tests::golden_markov_entropy_log2_markov_rs_186` |
| `spectral.rs:55` | `self.re.hypot(self.im)` (→ `Complex::abs`) | complex modulus | **pin-under-golden** — feeds `spectral_radius` → `classify_drift` (the LIVE FSM drift gate, `event_log.rs:425`); a decision/replay surface | `determinism::tests::golden_spectral_complex_abs_hypot_spectral_rs_55` + `golden_spectral_radius_through_drift_path` |
| `spectral.rs:59` | `self.im.atan2(self.re)` (→ `Complex::arg`) | complex argument | **pin-under-golden** — feeds `dominant_period` (advisory period signal) | `determinism::tests::golden_spectral_complex_arg_atan2_spectral_rs_59` |
| `token_bucket.rs:70–72` | `as_secs_f64()`, `refill_rate * elapsed` | wall-clock refill | **comparison-surface-exempt** — wall-clock-driven ⇒ non-deterministic by construction, never a replay/comparison surface (blueprint §6) | none (recorded, not pinned) |

**Acceptance (scope i): ZERO unclassified transcendental sites in the named plane.** ✔

### Decomposition of the `spectral.rs:55/59` classification (operator-decision, §10)
- `Complex::abs()` (hypot) → `spectral_radius` → `classify_drift` → the live
  drift gate `EventLog::commit_after_decide_drift_gate` (`event_log.rs:425`).
  This is the one in-plane site that feeds a **decision/replay surface**, so it
  is golden-covered AND additionally pinned through the real
  `spectral_radius → classify_drift` path (the gate consumes the integer-domain
  `DriftClass` enum, which is comparison-surface safe). Verified this worktree:
  the input `[[0,1],[1,0]]` (eigenvalues ±1) yields `ρ=1.0` → `Resonant`,
  `classify_drift == DriftClass::Resonant` (golden `golden_spectral_radius_through_drift_path`).
- `Complex::arg()` (atan2) → `dominant_period` → advisory period signal only
  (no gate). Golden-covered; advisory, never crosses a replay boundary.
- The item-7 note that `spectral_radius` for the lifecycle FSM is the proven
  const `0.0` (`order_machine.rs:383`, `FSM_SPECTRAL_RADIUS`) confirms the FSM
  drift path is integer-domain; the general `Complex` path above is the one
  pinned.

## Scope (ii): comparison-surface audit

Every value feeding a cross-version/cross-host comparison surface must be
**integer-domain OR golden-covered**. Enumerated:

| Surface | Domain | Status |
|---|---|---|
| `DriftClass::wire_code()` (`spectral.rs:709`) | integer (`u8`, 0/1/2) | integer-domain ✔ |
| `classify_drift` output consumed by `event_log.rs:425` gate | integer enum `DriftClass` | integer-domain ✔ |
| `golden signatures` (`verify_fsm_signature`, `FSM_GOLDEN_SIGNATURE`) | content-addressed `sha3_256` (integer) | integer-domain ✔ |
| `DRIFT_BAND`-class constant (`spectral.rs:720`) | `f64 = 1e-6` tolerance band | advisory band, NOT a comparison pin; feeds the integer enum branch ✔ |
| `import_unit` replay boundary (`decision/import.rs`) | integer/hash domain | out of the float plane ✔ |
| in-plane float values crossing the replay/decision boundary | `spectral_radius` via `Complex::abs` | **golden-covered** (`golden_spectral_radius_through_drift_path`) ✔ |

The only in-plane float that reaches a replay/decision surface is
`spectral_radius` (via `Complex::abs`), and it is golden-pinned. ✔

### Out-of-plane transcendental sites (scope i, one-line classification each)
Inventoried for completeness; each is display/edge/analytic (NOT a replay/
comparison surface). Provisional disposition per blueprint §2.3:

| Site | Call | Plane classification |
|---|---|---|
| `geo.rs:19–35` | haversine/bearing sin/cos/atan2 | display/routing, not authority → exempt |
| `field_eigenmodes.rs:163–178,395–396,600–601` | cos Laplacian eigenmodes | UI render → exempt |
| `spectral_laplacian.rs:143` | cos eigenvalues | UI render → exempt |
| `micrograd.rs:137–160,231–288` | autodiff sin/cos/exp/ln/powf/tanh | edge learning → exempt |
| `online.rs:148–150,234` | sigmoid exp / ln | edge learning → exempt |
| `simd.rs:46,123` | softmax exp | mirrors attention (dynamics) → exempt |
| `retrieval/bm25.rs:207,349` | idf ln | retrieval ranking → exempt |
| `intake.rs:430` | entropy ln | analytic → exempt |
| `ports/customer.rs:452` | `2f64.powf(...)` security estimate | display → exempt |
| `householder.rs:786,841` | sin in test fixtures only | test fixtures → exempt |

## Implementation note — the goldens
`kernel/src/determinism.rs` (new, `#[cfg(test)]`) pins the EXACT IEEE-754 bit
pattern (`f64::to_bits()`) of each in-plane value under the pinned toolchain.
The golden values (captured 2026-07-19):

| Golden | pinned `to_bits()` |
|---|---|
| `golden_attention_softmax_exp_attention_rs_33` | `4604167177386354576` |
| `golden_markov_budget_ln_markov_rs_73` | `4634314005443282009` |
| `golden_markov_entropy_log2_markov_rs_186` | `4602641559526520590` |
| `golden_spectral_complex_abs_hypot_spectral_rs_55` | `4612217596255138984` |
| `golden_spectral_complex_arg_atan2_spectral_rs_59` | `4612488097114038738` |
| `golden_spectral_radius_through_drift_path` (`spectral_radius`) | `4607182418800017408` |

Red-proven: perturbing any pinned `to_bits()` by 1 ULP turns the corresponding
test RED under the pinned toolchain (verified during implementation). These sit
in the always-on full-suite cargo test set (re-run by item 6's `hardening-gate`
and the toolchain-bump-gate), so a libm ULP drift on a new compiler/ISA turns
the bump PR RED.

### CHECKLIST 5-point standard
1. **Oracle:** the pinned bit pattern above, re-executed by the always-on suite. ✔
2. **dudect:** `N/A(no-secret-input)` — public dynamics/metrics, no secret-dependent timing. ✔
3. **Debug cross-check:** genuine float advisory metrics → `N/A(golden-oracle)`;
   the one decision-path float is golden-covered. ✔
4. **Assembly spot-check:** covered by the existing `toolchain-bump-gate` +
   `spot-check-<new>.md` `## Full-suite re-run` artifact. Item 46 adds no new asm surface. ✔

## Scope (iii): parked full fixed-point conversion

**Parked as an explicitly-flagged-LARGE item.** NOT built now (ADR-046).
Reopening triggers (either triggers a real migration of a site that crosses
`import_unit`'s replay boundary to integer-CORDIC):

1. **(a)** a *reproduced* cross-version golden divergence in basic IEEE-754
   float arithmetic under the pinned toolchain (a real libm ULP drift that the
   goldens catch and that a re-pin cannot absorb), OR
2. **(b)** a multi-ISA deployment requirement evaluated against **fleet
   heterogeneity incl. aarch64 consumer devices** (blueprint §2.5) — where a
   value crossing `import_unit`'s replay boundary is not golden-coverable by a
   single pinned binary.

Until a trigger fires, libm stays in the runtime (accepted risk, synthesis §2.3);
basic IEEE-754 arithmetic + the goldens above suffice for a fixed binary.

## Accuracy correction: where the CORDIC actually lives (§2.1)

The roadmap calls the row-25 fix the "Q30 CORDIC". **Verified this worktree:**
the integer-CORDIC `cordic_sincos` lives in `tools/eqc-rs/src/cordic.rs` — the
eqc equation→Rust **compile-time codegen tool**, NOT a `kernel/src` runtime
module (repo-wide grep: zero `cordic` hits under `kernel/src/`). Consequence:
the kernel's **live runtime transcendental sites call `std`/libm directly**
(confirmed: `attention.rs:33` exp, `markov.rs:73/186` ln/log2,
`spectral.rs:55/59` hypot/atan2). So "migrate-to-CORDIC-class" for a live site
is real work (route the site through an integer replacement, possibly
eqc-generated), not a status-quo. Item 46 pins those sites instead. **The
"Q30" nomenclature is unverified** — `tools/eqc-rs/src/cordic.rs` should be the
authority for the exact Q-format; ledger row 25 says "32-bit". Recorded here so
future readers do not assume the runtime is CORDIC-backed.

## HOT-PATHS.tsv registration
Added a data row pinning the always-on goldens:
`kernel/src/determinism.rs  -  determinism::tests::  6  lib  1  N/A(golden-oracle):item46`
so the `hardening-gate` floor asserts ≥6 in-plane determinism goldens are always run.
