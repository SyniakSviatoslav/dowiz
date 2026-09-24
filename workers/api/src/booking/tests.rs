use super::{ev_key, now_min, phone_index, user_key, SLOT_MAX};

/// WHAT P3 ACTUALLY BUYS, stated as a test. `now_min` read the wall clock
/// until 2026-09-22, so a booking window could only be exercised at the
/// real time — which is why the rule that decides whether a slot is in the
/// past had no test at all. It takes the instant now, so the boundary can
/// be put wherever the test wants it.
#[test]
fn a_slot_boundary_can_be_examined_at_a_chosen_instant() {
    // The minute the epoch's first hour ends, and the millisecond before.
    assert_eq!(now_min(3_600_000), 60);
    assert_eq!(now_min(3_599_999), 59, "the last millisecond is still the previous minute");
    // A second read of a real clock could land either side of that line;
    // one instant handed down cannot.
    assert_eq!(now_min(3_599_999), now_min(3_599_999));
}

/// AND IT IS NOT A DIVISION THAT ROUNDS TOWARDS ZERO BY ACCIDENT. Every
/// instant this sees is after 1970, so the behaviour below the epoch is
/// not a rule anyone relies on — it is pinned so that a future change to
/// signed arithmetic is a decision rather than a surprise.
#[test]
fn the_epoch_itself_is_minute_zero() {
    assert_eq!(now_min(0), 0);
    assert_eq!(now_min(59_999), 0);
    assert_eq!(now_min(60_000), 1);
}

/// KEYS SORT AS STRINGS, and a reservation's history replays in key order.
#[test]
fn event_keys_sort_numerically_because_they_are_padded() {
    let mut keys: Vec<String> = (1..=12).map(|n| ev_key("rsv_a", n)).collect();
    keys.sort();
    // Unpadded, "rsv_a/10" sorts before "rsv_a/2" and the fold would replay
    // the tenth transition third — reaching a different status.
    assert_eq!(keys[0], ev_key("rsv_a", 1));
    assert_eq!(keys[1], ev_key("rsv_a", 2));
    assert_eq!(keys[11], ev_key("rsv_a", 12));
}

#[test]
fn a_reservations_events_do_not_collide_with_another_reservations() {
    assert!(ev_key("rsv_a", 1) != ev_key("rsv_b", 1));
    assert!(ev_key("rsv_a", 1).starts_with("rsv_a/"));
}

/// `ORDER BY slot_min DESC` as an ASCENDING scan over the complement.
#[test]
fn a_users_bookings_scan_newest_slot_first() {
    let mut keys: Vec<String> = [100i64, 5000, 250, 99999]
        .iter()
        .map(|s| user_key("u1", *s, "id"))
        .collect();
    keys.sort();
    assert_eq!(keys[0], user_key("u1", 99999, "id"), "the latest slot comes first");
    assert_eq!(keys[3], user_key("u1", 100, "id"), "the earliest comes last");
}

#[test]
fn one_users_scan_prefix_cannot_reach_another_users_bookings() {
    assert!(user_key("u1", 10, "a").starts_with("rsv.user/u1/"));
    assert!(!user_key("u2", 10, "a").starts_with("rsv.user/u1/"));
}

/// The complement stays positive for every slot this product can mint, and
/// the padding stays wide enough that it does not itself sort wrongly.
#[test]
fn the_complement_is_positive_and_fits_its_padding() {
    let far = 60i64 * 24 * 365 * 100; // a century of minutes
    assert!(SLOT_MAX - far > 0);
    assert_eq!(format!("{:012}", SLOT_MAX).len(), 12, "SLOT_MAX overflows its own padding");
}

/// A phone's bookings are one prefix, newest first, and never another phone's.
#[test]
fn a_phones_bookings_are_one_prefix() {
    let mut keys: Vec<String> = [100i64, 900].iter().map(|s| phone_index("k1", *s, "id")).collect();
    keys.sort();
    assert_eq!(keys[0], phone_index("k1", 900, "id"), "the latest slot comes first");
    assert!(phone_index("k1", 10, "a").starts_with("rsv.phone/k1/"));
    assert!(!phone_index("k2", 10, "a").starts_with("rsv.phone/k1/"));
}
