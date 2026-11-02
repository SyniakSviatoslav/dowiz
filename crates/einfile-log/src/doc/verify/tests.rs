//! W-MG-SEAL kill tests through the toy seams: `holds` means a signed tip vouches for every
//! record. Real ML-DSA-65, unions and per-sync sealing are in dowiz-hub `tests/k3_union.rs`.

use super::{chains, unsealed, verify, Report, Trust};
use crate::doc::frontier::Frontiers;
use crate::doc::seal::{read_heads, stage_heads, Head};
use crate::doc::seg::{Frontier, Segment};
use crate::doc::tests::{doc_with, key_of, mix, Dev, ToyIds, ToySigner, ToyVerifier};
use crate::doc::write::{stage_doc, Refs};
use crate::doc::{Doc, Key};
use crate::{EvLog, Record, Store};

fn rep(st: &Store) -> Report {
    verify(st, &ToyIds, &ToyVerifier, Trust::Genesis).unwrap()
}

/// `r` appended to the log with no seal; HEADS, KEYS, the seals untouched. No key used.
fn forge_record(st: &mut Store, r: &Record) {
    let v = Doc::open(st).unwrap();
    let mut tx = st.begin().unwrap();
    let log = EvLog::stage_append_in(st, &mut tx, v.log.unwrap(), r, None).unwrap();
    let d = stage_doc(st, &mut tx, v.flags, Refs { log: Some(log), heads: v.heads, keys: v.keys, ..Refs::default() }).unwrap();
    st.commit_bytes(&tx, d);
}

/// HEADS rewritten; nothing is signed, so this needs no key either.
fn forge_heads(st: &mut Store, rows: &mut Vec<Head>) {
    let v = Doc::open(st).unwrap();
    let mut tx = st.begin().unwrap();
    let h = stage_heads(st, &mut tx, rows).unwrap();
    let d = stage_doc(st, &mut tx, v.flags, Refs { log: v.log, heads: Some(h), keys: v.keys, ..Refs::default() }).unwrap();
    st.commit_bytes(&tx, d);
}

/// The positive twin: every record sealed, each device on its own chain.
#[test]
fn every_sealed_record_is_vouched_for() {
    let (st, _) = doc_with(3);
    let r = rep(&st);
    assert_eq!(r.counters.unsealed, 0);
    assert!(r.holds());
}

#[test]
fn a_record_past_the_last_seal_is_unsealed() {
    let (mut st, mut a) = doc_with(3);
    let r = a.next(b"forged");
    forge_record(&mut st, &r);
    let rep = rep(&st);
    assert_eq!((rep.counters.chained, rep.counters.broken, rep.counters.seals_bad, rep.counters.unsealed), (4, 0, 0, 1));
    assert!(!rep.holds());
}

#[test]
fn a_stranger_chain_is_unsealed_whole() {
    let (mut st, _) = doc_with(2);
    let mut c = Dev::new(ToySigner { id: [0xC3; 32] });
    for p in [&b"x"[..], b"y", b"z"] {
        let r = c.next(p);
        forge_record(&mut st, &r);
    }
    let rep = rep(&st);
    assert_eq!(rep.counters.unsealed, 3);
    assert!(!rep.holds());
}

/// THE HEADS ROW IS NOT THE SIGNATURE: a row for a stranger names his record, no seal behind it.
/// `seals()` walks nothing for that row, so `seals_bad` stays 0 and only `unsealed` speaks.
#[test]
fn a_heads_row_vouches_for_nothing() {
    let (mut st, _) = doc_with(2);
    let mut c = Dev::new(ToySigner { id: [0xC3; 32] });
    let r = c.next(b"forged");
    forge_record(&mut st, &r);
    let v = Doc::open(&st).unwrap();
    let mut rows = read_heads(&st, v.heads);
    rows.push(Head { actor_id: c.s.id, seq: 1, tip: r.id, seal: None });
    forge_heads(&mut st, &mut rows);
    let rep = rep(&st);
    assert_eq!((rep.counters.seals_bad, rep.counters.forks, rep.counters.unsealed), (0, 0, 1), "{:?}", rep.counters);
    assert!(!rep.holds());
}

