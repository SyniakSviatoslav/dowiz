//! THE OWNER'S TILL TAB (W-EBILLS-TAB, operator 2026-11-03: "окрема вкладка
//! з детальною аналітикою та показом усіх замовлень за останні 30 днів").
//!
//! PURE: the venue's folded orders (`OrderView`) in, one JSON answer out --
//! every till order of the last `DAYS` days, newest first, and the numbers
//! over them. Nothing here is a second store: the rows ARE the log's orders
//! (`import.rs` placed them, channel `ebills`), read through the same
//! `status::venue_took` every other revenue figure uses, so this tab cannot
//! disagree with the dashboard or the analytics pane about one sale.
//!
//! WHY THE HOT LOG IS ENOUGH: rotation keeps every order newer than
//! `hubstore::HOT_KEEP_MS` (30 days) hot, and the window here is the same 30
//! days ending now -- so no order the window wants has left the log.
//!
//! EVERY FIELD IS ONE THE IMPORT WROTE (`map::to_order`): the till's sale id,
//! invoice number and FIC, the table, the operator's id (an id, never a
//! name: the wire carries none), the payment word, whether the course has
//! been paid by its bill, and each line's code, name and match. No field is
//! invented; a figure the data cannot support (a waiter's name, a tip) is
//! absent, not zero.

use crate::hubdo::OrderView;
use crate::services::orders::status;
use crate::services::ordering::{channel, fulfilment};
use dowiz_hub::tz::{local_ms, start_of_local_day_ms, Zone};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub(crate) const DAYS: i64 = 30;
const DAY_MS: i64 = 86_400_000;

#[derive(Default, Clone, Copy)]
struct Sum {
    orders: i64,
    revenue: i64,
}

impl Sum {
    fn add(&mut self, took: i64) {
        self.orders += 1;
        self.revenue += took;
    }
    fn json(self) -> Value {
        json!({ "orders": self.orders, "revenue": self.revenue })
    }
}

#[derive(Default)]
struct Lines {
    lines: i64,
    quantity: i64,
    revenue: i64,
}

impl Lines {
    fn add(&mut self, q: i64, money: i64) {
        self.lines += 1;
        self.quantity += q;
        self.revenue += money;
    }
    fn json(&self) -> Value {
        json!({ "lines": self.lines, "quantity": self.quantity, "revenue": self.revenue })
    }
}

/// The `DAYS` local midnights ending with today's, oldest first -- each
/// boundary asked of the zone (a local day is not always 24 h; `fold.rs`).
fn day_starts(zone: Zone, now: i64) -> Vec<i64> {
    let mut out = Vec::with_capacity(DAYS as usize);
    let mut at = start_of_local_day_ms(zone, now);
    for _ in 0..DAYS {
        out.push(at);
        at = start_of_local_day_ms(zone, at - 1);
    }
    out.reverse();
    out
}

fn s<'a>(v: &'a Value, path: &str) -> Option<&'a str> {
    v.pointer(path).and_then(Value::as_str)
}

fn n(v: &Value, path: &str) -> i64 {
    v.pointer(path).and_then(Value::as_i64).unwrap_or(0)
}

/// One line as the tab shows it. `code` is the till's `itemCode`: kept as
/// `ebills_code` once matched, else still the `ebills:<code>` product id.
fn line(it: &Value) -> Value {
    let pid = s(it, "/product_id").unwrap_or("");
    let (code, product_id) = match (s(it, "/ebills_code"), pid.strip_prefix("ebills:")) {
        (Some(c), _) => (c, Some(pid)),
        (None, Some(c)) => (c, None),
        (None, None) => ("", (!pid.is_empty()).then_some(pid)),
    };
    // `log.placed` v1 (before 2026-11-03) wrote no `matched_by`: a line matched then
    // (`ebills_code` kept) was matched by the owner's mapping, the only way there was.
    // The reader supplies that default; the log is never rewritten (migration-law, v2).
    let how = it.get("matched_by").cloned().or_else(|| s(it, "/ebills_code").map(|_| json!("map")));
    json!({
        "code": code, "name": s(it, "/name").unwrap_or(""), "product_id": product_id, "matched_by": how,
        "quantity": n(it, "/quantity"), "unit_price": n(it, "/unit_price"), "discount_pct": n(it, "/discount_pct"), "cost": it.get("cost"),
    })
}

