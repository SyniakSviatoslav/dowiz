//! The delta is the smallest payload that folds `old` into `new`. The Worker's
//! `fold.rs` keeps the history-shaped tests; these pin the diff itself.

use super::*;
use serde_json::json;

#[test]
fn a_delta_folds_the_old_state_into_the_new_one() {
    let old = json!({"id": "r1", "total": 1500, "fulfilment": {"table": "7", "kind": "dine_in"}, "note": "x"});
    let new = json!({"id": "r1", "total": 1800, "fulfilment": {"table": "9", "kind": "dine_in"}, "tip": null});
    let d = delta(&old, &new);
    assert_eq!(d, json!({"total": 1800, "fulfilment": {"table": "9"}, "tip": null, "_x": ["note"], "_d": true}));
    assert_eq!(fold_one(old, &d.to_string()), new);
}

#[test]
fn nothing_changed_is_the_bare_mark_and_a_snapshot_replaces() {
    let s = json!({"id": "r1", "total": 1});
    assert_eq!(delta(&s, &s), json!({"_d": true}));
    assert_eq!(fold_one(s.clone(), &delta(&s, &s).to_string()), s);
    // An unmarked object is a snapshot: it REPLACES, never merges.
    assert_eq!(fold_one(s, r#"{"id":"r2"}"#), json!({"id": "r2"}));
}

#[test]
fn a_broken_payload_leaves_the_state_alone() {
    let s = json!({"id": "r1"});
    assert_eq!(fold_one(s.clone(), "{not json"), s);
    assert_eq!(fold(["{\"id\":\"r1\"}", "[1,2]"]), s, "a non-object cannot empty an order");
}
