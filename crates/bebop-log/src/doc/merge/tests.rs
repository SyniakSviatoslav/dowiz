//! The MERGE's bytes and the union under TOY seams: layout and counters. Everything an attacker
//! does with a real key is `crates/dowiz-hub/tests/m2_merge.rs`.

use super::super::keys::KeyOp;
use super::super::merge_body::{merge_body, parse_merge, MergeView, Row, MERGE_MARK, STATE_COMMUTES, STATE_OBSTRUCTED};
use super::super::tests::{doc_with, key_of, Dev, ToyIds, ToySigner, ToyVerifier};
use super::super::verify::{verify, Report, Trust};
use super::super::{Doc, DocError};
use super::Union;
use crate::{EvLog, Store};

fn row(b: u8, seq: u64) -> Row {
    Row { actor_id: [b; 32], seq, tip: [b ^ 0xFF; 32] }
}

fn view() -> MergeView {
    MergeView { ours: vec![row(1, 4), row(2, 1)], theirs: vec![row(2, 3)], state: STATE_OBSTRUCTED, seg_root: [7; 32], conflicts: vec![b"one".to_vec(), Vec::new()] }
}

#[test]
fn a_merge_body_comes_back_exactly() {
    let v = view();
    let b = merge_body(&v);
    assert_eq!(parse_merge(&b), Some(v));
    assert_eq!(b.len(), MERGE_MARK.len() + 8 + 2 * 72 + 8 + 72 + 1 + 32 + 16 + 8 + 11 + 8);
}

/// A body with the mark that is not exactly a MERGE is `None`: every cut, every extra byte, every
/// lying count. `merges_bad` turns that `None` into a refusal to hold.
#[test]
fn a_damaged_merge_body_is_not_a_merge() {
    let b = merge_body(&view());
    for n in 0..b.len() {
        assert_eq!(parse_merge(&b[..n]), None, "cut at {n}");
    }
    let mut long = b.clone();
    long.push(0);
    assert_eq!(parse_merge(&long), None, "a trailing byte");
    let at = MERGE_MARK.len();
    let mut lying = b.clone();
    lying[at..at + 8].copy_from_slice(&u64::MAX.to_le_bytes());
    assert_eq!(parse_merge(&lying), None, "a row count past the body");
    let state = at + 8 + 2 * 72 + 8 + 72;
    for (bad, what) in [(0u8, "state 0"), (4, "state 4"), (STATE_COMMUTES, "Commutes with conflicts named")] {
        let mut x = b.clone();
        x[state] = bad;
        assert_eq!(parse_merge(&x), None, "{what}");
    }
    let mut none = view();
    none.conflicts.clear();
    assert_eq!(parse_merge(&merge_body(&none)), None, "Obstructed with no conflict named");
    let mut dup = b.clone();
    dup[state + 33] = 1;
    assert_eq!(parse_merge(&dup), None, "dup_declared is 0 in M2-min");
}

fn counts(st: &Store) -> Report {
    verify(st, &ToyIds, &ToyVerifier, Trust::Genesis).unwrap()
}

/// Genesis A, two records, ADMIT of B: the split. Then A and B each write offline.
fn split() -> (Store, Store, Dev<ToySigner>, Dev<ToySigner>) {
    let (mut sa, mut a) = doc_with(2);
    let mut b = Dev::new(ToySigner { id: [0xB2; 32] });
    let body = KeyOp::Admit(key_of(&b.s)).body();
    a.commit(&mut sa, &body, Some(&KeyOp::Admit(key_of(&b.s)))).unwrap();
    let bytes = sa.to_bytes();
    b.tip = EvLog::tip_at(&sa, Doc::open(&sa).unwrap().log).unwrap();
    (Store::from_bytes(&bytes), Store::from_bytes(&bytes), a, b)
}

fn merged(ours: &Store, theirs: &Store) -> Result<(Store, Union), DocError> {
    let mut fresh = Store::create_bytes(1 << 21).unwrap();
    Doc::merge_into(ours, theirs, &mut fresh, &ToyIds, &ToyVerifier, Trust::Genesis).map(|u| (fresh, u))
}

