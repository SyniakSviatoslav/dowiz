//! KILL TEST K-T1 FIRST (docs/design/MESH-TRANSPORT-OVER-EINFILE.md §14): for every fixture, the bytes
//! of `tail_into(ours, tail(theirs))` are the bytes of `merge_into(ours, theirs)`, in both directions,
//! and a refusal of one is the same refusal of the other. `Err` on the wrong side, or one byte apart:
//! there cannot be two doors. Real ML-DSA: `crates/dowiz-hub/tests/k_t1_tail.rs`.

use super::super::anch::KIND_PLATFORM_TSA;
use super::super::keys::{revoke_body, KeyOp};
use super::super::merge::Union;
use super::super::merge_body::{merge_body, MergeView, STATE_COMMUTES};
use super::super::seg::forgotten_body;
use super::super::tests::{doc_with, key_of, repair_edit, tombs_body, Dev, ToyIds, ToySigner, ToyVerifier};
use super::super::verify::{verify, Trust};
use super::super::write::{stage_doc, Refs};
use super::super::{Doc, DocError, Front, Ids, Limits, Signer, Tail};
use crate::{EvLog, Record, Store};
use core::mem::discriminant;

const SIZE: usize = 1 << 21;

fn fresh() -> Store {
    Store::create_bytes(SIZE).unwrap()
}

/// Genesis A, two records, ADMIT of B: the split (`merge/tests.rs`'s).
fn split() -> (Store, Store, Dev<ToySigner>, Dev<ToySigner>) {
    let (mut sa, mut a) = doc_with(2);
    let mut b = Dev::new(ToySigner { id: [0xB2; 32] });
    let op = KeyOp::Admit(key_of(&b.s));
    a.commit(&mut sa, &op.body(), Some(&op)).unwrap();
    let bytes = sa.to_bytes();
    b.tip = EvLog::tip_at(&sa, Doc::open(&sa).unwrap().log).unwrap();
    (Store::from_bytes(&bytes), Store::from_bytes(&bytes), a, b)
}

fn by_merge(ours: &Store, theirs: &Store) -> (Store, Result<Union, DocError>) {
    let mut f = fresh();
    let u = Doc::merge_into(ours, theirs, &mut f, &ToyIds, &ToyVerifier, Trust::Genesis);
    (f, u)
}

fn by_tail(ours: &Store, theirs: &Store) -> (Store, Result<Union, DocError>) {
    let mut f = fresh();
    let u = Doc::front(ours, &ToyIds).and_then(|at| Doc::tail_of(theirs, &at, &ToyIds)).and_then(|t| Doc::tail_into(ours, &t, &mut f, &ToyIds, &ToyVerifier, Trust::Genesis));
    (f, u)
}

/// K-T1, one direction.
fn one_door(ours: &Store, theirs: &Store, what: &str) {
    let ((fm, um), (ft, ut)) = (by_merge(ours, theirs), by_tail(ours, theirs));
    match (um, ut) {
        (Ok(m), Ok(t)) => {
            assert_eq!(m, t, "{what}: the Union");
            assert_eq!(fm.to_bytes(), ft.to_bytes(), "{what}: the bytes");
        }
        (Err(m), Err(t)) => assert_eq!(discriminant(&m), discriminant(&t), "{what}: {m:?} against {t:?}"),
        (m, t) => panic!("{what}: the doors differ: merge {m:?} tail {t:?}"),
    }
}

fn k_t1(a: &Store, b: &Store, what: &str) {
    one_door(a, b, what);
    one_door(b, a, what);
}

#[test]
fn k_t1_nothing_to_do() {
    let (sa, sb, ..) = split();
    k_t1(&sa, &sb, "equal files");
    let (u, r) = by_tail(&sa, &sb);
    assert!(!r.unwrap().wrote);
    assert_eq!(u.to_bytes(), fresh().to_bytes(), "wrote false leaves fresh as it was");
}

#[test]
fn k_t1_one_side_wrote() {
    let (mut sa, sb, mut a, _) = split();
    a.commit(&mut sa, b"a1", None).unwrap();
    a.commit(&mut sa, b"a2", None).unwrap();
    k_t1(&sa, &sb, "ours ahead / theirs ahead");
}

