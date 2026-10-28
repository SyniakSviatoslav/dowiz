//! The sales rows against the cube's own fold: same dishes, same numbers, every day, DST included.

use super::super::{fold_orders, DAY_MS};
use super::*;
use serde_json::json;
use std::collections::BTreeMap;

fn tirane() -> Zone {
    dowiz_hub::tz::zone("Europe/Tirane").expect("the venue's zone")
}
/// 2026-09-22 12:00 UTC.
const NOON: i64 = 1_790_078_400_000;
const STATUSES: [&str; 6] = ["DELIVERED", "REJECTED", "PENDING", "ACCEPTED", "CANCELLED", "PICKED_UP"];

/// 10^4 orders over 60 days from 2026-09-22 (Europe's clocks go back on 2026-10-25), at every
/// hour, every status, lines with and without a stamped cost, one line with no product.
fn orders(n: i64) -> Vec<(String, Value)> {
    let mut s: u64 = 2610280001;
    let mut next = |m: u64| {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (s >> 33) % m
    };
    (0..n)
        .map(|i| {
            let at = NOON - 12 * 3_600_000 + (next(60) as i64) * DAY_MS + (next(86_400) as i64) * 1000;
            let lines: Vec<Value> = (0..1 + next(4))
                .map(|_| {
                    let id = if next(40) == 0 { String::new() } else { format!("dish-{:02}", next(15)) };
                    let (q, price) = (next(4) as i64, 100 * (1 + next(30) as i64));
                    if next(3) == 0 { json!({ "product_id": id, "quantity": q, "unit_price": price }) } else { json!({ "product_id": id, "quantity": q, "unit_price": price, "unit_cost": price / 3 }) }
                })
                .collect();
            let st = STATUSES[next(STATUSES.len() as u64) as usize];
            (format!("ord-{i}"), json!({ "created_at_ms": at, "status": st, "total": 1000, "tip": 0, "items": lines }))
        })
        .collect()
}

#[test]
fn rows_equal_fold_over_ten_thousand_orders() {
    let z = tirane();
    let os = orders(10_000);
    let now = NOON + 61 * DAY_MS;
    let values: Vec<Value> = os.iter().map(|(_, o)| o.clone()).collect();
    let folded = fold_orders(&values, z, now);
    let rows = sale_rows(&os, z, now);
    let names: BTreeMap<i64, String> = rows.iter().map(|(s, id)| (s.dish, id.clone())).collect();
    let mut by_day: BTreeMap<i64, Vec<Sale>> = BTreeMap::new();
    for (s, _) in &rows {
        by_day.entry(i64::from(s.day)).or_default().push(*s);
    }
    assert!(by_day.contains_key(&20261025) && by_day.contains_key(&20261026), "the DST change is inside the span");
    let mut days = 0;
    for (d, r) in &folded {
        let want = &r.m;
        let got = by_day.get(d).map(|rs| dishes_of(rs, &names).unwrap()).unwrap_or_default();
        assert_eq!(&got, want, "day {d}");
        let agg = by_day.get(d).map(|rs| dishes_of(&dowiz_hub::block::kernels::aggregate(rs).unwrap(), &names).unwrap()).unwrap_or_default();
        assert_eq!(&agg, want, "day {d}, one row per dish");
        days += usize::from(!want.is_empty());
    }
    assert!(days >= 55, "{days} days with dishes");
    assert_eq!(by_day.keys().filter(|d| !folded.contains_key(*d)).count(), 0, "no day the fold does not have");
}

#[test]
fn a_refused_or_waiting_order_has_no_row_and_an_unstamped_line_no_cost() {
    let z = tirane();
    let line = |c: Option<i64>| match c {
        Some(c) => json!([{ "product_id": "sake", "quantity": 2, "unit_price": 450, "unit_cost": c }]),
        None => json!([{ "product_id": "sake", "quantity": 2, "unit_price": 450 }]),
    };
    let os = vec![
        ("a".to_string(), json!({ "created_at_ms": NOON, "status": "DELIVERED", "items": line(Some(120)) })),
        ("b".to_string(), json!({ "created_at_ms": NOON, "status": "REJECTED", "items": line(Some(120)) })),
        ("c".to_string(), json!({ "created_at_ms": NOON, "status": "PENDING", "items": line(None) })),
        ("d".to_string(), json!({ "created_at_ms": NOON, "status": "DELIVERED", "items": line(None) })),
    ];
    let rows = sale_rows(&os, z, NOON);
    assert_eq!(rows.len(), 2, "only a and d were taken");
    let r0 = rows[0].0;
    assert_eq!((r0.day, r0.qty, r0.gross, r0.cqty, r0.cgross, r0.cost), (20260922, 2, 900, 2, 900, 240));
    let r1 = rows[1].0;
    assert_eq!((r1.qty, r1.gross, r1.cqty, r1.cgross, r1.cost), (2, 900, 0, 0, 0), "an unstamped line is sold and not costed");
    let day = dowiz_hub::block::kernels::aggregate(&[r0, r1]).unwrap();
    assert_eq!(day.len(), 1, "one row per dish per day");
    assert_eq!((day[0].qty, day[0].gross, day[0].cqty, day[0].cgross, day[0].cost), (4, 1800, 2, 900, 240), "food cost stays exact: 240 over 900, not over 1800");
    let names = BTreeMap::from([(rows[0].0.dish, "sake".to_string())]);
    let rs: Vec<Sale> = rows.iter().map(|(s, _)| *s).collect();
    assert_eq!(dishes_of(&rs, &names).unwrap()["sake"], [4, 1800, 2, 240]);
    assert!(dishes_of(&rs, &BTreeMap::new()).is_err(), "a dish without a name is an error, not a gap");
}
