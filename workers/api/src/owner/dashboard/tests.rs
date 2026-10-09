//! W-LOOPA row 3: the tiles equal the OLD loop's (copied below as the oracle) on R-LOOPS B4's
//! fixture and its edge cases, and a `break` at the first older row would be WRONG on a
//! listing the projection really produces.

use super::*;
use crate::fold::projection::Orders;
use dowiz_hub::{Event, EventKind};
use serde_json::json;

/// The loop as it stood in `owner::dashboard` before W-LOOPA (parse, venue, THEN seq).
fn old_tiles(listed: &[OrderView], loc: &str, day_start: i64) -> Tiles {
    let (mut count, mut revenue, mut pending, mut active) = (0i64, 0i64, 0i64, 0i64);
    for e in listed {
        let Ok(v) = serde_json::from_str::<Value>(&e.order_json) else { continue };
        if !crate::services::orders::mine::belongs_to(&v, loc) {
            continue;
        }
        if (e.seq as i64) < day_start {
            continue;
        }
        count += 1;
        let status = v.get("status").and_then(|x| x.as_str()).unwrap_or("");
        if status == "PENDING" {
            pending += 1;
        } else if crate::services::orders::status::is_active(status) {
            active += 1;
        }
        revenue += crate::services::orders::status::venue_took(
            v.get("total").and_then(|t| t.as_i64()).unwrap_or(0),
            v.get("tip").and_then(|t| t.as_i64()).unwrap_or(0),
            status,
        );
    }
    Tiles { count, revenue, pending, active }
}

const DAY: i64 = 1_782_900_000_000;

/// R-LOOPS B4: n orders newest first, 20 s apart, ending `today` orders past `DAY`; with an
/// unparsable envelope, a missing and a foreign `location_id`, tips and every status.
fn fixture(n: usize, today: usize) -> Vec<OrderView> {
    const ST: [&str; 9] =
        ["PENDING", "CONFIRMED", "PREPARING", "READY", "IN_DELIVERY", "DELIVERED", "PICKED_UP", "REJECTED", "CANCELLED"];
    (0..n)
        .rev()
        .map(|i| {
            let seq = (DAY - (n - today) as i64 * 20_000 + i as i64 * 20_000) as u64;
            let mut v = json!({
                "id": format!("o_{i}"), "location_id": if i % 13 == 4 { "loc_other" } else { "loc_sushi" },
                "status": ST[i % ST.len()], "total": 1500 + (i as i64 % 13) * 250, "tip": (i as i64 % 4) * 50,
                "items": (0..4).map(|k| json!({"product_id": format!("p_{k}"), "name": "Roll me salmon", "quantity": 1,
                    "unit_price": 700, "modifiers": [], "allergens": ["fish"]})).collect::<Vec<_>>(),
                "history": (0..3).map(|k| json!({"status": "PENDING", "at": seq + k * 60_000})).collect::<Vec<_>>(),
            });
            if i % 17 == 3 {
                v.as_object_mut().unwrap().remove("location_id");
            }
            let text = if i % 19 == 7 { "{broken".to_string() } else { v.to_string() };
            OrderView { order_id: format!("o_{i}"), kind: 1, seq, order_json: text }
        })
        .collect()
}

#[test]
fn the_tiles_are_the_old_loops_tiles() {
    for (n, today) in [(0, 0), (1, 1), (100, 0), (300, 50), (300, 300), (1000, 750)] {
        let f = fixture(n, today);
        for loc in ["loc_sushi", "loc_other"] {
            for day in [DAY - 1, DAY, DAY + 1, i64::MIN, i64::MAX] {
                assert_eq!(tiles(&f, loc, day), old_tiles(&f, loc, day), "n={n} today={today} {loc} {day}");
            }
        }
    }
    // not vacuous: the 300/50 fixture has today's orders of every tile
    let t = tiles(&fixture(300, 50), "loc_sushi", DAY);
    assert!(t.count > 30 && t.pending > 0 && t.active > 0 && t.revenue > 0, "{t:?}");
}

/// WHY THERE IS NO `break`. The projection lists orders newest-first by LOG POSITION, and the
/// e-bill import appends with the till's time: an order placed today (log position 1) sits
/// BELOW a bill imported after it but stamped yesterday (position 2). Built through the real
/// `Orders::of`, the listing has an older row on top, and a loop that stopped there would
/// report no orders today.
#[test]
fn an_older_row_above_a_today_row_is_real_so_the_scan_does_not_stop() {
    let placed = |id: &str| json!({"id": id, "location_id": "loc_sushi", "status": "CONFIRMED", "total": 900}).to_string();
    let newest_first = vec![
        Event { kind: EventKind::Paid, order_id: "bill-1".into(), order_json: placed("bill-1"), seq: (DAY - 3_600_000) as u64 },
        Event { kind: EventKind::Placed, order_id: "o-today".into(), order_json: placed("o-today"), seq: (DAY + 60_000) as u64 },
    ];
    let listed = Orders::of(1, &newest_first).view();
    assert_eq!(listed.iter().map(|v| v.order_id.as_str()).collect::<Vec<_>>(), ["bill-1", "o-today"]);
    assert!((listed[0].seq as i64) < DAY && (listed[1].seq as i64) >= DAY, "older row on top");
    let with_break = |l: &[OrderView]| l.iter().take_while(|e| (e.seq as i64) >= DAY).count();
    assert_eq!(with_break(&listed), 0, "a break would count nothing");
    assert_eq!(tiles(&listed, "loc_sushi", DAY).count, 1, "the order placed today is counted");
    assert_eq!(tiles(&listed, "loc_sushi", DAY), old_tiles(&listed, "loc_sushi", DAY));
}

#[test]
#[ignore]
fn measure_loopa_row3() {
    let median_us = |f: &dyn Fn()| {
        let mut t: Vec<u128> = (0..9)
            .map(|_| {
                let s = std::time::Instant::now();
                f();
                s.elapsed().as_nanos()
            })
            .collect();
        t.sort_unstable();
        t[4] as f64 / 1e3
    };
    for (n, today) in [(100, 0), (300, 50), (1000, 750)] {
        let f = fixture(n, today);
        assert_eq!(tiles(&f, "loc_sushi", DAY), old_tiles(&f, "loc_sushi", DAY));
        let a = median_us(&|| { std::hint::black_box(old_tiles(&f, "loc_sushi", DAY)); });
        let b = median_us(&|| { std::hint::black_box(tiles(&f, "loc_sushi", DAY)); });
        println!("ROW3 dashboard n={n} today={today}: BEFORE {a:.0} us | AFTER {b:.0} us | {:.1}x | cap share {:.1}% -> {:.1}%",
            a / b.max(0.001), a / 100.0, b / 100.0);
    }
}
