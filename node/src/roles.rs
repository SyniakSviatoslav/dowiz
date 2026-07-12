//! Autonomous node roles (P4) — owner/merchant, courier, customer.
//!
//! Each role is an *intent → decide → event* state machine layered over the
//! custody-transfer [`Node`] API in `crate::lib`. A delivery order flows:
//!
//! ```text
//! Owner(PostOrder) ──make_bundle(dest=customer)──▶ Courier(accept: custody)
//!        │                                                    │
//!        │                                            forward(dest=customer)
//!        │                                                    │
//!        ▼                                                    ▼
//! Owner(Delivered) ◀──confirmation── Customer(confirm_receipt) ◀──accept+deliver
//! ```
//!
//! No crypto constants or money semantics are touched; we only compose the
//! existing `make_bundle` / `accept` / `forward` / `deliver` primitives.

use crate::{Bundle, Node};
use dowiz_kernel::pq::envelope::ENTROPY_LEN;

/// Shared lifecycle phase of one delivery order, observable across roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderPhase {
    /// Order posted by owner; no courier has taken custody yet.
    Pending,
    /// A courier has accepted custody and is carrying it.
    InCustody,
    /// Customer confirmed receipt; order closed.
    Delivered,
}

// ───────────────────────────── Owner / Merchant ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerState {
    /// No order posted yet.
    Idle,
    /// Intent `PostOrder` realized: an order bundle was created.
    Posted,
    /// Event received: customer confirmed receipt.
    Delivered,
}

/// What an owner wants to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerIntent {
    PostOrder,
}

/// The merchant that originates a delivery order.
pub struct Owner {
    pub node: Node,
    pub state: OwnerState,
    /// This owner's view of the order lifecycle.
    pub phase: OrderPhase,
}

impl Owner {
    pub fn new(eid: &str, seed: &[u8; ENTROPY_LEN], now: u64) -> Self {
        Owner {
            node: Node::new(eid, seed, now),
            state: OwnerState::Idle,
            phase: OrderPhase::Pending,
        }
    }

    /// DECIDE: turn the `PostOrder` intent into a custody bundle whose final
    /// destination is the *customer* (carried via the courier custody chain).
    /// Produces a [`Bundle`] for the courier to take into custody.
    pub fn decide_post(
        &mut self,
        customer_eid: &str,
        order: &[u8],
        creation_ts: u64,
        lifetime: u64,
    ) -> Bundle {
        assert_eq!(self.state, OwnerState::Idle, "owner must be Idle to post");
        let b = self
            .node
            .make_bundle(customer_eid, order, creation_ts, lifetime);
        self.state = OwnerState::Posted;
        b
    }

    /// EVENT: a signed confirmation bundle (addressed to this owner) arrives
    /// from the customer. Verifies + opens it, then transitions to `Delivered`.
    pub fn on_confirmation(&mut self, b: &Bundle) -> Result<(), &'static str> {
        // Only a bundle addressed to this owner can be opened/delivered here.
        let _plain = self.node.deliver(b)?;
        self.state = OwnerState::Delivered;
        self.phase = OrderPhase::Delivered;
        Ok(())
    }
}

// ──────────────────────────────── Courier ───────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourierState {
    /// Holding no order.
    Idle,
    /// Took custody of an order bundle (decided to accept).
    HasCustody,
    /// Handed custody to the final recipient (forwarded).
    Delivered,
}

/// The courier that carries an order in custody from owner to customer.
pub struct Courier {
    pub node: Node,
    pub state: CourierState,
}

impl Courier {
    pub fn new(eid: &str, seed: &[u8; ENTROPY_LEN], now: u64) -> Self {
        Courier {
            node: Node::new(eid, seed, now),
            state: CourierState::Idle,
        }
    }

    /// DECIDE: accept an offered bundle into custody (takes custody). This is
    /// the only way a courier can hold an order; `deliver` is reserved for the
    /// final recipient, so a courier can never *open* a customer-addressed order.
    pub fn decide_accept(&mut self, b: Bundle) -> Result<(), &'static str> {
        let r = self.node.accept(b);
        if r.is_ok() {
            self.state = CourierState::HasCustody;
        }
        r
    }

    /// DECIDE: forward custody to the final recipient (the customer node).
    /// Consumes the courier's custody and hands it on via [`Node::forward`].
    pub fn decide_forward(&mut self, recipient: &mut Node) -> usize {
        let handed = self.node.forward(recipient);
        if handed > 0 {
            self.state = CourierState::Delivered;
        }
        handed
    }

    /// Convenience: forward to a [`Customer`]'s node.
    pub fn forward_to_customer(&mut self, customer: &mut Customer) -> usize {
        self.decide_forward(&mut customer.node)
    }
}

