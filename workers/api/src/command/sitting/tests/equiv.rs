//! W-LOOPA row 1: ONE PARSE PER ORDER for the floor, the room and the QR placement.
//!
//! THE ORACLE IS THE OLD CODE, copied here verbatim (only renamed): distinct sitting ids by
//! `ids.iter().any`, then `rounds` per sitting, which parsed every order again -- S x N
//! parses. Every new answer must be the old answer byte for byte: the `Seated` facts the
//! floor reads, the room's JSON, the live sitting at every table, and the placer's
//! venue-filtered variant. Fixtures: R-LOOPS B3's envelope shape (1.3 KB orders) with the
//! edge cases the old code skipped (no `sitting_id`, a non-string one, an unparsable
//! envelope, another venue, equal `created_at_ms`), plus the existing room tests' floors.

use super::*;
use crate::command::floor::{self, table_of, Seated};
use crate::services::orders::room::table_link;
use dowiz_hub::tables::{Plan, Shape, Table, Zone};

// ── the old code (oracle) ───────────────────────────────────────────────────

fn old_rounds<'a>(listed: &'a [OrderView], sitting_id: &str) -> Vec<Round<'a>> {
    let mut out: Vec<Round<'a>> = listed
        .iter()
        .filter_map(|v| {
            let order: Value = serde_json::from_str(&v.order_json).ok()?;
            (order.get("sitting_id").and_then(Value::as_str) == Some(sitting_id)).then_some(Round { view: v, order })
        })
        .collect();
    out.sort_by_key(|r| r.int("created_at_ms"));
    out
}

fn old_ids(listed: &[OrderView]) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for v in listed {
        let Ok(o) = serde_json::from_str::<Value>(&v.order_json) else { continue };
        if let Some(s) = o.get("sitting_id").and_then(Value::as_str) {
            if !ids.iter().any(|x| x == s) {
                ids.push(s.to_string());
            }
        }
    }
    ids
}

fn old_floor_sittings(listed: &[OrderView]) -> Vec<Seated> {
    old_ids(listed).iter().filter_map(|s| floor::seated(s, &old_rounds(listed, s))).collect()
}

fn old_room(listed: &[OrderView]) -> Vec<Value> {
    old_ids(listed)
        .iter()
        .filter_map(|s| {
            let rs = old_rounds(listed, s);
            open(&rs).then(|| card(s, &rs))
        })
        .collect()
}

fn old_live_sitting(plan: &Plan, listed: &[OrderView], zone: &str, n: i64) -> Option<String> {
    let mut best: Option<(i64, String)> = None;
    let mut seen: Vec<String> = Vec::new();
    for v in listed {
        let Ok(o) = serde_json::from_str::<Value>(&v.order_json) else { continue };
        let Some(sid) = o.get("sitting_id").and_then(Value::as_str) else { continue };
        if seen.iter().any(|s| s == sid) {
            continue;
        }
        seen.push(sid.to_string());
        let rounds = old_rounds(listed, sid);
        let here = rounds
            .iter()
            .rev()
            .find_map(|r| r.order.pointer("/fulfilment/table").and_then(Value::as_str))
            .and_then(|t| table_of(plan, t))
            .is_some_and(|(z, k)| z == zone && k == n);
        if !here || !open(&rounds) {
            continue;
        }
        let newest = rounds.iter().map(|r| r.int("created_at_ms")).max().unwrap_or(0);
        if best.as_ref().is_none_or(|(at, _)| newest > *at) {
            best = Some((newest, sid.to_string()));
        }
    }
    best.map(|(_, s)| s)
}

/// The placer's old read: filter the venue's orders (one parse), then `live_sitting` (more).
fn old_placer(plan: &Plan, all: &[OrderView], loc: &str, zone: &str, n: i64) -> Option<String> {
    let listed: Vec<OrderView> = all
        .iter()
        .filter(|o| {
            serde_json::from_str::<Value>(&o.order_json)
                .is_ok_and(|v| v.get("location_id").and_then(|l| l.as_str()) == Some(loc))
        })
        .cloned()
        .collect();
    old_live_sitting(plan, &listed, zone, n)
}

