# Phase-Zero Step 3 — Sovereign Extraction: execution plan (for a fresh session)

**Why this is a handoff, not done-here:** Step 3 relocates 3 modules (incl. the 884-line pricing) and
recompiles the heavy `api` crate (sqlx/tokio/axum) to verify — a large, api-touching task. Per the
anti-context-rot rule, it runs in a FRESH session (this one is long). On branch
`feat/sovereign-core-phase-zero`, the S5 batch is NOT present, so the old S5 collision block does not
apply here — Step 3 is unblocked on this branch.

## What moves (strangler, one module per commit)

| From (api shell) | To (core) | Notes |
|---|---|---|
| `crates/api/src/routes/orders/request_hash.rs` (~310 ln) | `crates/domain/src/codec/request_hash.rs` (or `codec.rs` submodule) | Canonical command bytes = the signing surface. If it uses `sha2`/`hex`, add them to `dowiz-core` deps — both are pure + wasm-clean (sha2 pre-blessed in PHASE-ZERO §3). |
| `crates/api/.../state.rs` — `assert_owner_target_allowed`, `TransitionEffects`+`transition_effects`, `LifecycleEvent`, `cc1_strand_guard`+`BindingState`, `needs_honest_dispatch`, `idempotency_decision`+`IdempotencyDecision`+`ExistingKey` | `crates/domain/src/kernel/policy.rs` + `kernel/idempotency.rs` | All pure (side-effect-free decisions). |
| **STAYS in api:** `state.rs::classify_pg_error`+`PgErrorClass` | — | SQLSTATE is Postgres vocabulary; platform words never enter the core. |
| `crates/api/.../pricing.rs` (884 ln) | `crates/domain/src/kernel/pricing.rs` | Pure money composition (applyTax, LC1, compute_order_pricing, delivery_fee_for_order). Already uses `domain::Lek`. The crown jewel — do it LAST, after the pattern is proven on the smaller two. |
| **STAYS in api:** `pg.rs::apply_transition` + all repos | — | The SQL interpreter of kernel output — the one legitimate home for IO. |

## Per-module loop (each is one commit)

1. `git mv` the file into `crates/domain/src/...`; add its `pub mod` to `lib.rs`; move its `#[cfg(test)]`
   tests with it.
2. Update the api side: the handlers/`pg.rs`/`mod.rs` that called `super::state::X` / `super::pricing::X`
   now call `domain::kernel::X` (or the re-export). Keep a thin `pub use domain::... as ...` shim in the
   api orders module if it minimizes churn.
3. **Verify (all must pass):**
   - `bash rebuild/scripts/sovereign-gate.sh` — the relocated module must stay wasm/clock/entropy-clean
     (if pricing/request_hash pull a non-pure dep, that's a real finding — fix or leave in shell).
   - `cargo test --manifest-path rebuild/crates/domain/Cargo.toml` — moved tests green.
   - `cargo check --manifest-path rebuild/crates/api/Cargo.toml` — **the shell still compiles** with the
     new imports (the heavy but essential check; ~minutes first time).
4. Commit (`feat(sovereign-core): extract <module> into core`), push.

## After all three moved — wire the kernel (the point of extraction)

Extend `kernel::decide` so it composes the now-in-core corridors, in this order (matches the live
handler): `assert_transition` (machine) → `assert_owner_target_allowed` (actor-gate) → `cc1_strand_guard`
→ pricing/LC1 conservation corridor → emit events. Grow `OrderState`/`Event` to carry money
(`Lek` total, a `Priced`/`RefundObligated` event) and `Command::PlaceOrder{cart,…}`. Extend the Hard
Truth suite: LC1 no-double-tax + conservation as proptest corridors over the real composition.

## Guardrails
- Forward-only, no behavior change during the move (byte-identical logic; the move is mechanical).
- If a "pure" module turns out to touch IO/clock/entropy, the wasm gate WILL reject it — that's the
  gate doing its job; leave that piece in the shell and note why.
- Money is a red-line: the pricing move gets the doubt-pass + the LC1/integer invariants re-proven.
