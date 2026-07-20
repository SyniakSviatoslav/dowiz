# BLUEPRINT — P31d: Verification ladder — machine-checked proofs over the money/tax integer organs

- **Date:** 2026-07-20 · **Component:** CORE (P31d, absorbs S5) · **Status:** BLUEPRINT v1 (planning
  artifact, no code changed by this pass). Converts P31d's standing flag ("**Blueprint:** none —
  needs one before build. First deliverable IS the blueprint") into the real first-deliverable it
  called for: a candidate-invariant inventory ranked by (red-line severity × proof cheapness), an
  honest kani-vs-z3 tool selection, and one concrete proof harness fully specified against live code.
- **Sources read this session (verified against the live worktree, not memory):**
  `docs/design/ROADMAP.md` §10.5.1 P31d (the DoD this blueprint discharges — "≥5 candidate
  invariants," "at least one proof harness lands and runs in CI," "RED-able via a scratch-branch
  off-by-one"); `kernel/src/money.rs:270-300` (`apply_tax`, the hand-written law — note `tax_rate:
  f64`); `kernel/src/eqc_gen.rs:26-90` (`apply_tax_exclusive_int` / `apply_tax_inclusive_int`, the
  GENERATED integer-exact organs — `rate_micro: i64`, all-`i128` checked arithmetic, no float);
  `kernel/src/money.rs:568` (`apply_tax_generated_parity_exact_integers`, the existing exact-integer
  parity oracle); `kernel/src/lib.rs:122-125` (`#[cfg(kani)] mod kani_selftest;` — the item-7 gate
  hook); `kernel/src/kani_selftest.rs` (`#[kani::proof] #[kani::should_panic]` planted-fault
  self-test); existing `#[kani::proof]` sites in `kernel/src/token_bucket.rs`, `kernel/src/pq/dsa.rs`,
  `kernel/src/pq/keccak.rs`; `docs/design/BLUEPRINT-ITEM-07-kani-wiring-2026-07-19.md` (the
  separate-proof-job CI convention this blueprint reuses, not reinvents).

---

## 1. Scope / goal

P31d's job (roadmap §10.5.1): turn "VERIFIED-BY-MATH" from a discipline into a **machine-checked
artifact** for the invariants the repo already treats as red-lines — money-integer arithmetic first.
This blueprint delivers P31d's DoD item 1 (the ranked candidate-invariant inventory + tool choice)
and fully specifies DoD item 2 (one proof harness that lands and runs in CI) and DoD item 3 (its
RED-ability).

**Explicit non-goal / anti-scope (from the roadmap, restated so it binds):** no whole-kernel
verification — this is a *ladder*, cheapest-rung-first, money organs before anything else. The proof
tooling is **CI-only**, never a hard build dependency (Kani is its own toolchain — `cargo install
kani-verifier && cargo kani setup` — with minutes-scale proof times; it must not be folded into the
fast `--locked --offline` gates, per the item-7 CI header at `.github/workflows/ci.yml:551-552`).
This item hardens; it is not on the critical path and blocks nothing.

## 2. Why the *generated* integer organs are the correct first rung (grounding, not narrative)

The hand-written law `apply_tax` (`money.rs:270`) takes `tax_rate: f64` and does one float multiply
(`money.rs:275`, `(tax_rate * 1_000_000.0).round()`). Kani (a bit-precise model checker over CBMC)
reasons cleanly about bounded integers but is weak/expensive over IEEE-754 floats — so `apply_tax`
itself is a poor first Kani target. The repo already anticipated this: `eqc-rs` emitted
**integer-exact mirrors** of the law's two legs into `eqc_gen.rs`:

- `apply_tax_exclusive_int(sub: i64, rate_micro: i64) -> Result<i64, &'static str>`
  (`eqc_gen.rs:30`) — pure `i128` half-up division, `checked_mul`, `i64::try_from`, returns a **typed
  `Err`** on overflow/div-by-zero. Zero floats.
- `apply_tax_inclusive_int(sub: i64, rate_micro: i64) -> Result<i64, &'static str>`
  (`eqc_gen.rs:50`) — same discipline, plus a `sub == 0 ⇒ Ok(0)` short-circuit and a non-positive
  denominator refusal (`eqc_gen.rs:56,74`) that mirror the law's own guards (`money.rs:271,284`).

These are the ideal first rung: **integer-only, self-contained, already fault-tolerant by
construction (every arithmetic step checked), and already backed by an exact-integer parity oracle**
(`apply_tax_generated_parity_exact_integers`, `money.rs:568`) that pins them bit-for-bit to the
hand-written law. Proving properties of the integer organ *is* proving them of the law on the shared
integer domain, because the parity test already fixes the two as equal there. The organs' own
header (`eqc_gen.rs:16-24`) names them "SHADOW organs… the authority flip is W4-L1" — Kani proofs
strengthen the case for that eventual flip without performing it (out of scope here).

