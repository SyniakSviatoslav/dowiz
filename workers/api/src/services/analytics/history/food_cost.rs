//! PURE. FOOD COST BY DISH, OUT OF THE SALES BLOCKS (R-BEBOP-LEVERS §5.4, lane LV-COL-E; the operator's
//! 2026-10-12 ask: food cost feeds analytics). One plan over the span's day blocks and the hot tail's
//! rows (folded `full`, never stepped: the Z-set step waits for LV-INC-A):
//!   FILTER cqty > 0, GROUP dish, AGG cost | AGG cgross | AGG cqty
//! EXACT, unlike the cube row: the row keeps stamped portions and their cost but not their sales, so
//! a percentage from it divides by sales that include unstamped lines. The block's day row keeps
//! the STAMPED lines' sales (`cgross`) beside their cost, and the percentage is cost over the SAME
//! lines' sales. Unstamped sales (`gross - cgross`) are counted apart, never as zero cost.

use super::pm;
use dowiz_hub::block::kernels::{run_plan, Op, PlanRow, AGG, FILTER, GROUP};
use dowiz_hub::block::Block;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Column numbers of `schema::SALES`.
const DISH: i64 = 1;
const GROSS: i64 = 3;
const CQTY: i64 = 4;
const CGROSS: i64 = 5;
const COST: i64 = 6;

fn plan(filter: Option<(i64, Op)>, val: i64) -> Vec<PlanRow> {
    let mut p: Vec<PlanRow> = filter.map(|(c, op)| [FILTER, c, op as i64, 0, 0, 0, 0, 0]).into_iter().collect();
    p.push([GROUP, DISH, 0, 0, 0, 0, 0, 0]);
    p.push([AGG, val, 0, 0, 0, 0, 0, 0]);
    p
}

fn sums(blocks: &[Block], filter: Option<(i64, Op)>, val: i64) -> Result<BTreeMap<i64, i64>, String> {
    Ok(run_plan(blocks, &plan(filter, val)).map_err(|r| format!("food cost: {r:?}"))?.into_iter().collect())
}

/// The span's food cost: per dish (largest cost first, then id) and in total.
pub fn food_cost(blocks: &[Block], names: &BTreeMap<i64, String>, name: &dyn Fn(&str) -> Value) -> Result<Value, String> {
    let costed = Some((CQTY, Op::Gt));
    let (cost, sales, qty) = (sums(blocks, costed, COST)?, sums(blocks, costed, CGROSS)?, sums(blocks, costed, CQTY)?);
    let unstamped: i64 = sums(blocks, None, GROSS)?.values().sum::<i64>() - sums(blocks, None, CGROSS)?.values().sum::<i64>();
    let mut rows: Vec<(String, i64, i64, i64)> = Vec::new();
    for (k, c) in &cost {
        let id = names.get(k).ok_or(format!("food cost: dish {k:016x} has no name"))?;
        rows.push((id.clone(), *c, sales.get(k).copied().unwrap_or(0), qty.get(k).copied().unwrap_or(0)));
    }
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let (tc, ts): (i64, i64) = (rows.iter().map(|r| r.1).sum(), rows.iter().map(|r| r.2).sum());
    Ok(json!({
        "cost": tc, "sales": ts, "costPm": pm(tc, ts), "unstampedSales": unstamped,
        "dishes": rows.iter().map(|(id, c, s, q)| json!({ "id": id, "name": name(id.as_str()), "quantity": q, "sales": s, "cost": c, "costPm": pm(*c, *s) })).collect::<Vec<_>>(),
    }))
}
