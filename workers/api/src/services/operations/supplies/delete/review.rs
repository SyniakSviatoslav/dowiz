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

/// Take every line naming a `gone` supply out of the dish's option recipes
/// (`dowiz_hub::modifiers::bom::KEY`) and answer the marks, each naming its
/// `option`. An option left with no line loses its key; the map goes when
/// its last option does (the old-image rule of `modifiers::bom`). PURE.
pub(super) fn strip_option_bom(p: &mut Value, gone: &dyn Fn(&str) -> bool, name_of: &dyn Fn(&str) -> String) -> Vec<Value> {
    let key = dowiz_hub::modifiers::bom::KEY;
    let Some(map) = p.get(key).and_then(Value::as_object).cloned() else { return Vec::new() };
    let mut marks = Vec::new();
    let mut kept = serde_json::Map::new();
    for (opt, lines) in map {
        let Some(lines) = lines.as_array() else {
            kept.insert(opt, lines);
            continue;
        };
        let (lost, keep): (Vec<Value>, Vec<Value>) = lines.iter().cloned().partition(|l| l.get("supply").and_then(Value::as_str).is_some_and(gone));
        for l in &lost {
            let Some(s) = l.get("supply").and_then(Value::as_str) else { continue };
            let mut m = mark(s, &name_of(s), crate::recipe::Qty::from_stored(l).map(crate::recipe::Qty::json).unwrap_or(Value::Null), None);
            m["option"] = json!(opt);
            marks.push(m);
        }
        if !keep.is_empty() {
            kept.insert(opt, Value::Array(keep));
        }
    }
    if marks.is_empty() {
        return marks;
    }
    if let Some(obj) = p.as_object_mut() {
        if kept.is_empty() {
            obj.remove(key);
        } else {
            obj.insert(key.to_string(), Value::Object(kept));
        }
    }
    marks
}

/// A dish with no recipe of its own whose options lost a line: marked, and
/// its review row (`None` when no option lost anything). PURE.
pub(super) fn options_only(p: &mut Value, pid: &str, marks: Vec<Value>) -> Option<Value> {
    if marks.is_empty() {
        return None;
    }
    mark_on(p, "recipeReview", marks.clone());
    Some(json!({ "id": pid, "name": p.get("name").cloned().unwrap_or(json!(pid)), "removed": marks }))
}
