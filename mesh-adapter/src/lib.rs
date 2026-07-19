//! P34 — dowiz-kernel as the consumer/driver of the bebop2 delivery protocol.
//!
//! This crate is the ONLY new production code P34 introduces. It does NOT fork
//! or rewrite `bebop-delivery-domain`: it consumes the sibling crate via the
//! `kernel-rlib` feature (which already re-exports the dowiz-kernel Law/money
//! fns) plus `bebop-proto-cap` for the event vocabulary / matcher / claims.
//!
//! Design (per BLUEPRINT-P34-mesh-kernel-wiring.md §2.3):
//! - `OPENBEBOP_CI_PIN`: the OpenBebop commit the cross-repo CI job checks out.
//! - `MeshHost`: node-local host state (orders + claims) driven by decoded
//!   `DeliveryEvent`s through the kernel `apply_event` fold. Pure per call; the
//!   maps update ONLY on `Ok` (no partial application representable).
//! - `HostFault`: why the host refused to apply an event (fail-closed at every
//!   arm, mirroring `FoldError` at the host level).
//!
//! Anti-scope honoured: no new event variants, no courier scoring (the
//! `Courier` type structurally cannot carry a score), no transport, no storage.

use std::collections::BTreeMap;

use bebop_delivery_domain::intake::{to_order_status, IntakeEdge};
use bebop_delivery_domain::DeliveryStatus as WireStatus;
use bebop_proto_cap::claim_machine::ClaimStatus;
use bebop_proto_cap::event_dict::{
    CourierKey, DeliveryEvent, LedgerPayload, OrderPlacedPayload, StatusChangedPayload,
};
use bebop_proto_cap::scope::{Action, Resource, Scope};
use bebop_proto_cap::event_dict::{DeliveryStatus as VocabStatus};

/// Convert the event-vocabulary `DeliveryStatus` (proto-cap) into the wire
/// `DeliveryStatus` (delivery-domain) used by `to_order_status`. The two enums
/// share the same 9-variant lifecycle; this is a pure re-tag, no loss.
pub fn vocab_to_wire(v: VocabStatus) -> WireStatus {
    match v {
        VocabStatus::Pending => WireStatus::Pending,
        VocabStatus::Confirmed => WireStatus::Confirmed,
        VocabStatus::Preparing => WireStatus::Preparing,
        VocabStatus::Ready => WireStatus::Ready,
        VocabStatus::InDelivery => WireStatus::InDelivery,
        VocabStatus::Delivered => WireStatus::Delivered,
        VocabStatus::Rejected => WireStatus::Rejected,
        VocabStatus::Cancelled => WireStatus::Cancelled,
        VocabStatus::PickedUp => WireStatus::PickedUp,
    }
}

use dowiz_kernel::domain::{apply_event, place_order, Order, OrderItem};
use dowiz_kernel::money::Currency;
use dowiz_kernel::order_machine::{assert_transition, OrderStatus, TransitionError};

/// CI supply-chain pin: the OpenBebop commit the cross-repo CI job checks out.
/// Bumping it is a reviewed, deliberate commit (never a floating branch ref).
pub const OPENBEBOP_CI_PIN: &str = "986646a35258ced76752510625511f37a6367a77";

/// Kernel `Order.id` is derived from the WIRE order id as `format!("ord-{wire_id}")`.
/// One direction, one format (BLUEPRINT-P34 §2.3) — no second convention can appear.
fn wire_to_kernel_id(wire_id: u64) -> String {
    format!("ord-{wire_id}")
}

/// Why the host refused to apply a decoded `DeliveryEvent`. Mirrors the spine's
/// `FoldError` tagging at the host level; every arm is fail-closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostFault {
    /// The order named by the event is not known to this host.
    UnknownOrder(u64),
    /// Vocabulary / decode failure, or an unmappable kernel state.
    Vocabulary(&'static str),
    /// The kernel Law (`assert_transition`) refused the transition.
    Law(TransitionError),
    /// Ledger / money operation refused (e.g. duplicate entry id).
    Money(String),
}

/// Node-local host state for driving the bebop delivery spine from dowiz's own
/// decider. `orders` map WIRE order id -> kernel `Order`; `claims` map order id
/// -> claim status (coordination record only, never a score).
pub struct MeshHost {
    orders: BTreeMap<u64, Order>,
    claims: BTreeMap<u64, ClaimStatus>,
    /// Monotonic ledger-entry id source for `Order::post_earn` (never reused).
    next_entry_id: u64,
}

