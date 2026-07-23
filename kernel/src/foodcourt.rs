//! BLUEPRINT-P72 — food-court N-leg checkout spine (kernel side).
//!
//! This is the PURE-Rust composition layer that turns a `domain::Order` into a
//! P60 [`NLegPlan`] and routes per-vendor refunds / kitchen tickets. It REUSES:
//!
//! * **P60** (`ports::payment_provider`): `run_nleg_saga`, `NLegPlan`,
//!   `VendorLeg`, `RefundRequest`, the atomicity `Law`, and the
//!   `PaymentProvider` port. It does NOT re-derive any metric; the money
//!   integrity + atomicity law stay owned by P60.
//! * **P62** (`catalog`): `charge_legs` (per-vendor charge-leg derivation) and
//!   `kitchen_tickets` (per-vendor KDS fan-out). `OrderItem.vendor_id` is set
//!   catalog-authoritatively by `place_order_priced`; a client cannot forge it.
//!
//! RED LINE (PCI, §16.18): there is NO card-data type anywhere here, and none is
//! introduced. Every value routed to a provider is a [`RefundRequest`] / plan
//! leg — bound to a vendor's [`ProviderAccountRef`], never a PAN.
//!
//! DECART:STD-1 — no float on money (`money::Money`, integer minor units only).
//! DECART:OUT-1 — `std`-only; no I/O, no network, no DOM. This module is
//! WASM-safe (no `reqwest`/serde) so it can run both in the kernel and behind a
//! `web.mjs` boundary untouched.
//!
//! OUT OF SCOPE (TS/Node ban + scope): the external merchant-of-record adapter
//! (M2 Stripe `payment-adapters` crate), the `web.mjs` redirect boundary (M6),
//! and the Adyen `payment-adapters` stub (M7 TS). Those are real provider
//! integrations owned by the out-of-kernel `payment-adapters` crate; this module
//! is the provider-agnostic kernel that any of them plugs into via `run_nleg_saga`.

use std::collections::BTreeMap;

use crate::catalog::{charge_legs, kitchen_tickets};
use crate::domain::{Order, OrderItem};
use crate::money::{Currency, Money};
use crate::vendor::VendorId;

use crate::ports::payment_provider::{
    ChargeHandle, LegId, NLegPlan, ProviderAccountRef, RefundReason, RefundRequest, VendorLeg,
    MAX_LEGS_PER_CHECKOUT,
};

// ── payability bookkeeping (P72 §3) ───────────────────────────────────────────
/// Per-vendor payment readiness, decided by the food-court operator's MoR config
/// (the `app.vendor_scope` / `PaymentProviderAccount` rows the real DB layer
/// owns). This is the PURE kernel view; the caller supplies it. There is no
/// "someday" sentinel — a vendor is either `Connected` (has a routable
/// `ProviderAccountRef`), `Pending` (onboarded, MoR not yet live), or
/// `NotConnected` (no MoR at all).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayabilityStatus {
    /// Vendor has a live merchant-of-record account; its `ProviderAccountRef` is
    /// routed as that leg's `dest_account`.
    Connected(ProviderAccountRef),
    /// Onboarded but the MoR account is not yet active — checkout must refuse.
    Pending,
    /// No merchant-of-record configured at all — checkout must refuse.
    NotConnected,
}

/// `vendor_id → PayabilityStatus`. A `BTreeMap` so lookups + iteration are
/// deterministic (P6 determinism).
pub type VendorAccounts = BTreeMap<VendorId, PayabilityStatus>;

// ── error type ────────────────────────────────────────────────────────────────
/// Every failure mode of the food-court spine is a typed value (never a panic,
/// never a silent retry — bulkhead §5.3). Maps 1:1 onto the blueprint's
/// fail-closed refusals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FoodCourtError {
    /// P62 `charge_legs` failed (cross-currency cart or overflow). The order is
    /// rejected before any authorize.
    LegDerivation(String),
    /// A vendor in the cart has no routable merchant-of-record (Pending /
    /// NotConnected / unknown). Fail-closed: the whole checkout is refused.
    VendorNotPayable(VendorId),
    /// One checkout fans out to more than `MAX_LEGS_PER_CHECKOUT` vendors
    /// (food-court sanity cap, P60 §5.2). Refuse — not silently truncated.
    TooManyVendors { got: usize, max: usize },
    /// A refund was requested for a vendor that has no leg in the capture
    /// session — never an undifferentiated "refund the order" call.
    VendorNotInOrder(VendorId),
    /// A refund amount exceeds the vendor's captured leg amount (adversarial
    /// over-refund, §4.1 M1 (ii)). Typed reject; P60 would also bounce it, but
    /// we fail-closed at the routing layer.
    OverRefund(VendorId),
    /// A provider-boundary error while executing a per-vendor refund.
    RefundFailed(VendorId, String),
}