fn new_placer(plan: &Plan, all: &[OrderView], loc: &str, zone: &str, n: i64) -> Option<String> {
    let mine = sittings_where(all, |v| v.get("location_id").and_then(|l| l.as_str()) == Some(loc));
    table_link::live_in(plan, &mine, zone, n)
}

// ── fixtures ────────────────────────────────────────────────────────────────

const LOC: &str = "loc_sushi";

fn plan() -> Plan {
    let t = |n: i64| Table { n, x: 40 * n, y: 50, w: 30, h: 30, seats: 4, shape: Shape::Rect };
    Plan {
        zones: vec![
            Zone { id: "salla".into(), name: "Salla".into(), tables: (1..=20).map(t).collect() },
            Zone { id: "terasa".into(), name: "Terasa".into(), tables: (18..=24).map(t).collect() },
        ],
    }
}

fn ov(order_id: String, seq: u64, order_json: String) -> OrderView {
    OrderView { order_id, kind: 1, seq, order_json }
}

/// R-LOOPS B3's order (a ~1.3 KB kernel envelope), newest first, S sittings, with the
/// shapes the old code skipped or tied on mixed in at fixed strides.
fn fixture(n: usize, sittings: usize) -> Vec<OrderView> {
    const STATUSES: [&str; 9] =
        ["PENDING", "CONFIRMED", "PREPARING", "READY", "PICKED_UP", "DELIVERED", "REJECTED", "CANCELLED", "SCHEDULED"];
    (0..n)
        .rev()
        .map(|i| {
            let s = i % sittings;
            let status = STATUSES[(i / sittings + s) % STATUSES.len()];
            // Equal `created_at_ms` on neighbours: the stable sort's tie order is part of the answer.
            let at = 1_782_900_000_000 + (i as i64 / 2) * 20_000;
            let table = match s % 4 {
                0 => format!("salla:{}", 1 + s % 20),
                1 => format!("{}", 1 + s % 24), // bare: 18-20 are in both zones -> unplaced
                2 => format!("terasa/{}", 18 + s % 7),
                _ => "bar".to_string(),
            };
            let total = 1500 + (i as i64 % 13) * 250;
            let paid = if i % 3 == 0 { total } else if i % 5 == 0 { total / 2 } else { 0 };
            let mut v = json!({
                "id": format!("o_{i}"), "location_id": if i % 11 == 0 { "loc_other" } else { LOC },
                "status": status, "created_at_ms": at, "sitting_id": format!("s_{s}"),
                "placed_by": if i % 7 == 0 { "guest" } else { "staff" },
                "total": total, "currency": "ALL", "channel": "console",
                "fulfilment": { "kind": "dine_in", "table": table, "address": { "line": "" } },
                "contact": { "name": format!("Guest {i}"), "phone": "+355690000000" },
                "items": (0..4).map(|k| json!({ "product_id": format!("p_{}", (i * 7 + k) % 165),
                    "name": format!("Roll {} me salmon dhe avokado", (i * 7 + k) % 165), "quantity": 1 + k as i64 % 2,
                    "unit_price": 700 + 10 * k as i64, "modifiers": [{"id": "o_8", "name": "8 copë", "price": 0}],
                    "allergens": ["fish", "sesame"] })).collect::<Vec<_>>(),
                "payments": if paid > 0 { json!([{ "method": "cash", "amount": paid, "at": at + 600_000 }]) } else { json!([]) },
                "history": (0..3).map(|k| json!({ "status": status, "at": at + k * 60_000, "by": "staff-1" })).collect::<Vec<_>>(),
                "notes": "pa qepë, ju lutem", "promo": null, "tip": 0, "tax": { "ppm": 200000, "amount": 250 },
            });
            if i % 17 == 0 {
                v["cleared"] = json!({ "by": "w", "at": at + 900_000 });
            }
            match i % 23 {
                5 => {
                    v.as_object_mut().unwrap().remove("sitting_id");
                }
                9 => v["sitting_id"] = json!(42),
                _ => {}
            }
            let text = if i % 29 == 13 { "{not json".to_string() } else { v.to_string() };
            ov(format!("o_{i}"), at as u64, text)
        })
        .collect()
}

