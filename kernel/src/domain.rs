//! Domain aggregate: `Order` + `compute_order_total` + the `place_order`/`apply_event` Decider.
//!
//! This is the `decide/fold` Law applied to the Order aggregate (the kernel's
//! canonical domain object). It is a 1:1 port of the oracle (`apps/api/src/routes/orders.ts`
//! + `packages/shared-types/src/legacy.ts`) restricted to the kernel's scope:
//!
//! - `Order` / `OrderItem` mirror the oracle shapes (legacy.ts `OrderResponse`,
//!   `OrderItemResponse`, `CreateOrderInput`). Integer money only (`i64` minor units).
//! - `compute_order_total` reuses `money::apply_tax` (tax on the subtotal only, exactly
//!   as the oracle computes `taxTotal = applyTax(subtotal, ...)` at orders.ts:563) and adds
//!   an optional flat `fee` (the delivery fee slot). No discounts in this scope.
//! - The Decider creates an aggregate in `Pending` (`place_order`) and advances it one step
//!   at a time via `apply_event`, delegating the legality check to
//!   `order_machine::assert_transition` — the same transition table the oracle honors.
//!
//! RED LINE: no float on monetary values; no courier scoring/rating fields; `std` only,
//! no external dependencies. `order_machine.rs` / `money.rs` are NOT modified.

use crate::catalog::PriceCatalog;
use crate::money::{apply_tax, assert_non_negative};
use crate::order_machine::{assert_transition, verify_fsm_signature, OrderStatus, TransitionError};

/// A line item on an order. Mirrors the oracle `OrderItemInput` / `OrderItemResponse`.
/// `unit_price` is integer minor units at the moment of purchase (price snapshot).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderItem {
    pub product_id: String,
    pub modifier_ids: Vec<String>,
    pub quantity: i64,
    pub unit_price: i64,
}

/// The Order aggregate. Status enum is `crate::order_machine::OrderStatus` — kept
/// byte-for-byte identical to the oracle `OrderStatusEnum` (legacy.ts).
///
/// Deliberately NO courier scoring / rating fields (kernel scope: pure order domain).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    pub id: String,
    pub customer_id: Option<String>,
    pub status: OrderStatus,
    pub items: Vec<OrderItem>,
    pub subtotal: i64,
    pub total: i64,
    pub created_at_ms: i64,
    pub channel: Option<String>,
    /// Exact oracle field (`cash_pay_with`, legacy.ts `CreateOrderInput`). Stored as the
    /// string the oracle uses (integer minor units serialized as a string on this kernel
    /// surface); kept as `Option<String>` per the task's canonical Order shape.
    pub cash_pay_with: Option<String>,
    /// M1/M2 money-integrity flag: `true` when every line `unit_price` was
    /// RE-DERIVED from the trusted [`PriceCatalog`] (client value ignored);
    /// `false` on the legacy path where the caller price was accepted verbatim.
    /// Downstream (charge/settlement) MUST refuse to charge an untrusted order.
    pub price_trusted: bool,
}

impl Order {
    /// Sum of `unit_price * quantity` across all items (the gross subtotal, pre-tax/fee).
    /// Overflow-safe (BP-17): returns `Err` instead of panicking/wrapping.
    pub fn compute_subtotal(items: &[OrderItem]) -> Result<i64, String> {
        let mut sum: i64 = 0;
        for i in items {
            let line = i
                .unit_price
                .checked_mul(i.quantity)
                .ok_or("subtotal overflow (unit_price * quantity)")?;
            sum = sum.checked_add(line).ok_or("subtotal overflow (sum)")?;
        }
        Ok(sum)
    }

    /// Recompute `self.total` from the subtotal via `compute_order_total`.
    /// `fee` is the flat (delivery) fee slot; pass `None` when there is none.
    pub fn recompute_total(
        &mut self,
        tax_rate: f64,
        price_includes_tax: bool,
        fee: Option<i64>,
    ) -> Result<(), String> {
        self.total = compute_order_total(self.subtotal, tax_rate, price_includes_tax, fee)?;
        Ok(())
    }
}

