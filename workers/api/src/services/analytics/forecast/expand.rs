//! PURE. A day's dishes, through the recipes, into what the kitchen makes
//! and takes: the semi-finished products (ПФ) a dish's recipe names, and the
//! raw items under every card, with `dowiz_hub::prep::expand` -- the sale's
//! own expansion, exact in millionths and rounded ONCE per item.
//!
//! THE PREP LIST: per ПФ, the forecast's use, what the shelf holds READY
//! (`stock::basket::ready_micro`, read; nothing is written) and the
//! difference to make, with the raw items that batch takes.
//!
//! P7, the expiry check: the forecast's raw use per item per day ahead.

use serde_json::{json, Value};
use std::collections::BTreeMap;

use dowiz_hub::catalog::Catalog;
use dowiz_hub::forecast::History;
use dowiz_hub::prep::{self, MICRO};
use dowiz_hub::stock::meta::day_number;

/// What a day's dishes take.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Needs {
    /// ПФ named by a dish's recipe -> units of it the dishes use.
    pub preps: BTreeMap<String, i64>,
    /// Raw item -> whole units the dishes take, through every card.
    pub raw: BTreeMap<String, i64>,
    /// Dishes with no recipe: the list cannot say what they take.
    pub unmodelled: Vec<String>,
    /// Dishes whose recipe does not expand, and why.
    pub refused: Vec<(String, String)>,
}

/// Millionths to whole units, half up.
pub fn round_micro(v: i128) -> i64 {
    let m = i128::from(MICRO);
    i64::try_from((v + m / 2).div_euclid(m)).unwrap_or(i64::MAX)
}

/// A dish's recipe lines `(supply, qty)`; empty without one.
fn bom(cat: &Catalog, dish: &str) -> Vec<(String, i64)> {
    cat.product(dish).map(|j| dowiz_hub::stock::bom_of(&j).into_iter().map(|l| (l.supply, l.qty)).collect()).unwrap_or_default()
}

pub fn needs(portions: &BTreeMap<String, i64>, cat: &Catalog) -> Needs {
    let supply = |s: &str| cat.supply(s);
    let mut n = Needs::default();
    let mut raw: BTreeMap<String, i128> = BTreeMap::new();
    for (dish, q) in portions.iter().filter(|(_, q)| **q > 0) {
        let lines = bom(cat, dish);
        if lines.is_empty() {
            n.unmodelled.push(dish.clone());
            continue;
        }
        for (s, qty) in &lines {
            if supply(s).is_some_and(|j| prep::is_prep(&j)) {
                *n.preps.entry(s.clone()).or_default() += qty * q;
            }
        }
        match prep::expand(&lines, &supply) {
            Ok(leaves) => {
                for l in leaves {
                    *raw.entry(l.item).or_default() += i128::from(l.uq) * i128::from(*q);
                }
            }
            Err(why) => n.refused.push((dish.clone(), why.to_string())),
        }
    }
    n.raw = raw.into_iter().map(|(k, v)| (k, round_micro(v))).collect();
    n
}

/// The raw items it takes to make `qty` of the ПФ `id`, through its card.
pub fn components(id: &str, qty: i64, cat: &Catalog) -> Result<Vec<(String, i64)>, String> {
    let supply = |s: &str| cat.supply(s);
    prep::expand(&[(id.to_string(), qty)], &supply)
        .map(|v| v.into_iter().map(|l| (l.item, round_micro(i128::from(l.uq)))).filter(|(_, q)| *q > 0).collect())
        .map_err(|e| e.to_string())
}

/// `{id, name, unit, qty}` of a supply.
pub fn item_json(cat: &Catalog, id: &str, qty: i64) -> Value {
    let v = cat.supply(id).and_then(|j| serde_json::from_str::<Value>(&j).ok()).unwrap_or(Value::Null);
    json!({ "id": id, "name": v.get("name").cloned().unwrap_or(json!(id)), "unit": v.get("unit").and_then(Value::as_str).unwrap_or("g"), "qty": qty })
}

/// Whole units of a ПФ ready on the shelf now. The carried fraction of a
/// unit (`stock::carry`, under half a unit) is not read: the list is whole.
pub fn ready(led: &dowiz_hub::stock::StockLedger, id: &str) -> i64 {
    dowiz_hub::stock::basket::ready_micro(led, &dowiz_hub::stock::carry::Carry::default(), id) / MICRO
}

/// The prep list: `{id, name, unit, need, onHand, make, from[] | error}`.
pub fn prep_rows(n: &Needs, cat: &Catalog, led: &dowiz_hub::stock::StockLedger) -> Vec<Value> {
    n.preps
        .iter()
        .map(|(id, need)| {
            let on_hand = ready(led, id);
            let make = (need - on_hand).max(0);
            let mut v = item_json(cat, id, *need);
            if let Some(m) = v.as_object_mut() {
                m.remove("qty");
            }
            v["need"] = json!(need);
            v["onHand"] = json!(on_hand);
            v["make"] = json!(make);
            v["from"] = json!([]);
            if make > 0 {
                match components(id, make, cat) {
                    Ok(from) => v["from"] = json!(from.iter().map(|(i, q)| item_json(cat, i, *q)).collect::<Vec<_>>()),
                    Err(why) => v["error"] = json!(why),
                }
            }
            v
        })
        .collect()
}

/// P7: the forecast's raw use per item for each of the `days + 1` days from
/// `today`. Every item a recipe reaches is in it (at zero if nothing is
/// forecast); an item no recipe reaches is absent, and nothing is claimed
/// about it. `None` while the venue's total is still learning.
pub fn raw_by_day(h: &History, today: i64, cat: &Catalog, days: i64) -> Option<BTreeMap<String, Vec<i64>>> {
    let width = usize::try_from(days + 1).ok()?;
    let mut out: BTreeMap<String, Vec<i64>> = BTreeMap::new();
    let one: BTreeMap<String, i64> = cat.products().into_iter().map(|(id, _)| (id, 1)).collect();
    for item in needs(&one, cat).raw.keys() {
        out.insert(item.clone(), vec![0; width]);
    }
    for k in 0..width {
        let c = super::plan::cast(h, today, today + k as i64, false);
        c.portions.value()?;
        for (item, q) in needs(&super::plan::portions_of(&c), cat).raw {
            out.entry(item).or_insert_with(|| vec![0; width])[k] += q;
        }
    }
    Some(out)
}

/// The forecast's use of `item` from today through its best-before day
/// (`yyyymmdd`, inclusive). `None`: the table says nothing about the item.
pub fn use_until(table: &BTreeMap<String, Vec<i64>>, today: i64, item: &str, expiry: i64) -> Option<i64> {
    let row = table.get(item)?;
    let last = day_number(expiry) - today;
    Some(if last < 0 { 0 } else { row.iter().take(usize::try_from(last).ok()? + 1).sum() })
}
