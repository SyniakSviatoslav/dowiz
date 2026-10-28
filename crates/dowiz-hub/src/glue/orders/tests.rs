//! S2b: the op codec, the tablet's rail, the hub's rebase and its law, the held notes.

use super::*;
use crate::glue::desk;
use crate::EventKind;

const V: &str = "v1";

fn placed(h: &mut Hub, id: &str, status: &str, loc: &str, seq: u64) {
    let o = json!({"id": id, "status": status, "location_id": loc, "name": "Guest Name"}).to_string();
    h.append(EventKind::Placed, id, &o, seq, [0; 32]).unwrap();
}

fn hub() -> Hub {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    placed(&mut h, "o1", "CONFIRMED", V, 1);
    placed(&mut h, "o2", "PENDING", V, 2);
    placed(&mut h, "o3", "PENDING", V, 3);
    placed(&mut h, "x9", "CONFIRMED", "v2", 4);
    h
}

/// The test's stand-in for the venue object's deciders: one delta per op, as `command::advance`
/// and `command::kitchen_ack` write them (the Worker's own test drives the real ones).
fn write(h: &mut Hub, op: &Op) -> Result<(), String> {
    let seq = 100 + h.events().len() as u64;
    let (kind, body) = match &op.kind {
        Kind::Ack => (EventKind::Noted, json!({"_d": true, "kitchen": {"seen": {"at": op.at}}})),
        Kind::Advance { next, .. } => (EventKind::Advanced, json!({"_d": true, "status": next})),
    };
    h.append(kind, &op.order, &body.to_string(), seq, [0; 32]).map(|_| ()).map_err(|e| format!("{e:?}"))
}

fn adv(oid: &str, order: &str, from: &str, next: &str) -> Op {
    Op { oid: oid.into(), order: order.into(), kind: Kind::Advance { from: from.into(), next: next.into() }, at: 5 }
}

fn ack(oid: &str, order: &str) -> Op {
    Op { oid: oid.into(), order: order.into(), kind: Kind::Ack, at: 5 }
}

