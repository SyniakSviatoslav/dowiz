//! The messages come back exactly; every cut, every extra byte and every lying count is refused by
//! name, and a count past the budget is refused before anything is allocated for it.

use super::*;
use crate::toys::*;
use bebop_image::Store;
use bebop_log::doc::{Doc, KIND_PLATFORM_TSA};

fn a_tail() -> Tail {
    let (a, b) = (toy(A), toy(B));
    let g = genesis(&[&a, &b]);
    let base = Store::from_bytes(&g);
    let mut st = Store::from_bytes(&g);
    write(&mut st, &b, b"one", None);
    write(&mut st, &b, b"two", None);
    let over = bebop_log::EvLog::tip_at(&st, Doc::open(&st).unwrap().log).unwrap();
    Doc::anchor_bytes(&mut st, KIND_PLATFORM_TSA, &over, 9, b"proof").unwrap();
    Doc::tail_of(&st, &Doc::front(&base, &ToyIds).unwrap(), &ToyIds).unwrap()
}

/// `at`/`prev` are offsets of one file and do not travel.
fn plain(mut t: Tail) -> Tail {
    for s in &mut t.seals {
        (s.at, s.prev, s.time) = (0, None, None);
    }
    for a in &mut t.anchors {
        (a.at, a.prev) = (0, None);
    }
    t
}

fn bytes_of(t: &Tail) -> Vec<u8> {
    encode_tail(t, &Budget::default()).unwrap()
}

#[test]
fn every_message_comes_back_exactly() {
    let t = plain(a_tail());
    assert_eq!((t.recs.len(), t.seals.len(), t.anchors.len()), (2, 2, 1));
    let b = Budget::default();
    assert_eq!(decode(&bytes_of(&t), &b, &ToyIds), Ok(Msg::Tail(t.clone())));
    assert_eq!(decode(&encode_frontier(&t.front), &b, &ToyIds), Ok(Msg::Frontier(t.front.clone())));
    assert_eq!(decode(&encode_ack(&t.front), &b, &ToyIds), Ok(Msg::Ack(t.front.clone())));
    for n in [Need::Archive { seg_root: [3; 32] }, Need::Refused { name: "Fork".to_string() }] {
        assert_eq!(decode(&encode_need(&n), &b, &ToyIds), Ok(Msg::Need(n)));
    }
}

#[test]
fn a_cut_message_is_short_and_an_extra_byte_is_trailing() {
    let m = bytes_of(&a_tail());
    let b = Budget::default();
    for n in MARK.len() + 1..m.len() {
        assert!(matches!(decode(&m[..n], &b, &ToyIds), Err(SyncError::Short(_))), "cut at {n}");
    }
    let mut long = m.clone();
    long.push(0);
    assert_eq!(decode(&long, &b, &ToyIds), Err(SyncError::Trailing));
}

#[test]
fn a_foreign_mark_or_kind_is_refused() {
    let m = bytes_of(&a_tail());
    let b = Budget::default();
    let mut x = m.clone();
    x[0] ^= 1;
    assert_eq!(decode(&x, &b, &ToyIds), Err(SyncError::Mark("message")));
    let mut x = m;
    x[MARK.len()] = 9;
    assert_eq!(decode(&x, &b, &ToyIds), Err(SyncError::Mark("kind")));
}

/// Offset of the records' count: mark, kind, doc, places, four hashes.
fn records_at(t: &Tail) -> usize {
    MARK.len() + 1 + 32 + 8 + t.front.places.len() * 104 + 32 * 4
}

#[test]
fn a_lying_count_is_refused_before_it_is_believed() {
    let t = a_tail();
    let at = records_at(&t);
    let m = bytes_of(&t);
    let b = Budget::default();
    let mut x = m.clone();
    x[at..at + 8].copy_from_slice(&u64::MAX.to_le_bytes());
    assert_eq!(decode(&x, &b, &ToyIds), Err(SyncError::Over { what: "records", got: u64::MAX, max: b.records }));
    let mut x = m;
    x[at..at + 8].copy_from_slice(&b.records.to_le_bytes());
    assert_eq!(decode(&x, &b, &ToyIds), Err(SyncError::Short("records")), "under the budget, past the bytes");
}

#[test]
fn every_budget_is_a_refusal_with_a_positive_twin() {
    let t = a_tail();
    let m = bytes_of(&t);
    let cases: [(&str, Budget); 5] = [
        ("message", Budget { message: m.len() - 1, ..Budget::default() }),
        ("records", Budget { records: 1, ..Budget::default() }),
        ("seals", Budget { seals: 1, ..Budget::default() }),
        ("anchors", Budget { anchors: 0, ..Budget::default() }),
        ("anchor", Budget { proof_bytes: 4, ..Budget::default() }),
    ];
    for (what, b) in cases {
        assert!(matches!(decode(&m, &b, &ToyIds), Err(SyncError::Over { what: w, .. }) if w == what), "{what}");
    }
    assert!(decode(&m, &Budget { message: m.len(), ..Budget::default() }, &ToyIds).is_ok());
}

/// The sender refuses what no peer would read, by the same names.
#[test]
fn the_encoder_keeps_the_budget_too() {
    let t = a_tail();
    assert!(matches!(encode_tail(&t, &Budget { records: 1, ..Budget::default() }), Err(SyncError::Over { what: "records", .. })));
    assert!(matches!(encode_tail(&t, &Budget { message: 10, ..Budget::default() }), Err(SyncError::Over { what: "message", .. })));
}

#[test]
fn eight_admits_pass_and_the_ninth_is_refused() {
    let (a, b) = (toy(A), toy(B));
    let g = genesis(&[&a, &b]);
    let base = Doc::front(&Store::from_bytes(&g), &ToyIds).unwrap();
    let mut st = Store::from_bytes(&g);
    for i in 0..9u8 {
        let op = KeyOp::Admit(key_of([i + 1; 32]));
        write(&mut st, &b, &op.body(), Some(&op));
    }
    let m = bytes_of(&Doc::tail_of(&st, &base, &ToyIds).unwrap());
    let keys = Budget { keys: 8, ..Budget::default() };
    assert_eq!(decode(&m, &keys, &ToyIds), Err(SyncError::Over { what: "keys", got: 9, max: 8 }));
    assert!(decode(&m, &Budget { keys: 9, ..Budget::default() }, &ToyIds).is_ok());
}

use bebop_log::doc::KeyOp;
