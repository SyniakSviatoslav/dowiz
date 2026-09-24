//! BLIND-SPOTS §2.8 CHECK, natively: three entries with the middle refused ->
//! two Sent, one exception row, the drain continued; 49 h unsent -> named as
//! overdue; the same uuid sent twice -> one invoice.

use super::*;
use crate::fiscal::document::document;
use crate::fiscal::sender::{Mock, NotConfigured};
use crate::services::ordering::tax_block::stamp;
use crate::services::ordering::tax_cfg::VenueTax;
use dowiz_core::tax::RatePpm;
use serde_json::json;

const T: i64 = 1_790_000_000_000;
const H: i64 = 3600 * 1000;
const V20: VenueTax = VenueTax { default: RatePpm(200_000), inclusive: true, fee: RatePpm(200_000) };

fn doc(id: &str, issued: i64) -> Document {
    let mut o = json!({
        "id": id, "location_id": "v1", "status": "DELIVERED",
        "items": [{ "name": "Maki", "quantity": 2, "unit_price": 600 }], "total": 1200,
        "payments": [{ "method": "cash", "amount": 1200 }],
    });
    stamp(&mut o, &V20, 0, 0, 0).unwrap();
    document(&o, "ALL", issued).unwrap()
}

fn three() -> (Vec<Entry>, [Document; 3]) {
    let ds = [doc("o1", T), doc("o2", T + 1_000), doc("o3", T + 2_000)];
    (ds.iter().map(entry).collect(), ds)
}

#[test]
fn the_entry_is_an_outbox_entry_keyed_by_uuid_with_a_48h_deadline() {
    let d = doc("o1", T);
    let e = entry(&d);
    assert_eq!((e.kind.as_str(), e.to.as_str(), e.queued_at_ms), (KIND, "o1", T));
    assert_eq!(e.id, uuid_text(&d.uuid));
    assert_eq!(serde_json::from_str::<Document>(&e.text).unwrap(), d, "the text IS the document");
    assert_eq!(deadline_of(&e), T + 48 * H);
}

#[test]
fn three_entries_middle_refused_two_sent_one_exception_the_drain_continued() {
    let (es, ds) = three();
    let mock = Mock::refusing(&[ds[1].uuid]);
    let out = drain(&es, &mock, T + 5_000);

    let ids: Vec<&str> = out.verdicts.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, vec![es[0].id.as_str(), es[1].id.as_str(), es[2].id.as_str()], "in issue order");
    assert_eq!(out.verdicts[0].1, Verdict::Sent);
    assert_eq!(out.verdicts[1].1, Verdict::Abandon { after: 1 });
    assert_eq!(out.verdicts[2].1, Verdict::Sent, "the refusal did not block the one behind it");
    assert_eq!(out.sent.iter().map(|(o, _)| o.as_str()).collect::<Vec<_>>(), vec!["o1", "o3"]);
    assert_eq!(mock.calls(), 3);
    assert_eq!(mock.invoices(), 2);

    assert_eq!(out.exceptions.len(), 1, "one exception row");
    let row = &out.exceptions[0];
    assert_eq!((row.kind, row.order_id.as_deref()), ("fiscal.refused", Some("o2")));
    assert!(row.reason.as_deref().unwrap().contains(&es[1].id), "the platform's problem is kept");
    assert_eq!(out.corrective.len(), 1, "the corrective path");
    assert_eq!(out.corrective[0].corrects, Some(ds[1].uuid));
}

#[test]
fn nothing_refused_all_three_sent_in_issue_order_even_when_stored_out_of_order() {
    let (mut es, _) = three();
    es.reverse();
    let mock = Mock::new();
    let out = drain(&es, &mock, T + 5_000);
    assert_eq!(out.sent.iter().map(|(o, _)| o.as_str()).collect::<Vec<_>>(), vec!["o1", "o2", "o3"]);
    assert!(out.exceptions.is_empty() && out.corrective.is_empty());
}

#[test]
fn the_same_uuid_sent_twice_is_one_invoice() {
    let (es, _) = three();
    let one = &es[..1];
    let mock = Mock::new();
    mock.lose_next_response();
    let first = drain(one, &mock, T + 5_000);
    let Verdict::Retry { tries, next_at_ms } = first.verdicts[0].1.clone() else {
        panic!("a lost response is a retry: {:?}", first.verdicts)
    };
    assert!(first.sent.is_empty());
    let mut again = one[0].clone();
    again.tries = tries;
    again.next_at_ms = next_at_ms;
    let second = drain(&[again], &mock, next_at_ms);
    assert_eq!(second.verdicts[0].1, Verdict::Sent);
    assert_eq!(mock.calls(), 2, "it was sent twice");
    assert_eq!(mock.invoices(), 1, "and the platform holds ONE invoice: same uuid");
}

#[test]
fn a_fiscal_entry_is_not_abandoned_for_counting_its_tries() {
    let (es, _) = three();
    let mut e = es[0].clone();
    e.tries = crate::outbox::MAX_TRIES + 3;
    let mock = Mock::new();
    mock.lose_next_response();
    let out = drain(&[e], &mock, T + 5_000);
    assert!(matches!(out.verdicts[0].1, Verdict::Retry { .. }), "{:?}", out.verdicts);
}

#[test]
fn not_configured_sends_nothing_and_leaves_every_entry_where_it_was() {
    let (es, _) = three();
    let out = drain(&es, &NotConfigured, T + 5_000);
    assert_eq!(out.held, 3);
    assert!(out.verdicts.is_empty() && out.sent.is_empty() && out.exceptions.is_empty());
}

#[test]
fn an_entry_49h_old_and_unsent_is_named_overdue_and_one_47h_old_is_not() {
    let (es, _) = three();
    let old = entry(&doc("o-old", T - 49 * H));
    let mut all = es.clone();
    all.push(old.clone());
    let h = health(&all, T + 5_000);
    assert_eq!(h.backlog, 4);
    assert_eq!(h.oldest_issued_at, Some(T - 49 * H));
    assert_eq!(h.first_deadline, Some(T - H));
    assert_eq!(h.overdue.len(), 1);
    assert_eq!((h.overdue[0].order_id.as_str(), h.overdue[0].deadline), ("o-old", T - H));

    let young = entry(&doc("o-young", T - 47 * H));
    let h = health(&[young], T);
    assert!(h.overdue.is_empty(), "47 h: still inside the deadline");
    assert_eq!(h.backlog, 1);
}

#[test]
fn a_deadline_exactly_now_has_passed() {
    let e = entry(&doc("o1", T));
    assert!(health(&[e.clone()], T + 48 * H - 1).overdue.is_empty());
    assert_eq!(health(&[e], T + 48 * H).overdue.len(), 1);
}

#[test]
fn health_counts_only_fiscal_entries_and_is_empty_when_there_are_none() {
    let tg = Entry::new("o9:telegram".into(), "telegram", "chat".into(), "hi".into(), T - 100 * H);
    let h = health(&[tg.clone()], T);
    assert_eq!((h.backlog, h.oldest_issued_at, h.first_deadline), (0, None, None));
    let out = drain(&[tg], &Mock::new(), T);
    assert!(out.verdicts.is_empty(), "the drain never touches another rail's entry");
}

#[test]
fn an_unreadable_entry_is_an_exception_not_a_silent_drop() {
    let mut e = entry(&doc("o1", T));
    e.text = "{not json".into();
    let out = drain(&[e], &Mock::new(), T + 1);
    assert_eq!(out.exceptions.len(), 1);
    assert!(matches!(out.verdicts[0].1, Verdict::Abandon { .. }));
}