## 3. Candidate-invariant inventory — ranked by (red-line severity × proof cheapness)

Discharges P31d DoD item 1 (≥5, ranked). Rung 1 is the one this blueprint builds; the rest are the
ladder above it, named so the ordering is auditable.

| # | Invariant (property) | Target symbol (live) | Red-line severity | Proof cheapness | Rung |
|---|---|---|---|---|---|
| 1 | `apply_tax_exclusive_int` **never panics** — on any `(sub, rate_micro)` it returns `Ok` or a typed `Err`, never an arithmetic panic/overflow-abort | `eqc_gen.rs:30` | money (highest) | trivial — integer-only, self-contained | **BUILD NOW** |
| 2 | `apply_tax_inclusive_int` never panics; and its `sub == 0 ⇒ Ok(0)` + `denom ≤ 0 ⇒ Err` guards hold for all inputs | `eqc_gen.rs:50` | money | trivial | BUILD NOW (same harness family) |
| 3 | Tax result **sign law**: for `sub ≥ 0`, `rate_micro ≥ 0`, an `Ok(t)` result satisfies `0 ≤ t` (no negative tax from non-negative inputs) | `eqc_gen.rs:30/50` | money | cheap (bounded assume) | 1 |
| 4 | **Parity is total**, not just sampled: over a bounded integer domain the generated organ agrees with `apply_tax`'s integer path exactly (Kani-proving what `money.rs:568` only spot-checks) | `eqc_gen.rs` ↔ `money.rs:270` | money | medium (law has one float mul — prove over pre-quantized `rate_micro`) | 2 |
| 5 | **Ledger nets to zero**: `reverse_transfer` (`money.rs:256`) + its `reversed_leg` produce entries whose amounts sum to exactly 0 (double-entry closure) | `money.rs:256` | money | medium (needs a bounded ledger model) | 2 |
| 6 | `order_machine::assert_transition` is **total** over the `OrderStatus × OrderStatus` product (every pair is a typed `Ok`/`Err`, never a panic/unreachable) | `order_machine.rs` | order-lifecycle | cheap (finite enum) | 1 |

**Tool selection — Kani, not z3, justified by honest comparison (not appeal to authority):** the
targets are Rust functions over bounded integers with checked arithmetic. Kani model-checks the
**actual compiled Rust** (no re-modelling the function in a separate SMT DSL, so no model-drift
between proof and code), and it is **already wired into this kernel** — five `#[kani::proof]` sites
exist (`token_bucket.rs`, `pq/dsa.rs`, `pq/keccak.rs`, `kani_selftest.rs`) behind `#[cfg(kani)]`,
with a working planted-fault self-test and a CI gate (item 7). z3/SMT would require hand-translating
`apply_tax_*_int` into SMT-LIB — a second source of truth that can silently diverge from the Rust,
exactly the failure mode `eqc_gen`'s own parity-pin discipline exists to prevent. z3 wins only when
the property is not expressible as a Rust assertion over bounded inputs (e.g. an unbounded
∀-quantified algebraic identity); none of rungs 1–3 are that. **Decision: Kani for this ladder;**
revisit z3 only if a future rung needs unbounded quantification Kani cannot bound-model.

## 4. Design — rung 1, gated exactly like the existing Kani proofs

New `#[cfg(kani)]` module, sibling to `kani_selftest`, compiled out of every normal build (zero
footprint — same discipline as `lib.rs:124`). It reuses the existing `#[kani::proof]` convention
verbatim.

New file `kernel/src/kani_money.rs` (registered in `lib.rs` as `#[cfg(kani)] mod kani_money;`):

```rust
//! P31d rung 1 (space-grade §S5): machine-checked proofs over the money/tax integer organs.
//! Compiled ONLY under `cfg(kani)` (see lib.rs) — zero footprint in every normal build,
//! CI-only, never a build dependency. Reuses the item-7 kani-gate (BLUEPRINT-ITEM-07 §6.5).

use crate::eqc_gen::{apply_tax_exclusive_int, apply_tax_inclusive_int};

/// Invariant 1: `apply_tax_exclusive_int` never panics — for ANY (sub, rate_micro) it
/// returns Ok or a typed Err, never an arithmetic overflow-abort. (No `#[kani::should_panic]`
/// here — the property IS "does not panic"; if any input path panics, Kani reports it and the
/// gate goes RED.)
#[kani::proof]
fn proof_tax_exclusive_total() {
    let sub: i64 = kani::any();
    let rate_micro: i64 = kani::any();
    // Whole i64 domain — the organ's own checked arithmetic must absorb every value.
    let _ = apply_tax_exclusive_int(sub, rate_micro);
}

