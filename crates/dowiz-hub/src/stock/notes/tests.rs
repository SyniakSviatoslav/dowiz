//! A note rides in the chain and moves nothing: the shelf, the journal, the
//! cost book and every checkpoint fold exactly as they would without it.

use super::super::meta::Meta;
use super::super::{StockEvent, StockLedger, StockLog};
use super::{at_of, is_note};

fn log() -> StockLog {
    StockLog::create_sized(64 * 1024).expect("log")
}

fn received(item: &str, qty: i64) -> StockEvent {
    StockEvent::Received { item: item.into(), qty }
}

#[test]
fn a_note_moves_nothing_and_reads_back_oldest_first() {
    let mut with = log();
    let mut without = log();
    for l in [&mut with, &mut without] {
        l.append(&received("salmon", 1000)).unwrap();
    }
    with.append_note("supplier", r#"{"id":"sea","name":"Sea Fish"}"#).unwrap();
    with.append_note("ordered", r#"{"supplier":"sea","item":"salmon","qty":2000}"#).unwrap();
    with.append_note("supplier", r#"{"id":"sea","name":"Sea Fish 2"}"#).unwrap();
    for l in [&mut with, &mut without] {
        l.append(&StockEvent::Wasted { item: "salmon".into(), qty: 20, reason: super::super::WasteReason::Spoiled, by: "p1".into() })
            .unwrap();
    }
    assert_eq!(with.ledger().unwrap(), without.ledger().unwrap(), "the shelf is the same");
    assert_eq!(with.events(), without.events(), "a note is not an event");
    let (a, b) = (with.journal().unwrap(), without.journal().unwrap());
    assert_eq!(a.entries, b.entries, "the journal's rows are the same, seq included");
    assert_eq!(with.cost_book(), without.cost_book());
    assert_eq!(with.len(), without.len() + 3, "but the notes ARE in the chain");
    let cards = with.notes("supplier");
    assert_eq!(cards.len(), 2);
    assert!(cards[0].contains("Sea Fish\"") && cards[1].contains("Sea Fish 2"), "oldest first: {cards:?}");
    assert_eq!(with.notes("ordered").len(), 1);
    assert!(with.notes("nothing").is_empty());
    assert!(is_note(&cards[0], "supplier") && !is_note(&cards[0], "ordered"));
}

#[test]
fn a_note_that_is_not_an_object_is_refused_and_nothing_is_written() {
    let mut l = log();
    for bad in ["", "[1,2]", "name", "{\"a\":1"] {
        assert!(l.append_note("supplier", bad).is_err(), "{bad:?} refused");
    }
    assert!(l.append_note("  ", "{}").is_err(), "a note names what it is");
    let long = format!(r#"{{"x":"{}"}}"#, "a".repeat(super::BODY_MAX));
    assert!(l.append_note("supplier", &long).is_err(), "too long");
    assert_eq!(l.len(), 0, "a refusal writes nothing");
    // The positive twin: an empty object is a note.
    l.append_note("supplier", "{}").unwrap();
    assert_eq!(l.len(), 1);
}

#[test]
fn a_note_carries_the_request_clock() {
    let mut l = log();
    l.append_note("ordered", r#"{"item":"rice"}"#).unwrap();
    l.set_clock(1_700_000_000_000);
    l.append_note("ordered", r#"{"item":"nori"}"#).unwrap();
    let n = l.notes("ordered");
    assert_eq!(at_of(&n[0]), None, "no clock, no at");
    assert_eq!(at_of(&n[1]), Some(1_700_000_000_000));
}

#[test]
fn checkpoints_written_among_notes_still_agree_with_the_fold_from_genesis() {
    let mut l = log();
    l.set_checkpoint_every(2);
    let mut events = Vec::new();
    for i in 0..9 {
        let ev = received("rice", 100 + i);
        l.append_with(&ev, &Meta::at(1_000 + i)).unwrap();
        events.push(ev);
        l.append_note("supplier", &format!(r#"{{"id":"s{i}"}}"#)).unwrap();
    }
    assert!(l.verify_checkpoints().unwrap() > 0, "checkpoints were written and verified");
    assert_eq!(l.ledger().unwrap(), StockLedger::fold(&events).unwrap());
    let since = l.journal_since(1_005).unwrap();
    assert_eq!(since.ledger, l.journal().unwrap().ledger, "a journal from a checkpoint is today's shelf");
    assert_eq!(l.notes("supplier").len(), 9);
}