#[test]
fn k_t1_both_wrote() {
    let (mut sa, mut sb, mut a, mut b) = split();
    a.commit(&mut sa, b"a1", None).unwrap();
    b.commit(&mut sb, b"b1", None).unwrap();
    b.commit(&mut sb, b"b2", None).unwrap();
    k_t1(&sa, &sb, "two histories");
    let (_, u) = by_tail(&sa, &sb);
    let u = u.unwrap();
    assert!(u.wrote && u.both_wrote);
    assert_eq!((u.our_tail.len(), u.their_tail.len()), (1, 2));
}

#[test]
fn k_t1_one_seal_over_many_records() {
    let (sa, mut sb, _, mut b) = split();
    let recs = vec![b.next(b"m1"), b.next(b"m2"), b.next(b"m3")];
    Doc::commit_many_bytes(&mut sb, &recs, &b.s, 5).unwrap();
    k_t1(&sa, &sb, "one seal per sync");
}

/// New keys land in their table's order, whatever order the ADMITs came in.
#[test]
fn k_t1_keys_admitted_in_the_tail() {
    let (sa, mut sb, _, mut b) = split();
    for id in [0xF1u8, 0xC3, 0xD7] {
        let op = KeyOp::Admit(key_of(&ToySigner { id: [id; 32] }));
        b.commit(&mut sb, &op.body(), Some(&op)).unwrap();
    }
    k_t1(&sa, &sb, "three ADMITs");
}

#[test]
fn k_t1_a_new_device_writes_in_the_tail() {
    let (sa, mut sb, _, mut b) = split();
    let mut c = Dev::new(ToySigner { id: [0xC3; 32] });
    let op = KeyOp::Admit(key_of(&c.s));
    b.commit(&mut sb, &op.body(), Some(&op)).unwrap();
    c.tip = b.tip;
    c.commit(&mut sb, b"c1", None).unwrap();
    k_t1(&sa, &sb, "a device only theirs knows");
}

/// A REVOKE in the tail folds into the KEY row, or the union's table disagrees.
#[test]
fn k_t1_a_revoke_in_the_tail() {
    let (mut sa, mut sb, mut a, mut b) = split();
    b.commit(&mut sb, b"b1", None).unwrap();
    let (mut u, _) = by_merge(&sa, &sb);
    let tip = b.tip;
    let op = KeyOp::Revoke { actor_id: b.s.id, after_seq: 1, tip };
    a.commit(&mut u, &op.body(), Some(&op)).unwrap();
    k_t1(&u, &sb, "REVOKE on one side, nothing past it");
    let op = KeyOp::Revoke { actor_id: b.s.id, after_seq: 0, tip: [0; 32] };
    a.seq = 3;
    a.tip = EvLog::tip_at(&sa, Doc::open(&sa).unwrap().log).unwrap();
    a.commit(&mut sa, &op.body(), Some(&op)).unwrap();
    k_t1(&sa, &sb, "RevokedTail");
    let (_, r) = by_tail(&sa, &sb);
    assert!(matches!(r, Err(DocError::RevokedTail { .. })), "{r:?}");
}

#[test]
fn k_t1_anchors() {
    let (mut sa, mut sb, _, _) = split();
    let over = EvLog::tip_at(&sa, Doc::open(&sa).unwrap().log).unwrap();
    Doc::anchor_bytes(&mut sb, KIND_PLATFORM_TSA, &over, 7, b"theirs").unwrap();
    k_t1(&sa, &sb, "an anchor on one side");
    Doc::anchor_bytes(&mut sa, KIND_PLATFORM_TSA, &over, 8, b"ours").unwrap();
    k_t1(&sa, &sb, "one anchor each: equal counts, and neither may be lost");
}

/// The same anchor on both sides lands once: a shared one beside a new one.
#[test]
fn k_t1_a_shared_anchor_lands_once() {
    let (mut st, _) = doc_with(2);
    let over = EvLog::tip_at(&st, Doc::open(&st).unwrap().log).unwrap();
    Doc::anchor_bytes(&mut st, KIND_PLATFORM_TSA, &over, 7, b"shared").unwrap();
    let older = Store::from_bytes(&st.to_bytes());
    Doc::anchor_bytes(&mut st, KIND_PLATFORM_TSA, &over, 8, b"new").unwrap();
    k_t1(&st, &older, "one shared, one new");
}

