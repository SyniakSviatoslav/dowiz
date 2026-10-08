//! B3: `command/floor.rs:sittings` (every staff floor poll) -- distinct sitting ids by
//! `ids.iter().any` then `sitting::rounds(listed, s)` per sitting, which parses EVERY order again:
//! O(S × N) serde parses. REPLACEMENT: one pass, parse each order once, group by sitting_id.
//! B4: `owner.rs:660-674` dashboard -- parses every order BEFORE the `seq < day_start` test.
//! REPLACEMENT: test seq first (the list is newest-first, so stop at the first older row).
//! Equivalence: identical grouped (sitting, [order ids oldest-first]) lists / identical tiles.
use serde_json::{json, Value};

pub struct OrderView {
    pub order_id: String,
    pub seq: u64,
    pub order_json: String,
}

/// N orders, newest first, S sittings; ~1.5 KB each like a kernel envelope.
fn fixture(n: usize, sittings: usize, day_start: u64) -> Vec<OrderView> {
    (0..n)
        .rev()
        .map(|i| {
            let s = i % sittings;
            let status = ["PENDING", "CONFIRMED", "PREPARING", "DELIVERED", "PAID", "CANCELLED"][i % 6];
            let seq = day_start.wrapping_sub(5_000_000).wrapping_add(i as u64 * 20_000);
            let v = json!({
                "id": format!("o_{i}"), "location_id": "loc_sushi", "status": status, "created_at_ms": seq,
                "sitting_id": format!("s_{s}"), "placed_by": if i % 7 == 0 { "guest" } else { "staff" },
                "total": 1500 + (i as i64 % 13) * 250, "currency": "ALL", "channel": "room",
                "fulfilment": { "kind": "table", "table": format!("main/{}", s % 20), "address": { "line": "" } },
                "contact": { "name": format!("Guest {i}"), "phone": "+355690000000" },
                "items": (0..4).map(|k| json!({ "product_id": format!("p_{}", (i * 7 + k) % 165), "name": format!("Roll {} me salmon dhe avokado", (i * 7 + k) % 165), "quantity": 1 + k as i64 % 2, "unit_price": 700 + 10 * k as i64, "modifiers": [{"id": "o_8", "name": "8 copë", "price": 0}], "allergens": ["fish", "sesame"] })).collect::<Vec<_>>(),
                "payments": if i % 3 == 0 { json!([{ "method": "cash", "amount": 1500, "at": seq + 600_000 }]) } else { json!([]) },
                "history": (0..3).map(|k| json!({ "status": status, "at": seq + k * 60_000, "by": "staff-1" })).collect::<Vec<_>>(),
                "notes": "pa qepë, ju lutem", "promo": null, "tip": 0, "tax": { "ppm": 200000, "amount": 250 },
            });
            OrderView { order_id: format!("o_{i}"), seq, order_json: v.to_string() }
        })
        .collect()
}

// ── B3 ──
fn rounds_current<'a>(listed: &'a [OrderView], sitting_id: &str) -> Vec<(&'a str, Value)> {
    let mut out: Vec<(&str, Value)> = listed
        .iter()
        .filter_map(|v| {
            let order: Value = serde_json::from_str(&v.order_json).ok()?;
            (order.get("sitting_id").and_then(Value::as_str) == Some(sitting_id)).then_some((v.order_id.as_str(), order))
        })
        .collect();
    out.sort_by_key(|r| r.1.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0));
    out
}

fn sittings_current(listed: &[OrderView]) -> Vec<(String, Vec<String>)> {
    let mut ids: Vec<String> = Vec::new();
    for v in listed {
        let Ok(o) = serde_json::from_str::<Value>(&v.order_json) else { continue };
        if let Some(s) = o.get("sitting_id").and_then(Value::as_str) {
            if !ids.iter().any(|x| x == s) {
                ids.push(s.to_string());
            }
        }
    }
    ids.iter().map(|s| (s.clone(), rounds_current(listed, s).into_iter().map(|(id, _)| id.to_string()).collect())).collect()
}

fn sittings_replacement(listed: &[OrderView]) -> Vec<(String, Vec<String>)> {
    // One parse per order; sittings in first-seen order; rounds sorted by created_at_ms (stable).
    let mut order: Vec<String> = Vec::new();
    let mut groups: std::collections::HashMap<String, Vec<(i64, &str)>> = std::collections::HashMap::new();
    for v in listed {
        let Ok(o) = serde_json::from_str::<Value>(&v.order_json) else { continue };
        let Some(s) = o.get("sitting_id").and_then(Value::as_str) else { continue };
        let at = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        match groups.get_mut(s) {
            Some(g) => g.push((at, v.order_id.as_str())),
            None => {
                order.push(s.to_string());
                groups.insert(s.to_string(), vec![(at, v.order_id.as_str())]);
            }
        }
    }
    order
        .into_iter()
        .map(|s| {
            let mut g = groups.remove(&s).unwrap();
            g.sort_by_key(|(at, _)| *at);
            (s, g.into_iter().map(|(_, id)| id.to_string()).collect())
        })
        .collect()
}