/// Invariant 3 (sign law): for non-negative subtotal and non-negative rate, an Ok tax is
/// non-negative. Bounded assume keeps the model in the documented input domain.
#[kani::proof]
fn proof_tax_exclusive_nonnegative() {
    let sub: i64 = kani::any();
    let rate_micro: i64 = kani::any();
    kani::assume(sub >= 0);
    kani::assume(rate_micro >= 0);
    // Bound the magnitudes to keep the i128 intermediate in range so the property under test
    // is the sign law, not a restatement of the overflow guard (that is invariant 1's job).
    kani::assume(sub <= 1_000_000_000_000); // 1e12 minor units — far above any real order
    kani::assume(rate_micro <= 100_000_000); // 10 000% — far above any real tax
    if let Ok(tax) = apply_tax_exclusive_int(sub, rate_micro) {
        assert!(tax >= 0, "non-negative inputs produced a negative tax");
    }
}

/// Invariant 2: inclusive organ never panics; the sub==0 short-circuit and denom<=0 refusal
/// hold across the whole domain.
#[kani::proof]
fn proof_tax_inclusive_total() {
    let sub: i64 = kani::any();
    let rate_micro: i64 = kani::any();
    let r = apply_tax_inclusive_int(sub, rate_micro);
    if sub == 0 {
        assert!(matches!(r, Ok(0)), "sub==0 must be tax-free regardless of rate");
    }
}
```

(Exact symbol names — `apply_tax_exclusive_int`, `apply_tax_inclusive_int` — are the live public
API in `eqc_gen.rs`; the harnesses call only what already exists, adding no product surface.)

### 4.1 Cargo / CI wiring — reuse item 7, add nothing new to the fast path

- `lib.rs`: one line, `#[cfg(kani)] mod kani_money;`, adjacent to `mod kani_selftest;` at
  `lib.rs:124`. Invisible to every `cargo build`/`cargo test` (the `cfg(kani)` flag is set only by
  `cargo kani`).
- CI: the **existing** `kani-gate` proof job (separate from the fast `cargo-test` job, per
  `ci.yml:551-552` and `BLUEPRINT-ITEM-07`) already runs `cargo kani`, which discovers every
  `#[kani::proof]` harness in the crate — so the three new harnesses run with **zero CI-config
  change**. `docs/audits/hardening/HOT-PATHS.tsv` gets one row for `kernel/src/eqc_gen.rs`
  with `mode=kani` (matching item 6's manifest convention) so the hardening-gate knows the money
  organs now carry a proof obligation.
- No new dependency in the default graph: `cargo tree -e no-dev` is unchanged (Kani is an external
  toolchain, not a crate dep). The default build stays pure-`std`, serde-free, zero-external-dep.

## 5. Fits the existing architecture

- **Zero new primitives, zero new deps.** Reuses the item-7 `#[cfg(kani)]` pattern, the existing
  `kani-gate` job, and the existing `eqc_gen` public organs. Adds one gated module + one HOT-PATHS
  row — nothing else.
- **Respects the SHADOW-organ boundary.** The harnesses call `apply_tax_*_int` as *consumers*; they
  do not flip authority to them (W4-L1, out of scope) and do not touch `apply_tax` or the parity
  pin. Kani strengthens the eventual-flip case; it does not perform the flip.
- **Ladder discipline honored.** Only rung 1 (invariants 1–3) is built; rungs 2 (parity totality,
  ledger-nets-to-zero) and the `assert_transition`-total proof are named for the next pass, not
  attempted here — cheapest-rung-first, money organs before anything else.

## 6. Acceptance criteria (RED → GREEN, per this repo's standing "verified, not claimed" culture)

Discharges P31d DoD items 2 and 3.

1. **This blueprint itself discharges DoD item 1** — the ranked ≥5-invariant inventory (§3) with an
   honest kani-vs-z3 comparison (§3, tool-selection paragraph) exists and is on disk.
2. **GREEN:** with Kani installed (`cargo install kani-verifier && cargo kani setup`),
   `cd kernel && cargo kani --harness proof_tax_exclusive_total` (and the other two harnesses) reports
   VERIFICATION SUCCESSFUL. The `kani-gate` CI job stays green with the three new harnesses included.
3. **RED-able (DoD item 3), honestly obtained:** in a throwaway scratch branch, replace one
   `checked_mul` in `apply_tax_exclusive_int` with a plain `*` (removing the overflow guard) — Kani's
   `proof_tax_exclusive_total` must then report an arithmetic-overflow failure and the gate goes RED.
   Discard the scratch branch; ship only the GREEN version. (This mirrors the item-7 self-test's own
   planted-fault discipline, `kani_selftest.rs`.)
4. **No regression to the default build:** `cd kernel && cargo test` (no features) and
   `cargo tree -e no-dev` are unchanged — the module is `cfg(kani)`-only and additive.
5. **Roadmap update:** flip P31d's §10.5.1 status from PLANNED to PARTIAL (rung 1 landed; ladder
   rungs 2+ named), replace its "Blueprint: none — needs one" flag with a link to this file, and add
   the `eqc_gen.rs mode=kani` HOT-PATHS row — so a future audit does not re-derive that the money
   organs were the intended first rung.
