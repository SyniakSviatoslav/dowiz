//! PURE. FOOD COST BY DISH, OUT OF THE SALES BLOCKS (R-BEBOP-LEVERS §5.4, lane LV-COL-E; the operator's
//! 2026-10-12 ask: food cost feeds analytics). One plan over the span's day blocks and the hot tail's
//! rows (folded `full`, never stepped: the Z-set step waits for LV-INC-A):
//!   FILTER cqty > 0, GROUP dish, AGG cost | AGG cgross | AGG cqty
//! EXACT, unlike the cube row: the row keeps stamped portions and their cost but not their sales, so
//! a percentage from it divides by sales that include unstamped lines. The block's day row keeps
//! the STAMPED lines' sales (`cgross`) beside their cost, and the percentage is cost over the SAME
//! lines' sales. Unstamped sales (`gross - cgross`) are counted apart, never as zero cost.

use super::pm;
use dowiz_hub::block::kernels::image::{run_plan_days, Day};
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

/// W-BLOCKVIEW: the stored days read IN PLACE (`kernels::image`), then the hot tail's owned blocks
/// added in, every add checked -- the same sums as one `run_plan` over both, without copying a day out.
fn sums(days: &[Day], hot: &[Block], filter: Option<(i64, Op)>, val: i64) -> Result<BTreeMap<i64, i64>, String> {
    let p = plan(filter, val);
    let err = |r| format!("food cost: {r:?}");
    let mut m: BTreeMap<i64, i64> = run_plan_days(days, &p).map_err(err)?.into_iter().collect();
    for (k, v) in run_plan(hot, &p).map_err(err)? {
        let e = m.entry(k).or_insert(0);
        *e = e.checked_add(v).ok_or_else(|| format!("food cost: {k:016x} overflows"))?;
    }
    Ok(m)
}

/// The span's food cost: per dish (largest cost first, then id) and in total, over the stored `days`
/// and the `hot` blocks together.
pub fn food_cost(days: &[Day], hot: &[Block], names: &BTreeMap<i64, String>, name: &dyn Fn(&str) -> Value) -> Result<Value, String> {
    let costed = Some((CQTY, Op::Gt));
    let s = |f, v| sums(days, hot, f, v);
    let (cost, sales, qty) = (s(costed, COST)?, s(costed, CGROSS)?, s(costed, CQTY)?);
    let unstamped: i64 = s(None, GROSS)?.values().sum::<i64>() - s(None, CGROSS)?.values().sum::<i64>();
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
