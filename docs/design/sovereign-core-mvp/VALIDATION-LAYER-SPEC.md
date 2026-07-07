# Validation Layer — spec (orchestrator → core invariant gate)

> Operator 2026-07-07: *"a validation layer between the orchestrator and the core — a small Rust
> function that checks not just format (syntax) but logical invariants."*
>
> **NOT built this session** (context-budget saturated; kernel change is a red-line arc needing a clean
> session). This is the executable spec. Deterministic-core thesis: a rejected command is a *bug caught
> at the boundary*, never a bad state reaching `decide`.

## Where it slots

`orchestrator/shell → validate(cmd, &state, &ctx) → kernel::decide(&state, cmd, &ctx)`

Today the shell will call `kernel::decide` directly (step 0b-5). Insert `validate` as the gate **before**
`decide` so `decide`'s preconditions are guaranteed by a total, side-effect-free check. `decide` stays
pure; `validate` is the enforced contract at the seam.

Anchor types (all in `rebuild/crates/domain/src/kernel.rs`): `Command` (L84), `OrderState` (L221),
`Context<'a>` (L248), `decide()` (L301). Put the layer in a new `rebuild/crates/domain/src/kernel/validate.rs`.

## Signature (total function, no panics, no I/O)

```rust
pub fn validate(cmd: &Command, state: &OrderState, ctx: &Context<'_>) -> Result<(), Vec<Invariant>>;

#[non_exhaustive]
pub enum Invariant {                 // each = a named logical rule, not a serde/format error
    // ── format-plus (structural but semantic) ──
    NonPositiveMoney { field: &'static str },   // money is integer minor units AND > 0 where required
    EmptyLineItems,
    // ── logical / business invariants ──
    IllegalTransition { from: &'static str, cmd: &'static str },  // state-machine legality
    ActorNotAuthorized { actor: &'static str, cmd: &'static str },// actor-gate precondition
    PriceContextMismatch,            // cmd's money must reconcile with ctx-observed price (core invents nothing)
    IdempotencyKeyMissing,           // mutating cmds must carry a key (idempotency.rs precondition)
    QuantityOutOfRange { min: i64, max: i64 },
}
```

Return **all** violated invariants (a `Vec`, not first-fail) so the orchestrator gets the full picture in
one round-trip. `Ok(())` ⇒ `decide` may run.

## The invariants (format vs logical — the operator's distinction)

**Format-plus** (beyond serde): money fields are non-negative integers in minor units (never float —
matches the 0b-1 money boundary); required collections non-empty; enum discriminants in-range.

**Logical** (the point of this layer): (1) **transition legality** — `(state, cmd)` is a legal edge of the
order state machine (reuse `kernel::policy`); (2) **actor authorization** — `ctx.actor` may issue `cmd`
(reuse the actor-gate composed inside `decide`); (3) **price reconciliation** — any money in `cmd`
reconciles with `ctx`-observed pricing (the core invents no number — 0b-3 rule); (4) **idempotency
precondition** — mutating commands carry a key.

Reuse the predicates `decide` already composes (machine → actor-gate → cc1 → pricing); `validate` is those
preconditions *lifted before* the transition, returned as data instead of an internal early-return.

## Proof plan (Verified-by-Math — ship the RED case)

1. **Property test** (proptest, in `rebuild/crates/domain/tests/`): `∀ (state, cmd, ctx)` — if
   `validate(...).is_ok()` then `decide(...)` does not hit any precondition-reject branch (validate is
   sound: it accepts nothing decide would reject). This is the key invariant tying the two.
2. **RED cases** (must fail before the code exists): negative money → `NonPositiveMoney`; an illegal
   `(state,cmd)` edge → `IllegalTransition`; wrong actor → `ActorNotAuthorized`; cmd price ≠ ctx price →
   `PriceContextMismatch`. Each asserted as an `Err` containing that variant.
3. **Totality**: `validate` never panics — proptest with arbitrary inputs, `cargo test` + the wasm32
   disallowed-methods gate already in CI (no entropy/time/panic paths).
4. Wire into `kernel_hard_truth.rs` so the hard-truth suite covers the boundary.

## Red-line / gates (do NOT bypass)

`packages/*/migrations`, money, RLS, and the kernel are red-line globs. This touches the **kernel** →
human-gated. Build on a feature branch, guardian review, no shell cutover (0b-5) until `validate` is
proven. Keep `decide` unchanged in this step (add the gate around it; refactor shared predicates only if
tests stay green).

