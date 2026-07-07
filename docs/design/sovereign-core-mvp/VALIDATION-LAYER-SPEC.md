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

## First atomic step (next session)

Read `kernel.rs` L84–330 (Command/OrderState/Context/decide) + `kernel/policy.rs` → write `validate.rs`
with the `NonPositiveMoney` + `IllegalTransition` invariants only (smallest useful pair) + their two RED
proptests → green → extend. One invariant at a time; each with its RED case first.
