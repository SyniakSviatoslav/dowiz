//! D18 (G6): who may read or speak in a thread, each refusal beside its twin.

use super::*;
use crate::auth::Caps;

fn customer(order_id: &str) -> Principal {
    Principal::Customer { customer_id: order_id.into(), order_id: order_id.into(), location_id: "v1".into() }
}
fn staff(caps: &[Cap]) -> Principal {
    Principal::Staff { person_id: "p1".into(), active_location_id: "v1".into(), session_id: "s1".into(), caps: Caps::of(caps) }
}

/// THE HOLE: a customer token of this venue -- a coffee's, or a booking's --
/// read and posted in ANOTHER order's thread. Now both are a 404.
#[test]
fn a_customer_token_reaches_only_its_own_orders_thread() {
    for speaking in [false, true] {
        assert_eq!(thread_party(&customer("ord_theirs"), "ord_mine", speaking), Err((404, "not found")));
        // A booking's token names the reservation, which is no order's thread.
        assert_eq!(thread_party(&customer("rsv_1"), "ord_mine", speaking), Err((404, "not found")));
        assert_eq!(thread_party(&customer("ord_mine"), "ord_mine", speaking), Ok(Party::Customer));
    }
}

/// A courier read every conversation; a courier now reads none, and never spoke.
#[test]
fn a_courier_neither_reads_nor_speaks() {
    let c = Principal::Courier { courier_id: "c1".into(), active_location_id: "v1".into(), session_id: "s".into() };
    assert!(thread_party(&c, "ord_mine", false).is_err());
    assert!(thread_party(&c, "ord_mine", true).is_err());
}

/// Staff read with a capability that works orders and never speak for the
/// venue; the owner is the venue. The twins of the two refusals.
#[test]
fn staff_read_with_a_capability_and_the_owner_is_the_venue() {
    assert_eq!(thread_party(&staff(&[]), "o", false), Err((403, "no capability for the venue's conversations")));
    assert_eq!(thread_party(&staff(&[Cap::OpenTill]), "o", false).map_err(|e| e.0), Err(403));
    assert_eq!(thread_party(&staff(&[Cap::TakeOrders]), "o", false), Ok(Party::Venue));
    assert_eq!(thread_party(&staff(&[Cap::Advance]), "o", false), Ok(Party::Venue));
    assert_eq!(thread_party(&staff(&Cap::ALL), "o", true).map_err(|e| e.0), Err(403));
    let owner = Principal::Owner { user_id: "u".into(), active_location_id: Some("v1".into()) };
    assert_eq!(thread_party(&owner, "o", true), Ok(Party::Venue));
    assert_eq!(thread_party(&owner, "o", false), Ok(Party::Venue));
}
