//! W-O2: a courier's tap whose reply was lost after the object wrote runs
//! ONCE and the retry is given the first answer -- per route, through the real
//! table operations the Worker (`claim`, `release`) and the object (`commit`)
//! run, and the real answer and ops functions the handlers are made of.

use super::{accept_answer, committed, deliver_answer, refused_answer, settle_delivery, stamp_pickup};
use crate::command::advance::AdvanceOut;
use crate::command::refund::RefundOut;
use crate::courier::cash::Handover;
use crate::courier::{K_ASG, K_SHIFT};
use crate::idempotency::commit::{commit, Claim, Marked};
use crate::idempotency::verdict::{claim, release, Seen};
use crate::idempotency::IDEMPOTENCY_BYTES;
use dowiz_hub::table::Table;
use serde_json::{json, Value};

const T0: i64 = 1_790_000_000_000;

/// The whole lost-reply sequence: the Worker claims, the object writes and
/// marks the claim with `output`, the reply is lost (the Worker releases), the
/// retry claims again. Returns what the retry is told.
fn lost_reply(output: &str) -> Seen {
    let mut t = Table::create(IDEMPOTENCY_BYTES).unwrap();
    assert_eq!(claim(&mut t, "k", "p", T0).unwrap(), Seen::Claimed);
    assert!(commit(&mut t, &Claim { key: "k".into(), print: "p".into() }, output).unwrap());
    release(&mut t, "k");
    claim(&mut t, "k", "p", T0 + 5_000).unwrap()
}

fn output_of(seen: Seen) -> String {
    match seen {
        Seen::Committed { output } => output,
        other => panic!("the retry must be told the tap ran, got {other:?}"),
    }
}

fn ops(asg: Value, shift: Option<Value>) -> Table {
    let mut t = Table::create(64 * 1024).unwrap();
    t.put(K_ASG, "ord_1", &asg.to_string(), &[], &[]).unwrap();
    if let Some(s) = shift {
        t.put(K_SHIFT, "c1", &s.to_string(), &[], &[]).unwrap();
    }
    t
}

fn row(t: &Table, kind: &str, id: &str) -> Value {
    serde_json::from_str(&t.get(kind, id).unwrap()).unwrap()
}

/// ACCEPT. The retry is handed the first answer and never reaches `run::claim`
/// or the `Noted` append (the wiring test below holds that order).
#[test]
fn accept_a_lost_reply_retry_answers_the_first_answer() {
    let first = accept_answer("ord_1", 1200);
    let again: Value = committed(Some(&output_of(lost_reply(&first.to_string())))).unwrap();
    assert_eq!(again, first);
}

/// PICKUP. The retry's stamp does not move the first one.
#[test]
fn pickup_a_retry_keeps_the_first_pickup_stamp() {
    let mut t = ops(json!({"courier_id": "c1", "picked_up_at_ms": null}), None);
    assert!(stamp_pickup(&mut t, "ord_1", T0).unwrap());
    assert!(!stamp_pickup(&mut t, "ord_1", T0 + 5_000).unwrap(), "the retry writes nothing");
    assert_eq!(row(&t, K_ASG, "ord_1")["picked_up_at_ms"], json!(T0));
}

/// The twin: an order with no stamp is stamped, and one with no row is not invented.
#[test]
fn pickup_an_unstamped_run_is_stamped_and_a_missing_row_is_not_invented() {
    let mut t = ops(json!({"courier_id": "c1"}), None);
    assert!(stamp_pickup(&mut t, "ord_1", T0).unwrap());
    assert_eq!(row(&t, K_ASG, "ord_1")["picked_up_at_ms"], json!(T0));
    assert!(!stamp_pickup(&mut t, "ord_2", T0).unwrap());
    assert!(t.get(K_ASG, "ord_2").is_none());
}

#[test]
fn pickup_a_lost_reply_retry_answers_the_first_order() {
    let first = AdvanceOut { merged: json!({"id": "ord_1", "status": "IN_DELIVERY"}).to_string(), generation: 9 };
    let again: AdvanceOut = committed(Some(&output_of(lost_reply(&serde_json::to_string(&first).unwrap())))).unwrap();
    assert_eq!(again.merged, first.merged);
}

/// DELIVER. The first call wrote the log and the ops; its reply was lost. The
/// retry answers from the committed output -- shortfall and all -- and the
/// shift counts ONE delivery and the cash once.
#[test]
fn deliver_a_lost_reply_retry_counts_one_delivery_and_answers_the_shortfall() {
    let shift = json!({"deliveries": 3, "cash_collected": 5000, "ended_at_ms": null});
    let mut t = ops(json!({"courier_id": "c1", "cash_due": 1200, "delivered_at_ms": null}), Some(shift));
    let handed = Handover { collected: 1000, short: 200 };
    let first = deliver_answer(&json!({"id": "ord_1", "status": "DELIVERED"}), 1200, &handed);
    assert!(settle_delivery(&mut t, "ord_1", "c1", T0, handed.collected).unwrap());

    let again: Value = committed(Some(&output_of(lost_reply(&first.to_string())))).unwrap();
    assert_eq!(again, first, "the first answer, shortfall and all");
    let collected = again["cashCollected"].as_i64().unwrap();
    assert!(!settle_delivery(&mut t, "ord_1", "c1", T0 + 5_000, collected).unwrap(), "the retry writes nothing");

    let s = row(&t, K_SHIFT, "c1");
    assert_eq!((s["deliveries"].clone(), s["cash_collected"].clone()), (json!(4), json!(6000)));
    assert_eq!(row(&t, K_ASG, "ord_1")["delivered_at_ms"], json!(T0));
}

