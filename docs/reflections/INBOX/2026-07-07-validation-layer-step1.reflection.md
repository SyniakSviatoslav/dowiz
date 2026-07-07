# Reflection — the type is the proof; a boundary validator only needs to guard what escaped the type system at the seam

**Date:** 2026-07-07 · **Slug:** validation-layer-step1
**Qualified because:** kernel-dir change (Sovereign Core — red-line surface) + 4 files (`kernel/validate.rs`
new, `tests/validation_layer.rs` new, `kernel.rs` mod-decl, `lib.rs` re-export) + stage-close
(VALIDATION-LAYER-SPEC "First atomic step").

## CONTEXT
Built the first atomic step of the Validation Layer (VALIDATION-LAYER-SPEC): a total, IO-free
`validate(cmd, &state, &ctx) -> Result<(), Vec<Invariant>>` that lifts `decide`'s preconditions to the
orchestrator seam and returns EVERY violation as data. Scope was deliberately the smallest useful pair:
`NonPositiveMoney { field }` (format-plus) + `IllegalTransition { from, cmd }` (logical). `decide` was
left BYTE-UNCHANGED (the gate sits AROUND it); no wiring into `decide`, no 0b-5 shell cutover — both
stay human-gated. Guardian/council removed for the kernel per §0·GP, so the gate was: sovereign-gate
(wasm32 + `--lib` clippy `-D warnings`) + RED-proof + full `cargo test`.

## DECISIONS
1. **`NonPositiveMoney` guards ONLY the raw-`i64` money that escaped the `Lek` type at the shell seam**
   — `FeeLocation.{delivery_fee_flat, free_delivery_threshold, min_order_value}` (all `Option<i64>`).
   Product prices / tier fees are `Lek`, non-negative BY CONSTRUCTION, so the TYPE is their proof and a
   runtime check there would be dead code (un-RED-able — you cannot construct a negative `Lek`). The
   validator's money job is exactly the complement of what the type system already enforces.
2. **`IllegalTransition` folds the machine's THREE reject classes** (illegal edge | same-status |
   scaffold) into one invariant by delegating to `assert_transition(...).is_err()`, making `validate` a
   clean SUPERSET of `decide`'s machine precondition. Safe because the soundness property is one-
   directional — `validate.is_ok() ⟹ decide.is_ok()` — so being STRICTER never breaks it; it only
   moves a refusal earlier.
3. **Scoped the soundness proptest to TRANSITION commands.** `PlaceOrder` routes around the machine and
   its pricing preconditions (`ctx.pricing == None` → `CorridorBreach`) are not yet lifted, so a naïve
   full-`decide` soundness property would be FALSE today (a `PlaceOrder` with no pricing is `validate`-ok
   but `decide`-err). One invariant at a time means one soundness DIMENSION at a time.
4. **Modelled `Invariant` on `DomainError`, not on a wire type** — `Copy`/`Eq`, `#[non_exhaustive]`, NO
   serde. The `&'static str` fields make `Deserialize` impossible anyway, and the precedent is that only
   `ErrorCode` (the wire code) derives serde; internal decision types stay serde-free and map to a wire
   code at the shell, exactly like `DomainError::code()`.

## WHY (causal)
A validation layer is tempting to write as "check every field." But in a core where money is a
non-negative-by-construction newtype (`Lek`) and legality is a total function (`assert_transition`),
most "validation" is already discharged by the type system and the existing pure predicates — and a
check that duplicates a type guarantee is a FALSE-POSITIVE proof: it can never go RED, because its bad
input is unconstructible. The falsifiability test (the always-`Ok` mutant) surfaces this directly: a
`NonPositiveMoney` check over `Lek` fields would stay green under EVERY mutant, flagging it as inert.
So the design rule that fell out: **a boundary validator earns its keep only where a raw value crossed
INTO the type-safe core without going through the type** — the `f64→i64` / `Lek`-less seam. Everywhere
else, cite the type as the proof and write no check.

## HOW TO APPLY
- When adding a boundary/validation invariant, first ask "can I even construct the bad input?" If the
  type forbids it, the check is inert — cite the type instead. Guard only the raw values at the seam.
- State cross-layer soundness as `gate.ok ⟹ inner.ok` (one-directional) so the gate may be a strict
  superset; add one soundness DIMENSION per invariant, and scope the property to the command family
  that dimension actually ranges over (don't assert a property that is false for a sibling family).
- Two-sided assertions (bad→Err AND good→Ok) make one always-`Ok` mutant red exactly the rejection
  half while acceptance stays green — that split IS the signature of a falsifiable proof.

Links: [[verified-by-math-2026-07-07]] · [[ground-truth-over-proxy-2026-07-07]] · 0b-3 reflection
(2026-07-07-0b3-decide-composition) · VALIDATION-LAYER-SPEC.
