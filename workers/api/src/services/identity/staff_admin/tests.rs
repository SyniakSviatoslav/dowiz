//! W-FIX H2 (W-AUDIT O6): a staff invitation replaces THIS venue's pending one
//! and nobody else's, through the real table turn.

use super::invite_turn;
use crate::identity_store as ids;
use crate::services::identity::staff_rules as sr;
use dowiz_hub::table::Table;
use serde_json::{json, Value};

const EM: &str = "cook@example.com";

fn seeded(loc: &str, extra: Value) -> Table {
    let mut t = Table::create(1 << 20).unwrap();
    let mut rec = json!({"id": "inv_a", "location_id": loc, "role": "kitchen", "invited_email": EM,
                         "expires_at_ms": 1_000, "used_at_ms": null, "revoked_at_ms": null});
    for (k, v) in extra.as_object().unwrap() {
        rec[k] = v.clone();
    }
    t.put(
        sr::K_SINVITE,
        "inv_a",
        &rec.to_string(),
        &[(sr::sinvite_by_email(EM), "inv_a".to_string()), (sr::sinvite_at(loc, "inv_a"), "inv_a".to_string())],
        &[],
    )
    .unwrap();
    t
}

fn new_rec(loc: &str, id: &str) -> String {
    json!({"id": id, "location_id": loc, "role": "kitchen", "invited_email": EM, "expires_at_ms": 9_000,
           "used_at_ms": null, "revoked_at_ms": null})
    .to_string()
}

#[test]
fn another_venues_open_invite_is_left_alone_and_the_address_reads_as_taken() {
    let mut t = seeded("venue-a", json!({}));
    assert!(invite_turn(&mut t, EM, "venue-b", "inv_b", &new_rec("venue-b", "inv_b"), 5).unwrap(), "refused");
    let a = ids::rec(&t, sr::K_SINVITE, "inv_a").unwrap();
    assert!(a["revoked_at_ms"].is_null(), "venue A's invitation stands: {a}");
    assert!(ids::rec(&t, sr::K_SINVITE, "inv_b").is_none(), "nothing was written for B");
    assert_eq!(t.lookup(&sr::sinvite_by_email(EM)).as_deref(), Some("inv_a"), "A's code is still claimable");
}

/// The twin: the same venue inviting the same address again replaces its own code.
#[test]
fn the_same_venue_replaces_its_own_pending_invite() {
    let mut t = seeded("venue-a", json!({}));
    assert!(!invite_turn(&mut t, EM, "venue-a", "inv_a2", &new_rec("venue-a", "inv_a2"), 5).unwrap());
    assert_eq!(ids::rec(&t, sr::K_SINVITE, "inv_a").unwrap()["revoked_at_ms"], json!(5));
    assert_eq!(t.lookup(&sr::sinvite_by_email(EM)).as_deref(), Some("inv_a2"));
    assert_eq!(ids::s_of(&ids::rec(&t, sr::K_SINVITE, "inv_a2").unwrap(), "location_id"), "venue-a");
}

/// The other twin: another venue's SPENT, REVOKED or EXPIRED invitation does not
/// hold the address -- a cook may move from one venue to the next -- and it is
/// not written to either.
#[test]
fn another_venues_spent_revoked_or_expired_invite_is_untouched_and_does_not_hold_the_address() {
    for (why, extra, now) in [
        ("used", json!({"used_at_ms": 3}), 5),
        ("revoked", json!({"revoked_at_ms": 3}), 5),
        ("expired", json!({}), 1_000),
    ] {
        let mut t = seeded("venue-a", extra);
        let before = t.get(sr::K_SINVITE, "inv_a").unwrap();
        assert!(!invite_turn(&mut t, EM, "venue-b", "inv_b", &new_rec("venue-b", "inv_b"), now).unwrap(), "{why}");
        assert_eq!(t.get(sr::K_SINVITE, "inv_a").unwrap(), before, "{why}: A's record is not written");
        assert_eq!(t.lookup(&sr::sinvite_by_email(EM)).as_deref(), Some("inv_b"), "{why}");
    }
}

/// A first invitation to an address nobody has invited is simply written.
#[test]
fn a_first_invite_is_written_with_both_indexes() {
    let mut t = Table::create(1 << 20).unwrap();
    assert!(!invite_turn(&mut t, EM, "venue-b", "inv_b", &new_rec("venue-b", "inv_b"), 5).unwrap());
    assert_eq!(t.lookup(&sr::sinvite_by_email(EM)).as_deref(), Some("inv_b"));
    assert_eq!(t.lookup(&sr::sinvite_at("venue-b", "inv_b")).as_deref(), Some("inv_b"));
}
