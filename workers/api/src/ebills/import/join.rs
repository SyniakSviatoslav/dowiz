//! THE BILL-TO-COURSES JOIN, apart from the import it serves: the one rule
//! in the till link that is a heuristic, measured against a real day's
//! tables (2026-09-23: 18 of 22 bills joined; the 4 waiting were bills
//! whose courses were rung up before the window read).

use serde_json::Value;

/// THE JOIN (§1.6, §5.2): no GET names the courses a bill covers, so they are
/// found -- unbilled ebills courses at the bill's table, rung up before it.
/// All of them when they sum to the bill (the ordinary sitting, and a void
/// with its original: +4600 -4600 = a bill of 0, measured); else the newest
/// run that does (an older course is likelier an orphan than part of this
/// bill); else the oldest run. Nothing that does not add up is paid.
///
/// NOT BY `sale_unit_order_id`: on 2026-09-23 every detail inspected carried
/// its own id, so nothing shows it groups a sitting (BLUEPRINT §8.2 (2)).
pub(super) fn join<'a>(cands: impl Iterator<Item = (&'a String, &'a Value)>, paid: &Value) -> Option<Vec<String>> {
    let table = paid.get("table").and_then(Value::as_str)?;
    let (at, total) = (paid.get("at_ms").and_then(Value::as_i64)?, paid.get("total").and_then(Value::as_i64)?);
    let mut c: Vec<(i64, String, i64)> = cands
        .filter(|(_, o)| {
            o.get("channel").and_then(Value::as_str) == Some("ebills")
                && o.pointer("/fulfilment/kind").and_then(Value::as_str) == Some("dine_in")
                && o.pointer("/fulfilment/table").and_then(Value::as_str) == Some(table)
                && o.get("bill").is_none()
                && o.get("created_at_ms").and_then(Value::as_i64).is_some_and(|t| t <= at)
        })
        .map(|(id, o)| (o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0), id.clone(), o.get("total").and_then(Value::as_i64).unwrap_or(0)))
        .collect();
    c.sort();
    let sum = |v: &[(i64, String, i64)]| v.iter().map(|x| x.2).sum::<i64>();
    let ids = |v: &[(i64, String, i64)]| v.iter().map(|x| x.1.clone()).collect::<Vec<_>>();
    if c.is_empty() {
        return None;
    }
    (0..c.len())
        .map(|k| &c[k..])
        .chain((1..c.len()).map(|n| &c[..n]))
        .find(|run| sum(run) == total)
        .map(ids)
}
