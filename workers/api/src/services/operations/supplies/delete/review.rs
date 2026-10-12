//! THE REVIEW MARK a deletion leaves on what used the ingredient
//! (`/root/lanes/CONTRACT-QTY.md` §2): a dish stores `recipeReview:
//! [{supply, name, qty, via?}]`, a ПФ stores `review: [{supply, name, qty}]`.
//! Both are read back as they are; the dish's goes with the next hand-saved
//! recipe (`recipe::apply::set_bom`), the ПФ's with the next card saved
//! (`preps::record` rebuilds the record from the form's keys, without it).

use serde_json::{json, Value};

/// One removed line as the mark names it. `via` is the ПФ a dish reached it
/// through, when it did not name the ingredient itself.
pub(super) fn mark(id: &str, name: &str, qty: Value, via: Option<&str>) -> Value {
    let mut m = json!({ "supply": id, "name": name, "qty": qty });
    if let Some(v) = via {
        m["via"] = json!(v);
    }
    m
}

/// Add `marks` to a record's list under `key`, keeping what was there.
pub(super) fn mark_on(rec: &mut Value, key: &str, marks: Vec<Value>) {
    let mut all = rec.get(key).and_then(Value::as_array).cloned().unwrap_or_default();
    all.extend(marks);
    rec[key] = Value::Array(all);
}

/// Does this stored record carry a mark? A dish: `recipeReview`; a ПФ:
/// `review`. What `uses` rows and the stock row answer as `review: true`.
pub fn has_mark(record_json: &str) -> bool {
    let Ok(v) = serde_json::from_str::<Value>(record_json) else { return false };
    let marks = |x: Option<&Value>| x.and_then(Value::as_array).is_some_and(|a| !a.is_empty());
    marks(v.get("recipeReview")) || marks(v.get("review"))
}