// ─────────────────────────────── Customer ───────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomerState {
    /// No order received yet.
    Idle,
    /// An order bundle addressed to this customer is in custody (notified).
    Notified,
    /// Receipt confirmed back to the owner.
    Confirmed,
}

/// The recipient who confirms receipt of the delivered order.
pub struct Customer {
    pub node: Node,
    pub state: CustomerState,
}

impl Customer {
    pub fn new(eid: &str, seed: &[u8; ENTROPY_LEN], now: u64) -> Self {
        Customer {
            node: Node::new(eid, seed, now),
            state: CustomerState::Idle,
        }
    }

    /// DECIDE: receive (accept into custody) an order bundle addressed to this
    /// customer — the end of the courier's custody handoff.
    pub fn decide_receive(&mut self, b: Bundle) -> Result<(), &'static str> {
        let r = self.node.accept(b);
        if r.is_ok() {
            self.state = CustomerState::Notified;
        }
        r
    }

    /// EVENT: open the order (dest must be this customer) and produce a signed
    /// confirmation bundle addressed back to the owner. Takes the order
    /// [`Bundle`] and produces the confirmation [`Bundle`].
    pub fn confirm_receipt(
        &mut self,
        owner_eid: &str,
        b: &Bundle,
        creation_ts: u64,
        lifetime: u64,
    ) -> Result<Bundle, &'static str> {
        // Only a bundle addressed to this customer can be opened/delivered.
        let _plain = self.node.deliver(b)?;
        let confirmation = self
            .node
            .make_bundle(owner_eid, b"received", creation_ts, lifetime);
        self.state = CustomerState::Confirmed;
        Ok(confirmation)
    }
}

/// Run the full autonomous roundtrip over in-memory nodes:
/// owner posts → courier accepts → courier forwards → customer receives +
/// confirms → owner sees confirmation. Returns the three roles so callers can
/// assert final state. The order reaches [`OrderPhase::Delivered`].
pub fn demo_roundtrip() -> (Owner, Courier, Customer) {
    let seed_owner: [u8; ENTROPY_LEN] = [11u8; 32];
    let seed_courier: [u8; ENTROPY_LEN] = [22u8; 32];
    let seed_customer: [u8; ENTROPY_LEN] = [33u8; 32];
    let now = 1000u64;
    let lifetime = 3600u64;

    let owner_eid = "dtn://owner";
    let courier_eid = "dtn://courier";
    let customer_eid = "dtn://customer";

    let mut owner = Owner::new(owner_eid, &seed_owner, now);
    let mut courier = Courier::new(courier_eid, &seed_courier, now);
    let mut customer = Customer::new(customer_eid, &seed_customer, now);

    // Owner intent → decide: post the order (bundle addressed to customer).
    let order = b"deliver: 2kg durian to grid 9";
    let bundle = owner.decide_post(customer_eid, order, now, lifetime);
    assert_eq!(owner.state, OwnerState::Posted);

    // Courier intent → decide: take custody.
    courier
        .decide_accept(bundle)
        .expect("courier takes custody");
    assert_eq!(courier.state, CourierState::HasCustody);
    assert_eq!(courier.node.custody_len(), 1);

    // Courier intent → decide: forward custody to customer (uses Node::forward).
    let handed = courier.forward_to_customer(&mut customer);
    assert_eq!(handed, 1);
    assert_eq!(courier.state, CourierState::Delivered);
    assert_eq!(courier.node.custody_len(), 0);

    // Customer event: the order is now in customer custody; open it and confirm.
    let held = customer.node.custody_snapshot();
    assert_eq!(held.len(), 1);
    let order_bundle = &held[0];
    let confirmation = customer
        .confirm_receipt(owner_eid, order_bundle, now, lifetime)
        .expect("customer confirms receipt");
    assert_eq!(customer.state, CustomerState::Confirmed);

    // Owner event: see confirmation → Delivered.
    owner
        .on_confirmation(&confirmation)
        .expect("owner accepts confirmation");
    assert_eq!(owner.state, OwnerState::Delivered);
    assert_eq!(owner.phase, OrderPhase::Delivered);

    (owner, courier, customer)
}