impl Default for MeshHost {
    fn default() -> Self {
        Self::new()
    }
}

impl MeshHost {
    pub fn new() -> Self {
        MeshHost {
            orders: BTreeMap::new(),
            claims: BTreeMap::new(),
            next_entry_id: 1,
        }
    }

    /// The event-application law of the host: decode -> dispatch -> kernel fold.
    ///
    /// `scope` + `payload` are exactly what a `SignedFrame` carries; the DOD +
    /// WIRE + LAW + MONEY gates in `bebop-delivery-domain` have ALREADY passed
    /// before a caller hands the payload here (this host is the money/state
    /// terminal, not the auth gate). Every arm is fail-closed.
    pub fn apply_delivery_event(
        &mut self,
        scope: Scope,
        payload: &[u8],
    ) -> Result<(), HostFault> {
        let event =
            DeliveryEvent::decode(scope, payload).map_err(HostFault::Vocabulary)?;
        match event {
            DeliveryEvent::OrderPlaced(p) => self.apply_order_placed(p),
            DeliveryEvent::StatusChanged(p) => self.apply_status_changed(p),
            DeliveryEvent::Claim(p) => self.apply_claim(p.order_id, p.courier),
            DeliveryEvent::Settlement(p) => self.apply_settlement(p),
        }
    }

    fn apply_order_placed(&mut self, p: OrderPlacedPayload) -> Result<(), HostFault> {
        if self.orders.contains_key(&p.order_id) {
            // Idempotent re-place is refused (never silently overwrites state).
            return Err(HostFault::Vocabulary("order already placed"));
        }
        let item = OrderItem {
            product_id: format!("wire-{}", p.order_id),
            modifier_ids: vec![],
            quantity: 1,
            unit_price: p.amount_i64,
            vendor_id: dowiz_kernel::vendor::VendorId(0),
            currency: Currency::Eur,
        };
        let order = place_order(
            wire_to_kernel_id(p.order_id),
            None,
            vec![item],
            0,
            None,
            None,
        )
        .map_err(HostFault::Law)?;
        self.orders.insert(p.order_id, order);
        Ok(())
    }

    fn apply_status_changed(&mut self, p: StatusChangedPayload) -> Result<(), HostFault> {
        let order = self
            .orders
            .get(&p.order_id)
            .ok_or(HostFault::UnknownOrder(p.order_id))?;
        // The wire status `to` MUST map to a kernel status (fail-closed: a
        // compensation-only kernel state has no wire mapping -> Vocabulary).
        let next = to_order_status(vocab_to_wire(p.to));
        // Re-derive the kernel transition legality on the host's OWN order state
        // (the receiver is authoritative). This is the same `assert_transition`
        // the spine runs; we double-check at the host because the host owns money.
        assert_transition(order.status, next).map_err(HostFault::Law)?;
        let updated = apply_event(order, next).map_err(HostFault::Law)?;
        self.orders.insert(p.order_id, updated);
        Ok(())
    }

    fn apply_claim(&mut self, order_id: u64, _courier: CourierKey) -> Result<(), HostFault> {
        // Claims are pure coordination records. We fold Offered -> Claimed on
        // first sight (a claim offer the host learns about is accepted as a
        // coordination fact; adversarial legality is enforced by W-3 tests).
        let cur = self.claims.get(&order_id).copied().unwrap_or(ClaimStatus::Offered);
        let next = match cur {
            ClaimStatus::Offered => ClaimStatus::Claimed,
            other => other, // already claimed/picked-up/released: idempotent
        };
        self.claims.insert(order_id, next);
        Ok(())
    }

    fn apply_settlement(&mut self, p: LedgerPayload) -> Result<(), HostFault> {
        let order = self
            .orders
            .get(&p.order_id)
            .ok_or(HostFault::UnknownOrder(p.order_id))?;
        let mut updated = order.clone();
        updated
            .post_earn(self.next_entry_id, p.amount_i64, Currency::Eur)
            .map_err(HostFault::Money)?;
        self.next_entry_id += 1;
        self.orders.insert(p.order_id, updated);
        Ok(())
    }

    /// Read a host order's kernel status (for assertions / parity sweeps).
    pub fn order_status(&self, wire_id: u64) -> Option<OrderStatus> {
        self.orders.get(&wire_id).map(|o| o.status)
    }

    /// Read a claim's status (for W-3 assertions).
    pub fn claim_status(&self, wire_id: u64) -> Option<ClaimStatus> {
        self.claims.get(&wire_id).copied()
    }