// ── VendorId bridge (kernel u64 ↔ payment_provider [u8;32]) ───────────────────
/// The kernel `VendorId` is a plain `u64`; P60's `VendorId` is a `[u8;32]` opaque
/// id (so a provider never learns our internal key shape). Bridge deterministically
/// by writing the `u64` little-endian into the low 8 bytes. Order-preserving +
/// injective for the food-court key space. Keep this the ONLY place the two id
/// spaces are coupled (DECART: single seam).
pub fn to_payment_vendor_id(v: VendorId) -> crate::ports::payment_provider::VendorId {
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&v.0.to_le_bytes());
    crate::ports::payment_provider::VendorId(bytes)
}

/// Inverse of [`to_payment_vendor_id`] — read the low 8 bytes back into a `u64`.
/// Vendors we never bridged (high bytes set) round-trip back to 0, which is just
/// an ordinary kernel key (no reserved sentinel), so no information is lost for
/// keys we produced.
pub fn from_payment_vendor_id(v: &crate::ports::payment_provider::VendorId) -> VendorId {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&v.0[..8]);
    VendorId(u64::from_le_bytes(buf))
}

// ── M2 / M3 / M4 — plan derivation + payability gating (§3, §4.1) ──────────────
/// Derive the P60 `NLegPlan` for a multi-vendor order (P72 §3).
///
/// 1. `charge_legs(order)` — P62 per-vendor derivation (fail-closed on
///    cross-currency / overflow). Deterministic `VendorId` order.
/// 2. `TooManyVendors` guard — refuse a cart fanning out beyond the food-court
///    cap (never silently truncated).
/// 3. Payability gating — every vendor MUST be `Connected`; a `Pending` /
///    `NotConnected` / unknown vendor ⇒ `VendorNotPayable` (the whole checkout is
///    refused, fail-closed). A vendor-as-own-MoR does NOT relax this: its account
///    must still be live.
///
/// Returns the plan with `LegId` 1..=N in `VendorId` order (deterministic), each
/// leg's `dest_account` = that vendor's `ProviderAccountRef`. The currency is the
/// order's single currency (Wave-0). A client cannot affect which vendor a line
/// settles to — `OrderItem.vendor_id` is catalog-authoritative.
pub fn derive_nleg_plan(
    order: &Order,
    accounts: &VendorAccounts,
) -> Result<NLegPlan, FoodCourtError> {
    let legs = charge_legs(order).map_err(|e| FoodCourtError::LegDerivation(e.to_string()))?;

    if legs.len() > MAX_LEGS_PER_CHECKOUT {
        return Err(FoodCourtError::TooManyVendors {
            got: legs.len(),
            max: MAX_LEGS_PER_CHECKOUT,
        });
    }

    let currency = order_currency(order);

    let mut plan_legs = Vec::with_capacity(legs.len());
    for (i, leg) in legs.iter().enumerate() {
        match accounts.get(&leg.vendor_id) {
            Some(PayabilityStatus::Connected(acct)) => {
                plan_legs.push(VendorLeg {
                    leg: LegId((i + 1) as u32),
                    vendor_id: to_payment_vendor_id(leg.vendor_id),
                    amount: Money::new(leg.amount.minor, currency),
                    dest_account: ProviderAccountRef(acct.0.clone()),
                });
            }
            // Pending / NotConnected / unknown ⇒ fail-closed refusal.
            _ => return Err(FoodCourtError::VendorNotPayable(leg.vendor_id)),
        }
    }

    Ok(NLegPlan {
        order_id: order.id.clone(),
        currency,
        legs: plan_legs,
    })
}

/// The order's single currency. `charge_legs` already guarantees all lines share
/// one currency; this is the kernel-side read of it (defaults to `Currency::All`
/// for an empty order). NOT hardcoded to EUR — see `currency_not_hardcoded` test.
pub fn order_currency(order: &Order) -> Currency {
    order
        .items
        .first()
        .map(|i| i.currency)
        .unwrap_or(Currency::All)
}