#[test]
fn an_op_line_reads_back_and_a_foreign_key_is_tracking() {
    for op in [adv("5.0", "o1", "CONFIRMED", "PREPARING"), ack("5.1", "o2")] {
        assert_eq!(Op::parse(&op.line(), 0), Ok(op.clone()), "round trip");
    }
    let spy = r#"{"op":"ack","oid":"5.1","order":"o2","at":5,"phone":"+355"}"#;
    assert_eq!(Op::parse(spy, 3), Err(Bad::Tracking { pos: 3, key: "phone".into() }));
    assert_eq!(Op::parse(r#"{"op":"advance","oid":"1","order":"o1","from":"READY","next":"CANCELLED","at":5}"#, 1), Err(Bad::NotAnOp { pos: 1 }), "cancel stays online");
    assert_eq!(Op::parse(r#"{"op":"ack","oid":"1","order":"o 1","at":5}"#, 0), Err(Bad::NotAnOp { pos: 0 }));
    assert_eq!(Op::parse(&"x".repeat(REC_MAX + 1), 2), Err(Bad::TooLong { pos: 2 }));
    let many = vec![ack("1", "o1").line(); TAIL_MAX + 1];
    assert!(matches!(read_tail(&many), Err(Bad::TooLong { .. })));
    assert_eq!(read_tail(&many[..2]).unwrap().len(), 2, "the twin");
}

fn rail() -> Rail {
    let board = json!([{"id": "o1", "status": "CONFIRMED"}, {"id": "o2", "status": "PENDING", "kitchen": {"seen": {"at": 1}}}]);
    Rail { tickets: Rail::tickets_of(&board), ..Rail::default() }
}

#[test]
fn the_tablet_decides_by_the_order_machine_before_queueing() {
    let mut r = rail();
    assert!(r.record("o1", Some("PREPARING"), 10).unwrap().is_some());
    assert_eq!(r.tickets[0].status, "PREPARING", "the ticket moves on screen at once");
    let e = r.record("o1", Some("CONFIRMED"), 11).unwrap_err();
    assert!(matches!(e, Local::Illegal { code: "IllegalTransitionError", .. }), "{e:?}");
    assert_eq!(r.record("o2", None, 12), Ok(None), "seen already: nothing queued");
    assert_eq!(r.record("o9", None, 12), Err(Local::NotOnRail));
    assert_eq!(r.record("o1", Some("CANCELLED"), 12), Err(Local::NotABump("CANCELLED".into())));
    assert!(r.record("o1", Some("READY"), 13).unwrap().is_some(), "the next bump is from what the tablet now shows");
    assert_eq!(r.queue.len(), 2);
    let ops = read_tail(&r.queue).unwrap();
    assert_eq!(ops[1].kind, Kind::Advance { from: "PREPARING".into(), next: "READY".into() });
    assert_ne!(ops[0].oid, ops[1].oid);
}

#[test]
fn a_refresh_drops_what_was_sent_and_lays_the_rest_over_the_hubs_rail() {
    let mut r = rail();
    r.record("o1", Some("PREPARING"), 10).unwrap();
    r.record("o1", Some("READY"), 11).unwrap();
    let fresh = Rail::tickets_of(&json!([{"id": "o1", "status": "PREPARING"}]));
    r.refresh(fresh, 1);
    assert_eq!(r.queue.len(), 1);
    assert_eq!(r.tickets, vec![Ticket { id: "o1".into(), status: "READY".into(), seen: false }]);
}

#[test]
fn the_rebase_keeps_the_law_and_a_retried_tail_writes_nothing() {
    let mut h = hub();
    let ops = vec![
        ack("1", "o1"),
        adv("2", "o1", "CONFIRMED", "PREPARING"),
        adv("3", "o1", "PREPARING", "READY"),
        adv("4", "o2", "PENDING", "CONFIRMED"),
        adv("5", "o3", "CONFIRMED", "PREPARING"), // the tablet's rail was wrong: o3 is PENDING
        adv("6", "x9", "CONFIRMED", "PREPARING"), // another venue's order
        adv("7", "zz", "CONFIRMED", "PREPARING"),
    ];
    let a = rebase(&mut h, V, &ops, write);
    assert!(a.law(ops.len()), "{a:?}");
    assert_eq!((a.merged, a.applied), (4, 4));
    assert_eq!(a.held.iter().map(|(p, _)| *p).collect::<Vec<_>>(), vec![4, 5, 6]);
    assert!(a.held[0].1.starts_with("IllegalTransitionError"), "{}", a.held[0].1);
    assert_eq!((a.held[1].1.as_str(), a.held[2].1.as_str()), ("order not found", "order not found"), "never says it exists elsewhere");
    assert_eq!(history(&h, "o1").unwrap().1, vec!["CONFIRMED", "PREPARING", "READY"]);
    let n = h.events().len();
    let b = rebase(&mut h, V, &ops, write);
    assert!(b.law(ops.len()));
    assert_eq!((b.merged, b.applied, b.held.len()), (4, 0, 3), "the same tail twice is adopted once");
    assert_eq!(h.events().len(), n, "nothing written on the replay");
}

#[test]
fn a_decider_refusal_is_held_with_its_words() {
    let mut h = hub();
    let ops = vec![adv("1", "o1", "CONFIRMED", "PREPARING")];
    let r = rebase(&mut h, V, &ops, |_, _| Err("rice: 2 wanted, 0 available".into()));
    assert_eq!(r.held, vec![(0, "rice: 2 wanted, 0 available".to_string())]);
    assert!(r.law(1));
}

#[test]
fn a_stale_tap_whose_edge_is_legal_is_held_not_applied() {
    let mut h = hub();
    // The tablet saw o2 READY (it was not) and pressed... CONFIRMED is legal from PENDING.
    let ops = vec![adv("1", "o2", "READY", "CONFIRMED")];
    let r = rebase(&mut h, V, &ops, write);
    assert!(r.held[0].1.starts_with("stale"), "{:?}", r.held);
    let twin = rebase(&mut h, V, &[adv("2", "o2", "PENDING", "CONFIRMED")], write);
    assert_eq!(twin.applied, 1, "the twin");
}

#[test]
fn held_ops_land_on_the_decision_strip_once() {
    let mut h = hub();
    let ops = vec![adv("1", "o3", "CONFIRMED", "PREPARING"), ack("2", "o1")];
    let r = rebase(&mut h, V, &ops, write);
    let mut stock = StockLog::create_sized(64 * 1024).unwrap();
    assert_eq!(hold_notes(&mut stock, "kitchen", &ops, &r.held).unwrap(), 1);
    assert_eq!(hold_notes(&mut stock, "kitchen", &ops, &r.held).unwrap(), 0, "a retried tail adds no second note");
    let open = desk::open(&stock);
    assert_eq!(open.len(), 1);
    let d = &open[0];
    assert_eq!((d.did.as_str(), d.kind.as_str(), d.order.as_str(), d.item.as_str()), ("o-kitchen-1", "held", "o3", "o3 PREPARING"));
    assert_eq!(Op::parse(&d.rec, 0).unwrap(), ops[0], "the op is kept verbatim");
    assert!(!d.rec.contains("Guest"), "no guest data rides in the note");
}

/// The orders the hub cancels while the tablet is offline (none it also confirmed).
fn cancelled() -> impl Iterator<Item = u64> {
    (3..40u64).step_by(7).filter(|i| i % 5 != 0)
}

/// AC-LV-LOC-02/03 shape: a day without a link -- 40 tickets, ~160 taps in the tablet's order,
/// deterministic (no fuzzer) -- every tap merged or held, and the wire stays small.
#[test]
fn a_day_offline_loses_no_tap() {
    let mut h = Hub::create_sized(256 * 1024).unwrap();
    let mut r = Rail::default();
    for i in 0..40u64 {
        let id = format!("d{i}");
        placed(&mut h, &id, "PENDING", V, i + 1);
        r.tickets.push(Ticket { id, status: "PENDING".into(), seen: false });
    }
    // The hub moves every fifth order itself while the tablet is offline; a seventh is cancelled.
    for i in (0..40).step_by(5) {
        write(&mut h, &adv("h", &format!("d{i}"), "PENDING", "CONFIRMED")).unwrap();
    }
    for i in cancelled() {
        let o = format!("d{i}");
        h.append(EventKind::Advanced, &o, r#"{"_d":true,"status":"CANCELLED"}"#, 900 + i, [0; 32]).unwrap();
    }
    for (t, i) in (0..40).enumerate() {
        let id = format!("d{i}");
        r.record(&id, None, 1000 + t as i64).unwrap();
        for next in ["CONFIRMED", "PREPARING", "READY"] {
            r.record(&id, Some(next), 2000 + t as i64).unwrap();
        }
    }
    let ops = read_tail(&r.queue).unwrap();
    assert_eq!(ops.len(), 160);
    let out = rebase(&mut h, V, &ops, write);
    assert!(out.law(ops.len()), "{out:?}");
    assert_eq!(out.held.len(), cancelled().count() * 3, "a cancelled order's three bumps are held; its ack is applied");
    let wire: usize = r.queue.iter().map(String::len).sum();
    assert!(wire < 100 * 1024, "a day's taps on the wire: {wire} B");
}
