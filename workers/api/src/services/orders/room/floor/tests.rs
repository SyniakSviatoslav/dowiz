use super::*;
use crate::auth::{room_admits, Caps, Principal};

fn staff(venue: &str, caps: &[Cap]) -> Principal {
    Principal::Staff {
        person_id: "p1".into(),
        active_location_id: venue.into(),
        session_id: "s1".into(),
        caps: Caps::of(caps),
    }
}

/// THE DOOR BOTH ROUTES USE: `staff_at(.., CAP)` ends in `room_admits(.., CAP)`
/// for a staff token. A kitchen token (Advance only) is REFUSED 403; the
/// POSITIVE TWIN, a waiter's token (TakeOrders), is admitted and signs as `by`.
#[test]
fn clearing_needs_the_take_orders_capability() {
    let kitchen = staff("sushi-durres", &[Cap::Advance]);
    assert_eq!(room_admits(&kitchen, "sushi-durres", CAP).map(|x| x.0).map_err(|e| e.0), Err(403));
    let waiter = staff("sushi-durres", &[Cap::TakeOrders]);
    assert_eq!(room_admits(&waiter, "sushi-durres", CAP).map(|x| x.0), Ok("p1".to_string()));
    // And that is the waiter's capability, not something only an owner holds.
    assert_eq!(CAP, Cap::TakeOrders);
}

/// A waiter of one venue is nobody at another: 404, and its twin is admitted.
#[test]
fn a_token_for_another_venue_is_not_found() {
    let waiter = staff("sushi-durres", &[Cap::TakeOrders]);
    assert_eq!(room_admits(&waiter, "dubin-durres", CAP).map_err(|e| e.0).map(|x| x.0), Err(404));
    assert!(room_admits(&waiter, "sushi-durres", CAP).is_ok());
}

/// ONE VENUE PER REQUEST. The host decides where it names a venue; a request
/// naming a different one is refused, never served against either.
#[test]
fn the_host_decides_and_a_second_venue_is_refused() {
    assert_eq!(venue_for(Some("b"), Some("a")).map_err(|e| e.0), Err(400));
    // Twins: host alone, host agreeing, and the apex falling back to the request.
    assert_eq!(venue_for(None, Some("a")), Ok("a".to_string()));
    assert_eq!(venue_for(Some("a"), Some("a")), Ok("a".to_string()));
    assert_eq!(venue_for(Some("b"), None), Ok("b".to_string()));
    // Neither: nothing to act on.
    assert_eq!(venue_for(None, None).map_err(|e| e.0), Err(400));
    assert_eq!(venue_for(Some("  "), None).map_err(|e| e.0), Err(400));
}
