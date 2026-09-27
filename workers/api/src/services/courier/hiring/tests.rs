//! W-AUDIT S8 (2026-09-27): an invitation replaces THIS venue's pending one
//! and nobody else's, through the real table turn.

use super::invite_turn;
use crate::identity_store as ids;
use dowiz_hub::table::Table;
use serde_json::json;

fn seeded(loc: &str) -> Table {
    let mut t = Table::create(1 << 20).unwrap();
    let rec = json!({"id": "inv_a", "location_id": loc, "role": "courier", "invited_phone_hash": "ph", "revoked_at_ms": null}).to_string();
    t.put(
        ids::K_INVITE,
        "inv_a",
        &rec,
        &[(ids::invite_by_phone("ph"), "inv_a".to_string()), (ids::invite_at(loc, "inv_a"), "inv_a".to_string())],
        &[],
    )
    .unwrap();
    t
}

#[test]
fn another_venues_pending_invite_is_left_alone_and_the_phone_reads_as_taken() {
    let mut t = seeded("venue-a");
    assert!(invite_turn(&mut t, "ph", "venue-b", "own", "nm", "ch", "inv_b", 5).unwrap(), "refused as taken");
    let a = ids::rec(&t, ids::K_INVITE, "inv_a").unwrap();
    assert!(a["revoked_at_ms"].is_null(), "venue A's invitation stands: {a}");
    assert!(ids::rec(&t, ids::K_INVITE, "inv_b").is_none(), "nothing was written for B");
    assert_eq!(t.lookup(&ids::invite_by_phone("ph")).as_deref(), Some("inv_a"));
}

/// The twin: the same venue inviting the same phone again replaces its own code.
#[test]
fn the_same_venue_replaces_its_own_pending_invite() {
    let mut t = seeded("venue-a");
    assert!(!invite_turn(&mut t, "ph", "venue-a", "own", "nm", "ch", "inv_a2", 5).unwrap());
    assert_eq!(ids::rec(&t, ids::K_INVITE, "inv_a").unwrap()["revoked_at_ms"], json!(5));
    assert_eq!(t.lookup(&ids::invite_by_phone("ph")).as_deref(), Some("inv_a2"));
    assert_eq!(ids::s_of(&ids::rec(&t, ids::K_INVITE, "inv_a2").unwrap(), "location_id"), "venue-a");
}
