use super::*;
use serde_json::json;

const NOW: i64 = 1_790_000_000_000;

/// THE REASON THE SESSION ROW EXISTS. A staff token is signed and in date, but
/// the owner ended its session a minute ago: it must not open the door for the
/// remaining eleven hours of its life.
#[test]
fn a_revoked_staff_session_is_refused_before_its_token_expires() {
    let live = session_record("p1", "sushi-durres", Preset::Kitchen, NOW);
    assert_eq!(session_verdict(Some(&live), "p1", "sushi-durres", NOW + 1), Ok(()));
    let ended = revoked(live, NOW + 60_000);
    assert_eq!(
        session_verdict(Some(&ended), "p1", "sushi-durres", NOW + 61_000),
        Err("staff session revoked")
    );
}

#[test]
fn an_expired_staff_session_is_refused() {
    let live = session_record("p1", "sushi-durres", Preset::Kitchen, NOW);
    assert_eq!(
        session_verdict(Some(&live), "p1", "sushi-durres", NOW + STAFF_TTL_MS),
        Err("staff session expired")
    );
}

/// A session is somebody's, at somewhere. Presenting another person's session
/// id, or one minted at the venue next door, is the same answer as none.
#[test]
fn a_session_of_another_person_or_venue_is_no_session() {
    let live = session_record("p1", "sushi-durres", Preset::Kitchen, NOW);
    assert_eq!(session_verdict(Some(&live), "p2", "sushi-durres", NOW), Err("no such staff session"));
    assert_eq!(session_verdict(Some(&live), "p1", "dubin-durres", NOW), Err("no such staff session"));
    assert_eq!(session_verdict(None, "p1", "sushi-durres", NOW), Err("no such staff session"));
}

/// An owner is not made by a staff invite; the four staff words are invitable.
#[test]
fn only_the_ruled_staff_words_can_be_invited() {
    assert_eq!(invitable("kitchen"), Ok(Preset::Kitchen));
    assert_eq!(invitable(" counter-manager "), Ok(Preset::CounterManager));
    assert_eq!(invitable("waiter"), Ok(Preset::Waiter));
    assert_eq!(invitable("owner"), Err("an owner is not invited as staff"));
    assert_eq!(invitable("courier"), Err("unknown staff role"));
}

#[test]
fn an_invite_needs_an_address_and_a_name() {
    assert_eq!(invite_fields(" Ana@Dubin.AL ", " Ana "), Ok(("ana@dubin.al".into(), "Ana".into())));
    assert!(invite_fields("ana", "Ana").is_err());
    assert!(invite_fields("ana@dubin.al", "  ").is_err());
}

/// The console lists staff, never owners: an owner cannot demote or suspend
/// themselves from a list they are not on. Staff members are kitchen, waiter, and counter-manager.
#[test]
fn an_owner_membership_is_not_a_staff_row() {
    let owner = json!({"role": "owner", "status": "active"});
    let kitchen = json!({"role": "kitchen", "status": "active"});
    let waiter = json!({"role": "waiter", "status": "active"});
    let counter = json!({"role": "counter-manager", "status": "active"});
    let courier = json!({"role": "courier", "status": "active"});
    assert!(!is_staff_member(&owner));
    assert!(is_staff_member(&kitchen));
    assert!(is_staff_member(&waiter));
    assert!(is_staff_member(&counter));
    assert!(!is_staff_member(&courier));
}

/// Wrong code and no invite are one answer; expiry is told apart.
#[test]
fn an_invite_is_claimed_once_with_its_own_code_before_it_expires() {
    let inv = json!({"code_hash": "h", "expires_at_ms": NOW + 10, "used_at_ms": null, "revoked_at_ms": null});
    assert_eq!(claimable(Some(&inv), "h", NOW), Ok(()));
    assert_eq!(claimable(Some(&inv), "x", NOW), Err("that code does not match"));
    assert_eq!(claimable(None, "h", NOW), Err("that code does not match"));
    assert_eq!(claimable(Some(&inv), "h", NOW + 10), Err("that code has expired -- ask for a new one"));
    let used = json!({"code_hash": "h", "expires_at_ms": NOW + 10, "used_at_ms": NOW});
    assert_eq!(claimable(Some(&used), "h", NOW), Err("that code does not match"));
}

