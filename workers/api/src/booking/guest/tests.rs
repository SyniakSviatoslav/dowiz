use super::*;
use crate::auth::{Caps, Principal};
use dowiz_kernel::reservation::ReservationStatus::*;

fn customer(order_id: &str) -> Principal {
    Principal::Customer {
        customer_id: order_id.into(),
        order_id: order_id.into(),
        location_id: "v1".into(),
    }
}
fn owner() -> Principal {
    Principal::Owner { user_id: "u1".into(), active_location_id: Some("v1".into()) }
}
fn staff() -> Principal {
    Principal::Staff {
        person_id: "p1".into(),
        active_location_id: "v1".into(),
        session_id: "s1".into(),
        caps: Caps::of(&[]),
    }
}
fn courier() -> Principal {
    Principal::Courier {
        courier_id: "c1".into(),
        active_location_id: "v1".into(),
        session_id: "s1".into(),
    }
}

// ── whose booking ───────────────────────────────────────────────────────────

/// THE HOLE THIS CLOSES: `principal_at` asks only "does this principal belong
/// to this venue", and every guest who ever ordered a coffee here holds a
/// customer token that does. Before this, any of them could read any
/// booking's name and phone and cancel it.
#[test]
fn a_customer_token_for_another_booking_is_not_found() {
    assert_eq!(side_of(&customer("rsv_other"), "rsv_mine"), Err((404, "not found")));
    assert_eq!(side_of(&customer("ord_123"), "rsv_mine"), Err((404, "not found")));
}

#[test]
fn the_bookings_own_token_is_the_guest() {
    assert_eq!(side_of(&customer("rsv_mine"), "rsv_mine"), Ok(Side::Guest));
}

#[test]
fn the_owner_and_staff_are_the_venue_and_a_courier_is_neither() {
    assert_eq!(side_of(&owner(), "rsv_x"), Ok(Side::Venue));
    let floor = Principal::Staff {
        person_id: "p1".into(), active_location_id: "v1".into(), session_id: "s1".into(),
        caps: Caps::of(&[crate::auth::Cap::TakeOrders]),
    };
    assert_eq!(side_of(&floor, "rsv_x"), Ok(Side::Venue));
    assert_eq!(side_of(&courier(), "rsv_x"), Err((403, "forbidden role")));
}

/// D40 (G6): staff with no floor capability (the kitchen's `advance` only)
/// neither move a booking nor read the guest's phone on it.
#[test]
fn staff_without_the_floor_capability_are_not_the_venue() {
    assert_eq!(side_of(&staff(), "rsv_x"), Err((403, "no capability for the venue's bookings")));
    let kitchen = Principal::Staff {
        person_id: "k1".into(), active_location_id: "v1".into(), session_id: "s1".into(),
        caps: Caps::of(&[crate::auth::Cap::Advance]),
    };
    assert_eq!(side_of(&kitchen, "rsv_x").map_err(|e| e.0), Err(403));
}

/// D40 (G6): a booking is filed under the token's customer, never under a
/// `userId` the body chose; without a token it is filed under nobody.
#[test]
fn a_bookings_user_comes_from_the_token_not_the_body() {
    assert_eq!(booking_user(None, Some("someone-else")), None);
    assert_eq!(booking_user(Some(&staff()), Some("someone-else")), None);
    assert_eq!(booking_user(Some(&customer("c9")), Some("someone-else")), Some("c9".into()));
    // The twin: the owner, taking a booking for a customer, may name one.
    assert_eq!(booking_user(Some(&owner()), Some(" u7 ")), Some("u7".into()));
}

/// THE ACTOR COMES FROM THE PRINCIPAL. `create` wrote `CUSTOMER` for the owner.
#[test]
fn the_actor_is_the_side_not_the_body() {
    assert_eq!(Side::Guest.actor(), "CUSTOMER");
    assert_eq!(Side::Venue.actor(), "VENUE");
}