/// The floors the existing room tests draw (`floor/tests.rs`, `sitting/tests.rs`,
/// `table_link/tests.rs`), as one more set of fixtures.
fn small_floors() -> Vec<Vec<OrderView>> {
    let r = |id: &str, s: &str, table: &str, st: &str, total: i64, paid: i64, at: i64| {
        let payments: Vec<Value> =
            if paid > 0 { vec![json!({"amount": paid, "method": "card", "at": at + 50})] } else { vec![] };
        ov(id.into(), 10, json!({"id": id, "location_id": LOC, "sitting_id": s, "status": st, "total": total,
            "payments": payments, "created_at_ms": at, "fulfilment": {"kind": "dine_in", "table": table}}).to_string())
    };
    vec![
        vec![
            r("a", "s1", "1", "CONFIRMED", 900, 0, 1),
            r("b", "s2", "salla:2", "PREPARING", 900, 0, 1),
            r("c", "s3", "3", "READY", 900, 400, 1),
            r("d", "s4", "4", "PICKED_UP", 900, 900, 1),
        ],
        vec![r("b", "s2", "4", "PENDING", 900, 0, 50), r("a", "s1", "4", "PICKED_UP", 900, 900, 1)],
        vec![r("a", "s1", "4", "PICKED_UP", 900, 900, 1), r("b", "s1", "4", "REJECTED", 300, 0, 2)],
        vec![
            r("a", "s1", "t-1", "PREPARING", 1500, 0, 1),
            r("b", "s1", "t-5", "PENDING", 400, 0, 5),
            r("c", "s2", "t-1", "PICKED_UP", 900, 900, 1),
            ov("d".into(), 10, json!({"id": "d", "status": "PENDING", "total": 100}).to_string()),
        ],
        vec![
            r("a", "old-sitting-1", "salla:3", "PICKED_UP", 900, 900, 1),
            r("b", "live-sitting-2", "3", "CONFIRMED", 500, 0, 10),
            r("c", "other-table-3", "salla:4", "PENDING", 500, 0, 20),
        ],
        vec![r("d", "terrace-sit-4", "24", "PREPARING", 100, 0, 5)],
        vec![],
    ]
}

fn every_table(p: &Plan) -> Vec<(String, i64)> {
    p.zones.iter().flat_map(|z| z.tables.iter().map(|t| (z.id.clone(), t.n))).chain([("nowhere".into(), 1)]).collect()
}

/// Old == new on one listing, every way the four callers read it. Returns how many
/// sittings were compared, so a fixture that grouped nothing cannot pass silently.
/// `all_tables`: the live sitting at EVERY table on the plan; otherwise (the big fixtures,
/// where the old code is O(S x N) per table and a debug run would take minutes) at every
/// table the new code finds live plus two it does not.
pub(crate) fn assert_same(listed: &[OrderView], all_tables: bool) -> usize {
    let p = plan();
    // the grouping itself: same sittings, same rounds, same order
    let new: Vec<(String, Vec<String>)> = sittings_of(listed)
        .into_iter()
        .map(|(s, rs)| (s, rs.iter().map(|r| r.view.order_id.clone()).collect()))
        .collect();
    let old: Vec<(String, Vec<String>)> = old_ids(listed)
        .into_iter()
        .map(|s| {
            let ids = old_rounds(listed, &s).iter().map(|r| r.view.order_id.clone()).collect();
            (s, ids)
        })
        .collect();
    assert_eq!(new, old, "grouping");
    assert_eq!(floor::sittings(listed), old_floor_sittings(listed), "floor facts");
    assert_eq!(
        serde_json::to_string(&room(listed)).unwrap(),
        serde_json::to_string(&old_room(listed)).unwrap(),
        "room JSON bytes"
    );
    let mut tables = every_table(&p);
    if !all_tables {
        let (live, idle): (Vec<_>, Vec<_>) =
            tables.into_iter().partition(|(z, n)| table_link::live_sitting(&p, listed, z, *n).is_some());
        tables = live.into_iter().chain(idle.into_iter().take(2)).collect();
    }
    for (z, n) in tables {
        assert_eq!(table_link::live_sitting(&p, listed, &z, n), old_live_sitting(&p, listed, &z, n), "live {z}:{n}");
        assert_eq!(new_placer(&p, listed, LOC, &z, n), old_placer(&p, listed, LOC, &z, n), "placer {z}:{n}");
    }
    old.len()
}

