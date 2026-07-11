# P7 — Rust checkout bypasses `kernel::decide` (RED-LINE FLAG)

> **RED-LINE. DO NOT change code. Flag + exact fix description for operator sign-off only.**

## Verdict: 🔴 CONFIRMED (proven by reading the api crate's create funnel)

The Rust `rebuild/` sovereign core defines `Command::PlaceOrder` and a single `kernel::decide`
door, but the **api crate's `create_order` path never constructs `PlaceOrder` and never calls
`decide`**. It prices the cart by calling `compute_order_pricing` *directly* and writes the row.
The `decide` door is wired **only** into the order *status-transition* paths, not into order
*creation*. So the create funnel skips the `decide` corridor (actor-gate, CC-1 strand guard,
LC1 conservation) that the core authors intended to own centrally.

## Proof (file:line)

1. **`create_order` in the api crate uses the pricing fn directly, not `decide`**
   — `rebuild/crates/api/src/routes/orders/pg.rs:91` (`impl OrdersRepo::create_order`)
   — `rebuild/crates/api/src/routes/orders/pg.rs:292`
   ```rust
   let (subtotal, _priced_rows) =
       match pricing::compute_order_pricing(&pricing_items, &snapshot) { ... };
   ```
   No `Command::PlaceOrder { .. }` is built and no `decide(&order_state, cmd, &ctx)` is called in
   this function.

2. **`decide` IS called — but only on status transitions, never on create**
   — `rebuild/crates/api/src/routes/orders/pg.rs:491` and `:567`
   ```rust
   let events = match decide(&order_state, cmd, &ctx) { ... };
   ```
   Both sites are inside the `update_order_status` / transition arms (the comment at `:474-480`
   calls it "The ONE `decide` door (0b-5 shell flip)"). There is **no** `decide(...)` call in the
   create arm (lines 91-~460).

3. **`Command::PlaceOrder` is never constructed in the api crate**
   `grep` for `Command::PlaceOrder` across `rebuild/crates/api/src/**` returns **zero** hits.
   The only `Command::*` constructions in `pg.rs` are transition commands built by
   `owner_command_for(...)` at `pg.rs:1088-1094` (`Confirm`, `Reject`, `StartPreparing`, …) —
   all status changes, none a `PlaceOrder`.

4. **The core still defines the intended create door**
   — `rebuild/crates/domain/src/kernel/validate.rs:139` shows `validate` *does* branch on
   `Command::PlaceOrder { cart, .. }`, and the docs (`checkout.rs:13`) state the create funnel is
   "the SAME kernel math `domain::decide` runs for a `domain::Command::PlaceOrder`
   (`price_cart`)". The implementation diverged from that spec: the api crate prices via
   `compute_order_pricing` without going through `decide`/`validate`.

## Exact change needed (operator-applied, NOT by this agent)

Wire the create funnel through the same `decide` door the transitions use:

```rust
// In PgOrdersRepo::create_order (rebuild/crates/api/src/routes/orders/pg.rs, ~line 292):
// Build the PlaceOrder command + a genesis OrderState, then route through decide.
let cart: Vec<PricingItem> = pricing_items.clone();        // validated cart
let cmd = Command::PlaceOrder { cart };
let order_state = OrderState::genesis();
let ctx = build_create_context(&txn, location_id).await?;  // observed context (pricing snapshot already loaded)
let events = match decide(&order_state, cmd, &ctx) {
    Ok(ev) => ev,
    Err(e) => { txn.rollback().await.ok(); return Ok(CreateOutcome::Rejected(e.code, e.message)); }
};
// then persist using `events` (the decided `Priced` event) instead of the raw compute_order_pricing output.
```

Precise shape must mirror how the transition arms consume `decide` (`pg.rs:481-494`): build
`OrderState`, call `decide`, and persist from the emitted `Event::Priced` (the create-only event,
noted at `pg.rs:808` "Event::Priced (PlaceOrder only) never reaches this transition path").

### Additional gap discovered (flag, same area)
The Rust `OrderType` DTO is **2-valued**, missing the `scheduled` kind the P2 prod fix now
requires in `packages/shared-types/src/legacy.ts:42`:
- `rebuild/crates/api/src/routes/orders/dto.rs:120-122`
  ```rust
  pub enum OrderType { Delivery, Pickup }   // ← no Scheduled
  ```
- The commented mapping at `dto.rs:117` even cites the *old* 2-valued `legacy.ts:42`.
  So even after the `decide` gate is wired, the Rust checkout would **400 on `scheduled` orders**.
  The `scheduled` variant must be added to `OrderType` (and the `pg` INSERT enum cast at
  `pg.rs:51-55` already binds `type` as text → fine once the enum + DTO accept it). This is the
  Rust-side counterpart of the P2 fix already committed in `legacy.ts`.

## Unverified gaps
- Whether some *other* crate (e.g. a worker or a separate "sovereign" service) constructs
  `Command::PlaceOrder` and calls `decide` for creates — `grep` covered `rebuild/crates/api/src`
  and `rebuild/crates/domain`; a `PlaceOrder`/`decide` create path elsewhere is not ruled out, but
  the live HTTP checkout handler (`create_order`) is the one that matters and it does not.
- The exact `build_create_context` helper does not yet exist; the transition path uses
  `build_transition_context` (`pg.rs:481`). A create-context builder (pricing snapshot is already
  loaded at `:287-290`) must be added — left to the implementer.
- The Rust api crate is **staging-dark** (not in prod path — see P9 report); P7 is a correctness/
  hardening flag for when the kernel checkout is ramped via `HUB_CHECKOUT=true`.
