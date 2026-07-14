# P7 — Operator-Apply Patch: route `create_order` through `kernel::decide`

> **RED-LINE. DO NOT apply without operator sign-off.**
> This document is the operator-apply patch referenced by `P7-DECIDE-GATE-FLAG.md`
> and exercised by the RED gate at `rebuild/crates/api/tests/decide_gateway.sh`.
> The api crate (`rebuild/crates/api/`) is **staging-dark** (not in the prod path) and
> is currently **PARKED** on branches `feat/sovereign-core-phase-zero` /
> `backup-wip-2026-07-08` — it is absent from the current branch. Apply only when the
> crate is restored to the build.

---

## 0. Corrections to the original flag sketch (READ FIRST)

The sketch in `P7-DECIDE-GATE-FLAG.md` wrote:

```rust
let cmd = Command::PlaceOrder { cart };
let order_state = OrderState::genesis();
let ctx = build_create_context(&txn, location_id).await?;
let events = match decide(&order_state, cmd, &ctx) { ... };
```

That sketch is **incomplete and will not compile**. Grounded in the real
`domain` crate (`rebuild/crates/domain/src/kernel.rs` at `56f1f872`):

1. `Command::PlaceOrder` has **three** fields, not one:
   ```rust
   PlaceOrder { at: Ts, actor: Actor, cart: Vec<pricing::PricingItem> }
   ```
2. `decide` for a `PlaceOrder` **requires** `ctx.pricing` to be `Some(PriceInputs{..})`
   — otherwise it returns `DomainError::CorridorBreach { corridor: "pricing", code: Internal }`
   (see `decide` body: `ctx.pricing.as_ref().ok_or(...)`). A `Context` built with
   `pricing: None` (like `build_transition_context` does) will be REJECTED.
3. `decide(_, Command::PlaceOrder{..}, _)` returns `vec![ price_cart(...) ]` =
   a single `Event::Priced { subtotal, delivery_fee, tax_total, total }`. The create
   arm must persist from that event, replacing the manual `compute_order_pricing` +
   `delivery_fee_for_order` + `apply_tax` + `charged_tax` + `compose_total` calls
   (pg.rs create arm, ~lines 287–343) — which are exactly what `price_cart` does
   internally (the mirror-oracle contract).

So `build_create_context` must assemble a **full `PriceInputs`** from the in-tx reads
the create arm already performs (product/modifier/group snapshot, `is_pickup`,
`FeeLocation`, delivery distance, delivery tiers, `rate_micro`, `price_includes_tax`).

---

## 1. Diff — `rebuild/crates/api/src/routes/orders/pg.rs` (create arm)

### 1a. Replace the direct pricing block with the `decide` door

Find (create arm, ~lines 287–343):

```rust
        let (subtotal, _priced_rows) =
            match pricing::compute_order_pricing(&pricing_items, &snapshot) {
                Ok(v) => v,
                Err(e) => {
                    txn.rollback().await.ok();
                    return Ok(CreateOutcome::Rejected(e.code, e.message));
                }
            };

        // 6. Delivery-fee ladder (needs tiers for delivery). MIN_ORDER gate inside.
        let tiers: Vec<DeliveryTier> = if is_pickup { Vec::new() } else {
            /* ... existing tier read ... */
        };
        let fee_location = FeeLocation { /* ... existing ... */ };
        let delivery_fee =
            match pricing::delivery_fee_for_order(subtotal, is_pickup, fee_location, pin, &tiers) {
                Ok(f) => f,
                Err(e) => { txn.rollback().await.ok(); return Ok(CreateOutcome::Rejected(e.code, e.message)); }
            };

        // 7. Tax + LC1 + total (REV-S5-4) ...
        let tax_i64 = pricing::apply_tax(subtotal.minor_units(), tax_rate.unwrap_or(0.0), price_includes_tax)
            .map_err(|_e| RepoError(sqlx::Error::Protocol("tax overflow".into())))?;
        let tax_total = Lek::new(tax_i64).map_err(|_e| RepoError(sqlx::Error::Protocol("tax negative".into())))?;
        let charged = pricing::charged_tax(tax_total, price_includes_tax);
        let total = pricing::compose_total(subtotal, delivery_fee, charged, Lek::ZERO)
            .map_err(|_e| RepoError(sqlx::Error::Protocol("total composition".into())))?;
```

Replace with:

```rust
        // ── P7 FIX: route CREATE through the single kernel::decide door ──
        // Mirror the transition arms (pg.rs:481-494 / :559-568): build the
        // PlaceOrder command + a genesis OrderState, route through decide, and
        // persist from the emitted Event::Priced (the create-only event).
        let now = Ts(chrono::Utc::now().timestamp_millis());
        let cmd = Command::PlaceOrder {
            at: now,
            actor: Actor::Owner, // checkout actor (anonymous customer owner of their own cart)
            cart: pricing_items.clone(),
        };
        let order_state = OrderState::genesis();
        let ctx = build_create_context(
            &snapshot, is_pickup, fee_location, pin, &tiers,
            tax_rate.unwrap_or(0.0), price_includes_tax,
        ).await;
        let events = match decide(&order_state, cmd, &ctx) {
            Ok(ev) => ev,
            Err(e) => {
                txn.rollback().await.ok();
                // pricing corridor breach carries the exact pricing ErrorCode
                return Ok(CreateOutcome::Rejected(
                    e.code,
                    format!("decide rejected create: {}", e.message),
                ));
            }
        };
        // Persist from the decided Priced event (do NOT re-read the manual totals).
        let Event::Priced { subtotal, delivery_fee, tax_total, total } = events
            .into_iter()
            .find_map(|e| match e {
                Event::Priced { subtotal, delivery_fee, tax_total, total } =>
                    Some((subtotal, delivery_fee, tax_total, total)),
                _ => None,
            })
            .expect("decide(PlaceOrder) must emit exactly one Event::Priced");
```

> The `pricing_items`, `snapshot`, `is_pickup`, `fee_location`, `pin`, `tiers`,
> `tax_rate`, `price_includes_tax` bindings already exist in the create arm
> (built at pg.rs ~255–343). Do **not** duplicate those reads — reuse them.

### 1b. Add the `build_create_context` helper (pure, no DB)

Place near `build_transition_context` (pg.rs ~1037). It assembles the `Context`
with a real `PriceInputs` (required by `decide` for `PlaceOrder`):

```rust
/// P7 FIX — build the OBSERVED `Context` for a `Command::PlaceOrder`.
/// `decide` REQUIRES `ctx.pricing` to be `Some(PriceInputs{..})` for a create,
/// or it returns `CorridorBreach`. All inputs are already integerized by the
/// caller (0b-1 boundary); this just packages them. No DB/clock/RNG.
fn build_create_context(
    snapshot: &PricingSnapshot,
    is_pickup: bool,
    location: FeeLocation,
    distance_m: Option<i64>,
    tiers: &[DeliveryTier],
    rate_micro: i64,
    price_includes_tax: bool,
) -> Context<'static> {
    Context {
        binding: policy::BindingState { has_active_binding: false, has_delivered_binding: false },
        refundable_paid: Lek::ZERO,
        pricing: Some(PriceInputs {
            snapshot,
            is_pickup,
            location,
            distance_m,
            tiers,
            rate_micro,
            price_includes_tax,
        }),
    }
}
```

> Signature note: `distance_m` is the whole-meter delivery distance the create arm
> already computes for `FeeLocation`/the fee ladder. If the create arm does not yet
> thread it through, compute it the same way the fee ladder does and pass it here.
> `Context<'static>` because `PriceInputs` borrows `snapshot`/`tiers` — if those are
> owned locally, scope them to outlive `ctx` (they already do: they're built before
> the `decide` call in the create arm).

---

## 2. Diff — `rebuild/crates/api/src/routes/orders/dto.rs` (add `Scheduled`)

Find (dto.rs:118-122):

```rust
/// `type: z.enum(['delivery','pickup'])` (`legacy.ts:42`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderType {
    Delivery,
    Pickup,
}
```

Replace with:

```rust
/// `type: z.enum(['delivery','pickup','scheduled'])` — mirrors the P2 prod fix in
/// `packages/shared-types/src/legacy.ts:42` (added `scheduled`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderType {
    Delivery,
    Pickup,
    Scheduled,
}
```

> The `pg` INSERT already binds `$3::order_type` as text (pg.rs `CREATE_ORDER_SQL`,
> ~line 51), and Postgres coerces a bound lowercase text `'scheduled'` to the enum
> once the variant exists — no SQL/DDL change needed. The commented mapping at
> dto.rs:117 cited the *old* 2-valued `legacy.ts:42`; update that comment too.

---

## 3. Verification after applying

Run from the workspace root (api crate restored to build):

```bash
# In-crate RED gate (turns GREEN once the create arm calls decide):
cargo test -p api decide_gateway

# Offline / CI fallback RED gate (shell, works without a DB or the full build):
bash rebuild/crates/api/tests/decide_gateway.sh
```

Both must exit **0 (GREEN)**. Before applying, `decide_gateway.sh` exits **1 (RED)**
with the exact bypass citations — that is the expected pre-sign-off state.

---

## 4. RED-LINE DOCTRINE

- This patch is **operator-applied only**. The flagging agent must NOT edit
  `pg.rs` / `dto.rs` / any `rebuild/crates/api/src/*.rs`.
- The api crate is staging-dark; this is a correctness/hardening flag for when the
  kernel checkout is ramped via `HUB_CHECKOUT=true`.
- After applying, re-run the gate AND the existing money-column persisted test
  (`tests::create_order_sql_persists_every_computed_money_column`) to confirm the
  decided `Event::Priced` values match the prior manual computation (mirror oracle).