/// The twin: the first call wrote the log and was cut off BEFORE the ops
/// write. The retry finishes it, once.
#[test]
fn deliver_a_retry_after_a_cut_before_the_ops_write_finishes_it_once() {
    let shift = json!({"deliveries": 0, "cash_collected": 0, "ended_at_ms": null});
    let mut t = ops(json!({"courier_id": "c1", "cash_due": 800, "delivered_at_ms": null}), Some(shift));
    assert!(settle_delivery(&mut t, "ord_1", "c1", T0 + 5_000, 800).unwrap());
    let s = row(&t, K_SHIFT, "c1");
    assert_eq!((s["deliveries"].clone(), s["cash_collected"].clone()), (json!(1), json!(800)));
    assert_eq!(row(&t, K_ASG, "ord_1")["cash_collected"], json!(800));
}

/// REFUSED AT THE DOOR. The retry is given the refund the object wrote.
#[test]
fn refused_a_lost_reply_retry_answers_the_first_refund() {
    let out = RefundOut { merged: json!({"id": "ord_1", "status": "REFUNDING"}).to_string(), seq: 42, generation: 7 };
    let first = refused_answer(&out);
    let again: RefundOut = committed(Some(&output_of(lost_reply(&serde_json::to_string(&out).unwrap())))).unwrap();
    assert_eq!(refused_answer(&again), first);
    assert_eq!(first["seq"], json!(42));
}

/// No output, or one that does not read, is "run it": the behaviour before.
#[test]
fn no_committed_output_means_the_route_runs() {
    assert_eq!(committed::<Value>(None), None);
    assert_eq!(committed::<AdvanceOut>(Some("{\"nope\":1}")).map(|o| o.generation), None);
}

/// THE APPEND'S MARK travels beside the event and is absent without a key.
#[test]
fn the_append_mark_travels_beside_the_event() {
    let c = Claim { key: "k".into(), print: "p".into() };
    let m = Marked { claim: c.clone(), output: "{}".into() };
    let back: Marked = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
    assert_eq!((back.claim, back.output.as_str()), (c, "{}"));
}

fn body_of<'a>(src: &'a str, head: &str) -> &'a str {
    let at = &src[src.find(head).unwrap_or_else(|| panic!("{head}"))..];
    &at[..at.find("\n}\n").or_else(|| at.find("\n    }\n")).expect("end of fn")]
}

fn before(body: &str, a: &str, b: &str) {
    let (i, j) = (body.find(a).unwrap_or_else(|| panic!("missing {a}")), body.find(b).unwrap_or_else(|| panic!("missing {b}")));
    assert!(i < j, "{a} must come before {b}");
}

/// THE WIRING, read from the source (the object's turn has no native harness):
/// each courier route answers a committed output before it checks or writes
/// anything, and hands its claim to the write that decides.
#[test]
fn every_courier_tap_reads_its_committed_output_first_and_sends_its_claim() {
    let src = include_str!("../../courier.rs");
    let accept = body_of(src, "pub async fn accept(");
    before(accept, "idem.committed()", "with_ops(");
    before(accept, "idem.claim()", "append_claimed(");
    let pickup = body_of(src, "pub async fn pickup(");
    before(pickup, "idem.committed()", "asg_of(");
    assert!(pickup.contains("Claimed { input: &input, idem: claim"), "pickup sends its claim");
    assert!(pickup.contains("stamp_pickup("));
    let deliver = body_of(src, "pub async fn deliver(");
    before(deliver, "idem.committed()", "asg_of(");
    assert!(deliver.contains("settle_delivery("));
    let write = body_of(src, "async fn write_status_with(");
    assert!(write.contains("append_claimed("), "the delivery's log write carries the claim");
    let door = include_str!("../door.rs");
    before(door, "idem.committed()", "may_refuse(");
    assert!(door.contains("Claimed { input: &input, idem: claim"), "refused sends its claim");
}

/// ...and the object marks each claim in the turn that wrote the log, before
/// anyone is told.
#[test]
fn the_object_marks_advance_refund_and_append_after_the_log_write() {
    let hub = include_str!("../../hubdo.rs");
    let adv = body_of(hub, "    async fn advance(");
    before(adv, "\"the log generation moved during a transition\"", "self.commit_claim(claim.as_ref()");
    before(adv, "self.commit_claim(claim.as_ref()", "self.broadcast(");
    assert!(hub.contains("Claimed { input, idem } = req.json().await?;\n                    match self.advance(input, idem)"));
    assert!(hub.contains("Claimed { input, idem } = req.json().await?;\n                    match self.refund(input, idem)"));
    let arm = &hub[hub.find("(Method::Post, \"append\")").unwrap()..];
    before(arm, "Some((generation, len)) =>", "self.commit_claim(Some(&m.claim)");
    let refund = include_str!("../../hubdo/refund.rs");
    let refund = body_of(refund, "    pub(super) async fn refund(");
    before(refund, "self.write_both(", "self.commit_claim(claim.as_ref()");
    before(refund, "self.commit_claim(claim.as_ref()", "self.broadcast(");
}