/// Total = subtotal + tax(subtotal) + fee.
///
/// Mirrors the oracle's `total = subtotal + deliveryFee + taxTotal - discountTotal`
/// (orders.ts:565), restricted to this scope: tax on the subtotal only (no discount),
/// `fee` is the flat fee slot (delivery fee), and `None` fee means zero.
///
/// Returns `Err` if the resulting total would overflow or go negative (money invariant).
/// Overflow-safe (BP-17): every addition uses `checked_add`.
pub fn compute_order_total(
    subtotal: i64,
    tax_rate: f64,
    price_includes_tax: bool,
    fee: Option<i64>,
) -> Result<i64, String> {
    let tax = apply_tax(subtotal, tax_rate, price_includes_tax)?;
    let fee = fee.unwrap_or(0);
    let with_tax = subtotal
        .checked_add(tax)
        .ok_or("total overflow (subtotal + tax)")?;
    let total = with_tax
        .checked_add(fee)
        .ok_or("total overflow (subtotal + tax + fee)")?;
    assert_non_negative(total)?;
    Ok(total)
}

/// The Decider's `decide` half for order creation: place a new order in `Pending`.
///
/// `Pending` is the canonical genesis status honored by the transition table
/// (`order_machine::allowed_next` treats it as the start of every lifecycle). The
/// aggregate's `total` is initialized to the subtotal and refined later by
/// `recompute_total` once tax/fee are known.
///
/// Returns `Ok(Order)` in the normal case; the `Result` is kept for interface symmetry
/// and future validation (non-negative subtotal, etc.).
pub fn place_order(
    id: String,
    customer_id: Option<String>,
    items: Vec<OrderItem>,
    created_at_ms: i64,
    channel: Option<String>,
    cash_pay_with: Option<String>,
) -> Result<Order, TransitionError> {
    let _span = tracing::info_span!(
        "place_order",
        id = %id,
        n_items = items.len(),
        channel = ?channel
    )
    .entered();
    let subtotal = Order::compute_subtotal(&items).map_err(TransitionError::Invalid)?;
    tracing::debug!(subtotal_cents = subtotal, "order subtotal computed");
    Ok(Order {
        id,
        customer_id,
        status: OrderStatus::Pending,
        items,
        subtotal,
        total: subtotal,
        created_at_ms,
        channel,
        cash_pay_with,
        // Legacy path: caller-supplied unit_price accepted verbatim → UNTRUSTED.
        price_trusted: false,
    })
}

/// M1/M2 — catalog-authoritative order creation. Every line's `unit_price` is
/// RE-DERIVED from the trusted [`PriceCatalog`]; the caller-supplied `unit_price`
/// on each `OrderItem` is IGNORED (overwritten). This closes the money-integrity
/// gap where a client could set its own price.
///
/// Fail-closed: if any product is unknown to the catalog, the whole order is
/// rejected (`Err`) — an order is never priced from an untrusted client value
/// when a trusted catalog is in force. The resulting order has
/// `price_trusted = true`.
pub fn place_order_priced(
    id: String,
    customer_id: Option<String>,
    mut items: Vec<OrderItem>,
    created_at_ms: i64,
    channel: Option<String>,
    cash_pay_with: Option<String>,
    catalog: &PriceCatalog,
) -> Result<Order, TransitionError> {
    let _span = tracing::info_span!(
        "place_order_priced",
        id = %id,
        n_items = items.len(),
        channel = ?channel
    )
    .entered();
    // Re-derive every line price from the trusted catalog (ignore caller value).
    for it in items.iter_mut() {
        let trusted = catalog
            .unit_price(&it.product_id, &it.modifier_ids)
            .map_err(TransitionError::Invalid)?;
        it.unit_price = trusted;
    }
    let subtotal = Order::compute_subtotal(&items).map_err(TransitionError::Invalid)?;
    tracing::debug!(subtotal_cents = subtotal, "order subtotal (catalog-authoritative)");
    Ok(Order {
        id,
        customer_id,
        status: OrderStatus::Pending,
        items,
        subtotal,
        total: subtotal,
        created_at_ms,
        channel,
        cash_pay_with,
        // Every unit_price came from the trusted catalog → TRUSTED.
        price_trusted: true,
    })
}