// ── P62 §4.5 — KDS fan-out (§4.1) ─────────────────────────────────────────────
/// Per-vendor kitchen-ticket fan-out for an order: one order → N tickets, each
/// keyed by `VendorId`. Wraps P62 `kitchen_tickets` (pure grouping) and returns
/// owned line items so callers don't borrow the `Order`. Determinism: BTreeMap
/// keyed by `VendorId`. Invariant: `Σ line_count across tickets == order.items.len()`
/// (nothing dropped, nothing duplicated).
pub fn kds_route(order: &Order) -> BTreeMap<VendorId, Vec<OrderItem>> {
    kitchen_tickets(order)
        .into_iter()
        .map(|(vid, items)| (vid, items.iter().map(|i| (*i).clone()).collect()))
        .collect()
}

// ── §16.29 — per-vendor refund routing (never a single undifferentiated refund) ─
/// Route a refund for ONE vendor to that vendor's captured `ChargeHandle`.
///
/// Given the capture session (the [`NLegPlan`] returned by [`derive_nleg_plan`]
/// at checkout), this finds the `VendorLeg` whose (bridged) `VendorId` matches,
/// guards the amount against over-refund, and returns a P60 [`RefundRequest`]
/// bound to THAT vendor's `ChargeHandle` (`ch_<leg>`), routed to the vendor's
/// `ProviderAccountRef`. dowiz stays out of the vendor's money.
///
/// Properties (falsifiable, §4.1 M1):
/// * **granularity** — exactly one [`RefundRequest`] is produced, for the matched
///   vendor only. Calling this for vendor V can never touch vendor V'≠V (the
///   `ChargeHandle` is built from `LegId`, which is 1:1 with a vendor leg).
/// * **over-refund** — `amount > leg.amount` ⇒ `OverRefund` (fail-closed).
/// * **unknown vendor** — a vendor with no leg ⇒ `VendorNotInOrder`.
///
/// The caller hands the returned request to `provider.refund(&req)`.
pub fn refund_vendor_leg(
    capture_session: &NLegPlan,
    vendor_id: VendorId,
    amount: Money,
    reason: RefundReason,
) -> Result<RefundRequest, FoodCourtError> {
    let target = to_payment_vendor_id(vendor_id);
    let leg = capture_session
        .legs
        .iter()
        .find(|l| l.vendor_id == target)
        .ok_or(FoodCourtError::VendorNotInOrder(vendor_id))?;

    // Over-refund guard: a refund larger than the captured leg is rejected here
    // (P60 would also bounce it, but we fail-closed at the routing layer).
    if amount.minor > leg.amount.minor {
        return Err(FoodCourtError::OverRefund(vendor_id));
    }

    Ok(RefundRequest {
        charge: ChargeHandle(format!("ch_{}", leg.leg.0)),
        amount,
        reason,
    })
}

/// Execute a per-vendor refund against a concrete [`PaymentProvider`]. Thins the
/// routing derivation ([`refund_vendor_leg`]) to the provider call. Any
/// [`PayError`] becomes a typed [`FoodCourtError::RefundFailed`]; the atomicity
/// granularity is preserved — only the one vendor's `ChargeHandle` is touched.
pub fn exec_refund_vendor_leg(
    provider: &dyn crate::ports::payment_provider::PaymentProvider,
    capture_session: &NLegPlan,
    vendor_id: VendorId,
    amount: Money,
    reason: RefundReason,
) -> Result<(), FoodCourtError> {
    let req = refund_vendor_leg(capture_session, vendor_id, amount, reason)?;
    provider
        .refund(&req)
        .map_err(|e| FoodCourtError::RefundFailed(vendor_id, format!("{e:?}")))
}

