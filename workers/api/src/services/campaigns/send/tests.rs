//! The send: N entries not 2N, the budget, and G4 at the drain -- with a MOCK
//! sender that records what would have gone out. Nothing here reaches a
//! network.

use std::collections::BTreeSet;

use super::*;
use crate::services::campaigns::audience::{recipients, Recipient};
use crate::services::campaigns::campaign::{define, sent_to, DefIn, KIND_SENT};
use crate::services::campaigns::segment::{Now, Segment, DAY_MS};
use dowiz_hub::consent::{wording_id, Act, Method, State, KIND_ACT};
use dowiz_hub::logimage::LogImage;
use dowiz_hub::table::Table;
use serde_json::{json, Value};

const NOW: Now = Now { utc_ms: 100 * DAY_MS, local_ms: 100 * DAY_MS };

fn act(k: &str, state: State, at: i64) -> LogEntry {
    let a = Act {
        key: k.into(),
        purpose: "marketing".into(),
        channel: "whatsapp".into(),
        state,
        at_ms: at,
        method: Method::CheckoutBox,
        evidence: String::new(),
        wording_id: wording_id("sq"),
        via: "o".into(),
    };
    LogEntry { kind: KIND_ACT.into(), subject: format!("cust:{k}"), json: a.to_json(), seq: 0 }
}

fn def() -> Def {
    let b: DefIn = serde_json::from_value(json!({ "name": "A", "text": "Hello", "segment": { "kind": "everyone_consented" },
        "template": { "name": "autumn_soup", "lang": "sq", "params": ["SOUP10"] } })).unwrap();
    define(b, &[], "owner", 1_000).unwrap()
}

fn people(n: usize) -> (Vec<Value>, Vec<LogEntry>) {
    let phones: Vec<String> = (0..n).map(|i| format!("06910000{i:02}")).collect();
    let orders = phones.iter().map(|p| json!({ "contact": { "phone": p }, "created_at_ms": 1, "status": "DELIVERED" })).collect();
    let acts = phones.iter().map(|p| act(&format!("k{p}"), State::Given, 10)).collect();
    (orders, acts)
}

fn reach(orders: &[Value], acts: &[LogEntry]) -> Vec<Recipient> {
    recipients(orders, |p| format!("k{p}"), |k| k.to_string(), |_| Vec::new(), |_| None, acts, &Segment::EveryoneConsented, NOW)
}

/// The mock drain: what `outbox::rails::drain` does for a campaign entry,
/// with a Vec where WhatsApp would be. Returns what was "sent" and what was
/// dropped as withdrawn.
fn mock_drain(queue: &[Entry], acts: &[LogEntry]) -> (Vec<(String, String)>, Vec<String>) {
    let (mut sent, mut dropped) = (Vec::new(), Vec::new());
    for e in queue {
        match gate(e, acts) {
            Some(who) => {
                assert_eq!(parse_id(&e.id).map(|(_, k)| k), Some(who.key()));
                sent.push((e.to.clone(), e.text.clone()));
            }
            None => dropped.push(e.id.clone()),
        }
    }
    (sent, dropped)
}

#[test]
fn the_same_campaign_sent_twice_is_n_entries_not_2n() {
    let (orders, acts) = people(5);
    let d = def();
    let r = reach(&orders, &acts);
    let first = plan(&d, &r, &BTreeSet::new(), &[], 2_000).unwrap();
    assert_eq!((first.queue.len(), first.already, first.left), (5, 0, 0));

    // The history the first press wrote...
    let mut log = LogImage::create().unwrap();
    for (k, _, row) in &first.queue {
        log.append(KIND_SENT, k, row).unwrap();
    }
    // ...makes the second press queue nobody.
    let second = plan(&d, &r, &sent_to(&log.entries(), &d.id), &[], 3_000).unwrap();
    assert_eq!((second.queue.len(), second.already), (0, 5));

    // AND AT THE QUEUE: the same press retried before the history landed
    // produces the same ids, which the put-if-absent keeps as ONE entry each.
    let retry = plan(&d, &r, &BTreeSet::new(), &[], 3_000).unwrap();
    let ids: BTreeSet<String> = first.queue.iter().chain(retry.queue.iter()).map(|(_, e, _)| e.id.clone()).collect();
    assert_eq!(ids.len(), 5, "10 planned, 5 distinct entries");
}

