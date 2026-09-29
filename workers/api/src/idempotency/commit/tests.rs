//! W-FIX O2 (W-AUDIT O2): a command that RAN keeps its key through the failure
//! that follows it, through the real table operations the Worker (`claim`,
//! `release`, `record`) and the object (`commit`) run.

use super::{commit, stored_id, Claim, Claimed};
use crate::idempotency::verdict::{claim, record, release, Seen, LEASE_MS};
use crate::idempotency::IDEMPOTENCY_BYTES;
use dowiz_hub::table::Table;
use serde_json::{json, Value};

const T0: i64 = 1_790_000_000_000;
const OUT: &str = r#"{"stored":"{\"id\":\"ord_1\"}","generation":7,"events":3}"#;

fn claimed() -> (Table, Claim) {
    let mut t = Table::create(IDEMPOTENCY_BYTES).unwrap();
    assert_eq!(claim(&mut t, "k", "p", T0).unwrap(), Seen::Claimed);
    (t, Claim { key: "k".into(), print: "p".into() })
}

/// THE DEFECT. The object placed the order, the reply was lost, the Worker saw
/// a 503 and released the key: the retry was a second order. A committed claim
/// is not given back, and the retry -- inside the lease or long after it -- is
/// handed the order that exists.
#[test]
fn a_committed_claim_survives_the_release_and_the_retry_is_given_the_order() {
    let (mut t, c) = claimed();
    assert!(commit(&mut t, &c, OUT).unwrap());
    release(&mut t, "k");
    assert_eq!(claim(&mut t, "k", "p", T0 + 1).unwrap(), Seen::Committed { output: OUT.into() });
    assert_eq!(claim(&mut t, "k", "p", T0 + LEASE_MS * 10).unwrap(), Seen::Committed { output: OUT.into() });
}

/// The twin: a claim whose command never ran is still given back, and the retry runs.
#[test]
fn an_uncommitted_claim_is_released_and_the_retry_runs() {
    let (mut t, _) = claimed();
    release(&mut t, "k");
    assert_eq!(claim(&mut t, "k", "p", T0 + 1).unwrap(), Seen::Claimed);
}

/// The Worker's own answer replaces the mark, and is what every later retry gets.
#[test]
fn the_recorded_answer_replaces_the_committed_mark() {
    let (mut t, c) = claimed();
    assert!(commit(&mut t, &c, OUT).unwrap());
    record(&mut t, "k", "p", T0, 200, "{\"id\":\"ord_1\",\"token\":\"t\"}", "application/json").unwrap();
    let again = claim(&mut t, "k", "p", T0 + 1).unwrap();
    assert!(matches!(again, Seen::Answered { status: 200, .. }), "{again:?}");
    assert!(!commit(&mut t, &c, OUT).unwrap(), "an answer is never overwritten by a mark");
    assert!(matches!(claim(&mut t, "k", "p", T0 + 2).unwrap(), Seen::Answered { .. }));
}

/// Nothing is marked for a key that is absent or that another body holds.
#[test]
fn a_claim_that_is_not_this_ones_is_not_marked() {
    let mut t = Table::create(IDEMPOTENCY_BYTES).unwrap();
    let c = Claim { key: "k".into(), print: "p".into() };
    assert!(!commit(&mut t, &c, OUT).unwrap(), "absent");
    assert_eq!(claim(&mut t, "k", "other", T0).unwrap(), Seen::Claimed, "the absent commit wrote nothing");
    assert!(!commit(&mut t, &c, OUT).unwrap(), "another body's claim");
    release(&mut t, "k");
    assert_eq!(claim(&mut t, "k", "other", T0 + 1).unwrap(), Seen::Claimed, "it was never marked");
}

/// THE WIRE. `Claimed` adds one field beside the command's own, and without a
/// key it is byte-for-byte the command: an object and a Worker on either side
/// of a deploy still read each other.
#[test]
fn the_claim_travels_beside_the_command_and_is_absent_without_a_key() {
    let input = json!({"order_id": "ord_1", "seq": 5});
    let bare = serde_json::to_value(Claimed { input: &input, idem: None }).unwrap();
    assert_eq!(bare, input);
    let c = Claim { key: "k".into(), print: "p".into() };
    let with = serde_json::to_string(&Claimed { input: &input, idem: Some(c.clone()) }).unwrap();
    let back: Claimed<Value> = serde_json::from_str(&with).unwrap();
    assert_eq!(back.idem, Some(c));
    assert_eq!(back.input["order_id"], "ord_1");
    let old: Claimed<Value> = serde_json::from_str(r#"{"order_id":"ord_1"}"#).unwrap();
    assert_eq!(old.idem, None, "a Worker that sends no claim");
}

#[test]
fn the_stored_envelope_names_the_order() {
    assert_eq!(stored_id(r#"{"id":"ord_1","total":1200}"#).as_deref(), Some("ord_1"));
    assert_eq!(stored_id(r#"{"total":1200}"#), None);
    assert_eq!(stored_id("not json"), None);
}

/// THE WIRING, read from the source (the object's turn has no native harness):
/// the placement marks its claim after the log write and before it answers,
/// and the storefront asks for a committed output before it sends the command.
#[test]
fn the_placement_marks_its_claim_and_the_storefront_reads_it_first() {
    let hub = include_str!("../../hubdo.rs");
    let place = &hub[hub.find("async fn place(").expect("hubdo::place")..];
    let place = &place[..place.find("\n    }\n").expect("end of place")];
    let mark = place.find("self.commit_claim(claim.as_ref()").expect("place marks its claim");
    assert!(place.find("\"the log generation moved during a placement\"").unwrap() < mark, "after the log write");
    assert!(mark < place.find("self.broadcast(").expect("the broadcast"), "before anyone is told");
    assert!(hub.contains("Claimed { input, idem } = crate::body::parse(&mut req).await?"), "the object reads the claim");
    let front = include_str!("../../storefront.rs");
    let read = front.find("idem.committed()").expect("the storefront reads a committed output");
    let send = front.find("idem: idem.claim()").expect("the storefront sends its claim");
    assert!(read < send, "a committed retry sends nothing");
}