// ── RED+GREEN tests: every gate fails if the role logic breaks ───────────────

#[cfg(test)]
mod tests {
    use super::*;

    const S_OWNER: [u8; ENTROPY_LEN] = [1u8; 32];
    const S_COURIER: [u8; ENTROPY_LEN] = [2u8; 32];
    const S_CUSTOMER: [u8; ENTROPY_LEN] = [3u8; 32];
    const NOW: u64 = 1000;
    const LIFE: u64 = 3600;
    const OWNER: &str = "dtn://owner";
    const COURIER: &str = "dtn://courier";
    const CUSTOMER: &str = "dtn://customer";

    #[test]
    fn green_full_roundtrip_reaches_delivered() {
        let (owner, courier, customer) = demo_roundtrip();
        assert_eq!(owner.state, OwnerState::Delivered);
        assert_eq!(owner.phase, OrderPhase::Delivered);
        assert_eq!(courier.state, CourierState::Delivered);
        assert_eq!(courier.node.custody_len(), 0);
        assert_eq!(customer.state, CustomerState::Confirmed);
    }

    #[test]
    fn green_owner_posts_and_courier_takes_custody() {
        let mut owner = Owner::new(OWNER, &S_OWNER, NOW);
        let mut courier = Courier::new(COURIER, &S_COURIER, NOW);
        let bundle = owner.decide_post(CUSTOMER, b"order-1", NOW, LIFE);
        assert_eq!(owner.state, OwnerState::Posted);
        assert!(courier.decide_accept(bundle).is_ok());
        assert_eq!(courier.state, CourierState::HasCustody);
        assert_eq!(courier.node.custody_len(), 1);
    }

    #[test]
    fn green_customer_confirms_back_to_owner() {
        let mut owner = Owner::new(OWNER, &S_OWNER, NOW);
        let mut courier = Courier::new(COURIER, &S_COURIER, NOW);
        let mut customer = Customer::new(CUSTOMER, &S_CUSTOMER, NOW);
        let bundle = owner.decide_post(CUSTOMER, b"order-2", NOW, LIFE);
        courier.decide_accept(bundle).unwrap();
        courier.forward_to_customer(&mut customer);
        let held = customer.node.custody_snapshot();
        let order_bundle = &held[0];
        let confirmation = customer
            .confirm_receipt(OWNER, order_bundle, NOW, LIFE)
            .unwrap();
        assert!(owner.on_confirmation(&confirmation).is_ok());
        assert_eq!(owner.phase, OrderPhase::Delivered);
    }

    #[test]
    fn green_customer_direct_receive() {
        // Direct receipt path: courier hands the bundle to the customer role,
        // which accepts it into its own custody.
        let mut owner = Owner::new(OWNER, &S_OWNER, NOW);
        let mut courier = Courier::new(COURIER, &S_COURIER, NOW);
        let mut customer = Customer::new(CUSTOMER, &S_CUSTOMER, NOW);
        let bundle = owner.decide_post(CUSTOMER, b"order-3", NOW, LIFE);
        courier.decide_accept(bundle.clone()).unwrap();
        assert!(customer.decide_receive(bundle).is_ok());
        assert_eq!(customer.state, CustomerState::Notified);
    }

    #[test]
    fn red_courier_cannot_deliver_without_custody() {
        // A courier that has NOT taken custody of a customer-addressed order
        // (and is not the final recipient) must be rejected when trying to
        // deliver/open it.
        let mut owner = Owner::new(OWNER, &S_OWNER, NOW);
        let courier = Courier::new(COURIER, &S_COURIER, NOW);
        let customer = Customer::new(CUSTOMER, &S_CUSTOMER, NOW);
        let bundle = owner.decide_post(CUSTOMER, b"order-x", NOW, LIFE);

        // Courier never accepted → no custody. Attempting to deliver (open) a
        // bundle not addressed to the courier is rejected.
        assert_eq!(courier.node.custody_len(), 0);
        assert_eq!(courier.node.deliver(&bundle), Err("not-addressed-to-me"));
        // Still idle, never reached Delivered / HasCustody.
        assert_eq!(courier.state, CourierState::Idle);
        // The real recipient (customer) can deliver it.
        assert!(customer.node.deliver(&bundle).is_ok());
    }
}
