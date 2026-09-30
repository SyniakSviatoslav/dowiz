//! W0a/W0b producers: once per transition, once per crossing, once per late
//! order -- and nothing to a venue nobody told. Every refusal has a twin.
use super::*;
use crate::notify::route::events;
use crate::notify::route::groups::State;
use serde_json::json;

const T0: i64 = 1_768_435_200_000;
const MIN: i64 = 60_000;

fn group(evs: &[(&str, Mode)]) -> Group {
    let mut g = Group::fresh("g".into(), "-1".into(), None, "g".into(), "group".into(), "en");
    g.subs = evs.iter().map(|(e, m)| (e.to_string(), *m)).collect();
    g
}

#[test]
fn every_event_this_lane_produces_is_live_in_the_catalogue() {
    for k in ["order.status", "order.late", "stock.low", "stock.received", "stock.expiring", "stock.wasted", "stocktake.variance"] {
        assert!(events::get(k).is_some_and(|e| e.live), "{k} is produced, so it is live");
    }
}

#[test]
fn a_group_that_hears_it_wants_it_and_a_muted_or_silent_one_does_not() {
    assert!(wants(&[group(&[("order.status", Mode::Now)])], "order.status"));
    assert!(wants(&[group(&[("order.status", Mode::Digest)])], "order.status"), "a summary line is still wanted");
    assert!(!wants(&[group(&[("order.placed", Mode::Now)])], "order.status"));
    let mut muted = group(&[("order.status", Mode::Now)]);
    muted.state = State::Muted;
    assert!(!wants(&[muted], "order.status"));
    assert!(!wants(&[], "order.status"));
}

#[test]
fn a_status_is_one_entry_per_transition_and_renders_without_a_customer() {
    let a = status("abcdefgh123", "READY", T0);
    assert_eq!((a.id.as_str(), a.kind.as_str(), a.to.as_str()), ("abcdefgh123/st/READY/route", "route", "order.status"));
    assert_ne!(a.id, status("abcdefgh123", "PREPARING", T0).id, "each transition its own id");
    let p: Value = serde_json::from_str(&a.text).unwrap();
    let text = super::super::render::event("order.status", &p["data"], "en");
    assert_eq!(text, "🔔 order #abcdefgh → READY");
}

fn order(id: &str, status: &str, age_min: i64) -> (String, String) {
    (id.into(), json!({ "status": status, "created_at_ms": T0 - age_min * MIN, "customer": { "name": "A" } }).to_string())
}

#[test]
fn an_order_waiting_past_the_threshold_is_told_once_and_its_mark_is_dropped_when_it_moves_on() {
    let orders = vec![order("o1", "PENDING", 25), order("o2", "PREPARING", 5), order("o3", "DELIVERED", 90)];
    let (owed, mark, drop) = late(&orders, &[], T0, 20 * MIN);
    assert_eq!(owed.len(), 1, "only the open one past 20 min");
    assert_eq!((owed[0].id.as_str(), owed[0].to.as_str()), ("o1/late/route", "order.late"));
    let p: Value = serde_json::from_str(&owed[0].text).unwrap();
    assert_eq!(p["data"], json!({ "order": "o1", "minutes": 25 }));
    assert!(!owed[0].text.contains("\"A\""), "no customer travels");
    assert_eq!(mark, vec!["o1".to_string()]);
    assert!(drop.is_empty());
    // ITS TWIN: marked, it is not told again; once it moved on, the mark goes.
    let (again, _, _) = late(&orders, &["o1".into()], T0 + MIN, 20 * MIN);
    assert!(again.is_empty(), "once");
    let moved = vec![order("o1", "READY", 26)];
    let (_, _, drop) = late(&moved, &["o1".into(), "gone".into()], T0, 20 * MIN);
    assert_eq!(drop, vec!["o1".to_string(), "gone".to_string()]);
}

#[test]
fn a_threshold_of_zero_is_off_and_an_unset_one_is_twenty_minutes() {
    assert!(late(&[order("o1", "PENDING", 500)], &[], T0, 0).0.is_empty());
    assert_eq!(late_ms(None), 20 * MIN);
    assert_eq!(late_ms(Some(" 45 ")), 45 * MIN);
    assert_eq!(late_ms(Some("0")), 0);
    assert_eq!(late_ms(Some("soon")), 20 * MIN, "unreadable is the default, not off");
}

fn rice() -> Option<Supply> {
    Some(Supply { name: "Rice".into(), unit: "g".into(), low_at: 1000 })
}

#[test]
fn low_fires_on_the_crossing_only() {
    let items = vec!["rice".to_string(), "rice".to_string()];
    let before = |_: &str| Shelf { free: 1200, counted: true };
    let after = |_: &str| Shelf { free: 900, counted: true };
    let d = low(&items, &before, &after, &|_| rice()).expect("crossed");
    assert_eq!(d, json!({ "items": [{ "name": "Rice", "on_hand": 900, "low_at": 1000, "unit": "g" }] }), "once, though named twice");
    // Already below before: no second message.
    let under = |_: &str| Shelf { free: 950, counted: true };
    assert!(low(&items, &under, &after, &|_| rice()).is_none());
    // Uncounted: its zero is unknown, never an alarm.
    let unknown = |_: &str| Shelf { free: 900, counted: false };
    assert!(low(&items, &before, &unknown, &|_| rice()).is_none());
    // No threshold set, or not a supply: nothing.
    assert!(low(&items, &before, &after, &|_| Some(Supply { low_at: 0, ..rice().unwrap() })).is_none());
    assert!(low(&items, &before, &after, &|_| None).is_none());
    // Exactly at the threshold counts.
    let at = |_: &str| Shelf { free: 1000, counted: true };
    assert!(low(&items, &before, &at, &|_| rice()).is_some());
}

#[test]
fn a_turns_entries_are_routed_with_ids_unique_to_the_turn() {
    let e = entries("stock/12", vec![("stock.wasted", json!({ "name": "Rice" }))], T0);
    assert_eq!((e[0].id.as_str(), e[0].kind.as_str(), e[0].to.as_str()), ("stock/12/stock.wasted/route", "route", "stock.wasted"));
    assert_eq!(serde_json::from_str::<Value>(&e[0].text).unwrap(), json!({ "data": { "name": "Rice" } }));
}

/// W-PF3 T2: a production act is told only to a group that chose it.
#[test]
fn a_production_act_reaches_only_a_group_that_chose_it() {
    assert!(events::get("stock.cooked").is_some_and(|e| e.live && e.area == events::Area::Stock));
    assert!(events::holds_in_quiet("stock.cooked"), "a batch waits out the quiet hours");
    assert!(wants(&[group(&[("stock.cooked", Mode::Now)])], "stock.cooked"));
    // Twins: no group linked, and a freshly linked one that never chose it.
    assert!(!wants(&[], "stock.cooked"));
    let fresh = Group::fresh("g".into(), "-1".into(), None, "g".into(), "group".into(), "en");
    assert!(!wants(&[fresh], "stock.cooked"));
}
