//! W-MG-ATK5, HOLE 1: A NAMED CONFLICT LOCKS THE FILE. `conflicts_at` folds each side through
//! `StockLedger::fold`, whose `apply` runs `decide`, the WRITE door. A side that holds both halves
//! of an earlier conflict is a history the door would have refused, so every later merge of two
//! histories answers `MergeError::Ledger`, for ever: a person cannot write around a fold of the
//! past. CLOSED by `StockLedger::history` (W-MG-MERGE2). No key forged, no byte edited: three honest offline devices.
mod atk5_common;

use atk5_common::*;
use dowiz_hub::doc_seal::merge::{MergeError, Merged};
use dowiz_hub::stock::StockEvent;

/// The last salmon promised on A and on B, and the conflict named, as it should be.
fn after_conflict(tail_a: StockEvent, tail_b: StockEvent, past: &[StockEvent]) -> (Vec<u8>, Vec<u8>) {
    let s = genesis(past);
    let fb = offline(&s, 3, &b(), &[tail_b]);
    let (m, said) = merged(&offline(&s, 2, &a(), &[tail_a]), &fb, &a(), 4);
    assert!(named(&said).is_some_and(|t| t != "[]"), "the control: the first merge names it: {said:?}");
    assert!(holds(&m));
    (s, m)
}

fn must_glue(ours: &[u8], theirs: &[u8], case: &str) {
    let mut sh = opened(ours, 9);
    let before = trimmed(&sh);
    match sh.merge(theirs, &a(), 900) {
        Ok(Merged::Sealed { .. }) => {}
        Err(MergeError::Ledger(why)) => {
            assert_eq!(trimmed(&sh), before);
            panic!("HOLE 1 {case}: two honest files refused for good: Ledger({why})");
        }
        other => panic!("{case}: {other:?}"),
    }
}

/// Third device, rice only, nothing to do with the salmon.
#[test]
fn a_third_device_merges_after_a_reservation_conflict() {
    let (s, m) = after_conflict(reserved("salmon", 1, "o1"), reserved("salmon", 1, "o2"), &[received("salmon", 1), received("rice", 10)]);
    must_glue(&offline(&m, 5, &a(), &[served("rice", 1, "a2")]), &offline(&s, 6, &c(), &[received("tuna", 4)]), "third device");
}

/// The same two devices the next day: B took the merge in, each wrote one more line.
#[test]
fn the_same_two_devices_merge_again_the_next_day() {
    let (_, m) = after_conflict(reserved("salmon", 1, "o1"), reserved("salmon", 1, "o2"), &[received("salmon", 1), received("rice", 10)]);
    must_glue(&offline(&m, 5, &a(), &[served("rice", 1, "a2")]), &offline(&m, 6, &b(), &[served("rice", 1, "b2")]), "next day");
}

/// A person does what the sentence says, "перерахуйте полицю", and the file stays locked.
#[test]
fn a_recount_after_the_conflict_unlocks_the_file() {
    let (_, m) = after_conflict(reserved("salmon", 1, "o1"), reserved("salmon", 1, "o2"), &[received("salmon", 1), received("rice", 10)]);
    let fixed = offline(&m, 5, &a(), &[released("salmon", 1, "o2"), counted("salmon", 1, "n1")]);
    must_glue(&fixed, &offline(&m, 6, &b(), &[served("rice", 1, "b2")]), "after the recount");
}

/// One hold consumed on both: named `double`, then the second `Consumed` fails the door.
#[test]
fn a_double_take_does_not_lock_the_file() {
    let past = [received("salmon", 5), received("rice", 10), reserved("salmon", 1, "o1")];
    let (_, m) = after_conflict(consumed("salmon", 1, "o1"), released("salmon", 1, "o1"), &past);
    must_glue(&offline(&m, 5, &a(), &[served("rice", 1, "a2")]), &offline(&m, 6, &b(), &[served("rice", 1, "b2")]), "double take");
}

/// Waste on both: each side alone is inside the shelf, and the sides together are not.
#[test]
fn waste_on_both_sides_does_not_lock_the_file() {
    let (_, m) = after_conflict(wasted("salmon", 1), wasted("salmon", 1), &[received("salmon", 1), received("rice", 10)]);
    must_glue(&offline(&m, 5, &a(), &[served("rice", 1, "a2")]), &offline(&m, 6, &b(), &[served("rice", 1, "b2")]), "waste");
}

/// THE POSITIVE TWIN: sales never meet the door, so the same shape merges on.
#[test]
fn served_conflicts_do_not_lock_the_file() {
    let (_, m) = after_conflict(served("salmon", 1, "o1"), served("salmon", 1, "o2"), &[received("salmon", 1), received("rice", 10)]);
    must_glue(&offline(&m, 5, &a(), &[served("rice", 1, "a2")]), &offline(&m, 6, &b(), &[served("rice", 1, "b2")]), "served");
}
