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

use crate::money::{apply_tax, assert_non_negative};
use crate::order_machine::{assert_transition, OrderStatus, TransitionError};

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
}

impl Order {
    /// Sum of `unit_price * quantity` across all items (the gross subtotal, pre-tax/fee).
    pub fn compute_subtotal(items: &[OrderItem]) -> i64 {
        items.iter().map(|i| i.unit_price * i.quantity).sum()
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
/// Returns `Err` if the resulting total would be negative (money invariant).
pub fn compute_order_total(
    subtotal: i64,
    tax_rate: f64,
    price_includes_tax: bool,
    fee: Option<i64>,
) -> Result<i64, String> {
    // `apply_tax` is infallible for integer (`i64`) subtotals (money.rs guards `% 1`,
    // which is always 0 for i64). We propagate its `Result` for interface fidelity.
    let tax = apply_tax(subtotal, tax_rate, price_includes_tax)?;
    let fee = fee.unwrap_or(0);
    let total = subtotal + tax + fee;
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
    let subtotal = Order::compute_subtotal(&items);
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
    })
}

/// The Decider's `fold` half for state: advance an order one step, validating the
/// transition against the kernel's state machine (`order_machine::assert_transition`).
///
/// Single-step only (the `fold` reduction is the caller's responsibility when replaying a
/// sequence of events). Returns a new `Order` with the updated status on success.
pub fn apply_event(order: &Order, next: OrderStatus) -> Result<Order, TransitionError> {
    assert_transition(order.status, next)?;
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
}