/// The Decider's `fold` half for state: advance an order one step, validating the
/// transition against the kernel's state machine (`order_machine::assert_transition`).
///
/// Single-step only (the `fold` reduction is the caller's responsibility when replaying a
/// sequence of events). Returns a new `Order` with the updated status on success.
///
/// **FSM drift gate (fail-closed).** After a *successful* per-order transition we re-run
/// `verify_fsm_signature()`. A single-order fold cannot by construction alter
/// `allowed_next`/the lifecycle graph, so this must *always* return `Ok(())`. If it returns
/// `Err`, the topology underneath the running kernel changed out from under us (a hot-reload,
/// a stray edit reaching production, a corrupted build) — a structural regression, not a
/// per-order error. We treat it fail-closed: the fold is refused and the diverged fields are
/// surfaced. This is the blueprint §4 post-fold check — the gate fires at the earliest point
/// a topology drift could be exercised.
pub fn apply_event(order: &Order, next: OrderStatus) -> Result<Order, TransitionError> {
    assert_transition(order.status, next)?;
    // Fail-closed topology re-check: a successful fold must not move the golden signature.
    if let Err(drift) = verify_fsm_signature() {
        return Err(TransitionError::Invalid(format!(
            "fsm signature drift after fold: {drift} (lifecycle topology changed at runtime)"
        )));
    }
    let mut updated = order.clone();
    updated.status = next;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_items() -> Vec<OrderItem> {
        vec![
            OrderItem {
                product_id: "p1".into(),
                modifier_ids: vec!["m1".into()],
                quantity: 2,
                unit_price: 500,
            },
            OrderItem {
                product_id: "p2".into(),
                modifier_ids: vec![],
                quantity: 1,
                unit_price: 300,
            },
        ]
    }

    // ── RED: illegal aggregate transitions must be rejected ──
    #[test]
    fn red_illegal_pending_to_ready() {
        let o = place_order(
            "o1".into(),
            Some("c1".into()),
            sample_items(),
            0,
            None,
            None,
        )
        .unwrap();
        // Pending → Ready is not in the allowed transition table.
        assert!(matches!(
            apply_event(&o, OrderStatus::Ready),
            Err(TransitionError::Illegal(_, _))
        ));
    }

    #[test]
    fn red_terminal_order_cannot_advance() {
        // Drive to a terminal state (via the legal path), then attempt to move it — rejected.
        let o = place_order("o2".into(), None, sample_items(), 0, None, None).unwrap();
        let o = apply_event(&o, OrderStatus::Confirmed).unwrap();
        let o = apply_event(&o, OrderStatus::Preparing).unwrap();
        let o = apply_event(&o, OrderStatus::Ready).unwrap();
        let o = apply_event(&o, OrderStatus::InDelivery).unwrap();
        let o = apply_event(&o, OrderStatus::Delivered).unwrap();
        assert!(o.status.is_terminal());
        // Delivered → anything is illegal (terminal has no outgoing edges).
        assert!(matches!(
            apply_event(&o, OrderStatus::Confirmed),
            Err(TransitionError::Illegal(_, _))
        ));
    }

    #[test]
    fn red_same_status_is_rejected() {
        let o = place_order("o3".into(), None, sample_items(), 0, None, None).unwrap();
        assert!(matches!(
            apply_event(&o, OrderStatus::Pending),
            Err(TransitionError::SameStatus(_))
        ));
    }

    // ── GREEN: happy lifecycle matches the oracle exactly ──
    #[test]
    fn green_happy_lifecycle_pending_to_delivered() {
        let mut o = place_order(
            "o4".into(),
            Some("c2".into()),
            sample_items(),
            1_700_000_000_000,
            Some("web".into()),
            Some("5000".into()),
        )
        .unwrap();
        assert_eq!(o.status, OrderStatus::Pending);
        assert_eq!(o.subtotal, 2 * 500 + 1 * 300); // 1300
        assert_eq!(o.total, o.subtotal); // provisional until tax/fee applied

        o = apply_event(&o, OrderStatus::Confirmed).unwrap();
        o = apply_event(&o, OrderStatus::Preparing).unwrap();
        o = apply_event(&o, OrderStatus::Ready).unwrap();
        o = apply_event(&o, OrderStatus::InDelivery).unwrap();
        o = apply_event(&o, OrderStatus::Delivered).unwrap();
        assert_eq!(o.status, OrderStatus::Delivered);
        assert!(o.status.is_terminal());
    }

    #[test]
    fn green_pickup_path_ready_to_pickedup() {
        let o = place_order("o5".into(), None, sample_items(), 0, None, None).unwrap();
        let o = apply_event(&o, OrderStatus::Confirmed).unwrap();
        let o = apply_event(&o, OrderStatus::Preparing).unwrap();
        let o = apply_event(&o, OrderStatus::Ready).unwrap();
        let o = apply_event(&o, OrderStatus::PickedUp).unwrap();
        assert_eq!(o.status, OrderStatus::PickedUp);
    }

    // ── GREEN: total math (subtotal + tax + fee) ──
    #[test]
    fn green_total_exclusive_tax_plus_fee() {
        // subtotal 1000, 20% tax => 200, fee 50 => total 1250
        assert_eq!(
            compute_order_total(1000, 0.20, false, Some(50)).unwrap(),
            1250
        );
    }

    #[test]
    fn green_total_inclusive_tax_no_fee() {
        // subtotal 1200 inclusive 20% => tax 200, fee 0 => total 1400
        assert_eq!(compute_order_total(1200, 0.20, true, None).unwrap(), 1400);
    }

    #[test]
    fn green_total_no_fee_defaults_to_zero() {
        // subtotal 1000, 10% tax => 100, no fee => total 1100
        assert_eq!(compute_order_total(1000, 0.10, false, None).unwrap(), 1100);
    }

    #[test]
    fn green_order_recompute_total_ties_subtotal_and_tax() {
        let mut o = place_order("o6".into(), None, sample_items(), 0, None, None).unwrap();
        // subtotal = 1300; 20% exclusive tax => 260; no fee => total 1560
        o.recompute_total(0.20, false, None).unwrap();
        assert_eq!(o.subtotal, 1300);
        assert_eq!(o.total, 1300 + 260);
    }

    #[test]
    fn green_status_enum_matches_oracle() {
        // The kernel enum must stay identical to legacy.ts `OrderStatusEnum`.
        assert_eq!(OrderStatus::Pending.as_str(), "PENDING");
        assert_eq!(OrderStatus::PickedUp.as_str(), "PICKED_UP");
        assert_eq!(
            OrderStatus::from_str("IN_DELIVERY"),
            Some(OrderStatus::InDelivery)
        );
    }

    // ── M1/M2: catalog is the price authority ──────────────────────────────
    use crate::catalog::PriceCatalog;

    fn trusted_catalog() -> PriceCatalog {
        let mut c = PriceCatalog::new();
        c.insert_flat("p1", 5000); // real price
        c.insert_flat("p2", 300);
        c
    }

    // RED signature: the legacy path trusts the tampered client price.
    #[test]
    fn red_legacy_place_order_trusts_client_price() {
        let tampered = vec![OrderItem {
            product_id: "p1".into(),
            modifier_ids: vec![],
            quantity: 1,
            unit_price: 1, // attacker sets price=1 for a 5000 product
        }];
        let o = place_order("bad".into(), None, tampered, 0, None, None).unwrap();
        assert_eq!(o.subtotal, 1, "legacy path accepts tampered price (RED)");
        assert!(!o.price_trusted, "legacy order must be flagged untrusted");
    }

    // GREEN: the catalog path OVERRIDES the tampered client price.
    #[test]
    fn green_catalog_overrides_tampered_price() {
        let cat = trusted_catalog();
        let tampered = vec![
            OrderItem {
                product_id: "p1".into(),
                modifier_ids: vec![],
                quantity: 1,
                unit_price: 1, // ignored
            },
            OrderItem {
                product_id: "p2".into(),
                modifier_ids: vec![],
                quantity: 2,
                unit_price: 0, // ignored
            },
        ];
        let o = place_order_priced("ok".into(), None, tampered, 0, None, None, &cat).unwrap();
        // p1: 5000*1 + p2: 300*2 = 5600 — from the catalog, NOT the client.
        assert_eq!(o.subtotal, 5600, "catalog re-derives the trusted price");
        assert!(o.price_trusted, "catalog order is trusted");
        assert_eq!(o.items[0].unit_price, 5000);
        assert_eq!(o.items[1].unit_price, 300);
    }

    // Fail-closed: an unknown product is rejected, never priced from client value.
    #[test]
    fn red_catalog_rejects_unknown_product() {
        let cat = trusted_catalog();
        let items = vec![OrderItem {
            product_id: "ghost".into(),
            modifier_ids: vec![],
            quantity: 1,
            unit_price: 999,
        }];
        assert!(place_order_priced("x".into(), None, items, 0, None, None, &cat).is_err());
    }
}