#[test]
fn k_t1_a_fork() {
    let (mut sa, mut sb, mut a, _) = split();
    let (seq, tip) = (a.seq, a.tip);
    a.commit(&mut sa, b"this", None).unwrap();
    a.commit(&mut sa, b"longer", None).unwrap();
    (a.seq, a.tip) = (seq, tip);
    a.commit(&mut sb, b"that", None).unwrap();
    k_t1(&sa, &sb, "one device, two histories, the short side too");
    let r = Doc::tail_of(&sb, &Doc::front(&sa, &ToyIds).unwrap(), &ToyIds);
    assert!(r.is_ok(), "the short side has nothing to compare: the receiver names it, by the row");
    let r = Doc::tail_of(&sa, &Doc::front(&sb, &ToyIds).unwrap(), &ToyIds);
    assert!(matches!(r, Err(DocError::Fork { .. })), "the long side names it before a byte: {r:?}");
}

/// One record, two seals: equal rows, and only the fold sees it. Then the same under a later seal.
#[test]
fn k_t1_a_seal_fork() {
    let (mut sa, mut sb, mut a, _) = split();
    let r = a.next(b"twice");
    Doc::commit_bytes(&mut sa, &r, &a.s, 1, None).unwrap();
    Doc::commit_bytes(&mut sb, &r, &a.s, 2, None).unwrap();
    k_t1(&sa, &sb, "equal rows");
    let r = Doc::tail_of(&sa, &Doc::front(&sb, &ToyIds).unwrap(), &ToyIds);
    assert!(matches!(r, Err(DocError::SealFork { .. })), "the sender names it too: {r:?}");
    a.commit(&mut sa, b"past it", None).unwrap();
    k_t1(&sa, &sb, "the fork below the top");
}

#[test]
fn k_t1_a_key_clash() {
    let (mut sa, mut sb, mut a, mut b) = split();
    let k1 = key_of(&ToySigner { id: [0xC3; 32] });
    let k2 = super::super::keys::Key { pk: vec![1, 2, 3], ..k1.clone() };
    a.commit(&mut sa, &KeyOp::Admit(k1.clone()).body(), Some(&KeyOp::Admit(k1))).unwrap();
    b.commit(&mut sb, &KeyOp::Admit(k2.clone()).body(), Some(&KeyOp::Admit(k2))).unwrap();
    k_t1(&sa, &sb, "KeyClash");
}

#[test]
fn k_t1_a_forgotten_on_one_side() {
    let (mut sa, sb, mut a, _) = split();
    a.commit(&mut sa, &forgotten_body(&[0x5B; 32], &[]), None).unwrap();
    k_t1(&sa, &sb, "ForgetOneSide");
    let r = Doc::tail_of(&sa, &Doc::front(&sb, &ToyIds).unwrap(), &ToyIds);
    assert_eq!(r.err(), Some(DocError::ForgetOneSide), "not one byte is built for it");
}

#[test]
fn k_t1_a_cut_on_one_side() {
    let (old, mut a) = doc_with(5);
    let mut cut = fresh();
    let seg = Doc::cut_into(&old, &mut cut, 2, &ToyIds).unwrap();
    a.commit(&mut cut, &seg.truncate_body(), None).unwrap();
    k_t1(&cut, &old, "CutOneSide");
}

#[test]
fn k_t1_both_sides_cut_alike() {
    let (old, mut a) = doc_with(5);
    let mut cut = fresh();
    let seg = Doc::cut_into(&old, &mut cut, 2, &ToyIds).unwrap();
    a.commit(&mut cut, &seg.truncate_body(), None).unwrap();
    let older = Store::from_bytes(&cut.to_bytes());
    a.commit(&mut cut, b"past the cut", None).unwrap();
    k_t1(&cut, &older, "one segment chain");
}

#[test]
fn k_t1_another_document() {
    let (sa, ..) = split();
    let mut other = fresh();
    let mut e = Dev::new(ToySigner { id: [0xEE; 32] });
    Doc::init_bytes(&mut other, 0, &[key_of(&e.s)]).unwrap();
    e.commit(&mut other, b"elsewhere", None).unwrap();
    k_t1(&sa, &other, "ForeignRoot");
}