#[test]
fn a_toy_union_keeps_every_seal_and_a_merge_is_a_counted_record() {
    let (mut sa, mut sb, mut a, mut b) = split();
    a.commit(&mut sa, b"a1", None).unwrap();
    b.commit(&mut sb, b"b1", None).unwrap();
    b.commit(&mut sb, b"b2", None).unwrap();
    let (mut u, found) = merged(&sa, &sb).unwrap();
    assert!(found.wrote && found.both_wrote);
    assert_eq!((found.our_tail.len(), found.their_tail.len()), (1, 2));
    let c = counts(&u).counters;
    assert_eq!((c.records, c.seals_ok, c.seals_bad, c.broken, c.merges), (6, 6, 0, 0, 0));
    assert!(counts(&u).holds(), "THE LIMIT: a union with no MERGE holds: {c:?}");
    let body = merge_body(&MergeView { ours: found.ours.clone(), theirs: found.theirs.clone(), state: STATE_COMMUTES, seg_root: [0; 32], conflicts: Vec::new() });
    a.commit(&mut u, &body, None).unwrap();
    let r = counts(&u);
    assert_eq!((r.counters.merges, r.counters.merges_bad, r.holds()), (1, 0, true), "{:?}", r.counters);
}

#[test]
fn a_signed_row_no_record_holds_is_merges_bad() {
    let (mut sa, _, mut a, _) = split();
    let mut ghost = view();
    ghost.conflicts.clear();
    ghost.state = STATE_COMMUTES;
    a.commit(&mut sa, &merge_body(&ghost), None).unwrap();
    let r = counts(&sa);
    assert_eq!((r.counters.merges, r.counters.merges_bad, r.holds()), (1, 1, false), "{:?}", r.counters);
    let (mut sc, _, mut c, _) = split();
    c.commit(&mut sc, &MERGE_MARK.to_vec(), None).unwrap();
    let r = counts(&sc);
    assert_eq!((r.counters.merges, r.counters.merges_bad, r.holds()), (1, 1, false), "a body with the mark and nothing else: {:?}", r.counters);
}

#[test]
fn one_actor_with_two_keys_is_refused() {
    let (mut sa, mut sb, mut a, mut b) = split();
    let k1 = key_of(&ToySigner { id: [0xC3; 32] });
    let k2 = super::super::keys::Key { pk: vec![1, 2, 3], ..k1.clone() };
    a.commit(&mut sa, &KeyOp::Admit(k1.clone()).body(), Some(&KeyOp::Admit(k1))).unwrap();
    b.commit(&mut sb, &KeyOp::Admit(k2.clone()).body(), Some(&KeyOp::Admit(k2))).unwrap();
    assert_eq!(merged(&sa, &sb).err(), Some(DocError::KeyClash([0xC3; 32])));
}

#[test]
fn one_seal_per_sync_covers_every_record_under_it() {
    let (mut sa, mut a) = doc_with(1);
    let recs = vec![a.next(b"m1"), a.next(b"m2"), a.next(b"m3")];
    Doc::commit_many_bytes(&mut sa, &recs, &a.s, 5).unwrap();
    let r = counts(&sa);
    assert_eq!((r.counters.records, r.counters.seals_ok, r.counters.unsealed, r.holds()), (4, 2, 0, true), "{:?}", r.counters);
    let mut wrong = a.next(b"m4");
    wrong.actor_seq += 1;
    assert_eq!(Doc::commit_many_bytes(&mut sa, &[wrong], &a.s, 6), Err(DocError::Seq { want: 5, got: 6 }));
}

