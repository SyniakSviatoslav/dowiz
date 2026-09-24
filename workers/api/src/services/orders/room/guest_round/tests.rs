//! A guest's round, answered by the room; the guest's read of the bill.

use super::*;

fn order(by: &str, status: &str) -> Value {
    json!({"id": "o1", "location_id": "v1", "placed_by": by, "status": status, "total": 900})
}

#[test]
fn the_room_confirms_or_rejects_a_guests_pending_round() {
    assert_eq!(answer(&order(GUEST, "PENDING"), "v1", "confirm").ok(), Some("CONFIRMED"));
    assert_eq!(answer(&order(GUEST, "PENDING"), "v1", "reject").ok(), Some("REJECTED"));
}

#[test]
fn only_a_guests_pending_round_at_this_venue_is_answered_here() {
    // a waiter's round is the kitchen's to move, not this route's
    assert!(matches!(answer(&order("staff_7", "PENDING"), "v1", "confirm"), Err(Refused::Conflict(_))));
    // already confirmed
    assert!(matches!(answer(&order(GUEST, "CONFIRMED"), "v1", "confirm"), Err(Refused::Conflict(_))));
    // another venue's round does not exist here
    assert!(matches!(answer(&order(GUEST, "PENDING"), "v2", "confirm"), Err(Refused::NotFound)));
    // an action outside the two
    assert!(matches!(answer(&order(GUEST, "PENDING"), "v1", "preparing"), Err(Refused::Invalid(_))));
}

#[test]
fn the_guest_reads_amounts_not_signers_or_payments() {
    let card = json!({"sitting_id": "s1", "bill": 900, "paid": 0, "due": 900, "rounds": [
        {"id": "o1", "seq": 3, "status": "PENDING", "total": 900, "placed_by": "staff_7",
         "payments": [{"amount": 1, "by": "staff_7"}], "amended": [], "adjustments": []}]});
    let v = guest_view(&card);
    let r = &v["rounds"][0];
    for k in ["placed_by", "payments", "amended", "adjustments", "seq"] {
        assert!(r.get(k).is_none(), "{k} reached the guest");
    }
    assert_eq!(r["total"], 900);
    assert_eq!(v["due"], 900);
}