#[test]
fn one_pass_grouping_answers_exactly_what_the_per_sitting_parse_did() {
    let mut compared = 0;
    // 300 orders / 60 sittings is held to the oracle in `measure_loopa_row1` (release): the old
    // code's S x N parses take minutes in a debug build.
    for (n, s) in [(1, 1), (37, 5), (100, 20)] {
        compared += assert_same(&fixture(n, s), n <= 37);
    }
    for f in small_floors() {
        compared += assert_same(&f, true);
    }
    // 1 + 5 + 20 sittings from the generator, 13 from the small floors.
    assert_eq!(compared, 1 + 5 + 20 + 13, "every sitting was compared");
    // the fixture really holds the edge cases the old code skipped, and live tables
    let f = fixture(100, 20);
    assert!(f.iter().any(|o| o.order_json.starts_with("{not")));
    assert!(f.iter().any(|o| o.order_json.contains("\"sitting_id\":42")));
    let p = plan();
    let live = every_table(&p).iter().filter(|(z, n)| table_link::live_sitting(&p, &f, z, *n).is_some()).count();
    assert!(live >= 5, "only {live} live tables: the live-sitting comparison would be vacuous");
}

// ── MEASURED (release, `cargo test --release --lib measure_loopa_row1 -- --ignored --nocapture`) ──
// Integer ns (the float-money gate covers this path; a timing needs no float). `tenths(n)` prints n/10 as "i.d".
fn median_ns(reps: usize, mut f: impl FnMut()) -> u128 {
    let mut t: Vec<u128> = (0..reps).map(|_| (std::time::Instant::now(), f()).0.elapsed().as_nanos()).collect();
    t.sort_unstable();
    t[reps / 2]
}
fn tenths(n: u128) -> String { format!("{}.{}", n / 10, n % 10) }
#[test]
#[ignore]
fn measure_loopa_row1() {
    let p = plan();
    for (n, s) in [(100, 20), (300, 60)] {
        let f = fixture(n, s);
        assert_same(&f, false);
        let row = |what: &str, a: u128, b: u128| {
            println!("ROW1 {what} n={n} S={s}: BEFORE {} us | AFTER {} us | {}x | cap share (of 10 ms) {}% -> {}%",
                a / 1000, b / 1000, tenths(a * 10 / b.max(1)), tenths(a / 10_000), tenths(b / 10_000))
        };
        let a = median_ns(9, || { std::hint::black_box(old_floor_sittings(&f)); });
        let b = median_ns(9, || { std::hint::black_box(floor::sittings(&f)); });
        row("floor sittings", a, b);
        let a = median_ns(9, || { std::hint::black_box(old_room(&f)); });
        let b = median_ns(9, || { std::hint::black_box(room(&f)); });
        row("room", a, b);
        let a = median_ns(9, || { std::hint::black_box(old_placer(&p, &f, LOC, "salla", 3)); });
        let b = median_ns(9, || { std::hint::black_box(new_placer(&p, &f, LOC, "salla", 3)); });
        row("QR placement", a, b);
    }
}
