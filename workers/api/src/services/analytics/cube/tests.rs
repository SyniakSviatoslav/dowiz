//! The cube, pinned: its rows, its bytes, its catch-up arithmetic.

use super::*;
use serde_json::json;

fn tirane() -> Zone {
    dowiz_hub::tz::zone("Europe/Tirane").expect("the venue's zone")
}
/// 2026-09-22 12:00 UTC = 14:00 in Tirane.
const NOON: i64 = 1_790_078_400_000;

fn order(id: &str, at: i64, status: &str, items: Value) -> Value {
    json!({ "id": id, "created_at_ms": at, "status": status, "total": 1000, "tip": 100,
            "fulfilment": { "kind": "delivery" }, "items": items })
}
fn sake(q: i64) -> Value {
    json!([{ "product_id": "sake", "quantity": q, "unit_price": 450, "unit_cost": 120 }])
}

#[test]
fn a_day_row_counts_visits_money_hours_channels_and_dishes() {
    let os = [
        order("a", NOON, "DELIVERED", sake(2)),
        order("b", NOON + 60_000, "REJECTED", sake(5)),
        order("c", NOON + 86_400_000 * 9, "DELIVERED", sake(1)), // from the future
    ];
    let rows = fold_orders(&os, tirane(), NOON + 3_600_000);
    assert_eq!(rows.len(), 1, "{rows:?}");
    let r = &rows[&20260922];
    assert_eq!((r.o, r.x, r.t, r.f), (2, 1, 900, 900), "the refused one is a visit, not money");
    assert_eq!(r.k, [2, 0, 0]);
    assert_eq!((r.h[14], r.ht[14]), (2, 900), "14:00 in Tirane");
    assert_eq!(r.c["storefront"], [2, 900]);
    assert_eq!(r.m["sake"], [2, 900, 2, 240], "two portions, stamped at 120 each");
}

/// A line without a stamp is sold but not costed: a partial sum is not a cost.
#[test]
fn an_unstamped_line_is_sold_and_not_costed() {
    let os = [order("a", NOON, "DELIVERED", json!([{ "product_id": "cola", "quantity": 3, "unit_price": 200 }]))];
    let r = &fold_orders(&os, tirane(), NOON)[&20260922];
    assert_eq!(r.m["cola"], [3, 600, 0, 0]);
}

#[test]
fn the_image_round_trips_and_a_range_read_parses_only_its_days() {
    let mut cube = Cube::default();
    let mut parts = BTreeMap::new();
    for (i, d) in [20260901, 20260915, 20260930].into_iter().enumerate() {
        let mut r = DayCube::new(d);
        r.o = i as i64 + 1;
        r.m.insert("sake".into(), [1, 450, 0, 0]);
        parts.insert(d, r);
    }
    assert!(cube.absorb("log@3", parts));
    let bytes = cube.encode();
    assert_eq!(Cube::decode(&bytes).unwrap(), cube);
    let mid = Cube::decode_range(&bytes, 20260910, 20260920).unwrap();
    assert_eq!(mid.rows.keys().copied().collect::<Vec<_>>(), vec![20260915]);
    assert_eq!(mid.folded, vec!["log@3".to_string()]);
    assert_eq!(mid.rows[&20260915].src, vec!["log@3".to_string()], "a row names its archive");
}

/// AN UNREADABLE IMAGE IS AN ERROR, never an empty history. Twin: above.
#[test]
fn a_damaged_or_foreign_image_is_refused_not_read_as_empty() {
    let good = Cube::default().encode();
    assert!(Cube::decode(&good).is_ok());
    assert!(Cube::decode(b"{\"v\":2,\"folded\":[]}\n").unwrap_err().contains("format 2"));
    assert!(Cube::decode(b"not json\n").is_err());
    let mut bad = good.clone();
    bad.extend_from_slice(b"{\"d\":20260901,\"o\":\"x\"}\n");
    assert!(Cube::decode(&bad).unwrap_err().contains("20260901"));
    let mut dayless = good;
    dayless.extend_from_slice(b"{\"o\":1}\n");
    assert!(Cube::decode(&dayless).unwrap_err().contains("no day"));
}

/// A RETRIED CATCH-UP CANNOT COUNT A DAY TWICE. Twin: a second archive adds.
#[test]
fn an_archive_is_folded_once_and_a_second_one_adds_to_the_same_day() {
    let part = |o| BTreeMap::from([(20260901, DayCube { d: 20260901, o, ..DayCube::default() })]);
    let mut cube = Cube::default();
    assert!(cube.absorb("log@3", part(2)));
    assert!(!cube.absorb("log@3", part(2)), "already folded");
    assert_eq!(cube.rows[&20260901].o, 2);
    assert!(cube.absorb("log@7", part(1)));
    assert_eq!(cube.rows[&20260901].o, 3);
    assert_eq!(cube.rows[&20260901].src, vec!["log@3".to_string(), "log@7".to_string()]);
    assert_eq!(cube.pending(&["log@3".into(), "log@7".into(), "log@9".into()]), vec!["log@9".to_string()]);
}

/// ONLY WHAT LEFT. An order kept at the rotation is in the next image and is
/// counted when it leaves; one that left is not in it.
#[test]
fn moved_is_the_archive_minus_the_image_after_it() {
    let pair = |id: &str| (id.to_string(), order(id, NOON, "DELIVERED", sake(1)).to_string());
    let next: HashSet<String> = ["kept".to_string()].into();
    let out = moved(vec![pair("gone"), pair("kept")], &next);
    assert_eq!(out.len(), 1);
    assert_eq!((out[0].0.as_str(), &out[0].1["id"]), ("gone", &json!("gone")));
}

/// THE VERIFIER'S BYTES: the same orders give the same line, and one lek
/// more does not.
#[test]
fn a_row_and_its_refold_agree_byte_for_byte_and_one_lek_breaks_it() {
    let os = vec![order("a", NOON, "DELIVERED", sake(2))];
    let a = line(&fold_orders(&os, tirane(), NOON)[&20260922]);
    assert_eq!(a, line(&fold_orders(&os, tirane(), NOON)[&20260922]));
    let mut more = os.clone();
    more[0]["total"] = json!(1001);
    assert_ne!(a, line(&fold_orders(&more, tirane(), NOON)[&20260922]));
    assert!(a.starts_with("{\"d\":20260922,"), "the day is the first key: {a}");
}