/// THE ONE CHECK THIS DOOR DOES NOT HAVE: `theirs` alone. What must still hold: nothing false lands.
#[test]
fn a_sender_that_does_not_hold_lands_nothing_false() {
    for k in [0usize, 3, 4] {
        let (sa, mut sb, _, mut b) = split();
        b.commit(&mut sb, b"b1", None).unwrap();
        b.commit(&mut sb, b"b2", None).unwrap();
        repair_edit(&mut sb, k);
        assert_eq!(by_merge(&sa, &sb).1.err(), Some(DocError::NotWhole { ours: false }), "control {k}");
        let (f, u) = by_tail(&sa, &sb);
        match u {
            Err(_) => {}
            Ok(u) if !u.wrote => {}
            Ok(_) => assert!(verify(&f, &ToyIds, &ToyVerifier, Trust::Genesis).unwrap().holds(), "{k}"),
        }
    }
}

/// A tail built for an older position of ours lands the same: the overlap is dropped by id.
#[test]
fn a_stale_tail_lands_the_same() {
    let (mut sa, _, mut a, _) = split();
    let old: Front = Doc::front(&sa, &ToyIds).unwrap();
    a.commit(&mut sa, b"a1", None).unwrap();
    let mut sb = Store::from_bytes(&sa.to_bytes());
    a.commit(&mut sb, b"a2", None).unwrap();
    let t = Doc::tail_of(&sb, &old, &ToyIds).unwrap();
    assert_eq!((t.recs.len(), t.seals.len()), (2, 2), "built for the older place, no seal it held");
    let mut f = fresh();
    let u = Doc::tail_into(&sa, &t, &mut f, &ToyIds, &ToyVerifier, Trust::Genesis).unwrap();
    assert_eq!(u.their_tail.len(), 1);
    let (m, _) = by_merge(&sa, &sb);
    assert_eq!(f.to_bytes(), m.to_bytes());
}

/// A hole in a tail is named by the seq it wanted: the sender built it for a place we never held.
#[test]
fn a_tail_with_a_hole_is_refused_by_seq() {
    let (sa, mut sb, _, mut b) = split();
    b.commit(&mut sb, b"b1", None).unwrap();
    b.commit(&mut sb, b"b2", None).unwrap();
    let mut t: Tail = Doc::tail_of(&sb, &Doc::front(&sa, &ToyIds).unwrap(), &ToyIds).unwrap();
    assert_eq!(t.recs.len(), 2);
    t.recs.remove(0);
    let r = Doc::tail_into(&sa, &t, &mut fresh(), &ToyIds, &ToyVerifier, Trust::Genesis);
    assert_eq!(r.err(), Some(DocError::UnionNotWhole), "B had no record here: its first is the union's to judge");
    let (mut sb2, mut b2) = (Store::from_bytes(&sb.to_bytes()), b);
    let base = Doc::front(&sb2, &ToyIds).unwrap();
    b2.commit(&mut sb2, b"b3", None).unwrap();
    b2.commit(&mut sb2, b"b4", None).unwrap();
    let mut t = Doc::tail_of(&sb2, &base, &ToyIds).unwrap();
    t.recs.remove(0);
    let r = Doc::tail_into(&sb, &t, &mut fresh(), &ToyIds, &ToyVerifier, Trust::Genesis);
    assert_eq!(r.err(), Some(DocError::Seq { want: 3, got: 4 }));
}

/// A signed record with a bit moved has another id: refused, and the positive twin lands.
#[test]
fn a_changed_payload_is_refused_and_the_twin_lands() {
    let (sa, mut sb, _, mut b) = split();
    b.commit(&mut sb, b"b1", None).unwrap();
    let t = Doc::tail_of(&sb, &Doc::front(&sa, &ToyIds).unwrap(), &ToyIds).unwrap();
    assert!(Doc::tail_into(&sa, &t, &mut fresh(), &ToyIds, &ToyVerifier, Trust::Genesis).unwrap().wrote);
    let mut bad = t;
    bad.recs[0].payload[0] ^= 1;
    let r = Doc::tail_into(&sa, &bad, &mut fresh(), &ToyIds, &ToyVerifier, Trust::Genesis);
    assert!(matches!(r, Err(DocError::LegacyTail { .. })), "{r:?}");
}

