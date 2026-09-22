//! The pure half of the idempotency layer.
//!
//! Split out of `mod.rs` when the `Guard` helper pushed that file past the
//! 300-line ratchet. `tests.rs` is exempt from it, for the reason the gate's
//! own header gives: a gate that counts test files refuses the commit that
//! adds the tests a split was done for.

use super::*;

/// RULE 2, and it is the one that would be a tenancy defect.
#[test]
fn a_key_from_one_venue_cannot_replay_into_another() {
    assert_ne!(
        scope("k1", "dubin-durres", "cust", "place"),
        scope("k1", "sushi-durres", "cust", "place")
    );
}

#[test]
fn one_persons_key_is_not_anothers() {
    assert_ne!(
        scope("k1", "v", "customer:a", "place"),
        scope("k1", "v", "customer:b", "place")
    );
}

#[test]
fn one_routes_key_does_not_replay_into_another_route() {
    assert_ne!(scope("k1", "v", "p", "place"), scope("k1", "v", "p", "refund"));
}

/// The separator is a byte no caller can send, so a key containing the
/// separator cannot be crafted to collide with another scope.
#[test]
fn the_scope_cannot_be_forged_by_a_key_that_contains_a_separator() {
    let forged = scope("dubin-durres\u{1}cust\u{1}place\u{1}k1", "v", "p", "r");
    let real = scope("k1", "dubin-durres", "cust", "place");
    assert_ne!(forged, real);
}

#[test]
fn the_same_body_prints_the_same_and_a_changed_one_does_not() {
    assert_eq!(fingerprint(r#"{"a":1}"#), fingerprint(r#"{"a":1}"#));
    assert_ne!(fingerprint(r#"{"a":1}"#), fingerprint(r#"{"a":2}"#));
    // Whitespace is NOT normalised, on purpose: two bodies that differ by
    // it are two different requests as far as this layer is concerned, and
    // guessing which differences are meaningful is how a replay returns the
    // answer to a question nobody asked.
    assert_ne!(fingerprint(r#"{"a":1}"#), fingerprint(r#"{"a": 1}"#));
}