#[test]
fn a_spent_invite_is_not_listed_and_an_expired_one_is_marked() {
    let used = json!({"used_at_ms": NOW});
    assert!(invite_row("i", &used, NOW).is_none());
    let old = json!({"invited_name": "Ana", "role": "kitchen", "expires_at_ms": NOW - 1});
    assert_eq!(invite_row("i", &old, NOW).unwrap()["expired"], json!(true));
}

/// The session scan for one person cannot reach a person whose id starts the
/// same, nor the same person at a venue whose id starts the same.
#[test]
fn a_session_prefix_is_that_person_at_that_venue_alone() {
    let p = ssessions_prefix("dubin-durres", "u1");
    assert!(ssession_of("dubin-durres", "u1", "s").starts_with(&p));
    assert!(!ssession_of("dubin-durres", "u12", "s").starts_with(&p));
    assert!(!ssession_of("dubin-durres-2", "u1", "s").starts_with(&p));
}

// ── the room's door: `auth::room_admits` ──────────────────────────────────
use crate::auth::{room_admits, Cap, Caps, Principal};

fn staff(venue: &str, caps: &[Cap]) -> Principal {
    Principal::Staff {
        person_id: "p1".into(),
        active_location_id: venue.into(),
        session_id: "s1".into(),
        caps: Caps::of(caps),
    }
}

/// ITEM 1's CHECK, native half: a kitchen token without `advance`... and a
/// waiter's token asked to advance: refused by capability, 403.
#[test]
fn a_staff_token_without_the_capability_is_refused() {
    let taker = staff("sushi-durres", &[Cap::TakeOrders, Cap::TakePayment]);
    assert_eq!(room_admits(&taker, "sushi-durres", Cap::Advance).map(|x| x.0), Err((403, "this needs a capability your role does not hold")));
    let kitchen = staff("sushi-durres", &[Cap::Advance]);
    assert_eq!(room_admits(&kitchen, "sushi-durres", Cap::Advance).map(|x| x.0), Ok("p1".to_string()));
    assert!(room_admits(&kitchen, "sushi-durres", Cap::TakePayment).is_err());
}

/// ITEM 1's CHECK: a staff token for venue A is nobody at venue B — 404, not
/// 403, so the answer does not confirm B exists.
#[test]
fn a_staff_token_for_one_venue_is_refused_at_another() {
    let kitchen = staff("sushi-durres", &[Cap::Advance]);
    assert_eq!(room_admits(&kitchen, "dubin-durres", Cap::Advance).map(|x| x.0), Err((404, "not found")));
}

/// An owner of the venue holds every capability; an owner of another venue
/// and every other role hold none.
#[test]
fn an_owner_signs_with_every_capability_and_nobody_else_does() {
    let owner = Principal::Owner { user_id: "u1".into(), active_location_id: Some("sushi-durres".into()) };
    let (who, caps) = room_admits(&owner, "sushi-durres", Cap::Void).expect("owner");
    assert_eq!(who, "u1");
    assert_eq!(caps, Caps::of(&Cap::ALL));
    assert!(room_admits(&owner, "dubin-durres", Cap::Void).is_err());
    let courier = Principal::Courier {
        courier_id: "c1".into(), active_location_id: "sushi-durres".into(), session_id: "s".into(),
    };
    assert_eq!(room_admits(&courier, "sushi-durres", Cap::Advance).map(|x| x.0), Err((403, "forbidden role")));
}

/// The owner may re-word or suspend staff, never an owner row, and never to
/// the owner word.
#[test]
fn an_owner_changes_staff_rows_and_only_staff_rows() {
    let kitchen = json!({"role": "kitchen", "status": "active"});
    let m = member_changed(&kitchen, Some("counter-manager"), Some(false)).expect("changed");
    assert_eq!(m["role"], json!("counter-manager"));
    assert_eq!(m["status"], json!("suspended"));
    assert_eq!(member_changed(&kitchen, Some("owner"), None), Err("an owner is not invited as staff"));
    let owner = json!({"role": "owner", "status": "active"});
    assert_eq!(member_changed(&owner, None, Some(false)), Err("not found"));
}
