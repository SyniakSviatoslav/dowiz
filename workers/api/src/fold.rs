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
//! WHY HERE AND NOT IN `dowiz-hub`. That crate's `minijson` says it plainly: it
//! is not a general JSON parser and is never pointed at untrusted documents.
//! An order envelope holds a customer's own words -- a name, a doorbell, a note
//! -- so folding it needs a real parser, and the real parser lives on this side
//! of the boundary. `dowiz-hub` hands out events; the Worker folds them.

use serde_json::{Map, Value};

/// The key that says "this payload is a change, not a state".
///
/// Short because it is written into every event, and underscored because an
/// order envelope's own fields come from the kernel and never start with one.
pub const DELTA_MARK: &str = "_d";

/// Merge `delta` into `base`. A `null` DELETES the key -- that is how an
/// envelope that dropped a field (an unassigned courier, a cleared note) says
/// so, and without it a delta could only ever add.
fn merge(base: &mut Map<String, Value>, delta: &Map<String, Value>) {
    for (k, v) in delta {
        if k == DELTA_MARK {
            continue;
        }
        match (base.get_mut(k), v) {
            (_, Value::Null) => {
                base.remove(k);
            }
            // Both objects: recurse, so `{"fulfilment":{"eta_ms":…}}` does not
            // throw away the address beside it.
            (Some(Value::Object(b)), Value::Object(d)) => {
                let d = d.clone();
                merge(b, &d);
            }
            // Anything else -- an array, a number, a type change -- is replaced
            // whole. An items list is one value; merging two of them index by
            // index would invent an order nobody placed.
            _ => {
                base.insert(k.clone(), v.clone());
            }
        }
    }
}

/// Is this payload a delta rather than a snapshot?
pub fn is_delta(v: &Value) -> bool {
    v.get(DELTA_MARK).and_then(Value::as_bool) == Some(true)
}

fn object_is_delta(obj: &Map<String, Value>) -> bool {
    obj.get(DELTA_MARK).and_then(Value::as_bool) == Some(true)
}

/// Fold one order's events, OLDEST FIRST, into its current state.
///
/// A payload that is not a JSON object is skipped rather than allowed to wipe
/// the state: a damaged record must not be able to empty an order.
pub fn fold<'a>(payloads: impl IntoIterator<Item = &'a str>) -> Value {
    let mut state = Value::Null;
    for raw in payloads {
        state = fold_one(state, raw);
    }
    state
}

/// Apply ONE payload to a state — the step `fold` repeats, exposed so a caller
/// folding many orders in one pass does not have to re-walk a history per
/// order.
pub fn fold_one(state: Value, raw: &str) -> Value {
    let Ok(v) = serde_json::from_str::<Value>(raw) else { return state };
    let Value::Object(obj) = v else { return state };
    if !object_is_delta(&obj) {
        return Value::Object(obj);
    }
    let mut base = match state {
        Value::Object(m) => m,
        _ => Map::new(),
    };
    merge(&mut base, &obj);
    Value::Object(base)
}

/// The smallest payload that turns `old` into `new`.
///
/// DERIVED, NOT DECLARED. The alternative -- each caller listing the fields it
/// means to change -- is a list that drifts from what the kernel actually
/// returned, and the first thing it drops is the field somebody added last
/// week. Comparing the two states cannot drift: whatever moved is in here, and
/// whatever did not is not.
pub fn delta(old: &Value, new: &Value) -> Value {
    let (Value::Object(o), Value::Object(n)) = (old, new) else {
        // Nothing to diff against: the new state IS the payload, as a snapshot.
        return new.clone();
    };
    let mut out = Map::new();
    for (k, v) in n {
        match o.get(k) {
            Some(prev) if prev == v => {}
            Some(Value::Object(po)) if v.is_object() => {
                let nested = delta(&Value::Object(po.clone()), v);
                if let Value::Object(mut m) = nested {
                    m.remove(DELTA_MARK);
                    if !m.is_empty() {
                        out.insert(k.clone(), Value::Object(m));
                    }
                }
            }
            _ => {
                out.insert(k.clone(), v.clone());
            }
        }
    }
    // A key the new state no longer has is a DELETION, and it has to be said
    // out loud: silence means "unchanged" in a delta.
    for k in o.keys() {
        if !n.contains_key(k) {
            out.insert(k.clone(), Value::Null);
        }
    }
    out.insert(DELTA_MARK.to_string(), Value::Bool(true));
    Value::Object(out)
}

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
        assert_eq!(d["courier_id"], Value::Null);
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
        assert_eq!(folded, after);
    }
}
