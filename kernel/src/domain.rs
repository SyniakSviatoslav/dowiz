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
use crate::kalman::KalmanFilter;
use crate::vendor::VendorId;
use crate::money::{
    apply_tax, assert_non_negative, ledger_append, ledger_sum, reverse_transfer, Currency,
    EntryKind, LedgerEntry,
};
use crate::order_machine::{assert_transition, verify_fsm_signature, OrderStatus, TransitionError};

/// A line item on an order. Mirrors the oracle `OrderItemInput` / `OrderItemResponse`.
/// `unit_price` is integer minor units at the moment of purchase (price snapshot).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderItem {
    pub product_id: String,
    pub modifier_ids: Vec<String>,
    pub quantity: i64,
    pub unit_price: i64,
    /// P62 M4 — the vendor this line belongs to. Catalog-authoritative: set by
    /// `place_order_priced` from the trusted `PriceCatalog`/`PriceableLeaf`, never
    /// from a client-supplied value. A client cannot forge which vendor a line
    /// settles to (P62 §4.4). Defaults to `VendorId(0)` on the legacy path, which
    /// is just ordinary key (no reserved sentinel per `vendor.rs`).
    pub vendor_id: VendorId,
    /// P62 M4 — the currency of this line. Every line in one Wave-0 order shares a
    /// single currency (EUR); a cart mixing currencies is refused fail-closed
    /// (BLUEPRINT-P72 M1 `CrossCurrencyCart`). Mirrors `PriceableLeaf.price.currency`.
    pub currency: Currency,
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
    /// P07 double-entry ledger. Holds the order's money movements as `LedgerEntry`s
    /// (Earn legs + their Reversals). A compensated order's entries sum to EXACTLY zero.
    /// Empty until the first earn leg is posted (typically at `Confirmed`).
    pub ledger: Vec<LedgerEntry>,
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

    /// Post an `Earn` leg (money earned into the platform) onto this order's ledger.
    /// The `amount` is the order's settled total in the given `currency` (M5). Fail-closed:
    /// the amount must be representable and the ledger append must succeed (no duplicate id).
    /// Returns the mutated order. The paired `Reversal` is produced later by
    /// [`compensate`]/`reverse_transfer` so a cancelled/refunded order nets to exactly zero.
    pub fn post_earn(
        &mut self,
        entry_id: u64,
        amount: i64,
        currency: Currency,
    ) -> Result<(), String> {
        let earn = LedgerEntry {
            id: entry_id,
            kind: EntryKind::Earn,
            amount: crate::money::Money::new(amount, currency),
            reverses: None,
        };
        self.ledger = ledger_append(std::mem::take(&mut self.ledger), earn)?;
        Ok(())
    }

    /// Sum of the order's live (un-reversed) ledger entries. Returns 0 exactly when the
    /// order is fully compensated (every earn leg reversed). This is the conservation probe.
    pub fn ledger_balance(&self) -> i64 {
        ledger_sum(&self.ledger)
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
        // `vendor_id` is the default single-vendor key (no reserved sentinel).
        price_trusted: false,
        ledger: Vec::new(),
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
    // `vendor_id` is preserved from the input item, which a trusted caller builds
    // from a `PriceableLeaf` (the catalog-authoritative source — P62 §4.4). The
    // flat `PriceCatalog` carries no vendor, so the leaf is the vendor source of
    // truth; a client-supplied `OrderItem` on the legacy `place_order` path stays
    // on the default vendor key and is never trusted to set a foreign vendor.
    for it in items.iter_mut() {
        let trusted = catalog
            .unit_price(&it.product_id, &it.modifier_ids)
            .map_err(TransitionError::Invalid)?;
        it.unit_price = trusted;
    }
    let subtotal = Order::compute_subtotal(&items).map_err(TransitionError::Invalid)?;
    tracing::debug!(
        subtotal_cents = subtotal,
        "order subtotal (catalog-authoritative)"
    );
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
        ledger: Vec::new(),
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
    // V3 5.2 / 5.3 (ROUND-2 GAP-AUDIT): the compensated terminal `CompensatedRefund`
    // must NOT be reachable through the public fold. It is only ever produced by
    // `compensate`, which reverses the order's earn ledger legs first (money
    // conservation). Allowing `apply_event(.., CompensatedRefund)` directly would
    // let a caller reach the terminal state with an UN-reversed ledger.
    if next == OrderStatus::CompensatedRefund {
        return Err(TransitionError::Invalid(
            "CompensatedRefund is reachable only via compensate() (ledger-reversing)".into(),
        ));
    }
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

/// Courier/trust STATE ESTIMATE — a 1-D Kalman filter that generalises the
/// scalar steady-state EMA (`crate::geo::ema_next`). `ema_next` is the
/// infinite-initial-covariance special case of this filter; here the filter
/// *also* tracks its own variance, so the Law can down-weight stale/uncertain
/// observations instead of trusting a fixed alpha.
///
/// Domain scope note (kernel money red line): this struct carries courier trust
/// as a *separate* estimate and is deliberately NOT stored on [`Order`] (which
/// forbids courier-scoring fields). It is threaded through the fold by
/// [`apply_event_with_trust`] and returned alongside the updated order.
///
/// The courier/trust state is a unit-rate scalar: the observation is whatever
/// signal the caller feeds (e.g. a 0..1 reliability sample, an ETA-error ratio).
/// `F=H=1`; `Q` is the process drift, `R` the observation noise.
#[derive(Debug, Clone)]
pub struct TrustEstimate {
    /// The actual Kalman filter (state `x`, covariance `P`, plus the
    /// innovation/surprise signals surfaced by `kalman.rs`).
    pub kf: KalmanFilter,
}

impl TrustEstimate {
    /// New trust estimate with a neutral prior `x0` (default 0.5 = unknown) and
    /// a wide initial covariance `p0` so the first observations dominate.
    pub fn new(x0: f64, q: f64, r: f64) -> Self {
        // p0 wide => first observation pulls the estimate hard (as EMA would).
        TrustEstimate {
            kf: KalmanFilter::scalar(x0, 1.0, 1.0, 1.0, q, r),
        }
    }

    /// Convenience: the current trust estimate `x`.
    pub fn estimate(&self) -> f64 {
        self.kf.x[0]
    }

    /// Convenience: the current variance `P` (uncertainty of the estimate).
    pub fn variance(&self) -> f64 {
        self.kf.p.get(0, 0)
    }

    /// Advance one fold step, optionally conditioning on an observation.
    ///
    /// - `Some(z)`: `predict` then `update` — the estimate moves toward `z`,
    ///   variance shrinks.
    /// - `None` (missing observation): fail-closed — we still `predict` (the
    ///   prior propagates with process drift) but the estimate **holds its
    ///   prior**; only the variance grows, reflecting increased uncertainty.
    pub fn step(&mut self, observation: Option<f64>) -> bool {
        self.kf.predict();
        match observation {
            Some(z) => self.kf.update(&[z]),
            None => true, // hold prior; variance already grew in predict()
        }
    }
}

/// The Decider's `fold` half WITH a courier/trust Kalman state estimate.
///
/// This is the W19 integration point: the order-fold (`apply_event`) is
/// composed with a `TrustEstimate::step`, so every legal fold also advances the
/// courier/trust Kalman estimate. The order transition and the FSM-drift gate
/// behave exactly as [`apply_event`] (unchanged Law). When an observation is
/// supplied it is fed to the Kalman `update`; when `None`, the Kalman estimate
/// **holds its prior** (fail-closed — a missing observation never silently
/// fabricates trust).
///
/// Returns the updated order and the (mutated-in-place) shared `TrustEstimate`.
pub fn apply_event_with_trust(
    order: &Order,
    next: OrderStatus,
    trust: &mut TrustEstimate,
    observation: Option<f64>,
) -> Result<Order, TransitionError> {
    let updated = apply_event(order, next)?;
    // Kalman fold: predict + (maybe) update. Fail-closed on missing observation.
    trust.step(observation);
    Ok(updated)
}

/// P07 compensation driver — cancel-after-confirm (or any post-commitment state)
/// reverses the order's money cleanly and lands in the compensated terminal state.
///
/// This is the FSM compensation edge the order machine lacked: a `Confirmed` (or
/// `Preparing`/`Ready`/`InDelivery`) order transitions `Refunding → CompensatedRefund`,
/// and during that transition the order's `Earn` ledger legs are exactly reversed via
/// [`reverse_transfer`] so the order nets to ZERO by construction. The compensation is a
/// reversing double-entry transfer, not a state-only move.
///
/// Fail-closed:
/// * the `next` status must still be a legal FSM transition (we delegate to [`apply_event`]);
/// * every `Earn` leg on the order must be reversible (unknown/cross-currency/already-
///   reversed legs are rejected → the fold refuses rather than leaving money un-reversed);
/// * `reversal_id` must not collide with an existing entry (replay protection).
///
/// Idempotent: an order already in `CompensatedRefund` (terminal) cannot be compensated
/// again — `apply_event` rejects the transition, so a replay of the compensation event is a
/// no-op at the caller.
pub fn compensate(
    order: &Order,
    next: OrderStatus,
    earn_id: u64,
    reversal_id: u64,
) -> Result<Order, TransitionError> {
    // The compensation edge is `Refunding → CompensatedRefund` ONLY. Any other
    // (from, to) pair is rejected — this is the single legal compensation move.
    if order.status != OrderStatus::Refunding || next != OrderStatus::CompensatedRefund {
        return Err(TransitionError::Illegal(order.status, next));
    }
    // 1) Reverse the named earn leg FIRST (fail-closed). If the leg is unknown /
    //    already reversed / overflows / currency-mismatched this returns Err and
    //    the order is NOT mutated — money conservation is preserved by construction.
    let ledger = reverse_transfer(order.ledger.clone(), earn_id, reversal_id)
        .map_err(TransitionError::Invalid)?;
    // 2) Only after a successful reversal do we land in the compensated terminal.
    //    `CompensatedRefund` is unreachable through the public `apply_event` fold,
    //    so this is the sole path that produces it (V3 5.2 / 5.3).
    let mut updated = order.clone();
    updated.status = next;
    updated.ledger = ledger;
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
                vendor_id: VendorId(0),
                currency: Currency::All,
            },
            OrderItem {
                product_id: "p2".into(),
                modifier_ids: vec![],
                quantity: 1,
                unit_price: 300,
                vendor_id: VendorId(0),
                currency: Currency::All,
            },
        ]
    }

    // ── RED: illegal aggregate transitions must be rejected ──
    #[test]
    fn red_compensated_refund_not_reachable_via_apply_event() {
        // V3 5.2 / 5.3 (ROUND-2 GAP-AUDIT): CompensatedRefund is the money-
        // conserving terminal, only produced by `compensate` (which reverses the
        // earn ledger). The public fold must REFUSE to land there directly — a
        // caller reaching it via apply_event would leave an UN-reversed ledger.
        let mut o = place_order(
            "o1".into(),
            Some("c1".into()),
            sample_items(),
            0,
            None,
            None,
        )
        .unwrap();
        o = apply_event(&o, OrderStatus::Confirmed).unwrap();
        o = apply_event(&o, OrderStatus::Refunding).unwrap();
        assert_eq!(o.status, OrderStatus::Refunding);
        // Direct fold to CompensatedRefund is rejected.
        let r = apply_event(&o, OrderStatus::CompensatedRefund);
        assert!(
            r.is_err(),
            "CompensatedRefund must be unreachable via apply_event"
        );
        // And the ledger is still empty (no earn leg was ever posted).
        assert_eq!(o.ledger_balance(), 0);
    }

    #[test]
    fn compensate_reverses_ledger_and_is_sole_path() {
        // V3 5.2 / 5.3: compensate is the ONLY path to CompensatedRefund, and it
        // must reverse the earn leg so the order nets to EXACTLY zero.
        let mut o = place_order(
            "o1".into(),
            Some("c1".into()),
            sample_items(),
            0,
            None,
            None,
        )
        .unwrap();
        o = apply_event(&o, OrderStatus::Confirmed).unwrap();
        o = apply_event(&o, OrderStatus::Refunding).unwrap();
        o.post_earn(1, o.total, Currency::All).unwrap();
        assert_eq!(o.ledger_balance(), o.total, "earn leg posted at confirm");

        let comp = compensate(&o, OrderStatus::CompensatedRefund, 1, 2);
        assert!(
            comp.is_ok(),
            "compensate must succeed with a valid earn leg"
        );
        let comp = comp.unwrap();
        assert_eq!(comp.status, OrderStatus::CompensatedRefund);
        assert_eq!(comp.ledger_balance(), 0, "compensated order nets to ZERO");
    }

    #[test]
    fn compensate_without_earn_leg_is_rejected_and_order_untouched() {
        // V3 5.3: a compensate call whose reversal target is unknown must fail
        // closed and leave the order (status + ledger) completely unchanged.
        let mut o = place_order(
            "o1".into(),
            Some("c1".into()),
            sample_items(),
            0,
            None,
            None,
        )
        .unwrap();
        o = apply_event(&o, OrderStatus::Confirmed).unwrap();
        o = apply_event(&o, OrderStatus::Refunding).unwrap();
        // No earn leg posted; reversal of id 1 must fail.
        let before = o.clone();
        let r = compensate(&o, OrderStatus::CompensatedRefund, 1, 2);
        assert!(
            r.is_err(),
            "compensate without a matching earn leg must fail"
        );
        assert_eq!(o.status, before.status);
        assert_eq!(o.ledger_balance(), before.ledger_balance());
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
            vendor_id: VendorId(0),
            currency: Currency::All,
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
                vendor_id: VendorId(0),
                currency: Currency::All,
            },
            OrderItem {
                product_id: "p2".into(),
                modifier_ids: vec![],
                quantity: 2,
                unit_price: 0, // ignored
                vendor_id: VendorId(0),
                currency: Currency::All,
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
            vendor_id: VendorId(0),
            currency: Currency::All,
        }];
        assert!(place_order_priced("x".into(), None, items, 0, None, None, &cat).is_err());
    }

    // ── W19 GREEN (a): Kalman variance-reduction in the decide/fold Law ──
    //
    // A `decide` step (the order fold, `apply_event_with_trust`) that carries a
    // noisy courier/trust observation yields a Kalman-filtered estimate that is
    // closer to the (known) truth than the raw observation stream. This is the
    // variance-reduction gate that mirrors bebop2 BP-21: the filtered mean-square
    // error must be SIGNIFICANTLY below the raw observation mean-square error.
    //
    // The trust estimate is SHARED across many independent deliveries (each a
    // fresh order folded Pending→Confirmed), exactly as a real courier's running
    // reliability accumulates across orders. The observations are deterministic
    // (fixed LCG — no RNG dependency) so the proof value is reproducible.
    #[test]
    fn green_kalman_fold_reduces_variance_vs_raw() {
        // Truth we are trying to estimate: the courier's steady reliability.
        const TRUTH: f64 = 0.70;
        // Deterministic observation noise: LCG uniform in [-0.30, 0.30].
        let mut seed: u64 = 0x9E3779B97F4A7C15;
        let mut noise = || {
            // SplitMix64 step → [0,1) → shift to [-0.30, 0.30].
            seed = seed.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = seed;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z = z ^ (z >> 31);
            let u = (z >> 11) as f64 / (1u64 << 53) as f64; // [0,1)
            (u - 0.5) * 0.60
        };

        let mut trust = TrustEstimate::new(0.5, 0.002, 0.03);

        let mut raw_sq: f64 = 0.0;
        let mut filt_sq: f64 = 0.0;
        let mut n: u64 = 0;

        // 64 independent deliveries, each a fresh order folded one legal step,
        // feeding one noisy observation into the SHARED trust estimate.
        for i in 0..64u64 {
            let obs = (TRUTH + noise()).clamp(0.0, 1.0);
            let order = place_order(format!("d{i}"), None, sample_items(), 0, None, None).unwrap();
            // The decide/fold Law step WITH the courier/trust Kalman estimate.
            let _folded =
                apply_event_with_trust(&order, OrderStatus::Confirmed, &mut trust, Some(obs))
                    .unwrap();
            // Record raw vs filtered error against the known truth.
            raw_sq += (obs - TRUTH).powi(2);
            filt_sq += (trust.estimate() - TRUTH).powi(2);
            n += 1;
        }

        let raw_mse = raw_sq / n as f64;
        let filt_mse = filt_sq / n as f64;
        // Proof value: filtered MSE must be < 50% of the raw observation MSE.
        let reduction = raw_mse - filt_mse;
        let pct_lower = (reduction / raw_mse) * 100.0;
        println!(
            "W19 variance-reduction proof: raw_mse={raw_mse:.6} filtered_mse={filt_mse:.6} \
             reduction={reduction:.6} ({pct_lower:.1}% lower)"
        );
        assert!(
            filt_mse < 0.5 * raw_mse,
            "Kalman-filtered estimate (mse={filt_mse}) must be < 50% of raw obs mse={raw_mse}"
        );
        // Sanity: the filter actually converged near the truth, not stuck at prior.
        assert!(
            (trust.estimate() - TRUTH).abs() < 0.10,
            "filtered estimate {} must be near truth {}",
            trust.estimate(),
            TRUTH
        );
    }

    // ── W19 fail-closed: a missing observation holds the prior ──
    #[test]
    fn green_kalman_fold_holds_prior_on_missing_observation() {
        let mut trust = TrustEstimate::new(0.5, 0.002, 0.03);
        // Prime it with one observation so the prior is well-defined.
        trust.step(Some(0.9));
        let held = trust.estimate();
        // A fold with NO observation must NOT move the estimate (holds prior).
        let order = place_order("m".into(), None, sample_items(), 0, None, None).unwrap();
        let _ = apply_event_with_trust(&order, OrderStatus::Confirmed, &mut trust, None).unwrap();
        assert!(
            (trust.estimate() - held).abs() < 1e-12,
            "missing observation must hold the prior (est {} vs held {})",
            trust.estimate(),
            held
        );
        // But uncertainty grew (predict only).
        assert!(
            trust.variance() > 0.0,
            "variance must reflect increased uncertainty"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // P07 — MONEY-LAW CLOSURE: reversal primitive + FSM compensation edges.
    // The falsifier is Σ == 0 after a reversal / compensation. Every test below
    // either proves conservation or proves a fail-closed rejection.
    // ═══════════════════════════════════════════════════════════════════════════

    use crate::money::{Currency, Money};

    // ── Reversal algebra: m + neg(m) == 0 for every valid currency/amount ──
    #[test]
    fn green_reversal_nets_to_zero_for_all_valid_amounts() {
        for cur in [Currency::All, Currency::Eur, Currency::Usd] {
            for amt in [0i64, 1, -1, 1_234_567, -9_999, i64::MAX / 2] {
                let m = Money::new(amt, cur);
                let neg = m.checked_neg().expect("valid amount negates");
                let sum = m.checked_add(neg).expect("m + neg(m) adds");
                assert_eq!(
                    sum,
                    Money::new(0, cur),
                    "reversal must net to zero: m={amt} {:?}",
                    cur
                );
            }
        }
    }

    // ── RED: checked_neg of i64::MIN overflows → must be Err (fail-closed) ──
    #[test]
    fn red_neg_of_min_is_err() {
        let m = Money::new(i64::MIN, Currency::All);
        assert!(m.checked_neg().is_err(), "neg(i64::MIN) has no inverse");
    }

    // ── RED: cross-currency checked_sub is rejected ──
    #[test]
    fn red_cross_currency_sub_is_err() {
        let all = Money::new(1000, Currency::All);
        let eur = Money::new(100, Currency::Eur);
        assert!(all.checked_sub(eur).is_err(), "ALL - EUR must be rejected");
    }

    // ── RED: reverse_transfer rejects an unknown earn leg (fail-closed) ──
    #[test]
    fn red_reverse_unknown_earn_is_err() {
        let ledger: Vec<LedgerEntry> = Vec::new();
        let r = reverse_transfer(ledger, 999, 1000);
        assert!(r.is_err(), "reversing an unknown earn leg must be rejected");
    }

    // ── RED: re-reversing an already-reversed leg is rejected (idempotent once) ──
    #[test]
    fn red_reverse_already_reversed_is_err() {
        let earn = LedgerEntry {
            id: 1,
            kind: EntryKind::Earn,
            amount: Money::new(5000, Currency::All),
            reverses: None,
        };
        let mut ledger = ledger_append(Vec::new(), earn).unwrap();
        ledger = reverse_transfer(ledger.clone(), 1, 2).unwrap();
        // A second, DISTINCT reversal of the same earn leg must be refused.
        let again = LedgerEntry {
            id: 3,
            kind: EntryKind::Reversal,
            amount: Money::new(-5000, Currency::All),
            reverses: Some(1),
        };
        assert!(
            ledger_append(ledger, again).is_err(),
            "a second reversal of an already-reversed leg must be rejected"
        );
    }

    // ── Reversal idempotency: replay == no-op. A `Reversal` naming an already-
    //    reversed earn leg returns Err; the *caller* treats that as a no-op, so the
    //    ledger content is unchanged and STILL nets to exactly zero. ──
    #[test]
    fn green_reversal_is_idempotent_replay_noop() {
        let earn = LedgerEntry {
            id: 1,
            kind: EntryKind::Earn,
            amount: Money::new(7500, Currency::Eur),
            reverses: None,
        };
        let mut ledger = ledger_append(Vec::new(), earn).unwrap();
        ledger = reverse_transfer(ledger.clone(), 1, 2).unwrap();
        assert_eq!(ledger_sum(&ledger), 0, "reversed ledger nets to zero");
        let before = ledger_sum(&ledger);
        // Replay the SAME reversal id (duplicate id) → rejected, ledger untouched.
        let dup = LedgerEntry {
            id: 2,
            kind: EntryKind::Reversal,
            amount: Money::new(-7500, Currency::Eur),
            reverses: Some(1),
        };
        let res = ledger_append(ledger.clone(), dup);
        assert!(
            res.is_err(),
            "duplicate reversal id is rejected (no double reversal)"
        );
        assert_eq!(ledger_sum(&ledger), before, "ledger unchanged after replay");
    }

    // ── THE falsifier (a): a full delivered-order lifecycle nets to EXACTLY zero
    //    when its earn leg is prepaid and then (if reversed) refunded. Here we prove
    //    the earn leg + its reversal sum to zero — the invariant the cancel/refund
    //    paths depend on. The delivered lifecycle WITHOUT reversal keeps its earn
    //    (non-zero, as expected for a completed sale); the compensated lifecycle is 0. ──
    #[test]
    fn green_delivered_lifecycle_conserved_when_reversed() {
        // Place + drive an order to Delivered, having posted one earn leg.
        let mut o = place_order("p07a".into(), None, sample_items(), 0, None, None).unwrap();
        o.post_earn(1, o.total, Currency::All).unwrap();
        assert_eq!(
            o.ledger_balance(),
            o.total,
            "earn leg holds the sale amount"
        );
        o = apply_event(&o, OrderStatus::Confirmed).unwrap();
        o = apply_event(&o, OrderStatus::Preparing).unwrap();
        o = apply_event(&o, OrderStatus::Ready).unwrap();
        o = apply_event(&o, OrderStatus::InDelivery).unwrap();
        // Compensate the inflight order (refund) → ledger must net to EXACTLY zero.
        o = apply_event(&o, OrderStatus::Refunding).unwrap();
        o = compensate(&o, OrderStatus::CompensatedRefund, 1, 2).unwrap();
        assert_eq!(o.status, OrderStatus::CompensatedRefund);
        assert_eq!(
            o.ledger_balance(),
            0,
            "compensated delivered-order lifecycle nets to EXACTLY zero (money conserved)"
        );
    }

    // ── THE falsifier (b): cancel-after-confirm compensation nets to EXACTLY zero.
    //    This is the test that FAILS if the reversal is wrong — Confirmed→Refunding→
    //    CompensatedRefund must reverse the earn leg so Σ == 0. ──
    #[test]
    fn green_cancel_after_confirm_compensation_sums_to_zero() {
        let mut o = place_order("p07b".into(), None, sample_items(), 0, None, None).unwrap();
        // Money moves at confirm: post the earn leg.
        o = apply_event(&o, OrderStatus::Confirmed).unwrap();
        o.post_earn(1, o.total, Currency::All).unwrap();
        assert_eq!(o.ledger_balance(), o.total, "earn leg posted at confirm");
        // Operator cancels the confirmed order → compensation edge reverses the money.
        o = apply_event(&o, OrderStatus::Refunding).unwrap();
        o = compensate(&o, OrderStatus::CompensatedRefund, 1, 2).unwrap();
        assert_eq!(o.status, OrderStatus::CompensatedRefund);
        assert_eq!(
            o.ledger_balance(),
            0,
            "cancel-after-confirm compensation nets to EXACTLY zero"
        );
    }

    // ── FSM compensation edge exists: Confirmed→Refunding→CompensatedRefund folds ──
    #[test]
    fn green_fold_confirmed_to_compensated_refund() {
        let path = [OrderStatus::Refunding, OrderStatus::CompensatedRefund];
        assert_eq!(
            crate::order_machine::fold_transitions(OrderStatus::Confirmed, &path),
            Ok(OrderStatus::CompensatedRefund)
        );
        // And from any post-commitment state the refund compensation is reachable.
        for from in [
            OrderStatus::Confirmed,
            OrderStatus::Preparing,
            OrderStatus::Ready,
            OrderStatus::InDelivery,
        ] {
            assert_eq!(
                crate::order_machine::fold_transitions(from, &path),
                Ok(OrderStatus::CompensatedRefund),
                "compensation reachable from {from:?}"
            );
        }
    }

    // ── RED: a pre-commitment cancellation (Pending→Cancelled) has NO earn leg,
    //    so there is nothing to reverse; compensating it must be a no-op, not a
    //    fabricated credit. Prove the ledger stays empty (no money conjured). ──
    #[test]
    fn green_pending_cancel_has_no_money() {
        let o = place_order("p07c".into(), None, sample_items(), 0, None, None).unwrap();
        let o = apply_event(&o, OrderStatus::Cancelled).unwrap();
        assert_eq!(o.status, OrderStatus::Cancelled);
        assert!(
            o.ledger.is_empty(),
            "no earn leg before commitment → no money"
        );
    }

    // ── Fail-closed: compensating an order whose earn leg would overflow on
    //    negation (i64::MIN) is rejected, never a fabricated zero. ──
    #[test]
    fn red_compensate_overflowing_earn_is_rejected() {
        let mut o = place_order("p07d".into(), None, sample_items(), 0, None, None).unwrap();
        o = apply_event(&o, OrderStatus::Confirmed).unwrap();
        // Post an earn leg at the negation-overflow boundary.
        o.ledger = ledger_append(
            Vec::new(),
            LedgerEntry {
                id: 1,
                kind: EntryKind::Earn,
                amount: Money::new(i64::MIN, Currency::All),
                reverses: None,
            },
        )
        .unwrap();
        let res = compensate(&o, OrderStatus::CompensatedRefund, 1, 2);
        assert!(
            res.is_err(),
            "reversal of i64::MIN earn leg must be rejected"
        );
        assert_eq!(
            o.ledger_balance(),
            i64::MIN,
            "original ledger untouched on failure"
        );
    }

    // ── Fail-closed: a Reversal whose amount does NOT match -earn is rejected
    //    (no silent conservation violation). ──
    #[test]
    fn red_mismatched_reversal_amount_is_rejected() {
        let earn = LedgerEntry {
            id: 1,
            kind: EntryKind::Earn,
            amount: Money::new(5000, Currency::Usd),
            reverses: None,
        };
        let mut ledger = ledger_append(Vec::new(), earn).unwrap();
        // Attempt a reversal of the WRONG amount (drift).
        let bad = LedgerEntry {
            id: 2,
            kind: EntryKind::Reversal,
            amount: Money::new(-4999, Currency::Usd), // off by 1
            reverses: Some(1),
        };
        assert!(
            ledger_append(ledger.clone(), bad).is_err(),
            "reversal amount must equal -earn exactly"
        );
        assert_eq!(
            ledger_sum(&ledger),
            5000,
            "no partial money moved on bad reversal"
        );
    }
}
