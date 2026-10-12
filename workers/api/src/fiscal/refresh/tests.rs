//! A QUEUED DOCUMENT FOLLOWS ITS ORDER UNTIL IT IS SENT: a tip at pay time and
//! a comp are in the document the firing sends; one document per order; a
//! document the platform may hold is never rewritten. Each rule with its twin.

use super::*;
use crate::fiscal::ebills_sender::tests::order;
use crate::fiscal::queue::entry;
use serde_json::json;

const T: i64 = 1_790_000_000_000;

fn queued(o: &Value) -> Entry {
    entry(&document(o, "ALL", T).unwrap())
}

fn doc_of(e: &Entry) -> Document {
    serde_json::from_str(&e.text).unwrap()
}

fn tipped(mut o: Value, tip: i64) -> Value {
    o["tip"] = json!(tip);
    o["total"] = json!(200 + tip);
    o["payments"][0]["amount"] = json!(200 + tip);
    o
}

fn intent(e: &Entry, stage: Stage) -> Intent {
    Intent { uuid: e.id.clone(), order_id: e.to.clone(), stage, at_ms: T, unknowns: 0, sale_id: None, why: None, prev: None }
}

/// A TIP ADDED AT PAY TIME is in the document; the entry keeps its id, its
/// issue instant (so its deadline) and its tries. Twin: an order that did not
/// move rewrites nothing.
#[test]
fn a_tip_after_placement_is_in_the_queued_document() {
    let placed = order("o1");
    let mut e = queued(&placed);
    e.tries = 2;
    let later = T + 3_600_000;
    let got = refreshed(&e, None, &tipped(placed.clone(), 50), "ALL").expect("the tip moved the order");
    let (old, new) = (doc_of(&e), doc_of(&got));
    assert_eq!((new.tip, new.total), (50, 250));
    assert_eq!((new.uuid, new.issued_at_ms), (old.uuid, old.issued_at_ms), "one document, issued once");
    assert_eq!((&got.id, &got.to, got.queued_at_ms, got.tries), (&e.id, &e.to, e.queued_at_ms, 2));
    assert!(later > got.queued_at_ms);
    assert_eq!(refreshed(&e, None, &placed, "ALL"), None, "nothing moved, nothing written");
}

/// A COMP AFTER PLACEMENT (the line kept, `comped`, its amount in `discount`).
#[test]
fn a_comp_after_placement_is_in_the_queued_document() {
    let placed = order("o2");
    let e = queued(&placed);
    let mut comped = placed.clone();
    comped["items"][0]["comped"] = json!(true);
    comped["discount"] = json!(200);
    comped["total"] = json!(0);
    let got = doc_of(&refreshed(&e, None, &comped, "ALL").expect("the comp moved the order"));
    assert_eq!((got.discount, got.total, got.lines[0].comped), (200, 0, true));
}

/// NEVER A DOCUMENT THE PLATFORM MAY HOLD: `Sending` (an answer may be lost),
/// `Unfiscalised`, `Refused`, `Inconclusive`, and a `Blocked` row that once had
/// an unanswered send or a sale id. Twin: a clean `Blocked` row (the body was
/// refused here, e.g. the tip had no till item) IS rebuilt.
#[test]
fn a_document_a_firing_may_have_posted_is_never_rewritten() {
    let placed = order("o3");
    let e = queued(&placed);
    let now = tipped(placed, 30);
    for s in [Stage::Sending, Stage::Unfiscalised, Stage::Refused, Stage::Inconclusive] {
        assert!(!rewritable(Some(&intent(&e, s))), "{s:?}");
        assert_eq!(refreshed(&e, Some(&intent(&e, s)), &now, "ALL"), None, "{s:?}");
    }
    let once_unknown = Intent { unknowns: 1, ..intent(&e, Stage::Blocked) };
    let with_sale = Intent { sale_id: Some(9001), ..intent(&e, Stage::Blocked) };
    assert_eq!(refreshed(&e, Some(&once_unknown), &now, "ALL"), None);
    assert_eq!(refreshed(&e, Some(&with_sale), &now, "ALL"), None);
    assert!(rewritable(None) && rewritable(Some(&intent(&e, Stage::Blocked))));
    assert_eq!(doc_of(&refreshed(&e, Some(&intent(&e, Stage::Blocked)), &now, "ALL").unwrap()).tip, 30);
}

/// AN ORDER THAT NO LONGER OWES A DOCUMENT leaves the entry alone (the cancel
/// path owns it); twin: the same order still taking money is rebuilt.
#[test]
fn an_order_that_no_longer_owes_a_document_is_left_alone() {
    let placed = order("o4");
    let e = queued(&placed);
    let mut refunded = tipped(placed.clone(), 20);
    refunded["status"] = json!("COMPENSATED_REFUND");
    assert_eq!(refreshed(&e, None, &refunded, "ALL"), None);
    assert!(refreshed(&e, None, &tipped(placed, 20), "ALL").is_some());
}

/// ONE DOCUMENT PER ORDER over the whole queue: only the moved entries come
/// back, each under its own id, and an order with no entry gets none.
#[test]
fn the_queue_refresh_returns_only_the_moved_entries_under_their_own_ids() {
    let (a, b) = (order("a"), order("b"));
    let es = [queued(&a), queued(&b)];
    let view = |id: &str, o: &Value| OrderView { order_id: id.into(), kind: 1, seq: 2, order_json: o.to_string() };
    let orders = [view("a", &tipped(a, 10)), view("b", &b), view("c", &tipped(order("c"), 5))];
    let got = refresh_all(&es, &[], &orders, "ALL");
    assert_eq!(got.len(), 1);
    assert_eq!((got[0].id.as_str(), doc_of(&got[0]).tip), (es[0].id.as_str(), 10));
    let held = [intent(&es[0], Stage::Sending)];
    assert!(refresh_all(&es, &held, &orders, "ALL").is_empty(), "a claimed entry is not touched");
}