#[test]
fn each_entry_is_the_consented_key_carrying_the_approved_template() {
    let (orders, acts) = people(1);
    let p = plan(&def(), &reach(&orders, &acts), &BTreeSet::new(), &[], 2_000).unwrap();
    let (_, e, row) = &p.queue[0];
    assert_eq!(e.kind, OUTBOX_KIND);
    assert_eq!(parse_id(&e.id), Some((def().id.as_str(), "k0691000000")));
    assert_eq!(e.to, "355691000000");
    // THE ENTRY IS THE TEMPLATE, not free text: what the drain hands Meta.
    let t: Value = serde_json::from_str(&e.text).expect("the entry's text is the template object");
    assert_eq!(t, json!({ "name": "autumn_soup", "language": { "code": "sq" },
        "components": [{ "type": "body", "parameters": [{ "type": "text", "text": "SOUP10" }] }] }));
    assert!(row.contains("\"consentAt\":10"), "the proof travels with the history: {row}");
    assert_eq!(parse_id("camp:c1:"), None);
    assert_eq!(parse_id("order-1:telegram"), None);
}

/// A WHATSAPP CAMPAIGN WITHOUT A TEMPLATE IS NEVER PLANNED: `plan` refuses
/// it by name and nothing is queued. The twin is every other test here.
#[test]
fn a_campaign_without_a_template_plans_nobody() {
    let (orders, acts) = people(3);
    let mut d = def();
    d.template = None;
    let why = plan(&d, &reach(&orders, &acts), &BTreeSet::new(), &[], 2_000).unwrap_err();
    assert!(why.contains("template") && why.contains("24 hours"), "{why}");
}

#[test]
fn a_send_fills_at_most_half_the_outbox_and_the_rest_is_left() {
    let (orders, acts) = people(3);
    let r = reach(&orders, &acts);
    let d = def();
    let roomy = plan(&d, &r, &BTreeSet::new(), &[], 2_000).unwrap();
    assert_eq!((roomy.queue.len(), roomy.left), (3, 0), "the twin: an empty outbox takes all three");
    // A WAITING ORDER NOTICE sized so that exactly ONE campaign entry fits in
    // what remains of the budget -- derived from the real entry, not guessed.
    let one = serde_json::to_string(&roomy.queue[0].1).unwrap().len();
    let shell = serde_json::to_string(&Entry::new("o1:telegram".into(), "telegram", "chat".into(), String::new(), 0)).unwrap().len();
    let filler = BUDGET_BYTES - shell - one - one / 2;
    let big = Entry::new("o1:telegram".into(), "telegram", "chat".into(), "x".repeat(filler), 0);
    let p = plan(&d, &r, &BTreeSet::new(), &[big], 2_000).unwrap();
    assert_eq!((p.queue.len(), p.left), (1, 2), "one fits, two wait for the next press");
}

/// G4: ENQUEUE TWO, WITHDRAW ONE, DRAIN → ONE MESSAGE. The withdrawal came
/// after the entry was queued; the drain's own fold stops it.
#[test]
fn a_withdrawal_after_queueing_stops_the_message_at_the_drain() {
    let (orders, mut acts) = people(2);
    let p = plan(&def(), &reach(&orders, &acts), &BTreeSet::new(), &[], 2_000).unwrap();
    let queue: Vec<Entry> = p.queue.into_iter().map(|(_, e, _)| e).collect();
    let (sent, dropped) = mock_drain(&queue, &acts);
    assert_eq!((sent.len(), dropped.len()), (2, 0), "the twin: nobody withdrew, both go");

    acts.push(act("k0691000001", State::Withdrawn, 50));
    let (sent, dropped) = mock_drain(&queue, &acts);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, "355691000000");
    assert_eq!(dropped, vec![entry_id(&def().id, "k0691000001")]);
}

