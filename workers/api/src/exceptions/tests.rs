//! The exception report and the alert, each case beside its positive twin.
//! Amendments are written by the REAL `command::amend::apply`, so the rows are
//! read from the bodies the object actually appends.

use super::alert::{due, late_ms, local_stamp, threshold, Voice};
use super::fold::*;
use super::*;
use crate::command::amend::{apply, AmendIn, Op};
use crate::hubdo::OrderView;
use dowiz_hub::logimage::Entry;
use dowiz_hub::{Event, EventKind};
use serde_json::{json, Value};

const T0: i64 = 1_790_000_000_000;
const MIN: i64 = 60_000;

fn round(status: &str) -> Value {
    json!({
        "id": "r1", "status": status, "location_id": "v1", "currency": "ALL",
        "items": [
            {"product_id": "maki", "quantity": 2, "unit_price": 600, "name": "Maki"},
            {"product_id": "beer", "quantity": 1, "unit_price": 300, "name": "Beer"}
        ],
        "subtotal": 1500, "discount": 0, "delivery_fee": 0, "tip": 0, "total": 1500,
    })
}

fn ev(kind: EventKind, body: &Value, seq: i64) -> Event {
    Event { kind, order_id: "r1".into(), order_json: body.to_string(), seq: seq as u64 }
}

/// Placed at T0 with `status`, then one amendment by anna at `at`.
fn amended(status: &str, ops: Vec<Op>, reason: Option<&str>, at: i64) -> Vec<Event> {
    let old = round(status);
    let view = OrderView { order_id: "r1".into(), kind: 1, seq: T0 as u64, order_json: old.to_string() };
    let input = AmendIn {
        order_id: "r1".into(), location_id: "v1".into(), base_seq: T0 as u64, ops,
        by: "anna".into(), reason: reason.map(str::to_string), may_void: true, boms: vec![], now_ms: at,
    };
    let (new, _) = apply(&view, &input).expect("the amendment is legal");
    vec![ev(EventKind::Placed, &old, T0), ev(EventKind::Amended, &crate::fold::delta(&old, &new), at)]
}

fn kinds(rows: &[Row]) -> Vec<&'static str> {
    rows.iter().map(|r| r.kind).collect()
}

#[test]
fn a_void_after_the_kitchen_is_a_row_and_before_it_is_not() {
    let rows = order_rows(&amended("PREPARING", vec![Op::Remove { line: 0 }], Some("mistake"), T0 + MIN), 30 * MIN);
    assert_eq!(kinds(&rows), vec![VOID_AFTER_KITCHEN]);
    let r = &rows[0];
    assert_eq!((r.amount, r.by.as_str(), r.reason.as_deref()), (1200, "anna", Some("mistake")));
    assert_eq!((r.order_id.as_deref(), r.currency.as_deref(), r.at), (Some("r1"), Some("ALL"), T0 + MIN));
    // Twin: the same removal before the kitchen is an ordinary edit.
    let rows = order_rows(&amended("CONFIRMED", vec![Op::Remove { line: 0 }], Some("mistake"), T0 + MIN), 30 * MIN);
    assert!(rows.is_empty(), "{rows:?}");
}

#[test]
fn a_comp_is_a_row_with_its_amount_and_signer() {
    let rows = order_rows(&amended("CONFIRMED", vec![Op::Comp { line: 1 }], Some("guest_changed"), T0 + MIN), 30 * MIN);
    assert_eq!(kinds(&rows), vec![COMP]);
    assert_eq!((rows[0].amount, rows[0].by.as_str()), (300, "anna"));
}

#[test]
fn a_late_amendment_is_a_row_and_a_prompt_one_is_not() {
    let table = |at| amended("CONFIRMED", vec![Op::Table { table: "9".into() }], None, at);
    let late = order_rows(&table(T0 + 31 * MIN), 30 * MIN);
    assert_eq!(kinds(&late), vec![LATE_AMENDMENT]);
    assert_eq!(late[0].by, "anna");
    assert!(order_rows(&table(T0 + 5 * MIN), 30 * MIN).is_empty());
}

