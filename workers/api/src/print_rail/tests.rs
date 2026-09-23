//! The print rail's rules (LAST-MILE §3.1 steps 1 and 3), each refusal beside
//! the positive twin that proves it is not the only thing the rule can do.

use super::*;
use crate::outbox::MAX_TRIES;

const NOW: i64 = 1_790_000_000_000;

fn job(order: &str, queued: i64) -> Entry {
    entry_for("kitchen", order, &format!("ticket {order}"), queued).unwrap()
}

fn ack(order: &str, code: &str) -> AckIn {
    AckIn { token: token_of(&format!("{order}/print")), code: code.into(), now_ms: NOW }
}

fn order(id: &str) -> Value {
    json!({ "id": id, "status": "CONFIRMED", "location_id": "v1", "created_at_ms": NOW - 5_000 })
}

#[test]
fn a_venue_with_a_printer_queues_a_ticket_and_one_without_queues_nothing() {
    let e = entry_for(" kitchen ", "o1", "2x maki", NOW).expect("a printer is set");
    assert_eq!((e.id.as_str(), e.kind.as_str(), e.to.as_str(), e.text.as_str()), ("o1/print", KIND, "kitchen", "2x maki"));
    assert_eq!((e.handed_ms, e.code.clone()), (None, None));
    assert!(entry_for("", "o1", "t", NOW).is_none());
    assert!(entry_for("   ", "o1", "t", NOW).is_none());
}

#[test]
fn an_entry_written_before_the_rail_existed_is_byte_identical() {
    let s = serde_json::to_string(&Entry::new("o1/telegram".into(), "telegram", "c".into(), "t".into(), 1)).unwrap();
    assert!(!s.contains("handed_ms") && !s.contains("\"code\""), "{s}");
    let back: Entry = serde_json::from_str(&s).unwrap();
    assert_eq!(back.handed_ms, None, "and an old record still reads");
}

#[test]
fn a_token_names_exactly_one_print_job() {
    let t = token_of("o1/print");
    assert_eq!(id_of(&t).as_deref(), Some("o1/print"));
    assert_eq!(order_of("o1/print"), "o1");
    assert_eq!(id_of(&token_of("o1/telegram")), None, "a token cannot reach another kind's entry");
    assert_eq!(id_of("!!not base64"), None);
}

#[test]
fn the_poll_hands_out_the_oldest_due_job_once_per_lease() {
    let (a, b) = (job("a", NOW - 2_000), job("b", NOW - 1_000));
    let mut later = job("c", NOW - 3_000);
    later.next_at_ms = NOW + 10_000; // waiting out a backoff
    let mut tg = job("t", NOW - 9_000);
    tg.kind = "telegram".into();
    let got = decide_poll(&[b.clone(), a.clone(), later, tg], NOW).expect("a job is ready");
    assert_eq!((got.id.as_str(), got.handed_ms), ("a/print", Some(NOW)), "oldest due print job, marked printing");
    // A job the printer holds is not handed out again inside the lease...
    assert_eq!(decide_poll(&[got.clone()], NOW + LEASE_MS - 1), None);
    // ...and IS once the lease has run out (the printer lost it mid-job).
    assert!(decide_poll(&[got], NOW + LEASE_MS).is_some());
    assert_eq!(decide_poll(&[], NOW), None);
}

#[test]
fn a_2xx_code_is_printed_and_anything_else_is_not() {
    for c in ["200 OK", "200", " 211 Printed with warning"] {
        assert!(printed_ok(c), "{c}");
    }
    for c in ["410 Paper empty", "500", "", "OK", "20"] {
        assert!(!printed_ok(c), "{c:?}");
    }
}

#[test]
fn only_a_venue_key_signs_a_printer_in() {
    use base64::engine::general_purpose::STANDARD as B;
    assert_eq!(key_of("Bearer dowiz_k1.s").as_deref(), Some("dowiz_k1.s"));
    assert_eq!(key_of(&format!("Basic {}", B.encode("printer:dowiz_k1.s"))).as_deref(), Some("dowiz_k1.s"));
    assert_eq!(key_of(&format!("Basic {}", B.encode("dowiz_k1.s:"))).as_deref(), Some("dowiz_k1.s"));
    assert_eq!(key_of("Bearer eyJhbGciOi.owner.jwt"), None, "a session token is not a printer key");
    assert_eq!(key_of(&format!("Basic {}", B.encode("admin:hunter2"))), None);
    assert_eq!(key_of("Basic !!!"), None);
    assert_eq!(key_of(""), None);
}

#[test]
fn a_printed_delete_notes_the_order_and_removes_the_job() {
    let e = job("o1", NOW - 5_000);
    let o = order("o1");
    let plan = decide_ack(Some(&e), Some((&o, 7)), &ack("o1", "200 OK")).expect("a real job");
    let (oid, body, seq) = plan.note.expect("printed is a record");
    assert_eq!(oid, "o1");
    assert!(body.contains("\"printed\"") && body.contains("200 OK") && body.contains("\"_d\":true"), "{body}");
    assert!(seq > 7);
    assert_eq!(plan.change, Change::Remove);
    assert_eq!((plan.out.state, plan.out.abandoned, plan.out.replay), (State::Printed, false, false));
}