/// One order as the tab's row.
fn row(o: &Value) -> Value {
    json!({
        "id": s(o, "/id"), "sale_id": o.pointer("/external/sale_id"), "inv_ord_num": o.pointer("/external/inv_ord_num"),
        "fic": o.pointer("/external/fic"), "at_ms": n(o, "/created_at_ms"), "status": s(o, "/status"),
        "payment": s(o, "/payment"), "payment_status": o.get("payment_status"), "kind": fulfilment::of(o),
        "table": o.pointer("/fulfilment/table"), "operator_id": o.pointer("/external/operator_id"),
        "total": n(o, "/total"), "void_of": o.get("void_of"), "changed": o.get("ebills_changed").and_then(Value::as_bool).unwrap_or(false),
        "items": o.get("items").and_then(Value::as_array).map(|a| a.iter().map(line).collect::<Vec<_>>()).unwrap_or_default(),
    })
}

/// The whole tab: the rows and every number over them.
pub(crate) fn month(listed: &[OrderView], zone: Zone, now: i64) -> Value {
    let starts = day_starts(zone, now);
    let from = starts[0];
    let mut orders: Vec<Value> = listed
        .iter()
        .filter_map(|v| serde_json::from_str::<Value>(&v.order_json).ok())
        .filter(|o| channel::of(o) == Ok(channel::EBILLS))
        .filter(|o| (from..=now).contains(&n(o, "/created_at_ms")))
        .collect();
    orders.sort_by(|a, b| n(b, "/created_at_ms").cmp(&n(a, "/created_at_ms")).then_with(|| s(a, "/id").cmp(&s(b, "/id"))));

    let mut by_day: Vec<Sum> = vec![Sum::default(); starts.len()];
    let mut by_hour = [Sum::default(); 24];
    let mut by_weekday = [Sum::default(); 7];
    let mut by_payment: BTreeMap<String, Sum> = BTreeMap::new();
    let mut by_kind: BTreeMap<String, i64> = BTreeMap::new();
    let mut tables: BTreeMap<String, Sum> = BTreeMap::new();
    let mut operators: BTreeMap<String, Sum> = BTreeMap::new();
    let mut items: BTreeMap<(String, String), (Option<String>, Option<String>, i64, i64)> = BTreeMap::new();
    let mut unmatched: BTreeMap<(String, String), (i64, i64)> = BTreeMap::new();
    let (mut total, mut not_taken, mut unpaid) = (Sum::default(), 0i64, 0i64);
    let (mut by_map, mut by_name, mut open, mut sold) = (Lines::default(), Lines::default(), Lines::default(), 0i64);

    for o in &orders {
        let at = n(o, "/created_at_ms");
        let st = s(o, "/status").unwrap_or("");
        let took = status::venue_took(n(o, "/total"), n(o, "/tip"), st);
        let counted = status::in_takings(st);
        if !status::took_money(st) {
            not_taken += 1;
        }
        let kind = fulfilment::of(o);
        if kind == "dine_in" && s(o, "/payment_status") != Some("paid") && counted {
            unpaid += 1;
        }
        total.add(took);
        if let Some(i) = starts.iter().rposition(|d| *d <= at) {
            by_day[i].add(took);
        }
        let local = local_ms(zone, at);
        by_hour[(local.rem_euclid(DAY_MS) / 3_600_000).clamp(0, 23) as usize].add(took);
        by_weekday[(local.div_euclid(DAY_MS) + 3).rem_euclid(7) as usize].add(took);
        by_payment.entry(s(o, "/payment").unwrap_or("unknown").to_string()).or_default().add(took);
        *by_kind.entry(kind.to_string()).or_default() += 1;
        if let Some(t) = s(o, "/fulfilment/table") {
            tables.entry(t.to_string()).or_default().add(took);
        }
        if let Some(op) = o.pointer("/external/operator_id").and_then(Value::as_i64) {
            operators.entry(op.to_string()).or_default().add(took);
        }
        if !counted {
            continue;
        }
        for it in o.get("items").and_then(Value::as_array).into_iter().flatten() {
            let l = line(it);
            let (q, money) = (n(&l, "/quantity"), n(&l, "/quantity") * n(&l, "/unit_price"));
            let (code, name) = (s(&l, "/code").unwrap_or("").to_string(), s(&l, "/name").unwrap_or("").to_string());
            sold += money;
            let e = items.entry((name.clone(), code.clone())).or_insert((None, None, 0, 0));
            e.0 = e.0.take().or_else(|| s(&l, "/product_id").map(str::to_string));
            e.1 = e.1.take().or_else(|| s(&l, "/matched_by").map(str::to_string));
            e.2 += q;
            e.3 += money;
            match (s(&l, "/product_id"), s(&l, "/matched_by")) {
                (Some(_), Some("name")) => by_name.add(q, money),
                (Some(_), _) => by_map.add(q, money),
                (None, _) => {
                    open.add(q, money);
                    let u = unmatched.entry((name, code)).or_default();
                    u.0 += q;
                    u.1 += money;
                }
            }
        }
    }
    let counted = total.orders - not_taken;
    let mut top: Vec<Value> = items
        .into_iter()
        .map(|((name, code), (pid, how, q, money))| json!({ "name": name, "code": code, "product_id": pid, "matched_by": how, "quantity": q, "revenue": money }))
        .collect();
    top.sort_by(|a, b| n(b, "/revenue").cmp(&n(a, "/revenue")).then_with(|| n(b, "/quantity").cmp(&n(a, "/quantity"))).then_with(|| s(a, "/name").cmp(&s(b, "/name"))));
    let mut unmatched: Vec<Value> = unmatched.into_iter().map(|((name, code), (q, money))| json!({ "name": name, "code": code, "quantity": q, "revenue": money })).collect();
    unmatched.sort_by(|a, b| n(b, "/revenue").cmp(&n(a, "/revenue")).then_with(|| s(a, "/name").cmp(&s(b, "/name"))));
    let ranked = |m: BTreeMap<String, Sum>, key: &str| -> Vec<Value> {
        let mut v: Vec<(String, Sum)> = m.into_iter().collect();
        v.sort_by(|a, b| b.1.orders.cmp(&a.1.orders).then_with(|| b.1.revenue.cmp(&a.1.revenue)).then_with(|| a.0.cmp(&b.0)));
        v.into_iter().map(|(k, sum)| { let mut j = sum.json(); j[key] = json!(k); j }).collect()
    };
    json!({
        "days": DAYS, "from_ms": from, "to_ms": now,
        "count": total.orders, "counted": counted, "revenue": total.revenue,
        "average_check": if counted > 0 { total.revenue / counted } else { 0 },
        "not_taken": not_taken, "unpaid_courses": unpaid,
        "by_day": starts.iter().zip(by_day).map(|(at, sum)| { let mut j = sum.json(); j["at_ms"] = json!(at); j["day"] = json!(super::time::day_of(local_ms(zone, *at))); j }).collect::<Vec<_>>(),
        "by_hour": by_hour.iter().map(|x| x.json()).collect::<Vec<_>>(),
        "by_weekday": by_weekday.iter().map(|x| x.json()).collect::<Vec<_>>(),
        "by_payment": by_payment.into_iter().map(|(k, v)| (k, v.json())).collect::<BTreeMap<_, _>>(),
        "by_kind": by_kind,
        "tables": ranked(tables, "table"), "operators": ranked(operators, "operator_id"),
        "items": top,
        "sold": sold,
        "matched": { "map": by_map.json(), "name": by_name.json() },
        "unmatched": { "lines": open.lines, "quantity": open.quantity, "revenue": open.revenue,
                       "share_ppm": if sold > 0 { open.revenue * 1_000_000 / sold } else { 0 }, "names": unmatched },
        "orders": orders.iter().map(row).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests;