#[test]
fn cash_without_a_till_is_a_row_and_cash_in_a_till_is_not() {
    let mut o = round("DELIVERED");
    o["payments"] = json!([{"method": "cash", "amount": 1500, "currency": "ALL", "by": "ben", "at": T0}]);
    let rows = order_rows(&[ev(EventKind::Placed, &o, T0)], 30 * MIN);
    assert_eq!(kinds(&rows), vec![CASH_OUTSIDE_TILL]);
    assert_eq!((rows[0].amount, rows[0].by.as_str()), (1500, "ben"));
    o["payments"][0]["till_id"] = json!("t1");
    assert!(order_rows(&[ev(EventKind::Placed, &o, T0)], 30 * MIN).is_empty());
    // A card payment without a till is not cash.
    o["payments"] = json!([{"method": "card", "amount": 1500, "by": "ben", "at": T0}]);
    assert!(order_rows(&[ev(EventKind::Placed, &o, T0)], 30 * MIN).is_empty());
}

#[test]
fn a_refund_is_a_row() {
    let mut o = round("REFUNDING");
    o["refund"] = json!({"reason": "late", "by": "cara", "at": T0 + MIN, "owed": 1500});
    let rows = order_rows(&[ev(EventKind::Placed, &o, T0)], 30 * MIN);
    assert_eq!(kinds(&rows), vec![REFUND]);
    assert_eq!((rows[0].amount, rows[0].reason.as_deref()), (1500, Some("late")));
    assert!(order_rows(&[ev(EventKind::Placed, &round("DELIVERED"), T0)], 30 * MIN).is_empty());
}

fn till_log() -> Vec<Entry> {
    let e = |seq, kind: &str, j: Value| Entry { kind: kind.into(), subject: "t1".into(), json: j.to_string(), seq };
    vec![
        e(0, "till.opened", json!({"float": {"ALL": 5000}, "by": "dan", "at": T0})),
        e(1, "till.pay_out", json!({"currency": "ALL", "amount": 800, "reason": "supplier", "by": "dan", "at": T0 + MIN})),
        e(2, "till.closed", json!({"counted": {}, "expected": {}, "over_short": {"ALL": -200, "EUR": 0}, "by": "eve", "at": T0 + 9 * MIN})),
    ]
}

#[test]
fn a_pay_out_and_a_nonzero_over_short_are_rows_and_zero_is_not() {
    let log = till_log();
    let periods = crate::command::till::periods(&log).unwrap();
    let rows = till_rows(&log, &periods);
    assert_eq!(kinds(&rows), vec![PAY_OUT_KIND, OVER_SHORT], "EUR 0 is not a row");
    assert_eq!((rows[0].amount, rows[0].by.as_str(), rows[0].till_id.as_deref()), (800, "dan", Some("t1")));
    assert_eq!((rows[1].amount, rows[1].by.as_str(), rows[1].currency.as_deref()), (-200, "eve", Some("ALL")));
}

#[test]
fn the_report_windows_groups_and_names_no_score() {
    let events = amended("PREPARING", vec![Op::Remove { line: 0 }], Some("mistake"), T0 + MIN);
    let v = report(&events, &till_log(), vec![], 30 * MIN, T0, T0 + 10 * MIN).unwrap();
    assert_eq!(v["rows"].as_array().unwrap().len(), 3);
    let groups = v["groups"].as_array().unwrap();
    assert!(groups.iter().any(|g| g["kind"] == VOID_AFTER_KITCHEN && g["reason"] == "mistake" && g["count"] == 1));
    assert_eq!(v["rounds"][0]["orderId"], "r1");
    // Outside the window: nothing.
    let none = report(&events, &till_log(), vec![], 30 * MIN, T0 + 20 * MIN, T0 + 30 * MIN).unwrap();
    assert!(none["rows"].as_array().unwrap().is_empty());
    // THE GATE'S PATTERN over every key (no-scoring.sh, DECISIONS OD-8).
    let gate = regex_lite(&v);
    assert!(gate.is_empty(), "keys that rate a participant: {gate:?}");
    // And no group or round is keyed by a person.
    assert!(v["groups"].as_array().unwrap().iter().all(|g| g.get("by").is_none()));
}