/// A PART IS A POSITION: cut on a seal's edge, its rows name what landed, and parts sum to the whole.
#[test]
fn parts_land_as_positions_and_sum_to_the_whole() {
    let (mut sa, sb, mut a, _) = split();
    for i in 0..6 {
        a.commit(&mut sa, format!("a{i}").as_bytes(), None).unwrap();
    }
    let lim = Limits { records: 2, seals: 2, bytes: 1 << 20 };
    let mut at = sb;
    let mut parts = 0;
    loop {
        let t = Doc::tail_part(&sa, &Doc::front(&at, &ToyIds).unwrap(), &ToyIds, &lim).unwrap();
        if t.is_empty() {
            break;
        }
        assert!(t.recs.len() <= 2 && t.seals.len() <= 2, "{} {}", t.recs.len(), t.seals.len());
        let mut f = fresh();
        let u = Doc::tail_into(&at, &t, &mut f, &ToyIds, &ToyVerifier, Trust::Genesis).unwrap();
        assert!(u.wrote);
        at = f;
        parts += 1;
    }
    assert_eq!(parts, 3);
    assert_eq!(Doc::front(&at, &ToyIds).unwrap(), Doc::front(&sa, &ToyIds).unwrap());
}

/// ONE SYNC PAST THE LIMITS has no edge: it comes back whole, for the encoder to name.
#[test]
fn one_seal_over_more_than_a_part_stays_whole() {
    let (sa, mut sb, _, mut b) = split();
    let recs = vec![b.next(b"m1"), b.next(b"m2"), b.next(b"m3")];
    Doc::commit_many_bytes(&mut sb, &recs, &b.s, 5).unwrap();
    let lim = Limits { records: 2, seals: 2, bytes: 1 << 20 };
    let t = Doc::tail_part(&sb, &Doc::front(&sa, &ToyIds).unwrap(), &ToyIds, &lim).unwrap();
    assert_eq!(t.recs.len(), 3);
}

fn chained(prev: [u8; 32], actor: [u8; 32], seq: u64, payload: &[u8]) -> Record {
    let mut r = Record { id: [0; 32], prev, actor_pubkey: actor, actor_seq: seq, flags: 0, payload: payload.to_vec() };
    r.id = ToyIds.chained(&r);
    r
}

/// A record in the log and nothing else: no seal, no row. A forger's write.
fn forge_record(st: &mut Store, r: &Record) {
    let v = Doc::open(st).unwrap();
    let mut tx = st.begin().unwrap();
    let log = EvLog::stage_append_in(st, &mut tx, v.log.unwrap(), r, None).unwrap();
    let d = stage_doc(st, &mut tx, v.flags, Refs { log: Some(log), heads: v.heads, keys: v.keys, seg: v.seg, ..Refs::default() }).unwrap();
    st.commit_bytes(&tx, d);
}

/// `lean_tests`' fixture: each side holds and the union does not, and only its `verify` sees it.
#[test]
fn k_t1_a_key_on_one_side_undoes_the_other_sides_stranger() {
    for seq in [7u64, 1] {
        let (mut sa, mut sb, mut a, mut b) = split();
        let tip = EvLog::tip_at(&sb, Doc::open(&sb).unwrap().log).unwrap();
        let x = chained(tip, [0xC3; 32], seq, b"stranger");
        forge_record(&mut sb, &x);
        b.tip = x.id;
        b.commit(&mut sb, b"theirs", None).unwrap();
        let op = KeyOp::Admit(key_of(&ToySigner { id: [0xC3; 32] }));
        a.commit(&mut sa, &op.body(), Some(&op)).unwrap();
        k_t1(&sa, &sb, "UnionNotWhole");
        assert_eq!(by_tail(&sa, &sb).1.err(), Some(DocError::UnionNotWhole), "seq {seq}");
    }
}

/// A tail of nothing but a MERGE owes none: that is what ends the answering.
#[test]
fn k_t1_a_tail_that_is_only_a_merge() {
    let (mut sa, mut sb, mut a, mut b) = split();
    a.commit(&mut sa, b"a1", None).unwrap();
    b.commit(&mut sb, b"b1", None).unwrap();
    let (mut u, found) = by_tail(&sa, &sb);
    let found = found.unwrap();
    let body = merge_body(&MergeView { ours: found.ours, theirs: found.theirs, state: STATE_COMMUTES, seg_root: found.seg_root, conflicts: Vec::new() });
    a.commit(&mut u, &body, None).unwrap();
    let copy = by_merge(&sb, &sa).0;
    k_t1(&u, &copy, "the MERGE travels");
    let r = by_tail(&copy, &u).1.unwrap();
    assert!(r.wrote && !r.both_wrote, "{r:?}");
}

