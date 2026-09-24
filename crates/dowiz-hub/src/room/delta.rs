//! An order is a fold over its events, and an event carries only what changed.
//! MOVED FROM `workers/api/src/fold.rs` (D7 phase 1) so the room's deciders
//! can write a delta inside wasm; the Worker re-exports every item below, and
//! the fold's own tests stay in the Worker's `fold.rs`.
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
//! A REAL PARSER. `minijson` is not a general JSON parser and is never pointed
//! at untrusted documents; an order envelope holds a customer's own words, so
//! this module uses `serde_json` (see `room`'s header).

use serde_json::{Map, Value};

/// The key that says "this payload is a change, not a state".
///
/// Short because it is written into every event, and underscored because an
/// order envelope's own fields come from the kernel and never start with one.
pub const DELTA_MARK: &str = "_d";

/// The key that lists what a delta DELETES.
///
/// A NULL IS A VALUE, NOT AN ABSENCE, and conflating the two was a real
/// divergence. The first version of this module said a deletion by writing
/// `"k": null` -- and the kernel writes explicit nulls of its own
/// (`customer_id`, `channel`, `cash_pay_with`), as does the Worker
/// (`rejection_reason` for a rejection with no reason, `fulfilment.note` for a
/// pickup). Folding those deleted the key instead of storing it, so a snapshot
/// history and a delta history disagreed about a field the pre-delta log had
/// always carried.
///
/// Deletions travel in their own list instead, at each nesting level, so a
/// null can be exactly what it is.
pub const DELTA_DROP: &str = "_x";

/// Merge `delta` into `base`. A `null` DELETES the key -- that is how an
/// envelope that dropped a field (an unassigned courier, a cleared note) says
/// so, and without it a delta could only ever add.
fn merge(base: &mut Map<String, Value>, delta: &Map<String, Value>) {
    // The deletions first, so a delta that drops a key and sets it again in
    // the same breath ends with it set. (`delta` never produces that; a
    // hand-written one might.)
    if let Some(Value::Array(dropped)) = delta.get(DELTA_DROP) {
        for k in dropped.iter().filter_map(Value::as_str) {
            base.remove(k);
        }
    }
    for (k, v) in delta {
        if k == DELTA_MARK || k == DELTA_DROP {
            continue;
        }
        match (base.get_mut(k), v) {
            // Both objects: recurse, so `{"fulfilment":{"eta_ms":…}}` does not
            // throw away the address beside it.
            (Some(Value::Object(b)), Value::Object(d)) => {
                let d = d.clone();
                merge(b, &d);
            }
            // Anything else -- an array, a number, a NULL, a type change -- is
            // the value now. An items list is one value; merging two of them
            // index by index would invent an order nobody placed.
            _ => {
                base.insert(k.clone(), v.clone());
            }
        }
    }
}

/// Is this object a delta rather than a snapshot?
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
                    // The marker is the OUTER envelope's; the drop list is
                    // this level's and stays.
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
    // out loud: silence means "unchanged" in a delta. It is said in the drop
    // list rather than as a null, because a null is a value this envelope
    // really carries.
    let dropped: Vec<Value> =
        o.keys().filter(|k| !n.contains_key(*k)).map(|k| Value::String(k.clone())).collect();
    if !dropped.is_empty() {
        out.insert(DELTA_DROP.to_string(), Value::Array(dropped));
    }
    out.insert(DELTA_MARK.to_string(), Value::Bool(true));
    Value::Object(out)
}

#[cfg(test)]
mod tests;