// A cut can take a genesis device's whole history: only a second genesis key can lose it
// (`CutsAdmit` protects an admitted one, the signing root keeps a seal). Its HEADS row has no seal
// and the union must keep it, or the device is told seq 1 and its next commit breaks the file.
/// B (genesis) writes twice, A (genesis) twice, the cut takes exactly B's records, A declares it.
/// Returns (the cut file, a copy where A wrote once more, B's frontier tip).
fn whole_history_gone() -> (Store, Store, [u8; 32]) {
    let mut st = Store::create_bytes(1 << 20).unwrap();
    let mut a = Dev::new(ToySigner { id: [0xA1; 32] });
    let mut b = Dev::new(ToySigner { id: [0xB2; 32] });
    Doc::init_bytes(&mut st, 0, &[key_of(&a.s), key_of(&b.s)]).unwrap();
    b.commit(&mut st, b"b-1", None).unwrap();
    b.commit(&mut st, b"b-2", None).unwrap();
    a.commit(&mut st, b"a-1", None).unwrap();
    a.commit(&mut st, b"a-2", None).unwrap();
    let mut cut = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&st, &mut cut, 2, &ToyIds).unwrap();
    assert_eq!((seg.frontier.len(), seg.frontier[0].actor_id, seg.frontier[0].seq), (1, b.s.id, 2), "the cut took B's whole history and nothing of A's");
    a.commit(&mut cut, &seg.truncate_body(), None).unwrap();
    let r = counts(&cut);
    assert!(r.holds(), "the cut file holds: {:?}", r.counters);
    let row = r.heads.iter().find(|h| h.actor_id == b.s.id).copied().expect("the cut keeps B's row");
    assert_eq!((row.seq, row.tip, row.seal), (2, b.tip, None), "a row with no kept seal");
    let mut theirs = Store::from_bytes(&cut.to_bytes());
    a.commit(&mut theirs, b"a-3", None).unwrap();
    (cut, theirs, b.tip)
}

/// B's device placed on `st` by the hub's rules: `SealedHub::next_seq` (one past the row, 1 with no
/// row) and `device_prev` (the row's tip, else the frontier row's). `commit` then adds one to `seq`.
fn as_the_hub(st: &Store, frontier_tip: [u8; 32]) -> Dev<ToySigner> {
    let mut d = Dev::new(ToySigner { id: [0xB2; 32] });
    let row = counts(st).heads.iter().find(|h| h.actor_id == d.s.id).copied();
    d.seq = row.map_or(0, |h| h.seq);
    d.tip = row.map_or(frontier_tip, |h| h.tip);
    d
}

/// B at its true position: past its own second record, whatever the file's HEADS say.
fn at_its_own_place(frontier_tip: [u8; 32]) -> Dev<ToySigner> {
    let mut d = Dev::new(ToySigner { id: [0xB2; 32] });
    d.seq = 2;
    d.tip = frontier_tip;
    d
}

#[test]
fn with_no_merge_a_device_past_the_cut_commits_and_the_file_holds() {
    let (cut, _, tip) = whole_history_gone();
    let mut st = Store::from_bytes(&cut.to_bytes());
    let mut b = as_the_hub(&st, tip);
    assert_eq!(b.seq, 2, "the cut file has B's row");
    b.commit(&mut st, b"b-3", None).unwrap();
    let r = counts(&st);
    assert!(r.holds(), "control: {:?}", r.counters);
}

#[test]
fn a_union_keeps_the_row_of_a_device_with_no_kept_seal() {
    let (cut, theirs, tip) = whole_history_gone();
    let (u, found) = merged(&cut, &theirs).unwrap();
    assert!(found.wrote);
    let r = counts(&u);
    assert!(r.holds(), "{:?}", r.counters);
    let row = r.heads.iter().find(|h| h.actor_id == [0xB2; 32]).copied();
    assert_eq!(row.map(|h| (h.seq, h.tip)), Some((2, tip)), "B's HEADS row after the union");
}

#[test]
fn a_device_with_no_kept_seal_goes_on_from_its_own_place_after_a_union() {
    let (cut, theirs, tip) = whole_history_gone();
    let (mut u, _) = merged(&cut, &theirs).unwrap();
    let mut b = at_its_own_place(tip);
    let out = b.commit(&mut u, b"b-3", None);
    let r = counts(&u);
    assert!(out.is_ok(), "seq 3 on prev tip: {out:?}");
    assert!(r.holds(), "{:?}", r.counters);
}

#[test]
fn a_device_with_no_kept_seal_commits_as_the_hub_would_and_the_file_holds() {
    let (cut, theirs, tip) = whole_history_gone();
    let (mut u, _) = merged(&cut, &theirs).unwrap();
    let before = counts(&u);
    assert!(before.holds(), "the union holds before the commit: {:?}", before.counters);
    let mut b = as_the_hub(&u, tip);
    let seq = b.seq + 1;
    let out = b.commit(&mut u, b"b-3", None);
    let r = counts(&u);
    assert!(out.is_ok(), "seq {seq}: {out:?}");
    assert!(r.holds(), "the union held, the writer took seq {seq}, and now: {:?}", r.counters);
}
