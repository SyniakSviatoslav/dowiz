//! W-LOOPA row 9: the promotions page's use counts, folded ONCE, equal the old per-code
//! answer (copied below as the oracle) for every code, over a REAL hub of >= 600 events.

use crate::hubdo::OrderView;
use crate::services::ordering::promo_fields::{promo_uses_all, promo_uses_in};
use dowiz_hub::{EventKind, Hub};
use serde_json::json;

/// The page's old per-promo read: a whole refold of the log per code.
fn old_promo_uses(hub: &Hub, code: &str) -> i64 {
    promo_uses_in(&crate::hubstore::orders_state(hub).into_iter().map(OrderView::of_event).collect::<Vec<_>>(), code)
}

const CODES: [&str; 6] = ["WELCOME", "SUMMER10", "FRIDAY", "LUNCH", "AUTUMN", "welcome"];

/// `orders` orders, each placed with one of the codes (or none) and then advanced through
/// statuses as deltas, some to REJECTED/CANCELLED (a use given back), with non-order
/// events between and an unreadable payload: `events` records in all.
fn hub(orders: usize) -> (Hub, usize) {
    let mut h = Hub::create_sized(4 * 1024 * 1024).unwrap();
    let mut seq = 1_782_900_000_000u64;
    let mut put = |h: &mut Hub, kind: EventKind, id: &str, payload: String| {
        seq += 1;
        h.append(kind, id, &payload, seq, [0u8; 32]).unwrap();
    };
    for i in 0..orders {
        let id = format!("o_{i}");
        let promo = if i % 7 == 6 { json!(null) } else { json!({"code": CODES[i % CODES.len()], "discount": 150}) };
        let body = json!({"id": id, "location_id": "loc_sushi", "status": "PENDING", "total": 2650, "promo": promo,
            "items": [{"product_id": "maki", "name": "Maki salmon", "quantity": 2, "unit_price": 1250}],
            "contact": {"name": "Ana Hoxha", "phone": "+355691234567"}});
        put(&mut h, EventKind::Placed, &id, if i % 97 == 50 { "not json".into() } else { body.to_string() });
        let next = match i % 5 {
            0 => "CONFIRMED",
            1 => "REJECTED",
            2 => "PREPARING",
            3 => "CANCELLED",
            _ => "PENDING",
        };
        let mut delta = json!({"status": next});
        delta[crate::fold::DELTA_MARK] = json!(true);
        put(&mut h, EventKind::Advanced, &id, delta.to_string());
        if i % 4 == 0 {
            put(&mut h, EventKind::Revealed, &format!("cust:{i}"), json!({"who": "owner"}).to_string());
        }
    }
    let n = h.events().len();
    (h, n)
}

#[test]
fn one_fold_counts_every_code_as_the_per_code_refold_did() {
    let (h, events) = hub(270);
    assert!(events >= 600, "{events} events: the card asks for >= 600");
    let all = promo_uses_all(&h);
    for code in CODES.iter().copied().chain(["NOBODY-USED-THIS", ""]) {
        assert_eq!(all.get(code).copied().unwrap_or(0), old_promo_uses(&h, code), "{code}");
    }
    // nothing counted that no code asked about, and the counts are not all zero
    assert!(all.keys().all(|k| CODES.contains(&k.as_str())), "{all:?}");
    assert!(CODES.iter().filter(|c| old_promo_uses(&h, c) > 0).count() >= 5, "{all:?}");
}

#[test]
#[ignore]
fn measure_loopa_row9() {
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
    for (orders, promos) in [(270, 5), (270, 10), (600, 5)] {
        let (h, events) = hub(orders);
        let codes: Vec<String> = (0..promos).map(|k| CODES.get(k).map_or(format!("EXTRA{k}"), |c| c.to_string())).collect();
        let a = median_us(&|| {
            for c in &codes {
                std::hint::black_box(old_promo_uses(&h, c));
            }
        });
        let b = median_us(&|| {
            let all = promo_uses_all(&h);
            for c in &codes {
                std::hint::black_box(all.get(c).copied().unwrap_or(0));
            }
        });
        println!("ROW9 promotions events={events} promos={promos}: BEFORE {a:.0} us | AFTER {b:.0} us | {:.1}x | cap share {:.1}% -> {:.1}%",
            a / b.max(0.001), a / 100.0, b / 100.0);
    }
}
