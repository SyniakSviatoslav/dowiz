//! RENAMING, MOVING AND ORDERING (W-CRUD, 2026-09-29): the three edits the
//! menu had no door for.
//!
//! THE DEFECT, measured on qa-durres 2026-09-29 (`e2e/kit-regression/
//! _probe_crud.mjs`): `POST /api/owner/products/:id` took fourteen fields and
//! answered `{"ok":true}` to a `name`, a `description` or a `category_id` it
//! then dropped on the floor. A dish could not be renamed in the venue's own
//! language, could not change category, and neither a dish nor a category
//! could be moved up or down. The console had no control for any of it.
//!
//! ORDER IS A RENUMBERING, NOT A NUMBER. The console never types a sort key:
//! it says "up" or "down", and the category's dishes (or the venue's
//! categories) are renumbered 10, 20, 30... in their present order with the
//! two neighbours swapped. Ties -- an imported menu arrives at `sortOrder` 0
//! across the board -- are broken by id, which is the order the storefront
//! already shows them in (`fold::menu_venue::render` sorts by `sortOrder`
//! over the catalogue's key order). Everything here is PURE on the catalogue.

use dowiz_hub::catalog::Catalog;
use serde_json::{json, Value};

use super::{NAME_MAX, SORT_STEP};

/// A description is the venue's paragraph, not a page.
pub const DESCRIPTION_MAX: usize = 2000;

/// One step of the console's order buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Up,
    Down,
}

impl Move {
    pub fn from_wire(s: &str) -> Result<Move, String> {
        match s.trim() {
            "up" => Ok(Move::Up),
            "down" => Ok(Move::Down),
            other => Err(format!("{other:?} is not a move: up or down")),
        }
    }
}

/// The venue's own name and description on a stored dish. An absent field is
/// left alone; an empty or over-long name is refused; an empty description is
/// stored as "" (the dish has none), never refused.
pub fn rename(p: &mut Value, name: Option<&str>, description: Option<&str>) -> Result<(), String> {
    if let Some(n) = name {
        let n = n.trim();
        if n.is_empty() {
            return Err("a dish needs a name".into());
        }
        if n.chars().count() > NAME_MAX {
            return Err(format!("a name is at most {NAME_MAX} characters"));
        }
        p["name"] = json!(n);
    }
    if let Some(d) = description {
        let d = d.trim();
        if d.chars().count() > DESCRIPTION_MAX {
            return Err(format!("a description is at most {DESCRIPTION_MAX} characters"));
        }
        p["description"] = json!(d);
    }
    Ok(())
}

/// Move a stored dish to `category_id`. Refused when no such category exists;
/// the dish lands LAST in its new category, as a new dish does. A move to the
/// category it is already in changes nothing.
pub fn move_to_category(cat: &Catalog, p: &mut Value, category_id: &str) -> Result<(), String> {
    let category_id = category_id.trim();
    if !cat.categories().iter().any(|(c, _)| c == category_id) {
        return Err(format!("unknown category {category_id:?}"));
    }
    if p.get("categoryId").and_then(Value::as_str) == Some(category_id) {
        return Ok(());
    }
    let last = dishes_in_order(cat, category_id).iter().filter_map(|(_, d)| d.get("sortOrder").and_then(Value::as_i64)).max().unwrap_or(0);
    p["categoryId"] = json!(category_id);
    p["sortOrder"] = json!(last + SORT_STEP);
    Ok(())
}

fn sort_of(v: &Value) -> i64 {
    v.get("sortOrder").and_then(Value::as_i64).unwrap_or(0)
}

/// The dishes of `category`, as the storefront orders them.
fn dishes_in_order(cat: &Catalog, category: &str) -> Vec<(String, Value)> {
    let mut list: Vec<(String, Value)> = cat
        .products()
        .into_iter()
        .filter_map(|(id, j)| serde_json::from_str::<Value>(&j).ok().map(|v| (id, v)))
        .filter(|(_, v)| v.get("categoryId").and_then(Value::as_str) == Some(category))
        .collect();
    list.sort_by(|(ia, a), (ib, b)| sort_of(a).cmp(&sort_of(b)).then_with(|| ia.cmp(ib)));
    list
}

/// The venue's categories, as the storefront orders them.
fn categories_in_order(cat: &Catalog) -> Vec<(String, Value)> {
    let mut list: Vec<(String, Value)> = cat
        .categories()
        .into_iter()
        .filter_map(|(id, j)| serde_json::from_str::<Value>(&j).ok().map(|v| (id, v)))
        .collect();
    list.sort_by(|(ia, a), (ib, b)| sort_of(a).cmp(&sort_of(b)).then_with(|| ia.cmp(ib)));
    list
}

/// `order` with `id` swapped one place in direction `m`. At the edge it is
/// already where it was asked to go: the same list, not a refusal. PURE.
pub fn nudged(order: &[String], id: &str, m: Move) -> Result<Vec<String>, String> {
    let Some(at) = order.iter().position(|x| x == id) else {
        return Err(format!("unknown {id:?}"));
    };
    let mut out = order.to_vec();
    match m {
        Move::Up if at > 0 => out.swap(at, at - 1),
        Move::Down if at + 1 < out.len() => out.swap(at, at + 1),
        _ => {}
    }
    Ok(out)
}

/// Write `sortOrder` 10, 20, 30... over `records` in the order of `ids`,
/// touching only the records whose number moves. Answers how many were written.
fn renumber(records: &[(String, Value)], ids: &[String], mut write: impl FnMut(&str, &str)) -> usize {
    let mut written = 0;
    for (i, id) in ids.iter().enumerate() {
        let want = (i as i64 + 1) * SORT_STEP;
        if let Some((_, v)) = records.iter().find(|(r, _)| r == id) {
            if sort_of(v) != want {
                let mut v = v.clone();
                v["sortOrder"] = json!(want);
                write(id, &v.to_string());
                written += 1;
            }
        }
    }
    written
}

/// Move dish `id` one place up or down inside its category. Answers the
/// category's ids in their new order. `Err` names a dish the catalogue lacks.
pub fn nudge_product(cat: &mut Catalog, id: &str, m: Move) -> Result<Vec<String>, String> {
    let Some(p) = cat.product(id).and_then(|j| serde_json::from_str::<Value>(&j).ok()) else {
        return Err(format!("unknown product {id:?}"));
    };
    let category = p.get("categoryId").and_then(Value::as_str).unwrap_or("").to_string();
    let records = dishes_in_order(cat, &category);
    let order: Vec<String> = records.iter().map(|(i, _)| i.clone()).collect();
    let after = nudged(&order, id, m)?;
    renumber(&records, &after, |i, j| cat.set_product(i, j));
    Ok(after)
}

/// Move category `id` one place up or down. Answers the categories' ids in
/// their new order. `Err` names a category the catalogue lacks.
pub fn nudge_category(cat: &mut Catalog, id: &str, m: Move) -> Result<Vec<String>, String> {
    let records = categories_in_order(cat);
    let order: Vec<String> = records.iter().map(|(i, _)| i.clone()).collect();
    let after = nudged(&order, id, m).map_err(|_| format!("unknown category {id:?}"))?;
    renumber(&records, &after, |i, j| cat.set_category(i, j));
    Ok(after)
}

#[cfg(test)]
mod tests;