/// PRE-SEAL RECORDS: an actor with no key and no row, shared from the split. One of
/// them reads like a REVOKE of B and declares nothing, here as in `merge_into`.
fn over_a_past() -> (Store, Store, Dev<ToySigner>, Dev<ToySigner>) {
    let mut a = Dev::new(ToySigner { id: [0xA1; 32] });
    let mut b = Dev::new(ToySigner { id: [0xB2; 32] });
    let one = chained([0; 32], [0; 32], 1, b"before the seal");
    let two = chained(one.id, [0; 32], 2, &revoke_body(&b.s.id, 0, &[0; 32]));
    let mut st = fresh();
    Doc::init_over_bytes(&mut st, 0, &[key_of(&a.s)], &[one, two.clone()]).unwrap();
    a.tip = two.id;
    let op = KeyOp::Admit(key_of(&b.s));
    a.commit(&mut st, &op.body(), Some(&op)).unwrap();
    b.tip = a.tip;
    let r = verify(&st, &ToyIds, &ToyVerifier, Trust::Genesis).unwrap();
    assert!(r.holds(), "the control: {:?}", r.counters);
    let bytes = st.to_bytes();
    (Store::from_bytes(&bytes), Store::from_bytes(&bytes), a, b)
}

#[test]
fn k_t1_pre_seal_records_stay_home() {
    let (mut sa, mut sb, mut a, mut b) = over_a_past();
    a.commit(&mut sa, b"a1", None).unwrap();
    b.commit(&mut sb, b"b1", None).unwrap();
    k_t1(&sa, &sb, "a shared pre-seal past, an unkeyed line that reads like a REVOKE");
    let u = by_tail(&sa, &sb).1.unwrap();
    assert_eq!((u.our_tail.len(), u.their_tail.len(), u.both_wrote), (1, 1, true), "no pre-seal record counts as a side's own");
    let t = Doc::tail_of(&sb, &Doc::front(&sa, &ToyIds).unwrap(), &ToyIds).unwrap();
    assert_eq!(t.recs.len(), 1, "and none travels");
}

/// One shared erasure: same tombstone, same FORGOTTEN, both past it.
#[test]
fn k_t1_an_erasure_both_sides_share() {
    let (old, mut a) = doc_with(3);
    let mut st = fresh();
    let target = EvLog::tip_at(&old, Doc::open(&old).unwrap().log).unwrap();
    let rule = move |r: &Record| (r.id == target).then(|| vec![0x80u8]);
    Doc::redact_into(&old, &mut st, &ToyIds, &rule).unwrap();
    let body = tombs_body(&st);
    a.commit(&mut st, &body, None).unwrap();
    let older = Store::from_bytes(&st.to_bytes());
    a.commit(&mut st, b"past the erasure", None).unwrap();
    k_t1(&st, &older, "a tombstone in the common past");
}

/// `ours` fails on its own: nothing is glued to it, by either door.
#[test]
fn k_t1_ours_does_not_hold() {
    let (mut sa, sb, a, _) = split();
    forge_record(&mut sa, &chained(a.tip, a.s.actor_id(), a.seq + 1, b"never sealed"));
    one_door(&sa, &sb, "NotWhole");
    assert_eq!(by_tail(&sa, &sb).1.err(), Some(DocError::NotWhole { ours: true }));
}

fn hand_tail(from: &Store, to: &Store) -> Tail {
    Doc::tail_of(from, &Doc::front(to, &ToyIds).unwrap(), &ToyIds).unwrap()
}

fn into(ours: &Store, t: &Tail) -> Result<Union, DocError> {
    Doc::tail_into(ours, t, &mut fresh(), &ToyIds, &ToyVerifier, Trust::Genesis)
}

/// A SENDER THAT SKIPS ITS OWN CHECKS: the receiver's `agree`, with no tail_of in the way.
#[test]
fn the_receiver_sees_a_seal_fork_on_its_own() {
    let (mut sa, mut sb, mut a, _) = split();
    let r = a.next(b"twice");
    Doc::commit_bytes(&mut sa, &r, &a.s, 1, None).unwrap();
    Doc::commit_bytes(&mut sb, &r, &a.s, 2, None).unwrap();
    let t = Tail { front: Doc::front(&sb, &ToyIds).unwrap(), recs: Vec::new(), seals: Vec::new(), anchors: Vec::new() };
    assert!(matches!(into(&sa, &t), Err(DocError::SealFork { .. })));
}

