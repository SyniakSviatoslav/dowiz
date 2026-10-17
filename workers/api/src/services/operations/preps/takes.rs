//! WHAT ONE SALE OF A DISH TAKES OFF THE SHELF: its raw leaves, exact, and
//! the cost (`GET /api/owner/products/:id/takes`, answered by `super::answer`).

use serde_json::{json, Value};

use dowiz_hub::catalog::Catalog;
use dowiz_hub::prep;
use dowiz_hub::stock::cost::CostBook;

/// What one sale of `product_json` takes off the shelf, and what it costs
/// at list prices. PURE.
#[cfg(test)] // the routes read the priced twin; the tests keep the list-price read
pub fn takes(cat: &Catalog, product_json: &str) -> Value {
    takes_priced(cat, &CostBook::default(), product_json)
}

/// [`takes`], each leaf at the AVERAGE a priced delivery set, else its list
/// price (`recipe::prep::price_micro` over `averaged`, 2026-10-17); a leaf
/// priced by neither keeps the total null. PURE.
pub fn takes_priced(cat: &Catalog, book: &CostBook, product_json: &str) -> Value {
    let bare = |s: &str| cat.supply(s);
    let supply = crate::recipe::prep::averaged(&bare, book);
    let bom = dowiz_hub::stock::bom_of(product_json);
    // EXACT (W-QTYHUB H1): the lines' millionths, so half a sheet is half a sheet.
    let lines: Vec<(String, i64)> = bom.iter().map(|l| (l.supply.clone(), l.uq)).collect();
    let price = |s: &str| supply(s).and_then(|j| crate::recipe::prep::price_micro(&j));
    match prep::expand_micro(&lines, &supply) {
        Ok(leaves) => {
            let rows: Vec<Value> = leaves
                .iter()
                .map(|l| {
                    let sv: Value = supply(&l.item).and_then(|j| serde_json::from_str(&j).ok()).unwrap_or(json!({}));
                    let cost = price(&l.item).map(|p| (p * i128::from(l.uq) + i128::from(prep::MICRO) * i128::from(prep::MICRO) / 2) / (i128::from(prep::MICRO) * i128::from(prep::MICRO)));
                    // Three decimals of the base unit, rounded once.
                    let milli = (l.uq + 500) / 1000;
                    json!({ "supply": l.item, "name": sv.get("name").cloned().unwrap_or(json!(l.item)), "unit": sv.get("unit").cloned().unwrap_or(json!("g")),
                        "uq": l.uq, "qty": format!("{}.{:03}", milli / 1000, milli % 1000), "cost": cost.map(|c| json!(c as i64)).unwrap_or(Value::Null) })
                })
                .collect();
            let total = prep::cost_micro_uq(&lines, &supply, &price).ok().flatten().map(|m| (i128::from(m) + i128::from(prep::MICRO) / 2) / i128::from(prep::MICRO));
            json!({ "leaves": rows, "cost": total.map(|c| json!(c as i64)).unwrap_or(Value::Null), "lines": bom.len() })
        }
        Err(why) => json!({ "leaves": [], "cost": Value::Null, "lines": bom.len(), "refused": why.to_string() }),
    }
}