#[test]
fn the_gate_refuses_what_is_not_a_campaign_entry() {
    let acts = [act("k1", State::Given, 1)];
    let order_notice = Entry::new("camp:c1:k1".into(), "telegram", "chat".into(), "t".into(), 0);
    assert!(gate(&order_notice, &acts).is_none(), "wrong kind");
    let ok = Entry::new("camp:c1:k1".into(), OUTBOX_KIND, "355".into(), "t".into(), 0);
    assert!(gate(&ok, &acts).is_some());
    assert!(any_campaign([&ok].into_iter()));
    assert!(!any_campaign([&order_notice].into_iter()));
}

// ── the press: two writes, and a retry after a failure between them ────────

fn outbox() -> Table {
    Table::create(crate::outbox::OUTBOX_BYTES).unwrap()
}

/// DEFECT 3. The first press writes the outbox and FAILS before the history
/// (the second write is simply not made). The drain then delivers two of the
/// three entries and removes them. The second press plans all three again --
/// the log has no `sent` rows -- and must queue NOBODY: not the waiting one,
/// and not the two already delivered, which a put-if-absent on the entry
/// alone would have re-queued. Its history then files all three, and the
/// marks are pruned.
#[test]
fn a_retry_after_a_failure_between_the_writes_queues_nobody_twice() {
    let (orders, acts) = people(3);
    let (d, r) = (def(), reach(&orders, &acts));
    let mut t = outbox();
    let mut log = LogImage::create().unwrap();

    let first = plan(&d, &r, &sent_to(&log.entries(), &d.id), &[], 2_000).unwrap();
    let (fresh, _marked) = admit(&mut t, &first.queue).unwrap();
    assert_eq!(fresh, 3);
    // -- the history write fails here: nothing is appended to `log` --

    // The drain delivers two and removes them.
    for (_, e, _) in first.queue.iter().take(2) {
        assert!(t.remove(crate::outbox::KIND, &e.id));
    }

    let second = plan(&d, &r, &sent_to(&log.entries(), &d.id), &[], 3_000).unwrap();
    assert_eq!(second.queue.len(), 3, "the log is behind, so the plan sees everyone again");
    let (fresh, marked) = admit(&mut t, &second.queue).unwrap();
    assert_eq!(fresh, 0, "NOTHING queued twice, delivered or waiting");
    assert_eq!(t.all(crate::outbox::KIND).len(), 1, "only the one never delivered still waits");

    let filed = file_history(&mut log, &d.id, &second.queue, &marked).unwrap();
    assert_eq!(filed, 3);
    assert_eq!(sent_to(&log.entries(), &d.id).len(), 3, "the campaign is marked consistently");
    let pruned = prune(&mut t, &d.id, &crate::services::campaigns::campaign::sent_entries(&log.entries(), &d.id));
    assert_eq!((pruned, t.all(MARK).len()), (3, 0), "the marks go once their rows exist");

    // A THIRD PRESS, the ordinary way: the log now answers, nobody is planned.
    let third = plan(&d, &r, &sent_to(&log.entries(), &d.id), &[], 4_000).unwrap();
    assert_eq!((third.queue.len(), third.already), (0, 3));
}

/// The twin: a press with nothing before it queues everyone, once.
#[test]
fn a_clean_press_queues_everyone_and_files_everyone() {
    let (orders, acts) = people(2);
    let (d, r) = (def(), reach(&orders, &acts));
    let (mut t, mut log) = (outbox(), LogImage::create().unwrap());
    let p = plan(&d, &r, &BTreeSet::new(), &[], 2_000).unwrap();
    let (fresh, marked) = admit(&mut t, &p.queue).unwrap();
    assert_eq!((fresh, marked.len()), (2, 2));
    assert_eq!(file_history(&mut log, &d.id, &p.queue, &marked).unwrap(), 2);
    assert_eq!(file_history(&mut log, &d.id, &p.queue, &marked).unwrap(), 0, "the history is idempotent too");
    // Another campaign's marks are not this one's to prune.
    t.put(MARK, "camp:cother:k1", "1", &[], &[]).unwrap();
    prune(&mut t, &d.id, &crate::services::campaigns::campaign::sent_entries(&log.entries(), &d.id));
    assert_eq!(t.all(MARK).len(), 1);
}