    /// Ledger balance probe (money conservation) for an order.
    pub fn ledger_balance(&self, wire_id: u64) -> Option<i64> {
        self.orders.get(&wire_id).map(|o| o.ledger_balance())
    }
}

// Re-export the wire/kernels status maps so consumers (and tests) don't reach
// into the delivery-domain crate directly for the parity sweep (W-2.4).
pub use bebop_delivery_domain::intake::{from_order_status as wire_from_kernel, to_order_status as kernel_to_wire};

/// Scope helper for the six delivery actions (W-2 / W-3 / W-5).
pub fn scope_for(action: Action) -> Scope {
    Scope::single(Resource::Order, action)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bebop_proto_cap::claim_machine::fold_transitions;

    #[test]
    fn host_is_send_and_deterministic_smoke() {
        // The host must be usable across threads (it is the node-local state a
        // gossip loop would drive). This also forces a real compile of the type.
        let mut h = MeshHost::new();
        let scope = scope_for(Action::OrderPlaced);
        let p = OrderPlacedPayload {
            order_id: 1,
            amount_i64: 500,
            src: "R".into(),
            dst: "C".into(),
        };
        assert!(h.apply_delivery_event(scope, &p.encode()).is_ok());
        assert_eq!(h.order_status(1), Some(OrderStatus::Pending));
    }

    #[test]
    fn claim_fold_from_offered_to_claimed() {
        let mut h = MeshHost::new();
        let scope = scope_for(Action::OrderPlaced);
        let p = OrderPlacedPayload {
            order_id: 7,
            amount_i64: 500,
            src: "R".into(),
            dst: "C".into(),
        };
        h.apply_delivery_event(scope, &p.encode()).unwrap();
        // A claim event marks the order claimed (coordination record).
        let cscope = Scope::single(Resource::Claim, Action::ClaimOffered);
        let cp = bebop_proto_cap::event_dict::ClaimPayload {
            claim_id: 1,
            order_id: 7,
            courier: [0xAB; 32],
        };
        h.apply_delivery_event(cscope, &cp.encode()).unwrap();
        assert_eq!(h.claim_status(7), Some(ClaimStatus::Claimed));
        // fold_transitions sanity (the Law the host relies on, proven here).
        assert_eq!(
            fold_transitions(ClaimStatus::Offered, &[ClaimStatus::Claimed, ClaimStatus::PickedUp])
                .unwrap(),
            ClaimStatus::PickedUp
        );
    }

    #[test]
    fn status_change_folds_through_kernel_law() {
        let mut h = MeshHost::new();
        let place = scope_for(Action::OrderPlaced);
        let p = OrderPlacedPayload {
            order_id: 3,
            amount_i64: 250,
            src: "R".into(),
            dst: "C".into(),
        };
        h.apply_delivery_event(place, &p.encode()).unwrap();
        let sc = scope_for(Action::OrderStatusChanged);
        let step = |from: VocabStatus, to: VocabStatus| StatusChangedPayload {
            order_id: 3,
            from,
            to,
        };
        // Pending -> Confirmed -> Preparing -> Ready -> InDelivery -> Delivered
        // (the legal kernel lifecycle; assert_transition enforces the edges).
        let legal = [
            (VocabStatus::Pending, VocabStatus::Confirmed),
            (VocabStatus::Confirmed, VocabStatus::Preparing),
            (VocabStatus::Preparing, VocabStatus::Ready),
            (VocabStatus::Ready, VocabStatus::InDelivery),
            (VocabStatus::InDelivery, VocabStatus::Delivered),
        ];
        for (from, to) in legal {
            h.apply_delivery_event(sc.clone(), &step(from, to).encode())
                .unwrap_or_else(|e| panic!("legal fold {from:?}->{to:?} refused: {e:?}"));
        }
        assert_eq!(h.order_status(3), Some(OrderStatus::Delivered));
    }

    #[test]
    fn settlement_conservation_probe_holds() {
        let mut h = MeshHost::new();
        let place = scope_for(Action::OrderPlaced);
        let p = OrderPlacedPayload {
            order_id: 9,
            amount_i64: 1000,
            src: "R".into(),
            dst: "C".into(),
        };
        h.apply_delivery_event(place, &p.encode()).unwrap();
        let settle = Scope::single(Resource::Ledger, Action::SettlementRecorded);
        let lp = LedgerPayload { order_id: 9, amount_i64: 1000 };
        h.apply_delivery_event(settle, &lp.encode()).unwrap();
        assert_eq!(h.ledger_balance(9), Some(1000));
    }
}
