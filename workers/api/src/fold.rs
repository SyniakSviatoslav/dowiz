//! An order is a fold over its events, and an event carries only what changed.
//!
//! WHAT THIS REPLACES. Every `Advanced` event carried the WHOLE order envelope:
//! the items, the contact, the address, the money, all of it, six times for one
//! delivered order. Measured on the live venue that is 583 cells -- 4.66 KB --
//! per event, so a delivery wrote about 28 KB of which roughly 27 were copies
//! of what the log already held. A status change is a status change; it should
//! cost a status change.
//!
//! THE OLD IMAGES STILL READ CORRECTLY, and that is the constraint that shapes
//! everything here. A delta is MARKED (`"_d": true`) and anything unmarked is a
//! full snapshot that REPLACES the state. So an image written before this change
//! folds to exactly what `Hub::order` used to return -- the newest envelope --
//! while a new one folds the snapshot and the deltas after it. There is no
//! migration, no version flag, and no moment where the two disagree.
//!
//! MOVED to `dowiz_hub::room::delta` (D7 phase 1). The room's deciders write
//! their delta inside the hub now -- and inside wasm, for a tablet offline --
//! so the one fold and the one diff live there, beside them. The hub now
//! parses with `serde_json` (its `minijson` is still never pointed at an
//! envelope). Every name is re-exported here; the tests below are the fold's
//! own and run against the moved code.

pub use dowiz_hub::room::delta::{delta, fold, fold_one, DELTA_DROP, DELTA_MARK};