// ── whose move ──────────────────────────────────────────────────────────────

#[test]
fn a_guest_may_only_cancel() {
    assert_eq!(may_move(Side::Guest, CancelledByGuest), Ok(()));
    for to in [Confirmed, Declined, Seated, Completed, CancelledByVenue, NoShow] {
        assert!(may_move(Side::Guest, to).is_err(), "a guest moved a booking to {to:?}");
    }
}

#[test]
fn the_venue_may_not_sign_the_guests_cancel() {
    assert!(may_move(Side::Venue, CancelledByGuest).is_err());
    for to in [Confirmed, Declined, Seated, Completed, CancelledByVenue, NoShow] {
        assert_eq!(may_move(Side::Venue, to), Ok(()), "the venue was refused {to:?}");
    }
}

/// The console's buttons ARE this list. A request offers confirm, decline and
/// cancel; a confirmed booking offers seat, cancel and no-show; a seated one
/// offers complete; the guest's cancel is never among them.
#[test]
fn the_venues_moves_are_the_kernels_narrowed() {
    assert_eq!(moves(Side::Venue, Requested), vec![Confirmed, Declined, CancelledByVenue]);
    assert_eq!(moves(Side::Venue, Confirmed), vec![Seated, CancelledByVenue, NoShow]);
    assert_eq!(moves(Side::Venue, Seated), vec![Completed]);
    assert!(moves(Side::Venue, Completed).is_empty());
    assert_eq!(moves(Side::Guest, Requested), vec![CancelledByGuest]);
    assert!(moves(Side::Guest, Seated).is_empty(), "a seated guest cannot cancel");
}

// ── the guest's contact ─────────────────────────────────────────────────────

#[test]
fn a_booking_without_a_name_or_a_callable_phone_is_refused() {
    assert!(contact("", "+355 69 123 4567").is_err());
    assert!(contact("   ", "+355 69 123 4567").is_err());
    assert!(contact("Ana", "").is_err());
    assert!(contact("Ana", "12345").is_err(), "five digits is not a phone");
    assert!(contact(&"a".repeat(NAME_MAX + 1), "+355 69 123 4567").is_err());
}

#[test]
fn a_name_and_a_phone_are_accepted_trimmed() {
    assert_eq!(
        contact("  Ana  ", " 069 123 4567 "),
        Ok(("Ana".to_string(), "069 123 4567".to_string()))
    );
}

/// ONE PHONE, ONE KEY: the national and international spellings of one
/// Albanian number are one guest, so the cap cannot be dodged by retyping.
#[test]
fn two_spellings_of_one_phone_are_one_key() {
    let s = b"secret";
    let a = phone_key(s, "+355 69 123 4567").expect("a phone");
    assert_eq!(phone_key(s, "069 123 4567").as_deref(), Some(a.as_str()));
    assert_eq!(phone_key(s, "00355691234567").as_deref(), Some(a.as_str()));
    // A different number is a different guest.
    assert_ne!(phone_key(s, "069 123 4568"), Some(a.clone()));
    // And the key does not carry the digits.
    assert!(!a.contains("691234567"), "the index key leaked the phone: {a}");
}

#[test]
fn a_visitors_phone_still_has_a_key_and_a_short_one_has_none() {
    assert!(phone_key(b"s", "+39 333 123 4567").is_some());
    assert_eq!(phone_key(b"s", "1234"), None);
}

/// P5 (audit D38): the booking's key IS the person key the wallet and the
/// customer alias use -- one function, not a copy that could drift.
#[test]
fn the_booking_key_is_the_one_person_key() {
    use crate::services::customers::identity::person_key;
    for s in ["+355 69 123 4567", "069 123 4567", "00355691234567", "+39 333 123 4567"] {
        assert_eq!(phone_key(b"k", s), Some(person_key(b"k", s)), "{s:?}");
    }
}