/// Seals past ours that do not fold to the sender's own place: a chain that was swapped under it.
#[test]
fn a_seal_chain_that_does_not_fold_to_its_place_is_refused() {
    let (sa, mut sb, _, mut b) = split();
    b.commit(&mut sb, b"b1", None).unwrap();
    let t = hand_tail(&sb, &sa);
    assert!(into(&sa, &t).unwrap().wrote, "the twin");
    let mut bad = t;
    bad.seals[0].claimed_ms += 1;
    assert_eq!(into(&sa, &bad).err(), Some(DocError::SealFork { actor: b.s.id, seq: 1 }));
}

/// A seal of theirs inside our chain at a seq we sealed differently, and one under our top.
#[test]
fn a_seal_inside_our_chain_is_the_same_seal_or_a_fork() {
    let (sa, mut sb, _, mut b) = split();
    b.commit(&mut sb, b"b1", None).unwrap();
    let older = Store::from_bytes(&sb.to_bytes());
    b.commit(&mut sb, b"b2", None).unwrap();
    let t = hand_tail(&sb, &sa);
    assert!(!into(&older, &hand_tail(&sb, &older)).unwrap().both_wrote);
    let mut stale = t.clone();
    stale.seals[0].claimed_ms += 1;
    assert_eq!(into(&older, &stale).err(), Some(DocError::SealFork { actor: b.s.id, seq: 1 }));
    assert!(into(&older, &t).unwrap().wrote, "the same seal is dropped, and the twin lands");
    let mut under = hand_tail(&sb, &older);
    under.front = Doc::front(&sb, &ToyIds).unwrap();
    under.seals[0].seq = 0;
    assert_eq!(into(&sb, &under).err(), Some(DocError::SealFork { actor: b.s.id, seq: 0 }));
}

/// One id, other bytes, in a tail: an erasure seen from one side, whatever the sender claims.
#[test]
fn other_bytes_under_one_id_are_an_erasure_on_one_side() {
    let (sa, mut sb, _, mut b) = split();
    b.commit(&mut sb, b"b1", None).unwrap();
    let older = Store::from_bytes(&sb.to_bytes());
    let mut t = hand_tail(&sb, &sa);
    t.recs[0].payload[0] ^= 1;
    assert_eq!(into(&older, &t).err(), Some(DocError::ForgetOneSide));
}

/// One record, another id, at a seq we hold: `Fork`, and past our end with a `prev` that is not our
/// end: `Fork` too, each with the twin that lands.
#[test]
fn a_tail_that_leaves_our_chain_is_a_fork() {
    let (sa, mut sb, _, mut b) = split();
    b.commit(&mut sb, b"b1", None).unwrap();
    let base = Store::from_bytes(&sb.to_bytes());
    b.commit(&mut sb, b"b2", None).unwrap();
    let t = hand_tail(&sb, &sa);
    let mut other = t.clone();
    other.recs[0].id = [9; 32];
    assert!(matches!(into(&base, &other), Err(DocError::Fork { seq: 1, .. })));
    let mut off = hand_tail(&sb, &base);
    off.recs[0].prev = [9; 32];
    off.recs[0].id = ToyIds.chained(&off.recs[0]);
    assert!(matches!(into(&base, &off), Err(DocError::Fork { seq: 1, theirs, .. }) if theirs == [9; 32]));
    assert!(into(&base, &hand_tail(&sb, &base)).unwrap().wrote);
}

/// A sender's rows must be made good: a row naming another record, a row past what landed.
#[test]
fn a_row_the_tail_does_not_make_good_is_refused() {
    let (sa, mut sb, _, mut b) = split();
    b.commit(&mut sb, b"b1", None).unwrap();
    let t = hand_tail(&sb, &sa);
    let at = t.front.places.iter().position(|p| p.row.actor_id == b.s.id).unwrap();
    let mut past = t.clone();
    past.front.places[at].row.seq = 2;
    assert_eq!(into(&sa, &past).err(), Some(DocError::Seq { want: 2, got: 2 }));
    let mut other = t.clone();
    other.front.places[at].row.tip = [9; 32];
    assert!(matches!(into(&sa, &other), Err(DocError::Fork { seq: 1, theirs, .. }) if theirs == [9; 32]));
    let mut none = t;
    none.recs.clear();
    none.seals.clear();
    assert_eq!(into(&sa, &none).err(), Some(DocError::Seq { want: 1, got: 1 }), "a row with no record behind it");
}

