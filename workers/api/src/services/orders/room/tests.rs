use super::placer::{sitting_id_ok, staffed, Staffed};

/// ITEM 1: a waiter's round is a TABLE's round. A room token sending a
/// delivery basket is refused, not placed as somebody's delivery.
#[test]
fn a_room_token_places_only_at_a_table() {
    assert_eq!(
        staffed("p1".into(), "delivery", None, || Some("s".repeat(10))),
        Err((403, "a room token places rounds at a table, not deliveries"))
    );
    assert!(staffed("p1".into(), "pickup", None, || Some("s".repeat(10))).is_err());
}

/// Opening a table mints the sitting; the next round carries it.
#[test]
fn the_first_round_opens_a_sitting_and_the_next_one_joins_it() {
    let first = staffed("p1".into(), "dine_in", None, || Some("0190-sitting-a".into())).expect("opened");
    assert_eq!(first, Staffed { by: "p1".into(), sitting_id: "0190-sitting-a".into() });
    let second = staffed("p2".into(), "dine_in", Some("0190-sitting-a"), || None).expect("joined");
    assert_eq!(second.sitting_id, "0190-sitting-a");
    assert_eq!(second.by, "p2");
}

/// A sitting id is one the Worker minted, or it is refused.
#[test]
fn a_sitting_id_that_was_not_minted_here_is_refused() {
    assert!(sitting_id_ok("5f0c2b1e-8a4d-4e7b-9c3a-1b2c3d4e5f60"));
    assert!(!sitting_id_ok("short"));
    assert!(!sitting_id_ok("has space in it"));
    assert!(!sitting_id_ok(r#"x","total":0,"#));
    assert_eq!(
        staffed("p1".into(), "dine_in", Some("a\"b-12345"), || None),
        Err((400, "that sitting id was not minted here"))
    );
}