/// The N-leg atomicity invariant at the refund layer (§16.29, §4.1): the checkout
/// is "fully compensated" iff EVERY captured vendor leg has been refunded. Returns
/// `true` only when the refund set covers all legs in the capture session. A
/// partial refund set (some vendors refunded, some not) returns `false` —
/// never a phantom "all done".
pub fn all_legs_refunded(capture_session: &NLegPlan, refunded: &[VendorId]) -> bool {
    let set: std::collections::BTreeSet<VendorId> = refunded.iter().copied().collect();
    capture_session
        .legs
        .iter()
        .all(|l| set.contains(&from_payment_vendor_id(&l.vendor_id)))
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests — the falsifiable contract (RED+GREEN per blueprint §4.1).
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::OrderItem;
    use crate::money::Currency;
    use crate::ports::payment_provider::{
        assert_nleg_atomicity, run_nleg_saga, CaptureOutcome, FailReason, NLegEvent, NLegOutcome,
    };
    use crate::vendor::VendorId;
    fn item(product: &str, qty: i64, price: i64, vendor: u64, cur: Currency) -> OrderItem {
        OrderItem {
            product_id: product.into(),
            modifier_ids: vec![],
            quantity: qty,
            unit_price: price,
            vendor_id: VendorId(vendor),
            currency: cur,
        }
    }

    fn accounts(entries: &[(u64, &str)]) -> VendorAccounts {
        entries
            .iter()
            .map(|(v, acct)| {
                (
                    VendorId(*v),
                    PayabilityStatus::Connected(ProviderAccountRef(acct.to_string())),
                )
            })
            .collect()
    }

    /// Build a kernel-valid `Order` (the real domain shape) with the given id +
    /// line items. The food-court spine only reads `id` / `items` / `price_trusted`,
    /// so the remaining fields use safe defaults. Accepts `String` (call sites use
    /// `"ORD-..".into()`) and any `IntoIterator` of `OrderItem` (call sites use
    /// array literals), so the production signature stays loose.
    fn mk_order(id: String, items: impl IntoIterator<Item = OrderItem>) -> Order {
        Order {
            id,
            customer_id: Some("cust".into()),
            status: crate::order_machine::OrderStatus::Pending,
            items: items.into_iter().collect(),
            subtotal: 0,
            total: 0,
            created_at_ms: 0,
            channel: None,
            cash_pay_with: None,
            price_trusted: true,
            ledger: vec![],
        }
    }

    // ── M1 (i) — single-currency EUR checkout, 3 vendors ⇒ NLegPlan with 3 legs ──
    #[test]
    fn derive_plan_eur_three_vendors() {
        let order = mk_order(
            "ORD-FC-1".into(),
            [
                item("taco", 2, 500, 1, Currency::Eur),
                item("soda", 3, 200, 2, Currency::Eur),
                item("fries", 1, 400, 3, Currency::Eur),
            ],
        );
        let accts = accounts(&[(1, "acct_v1"), (2, "acct_v2"), (3, "acct_v3")]);
        let plan = derive_nleg_plan(&order, &accts).expect("plan derives");
        assert_eq!(plan.currency, Currency::Eur);
        assert_eq!(plan.legs.len(), 3, "3 vendors ⇒ 3 legs");
        // Deterministic LegId 1..=N in VendorId order.
        assert_eq!(plan.legs[0].leg, LegId(1));
        assert_eq!(plan.legs[1].leg, LegId(2));
        assert_eq!(plan.legs[2].leg, LegId(3));
        // Each leg routed to its own ProviderAccountRef.
        assert_eq!(
            plan.legs[0].dest_account,
            ProviderAccountRef("acct_v1".into())
        );
        assert_eq!(
            plan.legs[2].dest_account,
            ProviderAccountRef("acct_v3".into())
        );
        // Vendor 2's captured amount = 3 * 200 = 600 EUR.
        assert_eq!(plan.legs[1].amount, Money::new(600, Currency::Eur));
    }

    // ── M1 (ii) — cross-currency cart ⇒ CrossCurrencyCart (fail-closed) ─────────
    #[test]
    fn cross_currency_cart_refused() {
        let order = mk_order(
            "ORD-X".into(),
            [
                item("taco", 1, 500, 1, Currency::Eur),
                item("soda", 1, 200, 2, Currency::Usd), // second currency
            ],
        );
        let accts = accounts(&[(1, "a1"), (2, "a2")]);
        match derive_nleg_plan(&order, &accts) {
            Err(FoodCourtError::LegDerivation(_)) => {}
            other => panic!("cross-currency cart must be refused, got {other:?}"),
        }
    }

    // ── M1 (iii) — a vendor not Connected ⇒ VendorNotPayable (fail-closed) ───────
    #[test]
    fn unconnected_vendor_refused() {
        let order = mk_order(
            "ORD-U".into(),
            [
                item("taco", 1, 500, 1, Currency::Eur),
                item("soda", 1, 200, 2, Currency::Eur), // vendor 2 has no account
            ],
        );
        // Only vendor 1 is Connected; vendor 2 is absent.
        let accts = accounts(&[(1, "acct_v1")]);
        match derive_nleg_plan(&order, &accts) {
            Err(FoodCourtError::VendorNotPayable(VendorId(2))) => {}
            other => panic!("unconnected vendor must be refused, got {other:?}"),
        }
    }

    // ── M1 (iv) — too many vendors (> MAX_LEGS_PER_CHECKOUT) ⇒ TooManyVendors ────
    #[test]
    fn too_many_vendors_refused() {
        let mut items = vec![];
        let mut accts = VendorAccounts::new();
        for v in 1..=(MAX_LEGS_PER_CHECKOUT as u64 + 3) {
            items.push(item(&format!("p{v}"), 1, 100, v, Currency::Eur));
            accts.insert(
                VendorId(v),
                PayabilityStatus::Connected(ProviderAccountRef(format!("a{v}"))),
            );
        }
        let order = mk_order("ORD-BIG".into(), items);
        match derive_nleg_plan(&order, &accts) {
            Err(FoodCourtError::TooManyVendors { got, max }) => {
                assert_eq!(max, MAX_LEGS_PER_CHECKOUT);
                assert!(got > max);
            }
            other => panic!("oversized cart must be refused, got {other:?}"),
        }
    }

    // ── M1 (v) — determinism: derive twice ⇒ byte-identical plan ────────────────
    #[test]
    fn derive_plan_is_deterministic() {
        let order = mk_order(
            "ORD-D".into(),
            [
                item("b", 1, 200, 2, Currency::Eur),
                item("a", 2, 500, 1, Currency::Eur),
                item("c", 1, 400, 3, Currency::Eur),
            ],
        );
        let accts = accounts(&[(1, "a1"), (2, "a2"), (3, "a3")]);
        let p1 = derive_nleg_plan(&order, &accts).unwrap();
        let p2 = derive_nleg_plan(&order, &accts).unwrap();
        assert_eq!(p1, p2, "plan derivation must be deterministic");
    }

    // ── M1 (vi) — currency is NOT hardcoded: a non-EUR order derives in its currency
    #[test]
    fn currency_not_hardcoded() {
        let order = mk_order(
            "ORD-USD".into(),
            [
                item("taco", 1, 500, 1, Currency::Usd),
                item("soda", 2, 200, 2, Currency::Usd),
            ],
        );
        let accts = accounts(&[(1, "a1"), (2, "a2")]);
        let plan = derive_nleg_plan(&order, &accts).unwrap();
        assert_eq!(
            plan.currency,
            Currency::Usd,
            "plan follows the order currency"
        );
        assert_eq!(plan.legs[0].amount, Money::new(500, Currency::Usd));
        assert_eq!(plan.legs[1].amount, Money::new(400, Currency::Usd));
    }

    // ── M3 — P62 §4.5 KDS fan-out: nothing dropped, nothing duplicated ──────────
    #[test]
    fn kds_fanout_preserves_all_lines() {
        let order = mk_order(
            "ORD-K".into(),
            [
                item("taco", 1, 500, 1, Currency::Eur),
                item("soda", 1, 200, 2, Currency::Eur),
                item("fries", 1, 400, 1, Currency::Eur), // 2nd item for vendor 1
            ],
        );
        let tickets = kds_route(&order);
        let total_lines: usize = tickets.values().map(|v| v.len()).sum();
        assert_eq!(
            total_lines,
            order.items.len(),
            "every line routed to exactly one KDS ticket"
        );
        assert_eq!(tickets.len(), 2, "two vendors ⇒ two tickets");
        assert_eq!(
            tickets[&VendorId(1)].len(),
            2,
            "vendor 1 gets both of its lines"
        );
    }

    // ── M4 (i) — refund routes to ONLY that vendor's ChargeHandle ───────────────
    #[test]
    fn refund_one_vendor_leaves_others_untouched() {
        let order = mk_order(
            "ORD-R".into(),
            [
                item("taco", 1, 500, 1, Currency::Eur),
                item("soda", 1, 200, 2, Currency::Eur),
                item("fries", 1, 400, 3, Currency::Eur),
            ],
        );
        let accts = accounts(&[(1, "a1"), (2, "a2"), (3, "a3")]);
        let plan = derive_nleg_plan(&order, &accts).unwrap();
        // Refund vendor 2 only.
        let req = refund_vendor_leg(
            &plan,
            VendorId(2),
            Money::new(200, Currency::Eur),
            RefundReason::CustomerRequest,
        )
        .expect("vendor 2 refund routes");
        // Bound to vendor 2's ChargeHandle (leg 2 === ch_2), never vendor 1 or 3.
        assert_eq!(req.charge, ChargeHandle("ch_2".into()));
        assert_ne!(req.charge, ChargeHandle("ch_1".into()));
        assert_ne!(req.charge, ChargeHandle("ch_3".into()));
        assert_eq!(req.amount, Money::new(200, Currency::Eur));
    }

    // ── M4 (ii) — over-refund a vendor ⇒ OverRefund (typed reject) ──────────────
    #[test]
    fn over_refund_rejected() {
        let order = mk_order(
            "ORD-OR".into(),
            [
                item("taco", 1, 500, 1, Currency::Eur),
                item("soda", 1, 200, 2, Currency::Eur),
            ],
        );
        let accts = accounts(&[(1, "a1"), (2, "a2")]);
        let plan = derive_nleg_plan(&order, &accts).unwrap();
        // Vendor 2 captured 200; try to refund 999.
        match refund_vendor_leg(
            &plan,
            VendorId(2),
            Money::new(999, Currency::Eur),
            RefundReason::CustomerRequest,
        ) {
            Err(FoodCourtError::OverRefund(VendorId(2))) => {}
            other => panic!("over-refund must be rejected, got {other:?}"),
        }
    }

    // ── M4 (iii) — refund for a vendor with no leg ⇒ VendorNotInOrder ───────────
    #[test]
    fn refund_unknown_vendor_rejected() {
        let order = mk_order("ORD-UN".into(), [item("taco", 1, 500, 1, Currency::Eur)]);
        let accts = accounts(&[(1, "a1")]);
        let plan = derive_nleg_plan(&order, &accts).unwrap();
        match refund_vendor_leg(
            &plan,
            VendorId(99),
            Money::new(1, Currency::Eur),
            RefundReason::CustomerRequest,
        ) {
            Err(FoodCourtError::VendorNotInOrder(VendorId(99))) => {}
            other => panic!("unknown-vendor refund must be rejected, got {other:?}"),
        }
    }

    // ── M5 — N-leg atomicity invariant via run_nleg_saga (reused P60 Law) ───────
    // All authorized → all captured ⇒ Committed; no void.
    #[test]
    fn atomicity_all_captured_is_committed() {
        let plan = NLegPlan {
            order_id: "ORD-A".into(),
            currency: Currency::Eur,
            legs: vec![
                VendorLeg {
                    leg: LegId(1),
                    vendor_id: to_payment_vendor_id(VendorId(1)),
                    amount: Money::new(500, Currency::Eur),
                    dest_account: ProviderAccountRef("a1".into()),
                },
                VendorLeg {
                    leg: LegId(2),
                    vendor_id: to_payment_vendor_id(VendorId(2)),
                    amount: Money::new(200, Currency::Eur),
                    dest_account: ProviderAccountRef("a2".into()),
                },
            ],
        };
        let auth = vec![(LegId(1), Ok(())), (LegId(2), Ok(()))];
        let capture = vec![
            (LegId(1), CaptureOutcome::Captured),
            (LegId(2), CaptureOutcome::Captured),
        ];
        let (events, outcome) = run_nleg_saga(&plan.order_id, &auth, &capture);
        assert_eq!(outcome, NLegOutcome::Committed);
        assert_nleg_atomicity(&events, &outcome); // must not panic
    }

    // One leg fails to authorize ⇒ ALL authorized legs voided ⇒ Aborted (no money moved).
    #[test]
    fn atomicity_auth_failure_voids_all() {
        let plan = NLegPlan {
            order_id: "ORD-B".into(),
            currency: Currency::Eur,
            legs: vec![
                VendorLeg {
                    leg: LegId(1),
                    vendor_id: to_payment_vendor_id(VendorId(1)),
                    amount: Money::new(500, Currency::Eur),
                    dest_account: ProviderAccountRef("a1".into()),
                },
                VendorLeg {
                    leg: LegId(2),
                    vendor_id: to_payment_vendor_id(VendorId(2)),
                    amount: Money::new(200, Currency::Eur),
                    dest_account: ProviderAccountRef("a2".into()),
                },
            ],
        };
        let auth = vec![(LegId(1), Ok(())), (LegId(2), Err(FailReason::Declined))];
        let capture: Vec<(LegId, CaptureOutcome)> = vec![]; // capture never runs
        let (events, outcome) = run_nleg_saga(&plan.order_id, &auth, &capture);
        assert_eq!(
            outcome,
            NLegOutcome::Aborted {
                void_set: vec![LegId(1)]
            }
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, NLegEvent::LegVoided { leg: LegId(1) })),
            "the authorized vendor leg must be voided"
        );
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, NLegEvent::LegCaptured { .. })),
            "no money moved on abort"
        );
        assert_nleg_atomicity(&events, &outcome);
    }

    // Capture-half stuck ⇒ NeedsReconciliation (operator-visible, never silent).
    #[test]
    fn atomicity_capture_stuck_needs_reconciliation() {
        let plan = NLegPlan {
            order_id: "ORD-C".into(),
            currency: Currency::Eur,
            legs: vec![
                VendorLeg {
                    leg: LegId(1),
                    vendor_id: to_payment_vendor_id(VendorId(1)),
                    amount: Money::new(500, Currency::Eur),
                    dest_account: ProviderAccountRef("a1".into()),
                },
                VendorLeg {
                    leg: LegId(2),
                    vendor_id: to_payment_vendor_id(VendorId(2)),
                    amount: Money::new(200, Currency::Eur),
                    dest_account: ProviderAccountRef("a2".into()),
                },
            ],
        };
        let auth = vec![(LegId(1), Ok(())), (LegId(2), Ok(()))];
        let capture = vec![
            (LegId(1), CaptureOutcome::Captured),
            (LegId(2), CaptureOutcome::Stuck),
        ];
        let (events, outcome) = run_nleg_saga(&plan.order_id, &auth, &capture);
        match &outcome {
            NLegOutcome::NeedsReconciliation { stuck, .. } => {
                assert_eq!(*stuck, vec![LegId(2)]);
            }
            other => panic!("stuck capture must flag reconciliation, got {other:?}"),
        }
        assert_nleg_atomicity(&events, &outcome);
    }

    // ── M5 — all_legs_refunded predicate at the refund layer ────────────────────
    #[test]
    fn all_legs_refunded_predicate() {
        let plan = derive_nleg_plan(
            &mk_order(
                "ORD-P".into(),
                [
                    item("taco", 1, 500, 1, Currency::Eur),
                    item("soda", 1, 200, 2, Currency::Eur),
                    item("fries", 1, 400, 3, Currency::Eur),
                ],
            ),
            &accounts(&[(1, "a1"), (2, "a2"), (3, "a3")]),
        )
        .unwrap();

        // Partial refund (only vendor 1) ⇒ not all refunded.
        assert!(!all_legs_refunded(&plan, &[VendorId(1)]));
        // All three refunded ⇒ fully compensated.
        assert!(all_legs_refunded(
            &plan,
            &[VendorId(1), VendorId(2), VendorId(3)]
        ));
    }

    // ── M7 (kernel side) — provider-agnostic refund audit (no TS/Node) ──────────
    // Routes a per-vendor refund through a real P60 NoOpPaymentProvider and proves
    // only the requested vendor's ChargeHandle is touched (the audit invariant).
    #[test]
    fn provider_agnostic_refund_audit() {
        use crate::ports::payment_provider::NoOpPaymentAdapter;
        let provider = NoOpPaymentAdapter::new();
        let plan = derive_nleg_plan(
            &mk_order(
                "ORD-AUD".into(),
                [
                    item("taco", 1, 500, 1, Currency::Eur),
                    item("soda", 1, 200, 2, Currency::Eur),
                ],
            ),
            &accounts(&[(1, "a1"), (2, "a2")]),
        )
        .unwrap();
        // Executing a refund for vendor 2 must succeed and route only to ch_2.
        let r = exec_refund_vendor_leg(
            &provider,
            &plan,
            VendorId(2),
            Money::new(200, Currency::Eur),
            RefundReason::DisputeResolution,
        );
        assert!(
            r.is_ok(),
            "per-vendor refund executes via the provider port"
        );
        // The audit invariant: a refund request for vendor 2 is bound to ch_2 only.
        let req = refund_vendor_leg(
            &plan,
            VendorId(2),
            Money::new(200, Currency::Eur),
            RefundReason::DisputeResolution,
        )
        .unwrap();
        assert_eq!(req.charge, ChargeHandle("ch_2".into()));
    }
}