/// A cut part names only what it carries, so a MERGE over a part stands in `verify`.
#[test]
fn a_merge_over_a_part_holds() {
    let (mut sa, mut sb, mut a, mut b) = split();
    a.commit(&mut sa, b"a1", None).unwrap();
    for i in 0..4 {
        b.commit(&mut sb, format!("b{i}").as_bytes(), None).unwrap();
    }
    let lim = Limits { records: 2, seals: 2, bytes: 1 << 20 };
    let t = Doc::tail_part(&sb, &Doc::front(&sa, &ToyIds).unwrap(), &ToyIds, &lim).unwrap();
    let mut f = fresh();
    let u = Doc::tail_into(&sa, &t, &mut f, &ToyIds, &ToyVerifier, Trust::Genesis).unwrap();
    assert!(u.theirs.iter().any(|r| r.actor_id == b.s.id && r.seq == 2), "{:?}", u.theirs);
    let body = merge_body(&MergeView { ours: u.ours, theirs: u.theirs, state: STATE_COMMUTES, seg_root: u.seg_root, conflicts: Vec::new() });
    a.commit(&mut f, &body, None).unwrap();
    let r = verify(&f, &ToyIds, &ToyVerifier, Trust::Genesis).unwrap();
    assert!(r.holds() && r.counters.merges_bad == 0, "{:?}", r.counters);
}

/// A device the peer has never seen, not yet sent: its place stays out of the part.
#[test]
fn a_part_names_no_device_it_does_not_carry() {
    let (sa, mut sb, _, mut b) = split();
    for i in 0..3 {
        b.commit(&mut sb, format!("b{i}").as_bytes(), None).unwrap();
    }
    let mut c = Dev::new(ToySigner { id: [0xC3; 32] });
    let op = KeyOp::Admit(key_of(&c.s));
    b.commit(&mut sb, &op.body(), Some(&op)).unwrap();
    c.tip = b.tip;
    c.commit(&mut sb, b"c1", None).unwrap();
    let over = EvLog::tip_at(&sb, Doc::open(&sb).unwrap().log).unwrap();
    Doc::anchor_bytes(&mut sb, KIND_PLATFORM_TSA, &over, 7, b"waits for the last part").unwrap();
    let lim = Limits { records: 2, seals: 2, bytes: 1 << 20 };
    let t = Doc::tail_part(&sb, &Doc::front(&sa, &ToyIds).unwrap(), &ToyIds, &lim).unwrap();
    assert!(t.front.places.iter().all(|p| p.row.actor_id != c.s.id));
    assert!(t.anchors.is_empty() && !t.recs.is_empty());
    assert!(into(&sa, &t).unwrap().wrote);
}

/// A device the part moves past is left where the peer has it: dropped, every record the peer holds
/// of it would read as the peer's own news, and a MERGE would be owed on nothing.
#[test]
fn a_device_the_part_does_not_reach_stays_at_the_peers_place() {
    let (mut sa, mut sb, mut a, mut b) = split();
    let (seq, tip) = (a.seq, a.tip);
    for i in 0..3 {
        b.commit(&mut sb, format!("b{i}").as_bytes(), None).unwrap();
    }
    a.commit(&mut sb, b"a late", None).unwrap();
    (a.seq, a.tip) = (seq, tip);
    let peer = Doc::front(&sa, &ToyIds).unwrap();
    let lim = Limits { records: 2, seals: 2, bytes: 1 << 20 };
    let t = Doc::tail_part(&sb, &peer, &ToyIds, &lim).unwrap();
    let row = |f: &Front| f.places.iter().find(|p| p.row.actor_id == a.s.id).copied();
    assert_eq!(row(&t.front), row(&peer));
    a.commit(&mut sa, b"a late", None).unwrap();
    let u = into(&sa, &t).unwrap();
    assert_eq!(u.our_tail.len(), 1, "only what the peer really lacks");
}
