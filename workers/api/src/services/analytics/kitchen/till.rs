//! TILL LINES COUNTED UNDER THEIR DISH, AT READ TIME, FOR THE HISTORY ALONE
//! (operator 2026-11-23). A sold line the import could not match keeps
//! `ebills:<code>` in the log and drew nothing (`ebills/names.rs`: not
//! retroactive, by decision). When the owner has since mapped that code, or
//! its name now belongs to exactly one dish, the kitchen's numbers count its
//! portions and revenue under that dish -- unstamped, so they cost today's
//! recipe ("today" in `menu::cogs`). The rule is the import's own: the
//! owner's mapping first, else `names::how` over every name every dish has;
//! a mapping to a dish no longer in the catalogue is no mapping.
//!
//! NOTHING IS REWRITTEN. The stored orders, the shelf and the draws stay as
//! they were; only the dish rows of this one answer move. The ingredient
//! side (`Sales::uses`, `unmodelled`) is what the stored orders drew, so it
//! is left as it is. A line still matched to no one dish stays its own row,
//! in revenue, and is NAMED in `unmatched` (by revenue), with why.
//!
//! An archived day (the cube) keeps the code and not the name: there only
//! the owner's mapping can attribute it.

use super::sales::{Dish, DishRow, Sales};
use crate::ebills::names::{self, Match};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};

/// What the venue's object knows about its till: the owner's code mapping
/// (`ebills::state::K_MAP`) and the translation table's rows.
pub struct TillNames<'a> {
    pub map: &'a dyn Fn(&str) -> Option<String>,
    pub i18n: &'a [(String, String)],
}

/// What [`attribute`] did: portions moved onto each dish, and every till
/// code still matched to no dish.
#[derive(Debug, Default)]
pub struct Till {
    pub attributed: BTreeMap<String, i64>,
    pub unmatched: Vec<Value>,
}

/// The last name the till gave each code, from the hot log.
fn names_of(orders: &[Value]) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for it in orders.iter().flat_map(|o| o.get("items").and_then(Value::as_array).into_iter().flatten()) {
        let Some(code) = it.get("product_id").and_then(Value::as_str).and_then(|p| p.strip_prefix("ebills:")) else { continue };
        if let Some(n) = it.get("name").and_then(Value::as_str) {
            out.insert(code.to_string(), n.to_string());
        }
    }
    out
}

fn add(a: &mut [i64], b: &[i64]) {
    a.iter_mut().zip(b).for_each(|(x, y)| *x += y);
}

fn merge(into: &mut DishRow, from: &DishRow) {
    into.sold += from.sold;
    into.revenue += from.revenue;
    into.stamped += from.stamped;
    into.stamped_cogs += from.stamped_cogs;
    add(&mut into.by_day, &from.by_day);
    add(&mut into.stamped_by_day, &from.stamped_by_day);
    add(&mut into.stamped_cogs_by_day, &from.stamped_cogs_by_day);
}

/// Move every attributable `ebills:<code>` row of `s` onto its dish.
pub fn attribute(s: &mut Sales, dishes: &HashMap<String, Dish>, orders: &[Value], products: &[(String, String)], till: &TillNames) -> Till {
    let idx = names::index(products, till.i18n);
    let named = names_of(orders);
    let codes: Vec<String> = s.dishes.keys().filter(|k| k.starts_with("ebills:")).cloned().collect();
    let mut out = Till::default();
    for key in codes {
        let code = &key["ebills:".len()..];
        let name = named.get(code).map_or("", String::as_str);
        let how = match (till.map)(code).filter(|p| dishes.contains_key(p)) {
            Some(p) => Match::One(p),
            None => names::how(&idx, name),
        };
        let pid = match how {
            Match::One(p) if dishes.contains_key(&p) => p,
            other => {
                let row = &s.dishes[&key];
                let why = if matches!(other, Match::Ambiguous(_)) { "two_dishes" } else { "no_dish" };
                out.unmatched.push(json!({ "code": code, "name": name, "portions": row.sold, "revenue": row.revenue, "why": why }));
                continue;
            }
        };
        let Some(row) = s.dishes.remove(&key) else { continue };
        *out.attributed.entry(pid.clone()).or_default() += row.sold;
        let n = row.by_day.len();
        let into = s.dishes.entry(pid).or_insert_with(|| DishRow { by_day: vec![0; n], stamped_by_day: vec![0; n], stamped_cogs_by_day: vec![0; n], ..DishRow::default() });
        merge(into, &row);
    }
    let num = |v: &Value, k: &str| v[k].as_i64().unwrap_or(0);
    out.unmatched.sort_by(|a, b| num(b, "revenue").cmp(&num(a, "revenue")).then(a["code"].as_str().cmp(&b["code"].as_str())));
    out
}

#[cfg(test)]
mod tests;
