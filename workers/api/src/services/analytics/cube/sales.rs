//! PURE. THE SALES FACT ROWS (R-BEBOP-LEVERS §5.4, lane LV-COL-E): one `dowiz_hub::block::kernels::Sale`
//! per order line the venue took, by EXACTLY the rules `fold_orders` uses for a row's dishes (`m`):
//! an order refused or still waiting has no dish line, a line with no product is not a dish, gross
//! is unit price x quantity; a line with a stamped cost (`cost::line_cost`) also counts in `cqty`,
//! `cgross` and `cost`. No order id is kept. `rows_equal_fold` (sales_tests) holds the two equal,
//! so the block and the JSON row can only say the same thing.
//!
//! The lines go into the venue's `sales` image (`kernels::pack`): at a catch-up, summed into ONE ROW
//! PER DISH PER DAY (`kernels::aggregate`) beside the cube's JSON row; the trace refolds a day's
//! block from the archives and compares bytes.

use dowiz_hub::block::kernels::Sale;
use dowiz_hub::block::schema::k64;
use dowiz_hub::tz::Zone;
use serde_json::Value;

/// The image's name in the venue's object. Not `log@`: no history route can read it as orders.
pub const IMAGE: &str = "sales";

/// Every dish line of the accepted orders dated at or before `now`, with its product id. The
/// orders come as `(order id, order)` pairs, as the archives give them; the id is not kept.
pub fn sale_rows(orders: &[(String, Value)], zone: Zone, now: i64) -> Vec<(Sale, String)> {
    use crate::services::orders::status;
    let mut out = Vec::new();
    for (_, o) in orders {
        let at = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        let st = o.get("status").and_then(Value::as_str).unwrap_or("");
        if at > now || !status::took_money(st) || !status::in_takings(st) {
            continue;
        }
        let day = super::day_of(zone, at) as i32;
        for it in o.get("items").and_then(Value::as_array).into_iter().flatten() {
            let Some(id) = it.get("product_id").and_then(Value::as_str).filter(|s| !s.is_empty()) else { continue };
            let q = it.get("quantity").and_then(Value::as_i64).unwrap_or(0);
            let gross = it.get("unit_price").and_then(Value::as_i64).unwrap_or(0) * q;
            let (cqty, cgross, cost) = match crate::command::place::cost::line_cost(it, q) {
                Some(c) => (q as i32, gross, c),
                None => (0, 0, 0),
            };
            out.push((Sale { day, dish: k64(id.as_bytes()) as i64, qty: q as i32, gross, cqty, cgross, cost }, id.to_string()));
        }
    }
    out
}

/// A day's dishes as the cube's `m` map says them -- `[portions, food sales, stamped portions,
/// their stamped cost]` -- folded from rows (lines or day rows alike) by the `sum_by` kernel over
/// `qty`, `gross`, `cqty` and `cost`, the dish ids resolved through `names`. Overflow is an error.
pub fn dishes_of(rows: &[Sale], names: &std::collections::BTreeMap<i64, String>) -> Result<std::collections::BTreeMap<String, [i64; 4]>, String> {
    use dowiz_hub::block::kernels::sum_by;
    let dish: Vec<i64> = rows.iter().map(|s| s.dish).collect();
    let col = |f: fn(&Sale) -> i64| rows.iter().map(f).collect::<Vec<i64>>();
    let e = |r| format!("sales kernel: {r:?}");
    let sums = [
        sum_by(&dish, &col(|s| i64::from(s.qty)), None, "qty").map_err(e)?,
        sum_by(&dish, &col(|s| s.gross), None, "gross").map_err(e)?,
        sum_by(&dish, &col(|s| i64::from(s.cqty)), None, "cqty").map_err(e)?,
        sum_by(&dish, &col(|s| s.cost), None, "cost").map_err(e)?,
    ];
    let mut out = std::collections::BTreeMap::new();
    for k in sums[0].keys() {
        let id = names.get(k).ok_or(format!("sales: dish {k:016x} has no name"))?;
        out.insert(id.clone(), [0, 1, 2, 3].map(|i| sums[i].get(k).copied().unwrap_or(0)));
    }
    Ok(out)
}

#[cfg(test)]
#[path = "sales_tests.rs"]
mod tests;