#[cfg(test)]
use serde_json::Value;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn placed() -> Value {
        json!({
            "id": "ord_1",
            "status": "PENDING",
            "total": 2650,
            "created_at_ms": 1789000000000i64,
            "items": [{"product_id": "item-01", "quantity": 2, "unit_price": 900}],
            "contact": {"name": "Ana Hoxha", "phone": "+355691234567"},
            "fulfilment": {"kind": "delivery", "address": {"line": "Rruga Taulantia 12"}}
        })
    }

    /// THE PROPERTY THE WHOLE CHANGE RESTS ON: a history of full envelopes and
    /// a history of deltas fold to the same order. If this ever fails, some
    /// venue's live image reads differently after a deploy.
    #[test]
    fn a_delta_history_folds_to_the_same_order_as_a_full_one() {
        let mut full: Vec<Value> = vec![placed()];
        let mut deltas: Vec<Value> = vec![placed()];
        let mut state = placed();
        for (status, stamp) in [
            ("CONFIRMED", "confirmed_ms"),
            ("COOKING", "cooking_ms"),
            ("READY", "ready_ms"),
            ("IN_DELIVERY", "picked_ms"),
            ("DELIVERED", "delivered_ms"),
        ] {
            let mut next = state.clone();
            next["status"] = json!(status);
            next[stamp] = json!(1789000000000i64);
            full.push(next.clone());
            deltas.push(delta(&state, &next));
            state = next;
        }
        let as_strings = |v: &Vec<Value>| -> Vec<String> { v.iter().map(|x| x.to_string()).collect() };
        let f = as_strings(&full);
        let d = as_strings(&deltas);
        let folded_full = fold(f.iter().map(String::as_str));
        let folded_delta = fold(d.iter().map(String::as_str));
        assert_eq!(folded_full, folded_delta);
        assert_eq!(folded_delta["status"], json!("DELIVERED"));
        assert_eq!(folded_delta["contact"]["name"], json!("Ana Hoxha"));
        assert_eq!(folded_delta["items"][0]["quantity"], json!(2));

        // And the deltas really are small: the point of the exercise.
        let full_bytes: usize = f.iter().map(String::len).sum();
        let delta_bytes: usize = d.iter().map(String::len).sum();
        assert!(
            delta_bytes * 3 < full_bytes,
            "deltas {delta_bytes} should be far under full {full_bytes}"
        );
    }

    /// An image written before this change has no deltas in it at all, and
    /// must fold to exactly what `Hub::order` returned then: the newest
    /// envelope, including the fields the newest envelope DROPPED.
    #[test]
    fn a_history_of_snapshots_folds_to_the_newest_snapshot() {
        let a = json!({"id": "ord_1", "status": "PENDING", "note": "no onions"});
        let b = json!({"id": "ord_1", "status": "CONFIRMED"});
        let raw = [a.to_string(), b.to_string()];
        assert_eq!(fold(raw.iter().map(String::as_str)), b, "a snapshot replaces, never merges");
    }

    /// A field the new state dropped is deleted by the delta, not resurrected.
    #[test]
    fn a_dropped_field_is_deleted_rather_than_kept() {
        let before = json!({"id": "o", "courier_id": "c1", "status": "IN_DELIVERY"});
        let after = json!({"id": "o", "status": "READY"});
        let d = delta(&before, &after);
        assert_eq!(d[DELTA_DROP], json!(["courier_id"]));
        let raw = [before.to_string(), d.to_string()];
        assert_eq!(fold(raw.iter().map(String::as_str)), after);
    }

    /// A NULL IS A VALUE AND SURVIVES. The kernel writes explicit nulls --
    /// `customer_id`, `channel`, `cash_pay_with` -- and so does this Worker:
    /// `rejection_reason` is null when an order is rejected without a reason.
    /// While a null MEANT deletion, folding a delta lost a key that every
    /// pre-delta snapshot carried, and a snapshot history and a delta history
    /// disagreed about the shape of the same order.
    #[test]
    fn an_explicit_null_is_stored_rather_than_treated_as_a_deletion() {
        let before = json!({"id": "o", "status": "PENDING"});
        let after = json!({"id": "o", "status": "REJECTED", "rejection_reason": null});
        let d = delta(&before, &after);
        assert_eq!(d["rejection_reason"], Value::Null, "the null is in the delta");
        assert!(d.get(DELTA_DROP).is_none(), "and nothing was dropped");

        let raw = [before.to_string(), d.to_string()];
        let folded = fold(raw.iter().map(String::as_str));
        assert_eq!(folded, after, "the fold keeps the key, with its null");
        assert!(
            folded.as_object().unwrap().contains_key("rejection_reason"),
            "the key must be present: {folded}"
        );
    }

    /// A null that turns into a value, and a value that turns into a null,
    /// both travel as themselves.
    #[test]
    fn a_null_and_a_value_replace_each_other_in_both_directions() {
        let none = json!({"note": null, "id": "o"});
        let some = json!({"note": "ring twice", "id": "o"});
        let there = [none.to_string(), delta(&none, &some).to_string()];
        assert_eq!(fold(there.iter().map(String::as_str)), some);
        let back = [some.to_string(), delta(&some, &none).to_string()];
        assert_eq!(fold(back.iter().map(String::as_str)), none);
    }

    /// A nested deletion is said at the level it happened.
    #[test]
    fn a_nested_deletion_travels_with_its_own_level() {
        let before = json!({"fulfilment": {"kind": "delivery", "eta_ms": 900}});
        let after = json!({"fulfilment": {"kind": "delivery"}});
        let d = delta(&before, &after);
        assert_eq!(d["fulfilment"][DELTA_DROP], json!(["eta_ms"]));
        let raw = [before.to_string(), d.to_string()];
        assert_eq!(fold(raw.iter().map(String::as_str)), after);
    }

    /// A nested change carries only the nested key, and the siblings survive.
    #[test]
    fn a_nested_change_does_not_flatten_its_siblings() {
        let before = placed();
        let mut after = before.clone();
        after["fulfilment"]["eta_ms"] = json!(1789000600000i64);
        let d = delta(&before, &after);
        assert_eq!(d["fulfilment"], json!({"eta_ms": 1789000600000i64}), "only the new key");
        let raw = [before.to_string(), d.to_string()];
        let folded = fold(raw.iter().map(String::as_str));
        assert_eq!(folded["fulfilment"]["address"]["line"], json!("Rruga Taulantia 12"));
        assert_eq!(folded["fulfilment"]["eta_ms"], json!(1789000600000i64));
    }

    /// An items list is replaced whole. Merging two arrays index by index would
    /// leave a line from the old order inside the new one.
    #[test]
    fn a_changed_items_list_is_replaced_not_merged() {
        let before = json!({"items": [{"product_id": "a", "quantity": 2}, {"product_id": "b", "quantity": 1}]});
        let after = json!({"items": [{"product_id": "a", "quantity": 1}]});
        let d = delta(&before, &after);
        let raw = [before.to_string(), d.to_string()];
        assert_eq!(fold(raw.iter().map(String::as_str)), after);
    }

    /// A damaged record must not be able to empty an order.
    #[test]
    fn a_broken_payload_is_skipped_rather_than_obeyed() {
        let good = placed();
        let raw = [good.to_string(), "not json".to_string(), "[1,2,3]".to_string(), "null".to_string()];
        assert_eq!(fold(raw.iter().map(String::as_str)), good);
    }

    /// Nothing changed: the delta is empty but still a delta, and folding it
    /// leaves the state exactly where it was.
    #[test]
    fn an_empty_delta_changes_nothing() {
        let s = placed();
        let d = delta(&s, &s);
        assert_eq!(d, json!({ DELTA_MARK: true }));
        assert!(d.get(DELTA_DROP).is_none());
        let raw = [s.to_string(), d.to_string()];
        assert_eq!(fold(raw.iter().map(String::as_str)), s);
    }

    /// An order whose first event is somehow a delta still answers with what is
    /// known, rather than with nothing.
    #[test]
    fn a_delta_with_no_snapshot_before_it_is_still_read() {
        let d = json!({ DELTA_MARK: true, "status": "COOKING" });
        let raw = [d.to_string()];
        assert_eq!(fold(raw.iter().map(String::as_str)), json!({"status": "COOKING"}));
    }

    /// The marker never reaches the folded order: it is envelope machinery, and
    /// a consumer that saw it would have to know to ignore it.
    #[test]
    fn the_marker_is_not_part_of_the_order() {
        let before = placed();
        let mut after = before.clone();
        after["status"] = json!("CONFIRMED");
        let raw = [before.to_string(), delta(&before, &after).to_string()];
        let folded = fold(raw.iter().map(String::as_str));
        assert!(folded.get(DELTA_MARK).is_none(), "{folded}");
        assert!(folded.get(DELTA_DROP).is_none(), "nor the drop list: {folded}");
        assert_eq!(folded, after);
    }
}