/// A dropped row: the seals are still in the file and nothing reaches them. No seal is bad, no
/// record broken, the count of records stays: `unsealed` alone catches it.
#[test]
fn a_dropped_heads_row_is_caught_by_unsealed_alone() {
    let mut st = Store::create_bytes(1 << 20).unwrap();
    let (mut a, mut b) = (Dev::new(ToySigner { id: [0xA1; 32] }), Dev::new(ToySigner { id: [0xB2; 32] }));
    let keys: Vec<Key> = vec![key_of(&a.s), key_of(&b.s)];
    Doc::init_bytes(&mut st, 0, &keys).unwrap();
    a.commit(&mut st, b"a", None).unwrap();
    b.commit(&mut st, b"b", None).unwrap();
    assert!(rep(&st).holds());
    let v = Doc::open(&st).unwrap();
    let mut rows = read_heads(&st, v.heads);
    rows.retain(|h| h.actor_id != b.s.id);
    forge_heads(&mut st, &mut rows);
    let r = rep(&st);
    let c = r.counters;
    assert_eq!((c.records, c.broken, c.seals_ok, c.seals_bad, c.forks, c.unsealed), (2, 0, 1, 0, 0, 1), "{c:?}");
    assert!(!r.holds());
}

/// A record gone from UNDER a signed tip: the walk leaves the file at an id that is neither
/// 0^32 nor a frontier tip. With a frontier row at that id the same walk ends clean.
#[test]
fn a_chain_ends_at_zero_or_at_a_frontier_tip() {
    let (mut st, a) = doc_with(0);
    let lost = [7u8; 32];
    let id = mix(&[&lost, b"second"]);
    forge_record(&mut st, &Record { id, prev: lost, actor_pubkey: a.s.id, actor_seq: 2, flags: 0, payload: b"second".to_vec() });
    let ch = chains(&st, Doc::open(&st).unwrap().log);
    assert_eq!(unsealed(&ch, &[id], &Frontiers::of(&[]), &|_| false), 1, "reached, and the walk ended on an unknown id");
    let row = Frontier { actor_id: a.s.id, seq: 1, tip: lost, scheme: 0 };
    let seg = Segment { first: 0, last: 0, root: [0; 32], prev: None, prev_root: [0; 32], blob: None, redacted: 0, declared: 0, seg_hash: [0; 32], frontier: vec![row] };
    assert_eq!(unsealed(&ch, &[id], &Frontiers::of(&[seg]), &|_| false), 0);
    assert_eq!(unsealed(&ch, &[], &Frontiers::of(&[]), &|_| false), 1, "no tip: the record itself is unsealed");
}

/// The reader that judges nothing walks from every seal: on an honest file it agrees, and the
/// forged record is unsealed to it as well. `holds` stays false there for its own reason.
#[test]
fn a_blind_reader_walks_from_every_seal() {
    struct Blind;
    impl crate::doc::Verifier for Blind {
        fn verify(&self, _: i64, _: &[u8], _: &[u8], _: &[u8]) -> bool {
            false
        }
        fn checks_signatures(&self) -> bool {
            false
        }
    }
    let (mut st, mut a) = doc_with(3);
    assert_eq!(verify(&st, &ToyIds, &Blind, Trust::Genesis).unwrap().counters.unsealed, 0);
    let r = a.next(b"forged");
    forge_record(&mut st, &r);
    assert_eq!(verify(&st, &ToyIds, &Blind, Trust::Genesis).unwrap().counters.unsealed, 1);
}

/// A record object shorter than its head reads its id from whatever follows it: refused by name.
#[test]
fn a_record_shorter_than_its_head_is_refused() {
    let (mut st, _) = doc_with(2);
    assert!(verify(&st, &ToyIds, &ToyVerifier, Trust::Genesis).is_ok(), "the twin");
    let o = EvLog::objs_at(&st, Doc::open(&st).unwrap().log)[1];
    st.cells[o] = (st.cells[o] & !0xFFFF_FFFF) | 5;
    st.seal(o);
    assert!(matches!(verify(&st, &ToyIds, &ToyVerifier, Trust::Genesis), Err(crate::doc::DocError::Corrupt("the log chain never ends"))));
}

/// A sum of cells is one number on every width: saturating at `i64::MAX`, never wrapping.
#[test]
fn a_sum_of_cells_saturates_at_i64_max() {
    assert_eq!(super::sum_of(&[u64::MAX, 1]), i64::MAX as u64);
    assert_eq!(super::sum_of(&[i64::MAX as u64, 1]), i64::MAX as u64);
    assert_eq!(super::sum_of(&[2, 3]), 5);
}