## Invariants landed so far — ✅ (2026-07-07)

Five invariants, each landed RED-first, one at a time; `decide` byte-unchanged throughout. Two
DISJOINT command families:

**TRANSITION commands** — `IllegalTransition` (step 1), **`ActorNotAuthorized`** (step 2, lifts
`policy::assert_owner_target_allowed`), **`CourierStrandGuard`** (step 3, lifts `policy::cc1_strand_guard`
over the OBSERVED `ctx.binding`; `reason` ∈ {`ACTIVE_BINDING`, `REQUIRES_DELIVER_FLOW`}). These mirror
`decide`'s preconditions EXACTLY, so **transition-command soundness is CLOSED**: over any observed
binding, `validate.is_ok() ⟺ machine ∧ actor-gate ∧ cc1` — the gate accepts EXACTLY what `decide`'s
transition path does.

**`PlaceOrder`** (the CREATE+PRICE door) — `NonPositiveMoney` (step 1), **`EmptyLineItems`** (step 4,
cart carries ≥1 line item). These are BOUNDARY business rules the orchestrator enforces, STRICTER than
`decide`'s permissive pricing corridor (an empty cart / a zero-qty line prices fine in the pure core),
NOT lifts of a `decide` precondition — so their soundness is validate's OWN definition (`err iff the
input is malformed`), not a `decide` biconditional.

Each dimension RED-proven via a scoped mutant (disabling that one predicate reds its concrete cases +
its proptest; the other dimensions + acceptance stay green). Dimension-isolated proptests condition
INSIDE the body, not via `prop_assume` — legal edges are ~15% of `status×cmd` pairs, so a `prop_assume`
there exhausts proptest's rejection limit; concrete RED cases are pinned by deterministic inline unit
tests so falsifiability never rides on the sampler.

## First atomic step — ✅ DONE (2026-07-07)

Read `kernel.rs` L84–330 (Command/OrderState/Context/decide) + `kernel/policy.rs` → write `validate.rs`
with the `NonPositiveMoney` + `IllegalTransition` invariants only (smallest useful pair) + their two RED
proptests → green → extend. One invariant at a time; each with its RED case first.

**Delivered:** `rebuild/crates/domain/src/kernel/validate.rs` (`validate` + `#[non_exhaustive] Invariant`
with the two variants); `decide` byte-unchanged. `NonPositiveMoney` guards only the raw-`i64` money that
escaped the `Lek` type at the seam (`FeeLocation.{delivery_fee_flat,free_delivery_threshold,min_order_value}`
— the `Lek` fields are non-negative by construction). `IllegalTransition` delegates to `assert_transition`
(folds illegal|same-status|scaffold), a clean superset of `decide`'s machine precondition.
**Proof:** 6 inline unit tests (concrete RED cases + green) + `tests/validation_layer.rs` proptests
(totality; machine-soundness biconditional `gate.ok == machine.ok` for transition cmds; money `err iff a
fee < 0`). Falsifiability shown with an always-`Ok` mutant → the 4 rejection assertions + 2 proptests go
RED while acceptance stays green. Sovereign-gate green (wasm32 + `--lib` clippy `-D warnings`); full
`cargo test` green (119). Reflection: `docs/reflections/INBOX/2026-07-07-validation-layer-step1`.

**NEXT (extend, one invariant at a time — each with its RED case first):** `QuantityOutOfRange`
(per-line quantity ≥ 1 — a boundary rule; note the pure core accepts qty 0 and REJECTS qty < 0 via
`checked_mul_qty`, so the sound floor that mirrors `decide` is `≥ 0`, but the business floor is `≥ 1`),
then `PriceContextMismatch` (cart product_ids must resolve against the `ctx` snapshot — this one DOES
lift a `decide` precondition: `compute_order_pricing` rejects an unknown product). Lifting the pricing
resolution moves toward full cross-`decide` soundness on the PlaceOrder side. `IdempotencyKeyMissing` is
DEFERRED — the current `Command` carries no idempotency key, so it needs a command-vocabulary change (a
separate, larger step). STILL HUMAN-GATED: wiring `validate` INTO the seam and the 0b-5 shell cutover
(no cutover until the invariant set the shell relies on is proven).
