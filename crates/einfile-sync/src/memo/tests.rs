//! A signature that verified is not asked again, a forged one is asked every time, and the answers
//! are the real verifier's answers.

use super::*;
use crate::toys::*;
use einfile_log::doc::Signer;

#[test]
fn a_good_signature_is_asked_once() {
    let s = ToySigner { id: A };
    let sig = s.sign(b"m");
    let memo = Memo::new(&V, &IDS);
    assert!(memo.verify(ALG_TOY, &A, b"m", &sig));
    assert!(memo.verify(ALG_TOY, &A, b"m", &sig));
    assert_eq!(memo.calls(), 1);
}

#[test]
fn a_forged_signature_is_asked_every_time_and_stays_false() {
    let memo = Memo::new(&V, &IDS);
    assert!(!memo.verify(ALG_TOY, &A, b"m", b"forged"));
    assert!(!memo.verify(ALG_TOY, &A, b"m", b"forged"));
    assert_eq!(memo.calls(), 2);
}

/// One field's bytes moved into the next is another question.
#[test]
fn the_fields_cannot_trade_bytes() {
    let s = ToySigner { id: A };
    let sig = s.sign(b"mm");
    let memo = Memo::new(&V, &IDS);
    assert!(memo.verify(ALG_TOY, &A, b"mm", &sig));
    assert!(!memo.verify(ALG_TOY, &A, b"m", &[b"m".as_slice(), &sig].concat()));
    assert!(!memo.verify(ALG_TOY + 1, &A, b"mm", &sig));
}

/// A whole session through it: the union's `verify` pays only for the seals the tail brought.
#[test]
fn the_union_pays_only_for_what_is_new() {
    let (a, b) = (toy(A), toy(B));
    let g = genesis(&[&a, &b]);
    let mut pa = peer(&g, &a);
    let mut st = einfile_image::Store::from_bytes(&g);
    write(&mut st, &b, b"new", None);
    let t = einfile_log::doc::Doc::tail_of(&st, &pa.front().unwrap(), &ToyIds).unwrap();
    let memo = Memo::new(&V, &IDS);
    pa.v = &memo;
    pa.adopt(&t).unwrap();
    let seals = report(&pa.store).counters.seals_ok as u64;
    assert_eq!(memo.calls(), seals, "each distinct seal once, across two `verify`s");
}