#[test]
fn a_failed_print_is_retried_and_the_sixth_is_given_up_on_out_loud() {
    let mut e = job("o1", NOW - 5_000);
    e.handed_ms = Some(NOW - 1_000);
    let o = order("o1");
    let plan = decide_ack(Some(&e), Some((&o, 7)), &ack("o1", "410 Paper empty")).unwrap();
    assert!(plan.note.is_none(), "a retry writes nothing to the order");
    let Change::Put(n) = plan.change else { panic!("a retry keeps the job") };
    assert_eq!((n.tries, n.handed_ms, n.code.as_deref()), (1, None, Some("410 Paper empty")));
    assert!(n.next_at_ms > NOW, "and waits out the backoff");
    assert_eq!(plan.out.state, State::Queued);

    e.tries = MAX_TRIES - 1;
    let plan = decide_ack(Some(&e), Some((&o, 7)), &ack("o1", "410 Paper empty")).unwrap();
    assert!(plan.note.as_ref().is_some_and(|n| n.1.contains("print_failed")), "failed is a record");
    assert_eq!(plan.change, Change::Remove);
    assert_eq!((plan.out.state, plan.out.abandoned, plan.out.tries), (State::Failed, true, MAX_TRIES));
}

#[test]
fn a_repeated_delete_is_answered_from_the_record_and_writes_nothing() {
    let (mut o, _) = noted(&order("o1"), "printed", json!({"at": NOW - 1, "printer": "kitchen", "code": "200"}));
    let plan = decide_ack(None, Some((&o, 9)), &ack("o1", "200 OK")).expect("the record answers");
    assert!(plan.note.is_none());
    assert_eq!(plan.change, Change::Keep);
    assert_eq!((plan.out.state, plan.out.replay), (State::Printed, true));
    // The log write landed and the outbox write did not: the entry is back,
    // and the second DELETE removes it WITHOUT stacking a second note.
    let plan = decide_ack(Some(&job("o1", NOW - 5_000)), Some((&o, 9)), &ack("o1", "200 OK")).unwrap();
    assert!(plan.note.is_none());
    assert_eq!(plan.change, Change::Remove);
    // Twin: no entry and no record is no such job.
    o["kitchen"] = json!({});
    assert!(decide_ack(None, Some((&o, 9)), &ack("o1", "200 OK")).is_none());
    assert!(decide_ack(None, None, &ack("o1", "200 OK")).is_none());
    assert!(decide_ack(None, None, &AckIn { token: "junk".into(), code: "200".into(), now_ms: NOW }).is_none());
}

#[test]
fn a_ticket_has_four_states_and_each_is_a_record() {
    let o = order("o1");
    let mut e = job("o1", NOW - 5_000);
    assert_eq!(state_of(Some(&e), &o, NOW), Some(State::Queued));
    e.handed_ms = Some(NOW - 1_000);
    assert_eq!(state_of(Some(&e), &o, NOW), Some(State::Printing));
    assert_eq!(state_of(Some(&e), &o, NOW + LEASE_MS), Some(State::Queued), "a lapsed lease is queued again");
    let printed = noted(&o, "printed", json!({"at": NOW})).0;
    assert_eq!(state_of(None, &printed, NOW), Some(State::Printed));
    let failed = noted(&o, "print_failed", json!({"at": NOW, "code": "410", "tries": 6})).0;
    assert_eq!(state_of(None, &failed, NOW), Some(State::Failed));
    assert_eq!(state_of(None, &o, NOW), None, "no printer, no ticket");
}

/// THROUGH THE REAL FOLD: the note lands on the order every reader sees, and
/// the kitchen's "seen" beside it survives.
#[test]
fn the_printed_note_folds_onto_the_order_and_keeps_the_seen() {
    let mut hub = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
    let mut o = order("o1");
    o["kitchen"] = json!({"seen": {"by": "cook1", "at": NOW - 100}});
    hub.append(dowiz_hub::EventKind::Placed, "o1", &o.to_string(), 1, [0u8; 32]).unwrap();
    let plan = decide_ack(Some(&job("o1", NOW - 5_000)), Some((&o, 1)), &ack("o1", "200 OK")).unwrap();
    let (oid, body, seq) = plan.note.unwrap();
    hub.append(dowiz_hub::EventKind::Noted, &oid, &body, seq, [0u8; 32]).unwrap();
    let folded = crate::hubstore::orders_state(&hub).into_iter().next().unwrap();
    let v: Value = serde_json::from_str(&folded.order_json).unwrap();
    assert_eq!(v["kitchen"]["printed"]["printer"], json!("kitchen"));
    assert_eq!(v["kitchen"]["seen"]["by"], json!("cook1"));
}