// ── B4 ──
fn is_active(st: &str) -> bool {
    matches!(st, "CONFIRMED" | "PREPARING" | "READY" | "IN_DELIVERY")
}
fn belongs_to(v: &Value, loc: &str) -> bool {
    v.get("location_id").and_then(Value::as_str).is_none_or(|l| l == loc)
}
type Tiles = (i64, i64, i64, i64);

fn dashboard_current(listed: &[OrderView], loc: &str, day_start: i64) -> Tiles {
    let (mut count, mut revenue, mut pending, mut active) = (0i64, 0i64, 0i64, 0i64);
    for e in listed {
        let Ok(v) = serde_json::from_str::<Value>(&e.order_json) else { continue };
        if !belongs_to(&v, loc) {
            continue;
        }
        if (e.seq as i64) < day_start {
            continue;
        }
        count += 1;
        let status = v.get("status").and_then(|x| x.as_str()).unwrap_or("");
        if status == "PENDING" {
            pending += 1;
        } else if is_active(status) {
            active += 1;
        }
        revenue += v.get("total").and_then(Value::as_i64).unwrap_or(0);
    }
    (count, revenue, pending, active)
}

fn dashboard_replacement(listed: &[OrderView], loc: &str, day_start: i64) -> Tiles {
    let (mut count, mut revenue, mut pending, mut active) = (0i64, 0i64, 0i64, 0i64);
    for e in listed {
        if (e.seq as i64) < day_start {
            break; // newest first: everything after is older still
        }
        let Ok(v) = serde_json::from_str::<Value>(&e.order_json) else { continue };
        if !belongs_to(&v, loc) {
            continue;
        }
        count += 1;
        let status = v.get("status").and_then(|x| x.as_str()).unwrap_or("");
        if status == "PENDING" {
            pending += 1;
        } else if is_active(status) {
            active += 1;
        }
        revenue += v.get("total").and_then(Value::as_i64).unwrap_or(0);
    }
    (count, revenue, pending, active)
}

fn us(n: u128) -> f64 {
    n as f64 / 1e3
}

pub fn run(reps: usize) {
    let day_start: u64 = 1_782_900_000_000;
    for &(n, s) in &[(100usize, 20usize), (300, 60), (1000, 150)] {
        let listed = fixture(n, s, day_start);
        let bytes: usize = listed.iter().map(|o| o.order_json.len()).sum();
        let today = listed.iter().filter(|o| o.seq >= day_start).count();
        println!("B3/B4 fixture: n={n} orders ({} KB JSON), {s} sittings, {today} of them today", bytes / 1024);
        let a = sittings_current(&listed);
        let b = sittings_replacement(&listed);
        assert_eq!(a, b, "B3 grouping must be identical");
        println!("B3 EQUIV ok (n={n}): {} sittings, {} rounds", a.len(), a.iter().map(|(_, r)| r.len()).sum::<usize>());
        let (_, cur, _) = super::median_ns(reps, || super::time_ns(|| { std::hint::black_box(sittings_current(&listed)); }));
        let (_, rep, _) = super::median_ns(reps, || super::time_ns(|| { std::hint::black_box(sittings_replacement(&listed)); }));
        println!("B3 n={n} S={s}: CURRENT {:.0} us | REPLACE {:.0} us | {:.1}x | share of 10 ms cap {:.0}% -> {:.0}%", us(cur), us(rep), us(cur) / us(rep).max(0.001), us(cur) / 100.0, us(rep) / 100.0);

        let ds = day_start as i64;
        let t1 = dashboard_current(&listed, "loc_sushi", ds);
        let t2 = dashboard_replacement(&listed, "loc_sushi", ds);
        assert_eq!(t1, t2, "B4 tiles must be identical");
        println!("B4 EQUIV ok (n={n}): tiles {:?}", t1);
        let (_, cur, _) = super::median_ns(reps, || super::time_ns(|| { std::hint::black_box(dashboard_current(&listed, "loc_sushi", ds)); }));
        let (_, rep, _) = super::median_ns(reps, || super::time_ns(|| { std::hint::black_box(dashboard_replacement(&listed, "loc_sushi", ds)); }));
        println!("B4 n={n} today={today}: CURRENT {:.0} us | REPLACE {:.0} us | {:.1}x | share of 10 ms cap {:.1}% -> {:.1}%", us(cur), us(rep), us(cur) / us(rep).max(0.001), us(cur) / 100.0, us(rep) / 100.0);
    }
}