/// Keys matching `no-scoring.sh`'s pattern, walked over the whole document.
fn regex_lite(v: &Value) -> Vec<String> {
    let who = ["courier", "customer", "client", "venue", "staff", "waiter", "user", "diner", "guest", "partner"];
    // The two whole words are spelled in halves: the gate greps this file too.
    let (rep, v_ip) = (["repu", "tation"].concat(), ["v", "ip"].concat());
    let judge = ["score", "rating", "rank", "tier", rep.as_str()];
    let mut bad = Vec::new();
    let mut walk = vec![v];
    while let Some(x) = walk.pop() {
        match x {
            Value::Object(m) => {
                for (k, c) in m {
                    let k2 = k.to_lowercase();
                    let hit = who.iter().any(|w| judge.iter().any(|j| k2.contains(&format!("{w}_{j}"))
                        || k2.contains(&format!("{j}_of_{w}")) || k2.contains(&format!("{w}{j}"))))
                        || k2.contains(rep.as_str()) || k2 == v_ip;
                    if hit {
                        bad.push(k.clone());
                    }
                    walk.push(c);
                }
            }
            Value::Array(a) => walk.extend(a.iter()),
            _ => {}
        }
    }
    bad
}

#[test]
fn the_gate_walker_is_not_blind() {
    // Built at run time so the gate's own grep does not count this probe.
    let (bad, camel) = (["staff", "score"].join("_"), ["user", "Rank"].concat());
    let doc = json!({ "rows": [{ bad.clone(): 1 }], camel.clone(): 2 });
    assert_eq!(regex_lite(&doc), vec![camel, bad]);
}

fn voids(ats: &[i64]) -> Vec<Row> {
    ats.iter()
        .enumerate()
        .map(|(i, at)| Row {
            at: *at, kind: VOID_AFTER_KITCHEN, order_id: Some(format!("r{i}")), till_id: None,
            reason: Some("mistake".into()), amount: 100, currency: Some("ALL".into()), by: format!("p{i}"), tx_id: None,
        })
        .collect()
}

#[test]
fn the_alert_fires_at_the_threshold_and_names_the_events_and_signers() {
    let now = T0 + 10 * MIN;
    let got = due(&voids(&[T0 + MIN, T0 + 2 * MIN, now]), T0, now, 3, &voice("en"), "chat-1");
    assert_eq!(got.len(), 1);
    let e = &got[0];
    assert_eq!((e.kind.as_str(), e.to.as_str()), ("telegram", "chat-1"));
    assert_eq!(e.id, format!("exceptions/{T0}/{VOID_AFTER_KITCHEN}/3"));
    for (id, by) in [("r0", "p0"), ("r1", "p1"), ("r2", "p2")] {
        assert!(e.text.contains(id) && e.text.contains(&format!("by {by}")), "{}", e.text);
    }
    // The period start in VENUE-LOCAL time: T0 is 2026-09-21 14:13:20 UTC, 16:13 in Tirane (CEST).
    assert!(e.text.starts_with("v1: 3 exceptions · void after kitchen · since 2026-09-21 16:13"), "{}", e.text);
    let sq = due(&voids(&[T0 + MIN, T0 + 2 * MIN, now]), T0, now, 3, &voice("sq"), "c");
    assert!(sq[0].text.contains("anulim pas kuzhinës") && sq[0].text.contains("nga p0"), "{}", sq[0].text);
}

#[test]
fn the_alert_is_quiet_below_after_and_when_off() {
    let now = T0 + 10 * MIN;
    assert!(due(&voids(&[T0 + MIN, now]), T0, now, 3, &voice("en"), "c").is_empty(), "below");
    assert!(due(&voids(&[T0 + MIN, T0 + 2 * MIN, T0 + 3 * MIN]), T0, now, 3, &voice("en"), "c").is_empty(), "crossed earlier");
    assert!(due(&voids(&[T0 - MIN, T0 + MIN, now]), T0, now, 3, &voice("en"), "c").is_empty(), "one is before the period");
    assert!(due(&voids(&[T0 + MIN, T0 + 2 * MIN, now]), T0, now, 0, &voice("en"), "c").is_empty(), "off");
    assert!(due(&voids(&[T0 + MIN, T0 + 2 * MIN, now]), T0, now, 3, &voice("en"), " ").is_empty(), "no chat");
    // The sixth crosses again, at level 6.
    let six = voids(&[T0 + 1, T0 + 2, T0 + 3, T0 + 4, T0 + 5, now]);
    assert_eq!(due(&six, T0, now, 3, &voice("en"), "c")[0].id, format!("exceptions/{T0}/{VOID_AFTER_KITCHEN}/6"));
}

#[test]
fn the_owner_settings_parse_with_defaults() {
    assert_eq!((threshold(None), threshold(Some("5")), threshold(Some("0")), threshold(Some("x"))), (3, 5, 0, 3));
    assert_eq!((late_ms(None), late_ms(Some("10")), late_ms(Some("-1"))), (30 * MIN, 10 * MIN, 30 * MIN));
}

#[test]
fn the_window_reads_from_to_and_the_till_period() {
    let q = |p: &[(&str, &str)]| p.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect::<Vec<_>>();
    let periods = crate::command::till::periods(&till_log()).unwrap();
    assert_eq!(window_of(&q(&[]), &periods, T0), (T0 - alert::WINDOW_MS, T0));
    assert_eq!(window_of(&q(&[("from", "5"), ("to", "9")]), &periods, T0), (5, 9));
    assert_eq!(window_of(&q(&[("period", "till")]), &periods, T0 + 99 * MIN), (T0, T0 + 9 * MIN));
}

fn voice(lang: &'static str) -> Voice<'static> {
    Voice { venue: "v1", zone: dowiz_hub::tz::zone("Europe/Tirane").unwrap(), lang }
}

/// Venue-local, across the October change: CEST is +2, CET is +1.
#[test]
fn the_period_start_is_venue_local_on_both_sides_of_the_clock_change() {
    let z = dowiz_hub::tz::zone("Europe/Tirane").unwrap();
    assert_eq!(local_stamp(z, 1_790_000_000_000), "2026-09-21 16:13");
    // 2026-12-01 00:30 UTC = 01:30 in Tirane (CET, +1).
    assert_eq!(local_stamp(z, 1_796_085_000_000), "2026-12-01 01:30");
    assert_eq!(local_stamp(dowiz_hub::tz::zone("UTC").unwrap(), 0), "1970-01-01 00:00");
}

/// The object's hook runs `due` after a till command with the command's own
/// `now_ms`: the records written by the REAL `till::decide` carry that clock,
/// so the third pay-out of an open period is the one that alerts.
#[test]
fn the_third_pay_out_through_the_real_till_alerts_and_the_second_does_not() {
    use crate::command::till::{decide, periods, Cmd, MoveIn, OpenIn};
    let mut log: Vec<Entry> = Vec::new();
    let mut run = |cmd: Cmd| {
        let (kind, subject, rec) = decide(&periods(&log).unwrap(), &[], &cmd).expect("the till takes it");
        log.push(Entry { kind: kind.into(), subject, json: rec.to_string(), seq: log.len() as u64 });
        let rows = till_rows(&log, &periods(&log).unwrap());
        let now = match &cmd { Cmd::PayOut(i) => i.now_ms, _ => T0 };
        due(&rows, T0, now, 3, &voice("en"), "chat-1")
    };
    run(Cmd::Open(OpenIn { location_id: "v1".into(), till_id: "t1".into(), float: [("ALL".to_string(), 9000)].into(), by: "dan".into(), now_ms: T0 }));
    let out = |n: i64| Cmd::PayOut(MoveIn {
        location_id: "v1".into(), till_id: "t1".into(), currency: "ALL".into(), amount: 500 + n,
        reason: "supplier".into(), source: None, by: format!("s{n}"), now_ms: T0 + n * MIN,
    });
    assert!(run(out(1)).is_empty());
    assert!(run(out(2)).is_empty(), "two pay-outs are below the threshold");
    let got = run(out(3));
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].id, format!("exceptions/{T0}/{PAY_OUT_KIND}/3"));
    for by in ["by s1", "by s2", "by s3", "503 ALL"] {
        assert!(got[0].text.contains(by), "{}", got[0].text);
    }
}
