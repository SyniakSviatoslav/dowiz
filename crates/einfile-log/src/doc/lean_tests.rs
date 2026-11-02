//! W-MG-LEAN's three disagreements between `holds` and its model, each a file that held and must
//! not (`da_`, `db_`) or a merge refused for no reason (`dc_`); then what a merge does when the two
//! sides differ in something other than records.
use super::keys::KeyOp;
use super::seal::{message, read_heads, read_seal, stage_heads, stage_seal, Head};
use super::tests::{doc_with, key_of, tombs_body, Dev, ToyIds, ToySigner, ToyVerifier};
use super::verify::{verify, Report, Trust};
use super::write::{stage_doc, Refs};
use super::{Doc, DocError, Ids, Signer};
use crate::{EvLog, Record, Store};

fn rep(st: &Store) -> Report {
    verify(st, &ToyIds, &ToyVerifier, Trust::Genesis).unwrap()
}
fn forge_record(st: &mut Store, r: &Record) {
    let v = Doc::open(st).unwrap();
    let mut tx = st.begin().unwrap();
    let log = EvLog::stage_append_in(st, &mut tx, v.log.unwrap(), r, None).unwrap();
    let d = stage_doc(st, &mut tx, v.flags, Refs { log: Some(log), heads: v.heads, keys: v.keys, seg: v.seg, ..Refs::default() }).unwrap();
    st.commit_bytes(&tx, d);
}
/// `r` plus a seal over it by `s`, as `commit_bytes` would, but with no check of the seq.
fn forge_sealed(st: &mut Store, r: &Record, s: &ToySigner) {
    let v = Doc::open(st).unwrap();
    let mut heads = read_heads(st, v.heads);
    let mut tx = st.begin().unwrap();
    let log = EvLog::stage_append_in(st, &mut tx, v.log.unwrap(), r, Some(&r.id)).unwrap();
    let ms = 1_700_000_000_000i64;
    let sig = s.sign(&message(&r.actor_pubkey, r.actor_seq, &r.id, ms, &[0u8; 32]));
    let seal = stage_seal(st, &mut tx, &r.actor_pubkey, r.actor_seq, &r.id, ms, None, s.alg(), &sig).unwrap();
    heads.push(Head { actor_id: r.actor_pubkey, seq: r.actor_seq, tip: r.id, seal: Some(seal) });
    let h = stage_heads(st, &mut tx, &mut heads).unwrap();
    let d = stage_doc(st, &mut tx, v.flags, Refs { log: Some(log), heads: Some(h), keys: v.keys, seg: v.seg, anch: v.anch, ..Refs::default() }).unwrap();
    st.commit_bytes(&tx, d);
}
fn chained(prev: [u8; 32], actor: [u8; 32], seq: u64, payload: &[u8]) -> Record {
    let mut r = Record { id: [0; 32], prev, actor_pubkey: actor, actor_seq: seq, flags: 0, payload: payload.to_vec() };
    r.id = ToyIds.chained(&r);
    r
}
fn admit_b(st: &mut Store, a: &mut Dev<ToySigner>) -> Dev<ToySigner> {
    let b = Dev::new(ToySigner { id: [0xB2; 32] });
    let op = KeyOp::Admit(key_of(&b.s));
    a.commit(st, &op.body(), Some(&op)).unwrap();
    b
}


/// D-A: a key holder's device opens on a fabricated tombstone, seq 0, `prev` = its own id, listed by
/// its own FORGOTTEN. The chain under the signed tip ends on a ring.
fn tombstone_opening(id: [u8; 32], prev: [u8; 32], seq: u64) -> Report {
    let (mut st, mut a) = doc_with(2);
    let b = admit_b(&mut st, &mut a);
    let t = Record { id, prev, actor_pubkey: b.s.id, actor_seq: seq, flags: 0, payload: vec![0x80] };
    forge_record(&mut st, &t);
    let body = tombs_body(&st);
    forge_sealed(&mut st, &chained(t.id, b.s.id, seq + 1, &body), &b.s);
    rep(&st)
}
#[test]
fn da_a_tombstone_ring_does_not_hold() {
    let r = tombstone_opening([9; 32], [9; 32], 0);
    assert!(!r.holds(), "{:?}", r.counters);
    let r = tombstone_opening([9; 32], [9; 32], 1);
    assert_eq!((r.counters.unsealed, r.holds()), (1, false), "dense and still a ring: {:?}", r.counters);
}
/// No ring here: `prev` 0^32, so only the seq says the tombstone is off its chain.
#[test]
fn da_a_tombstone_off_its_seq_is_broken() {
    let r = tombstone_opening([9; 32], [0; 32], 0);
    assert_eq!((r.counters.broken, r.holds()), (1, false), "{:?}", r.counters);
    let r = tombstone_opening([9; 32], [0; 32], 1);
    assert!(r.holds(), "its own key's tombstone at its place: the id of an erased record is checked by nothing: {:?}", r.counters);
}
/// A two-record ring reached from a signed tip, ids free, no tombstone listed for a ring to hide in.
#[test]
fn da_twin_unlisted_does_not_hold() {
    let (mut st, mut a) = doc_with(2);
    let mut b = admit_b(&mut st, &mut a);
    let t = Record { id: [9; 32], prev: [9; 32], actor_pubkey: b.s.id, actor_seq: 0, flags: 0, payload: vec![0x80] };
    forge_record(&mut st, &t);
    b.tip = t.id;
    b.commit(&mut st, b"plain", None).unwrap();
    assert!(!rep(&st).holds());
}

/// (kept, frontier tip of the owner, b): six owner records, b admitted at the fifth, cut down to two.
fn cut_file() -> (Store, [u8; 32], Dev<ToySigner>) {
    let (mut old, mut a) = doc_with(4);
    let b = admit_b(&mut old, &mut a);
    a.commit(&mut old, b"six", None).unwrap();
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&old, &mut fresh, 2, &ToyIds).unwrap();
    a.commit(&mut fresh, &seg.truncate_body(), None).unwrap();
    assert!(rep(&fresh).holds(), "the cut file holds before b writes");
    (fresh, seg.frontier[0].tip, b)
}

/// D-B: a keyed device with no frontier row of its own opens on another device's frontier tip: its
/// seq runs from 1, whatever the tip.
#[test]
fn db_keyed_device_on_a_foreign_frontier_tip_starts_at_1() {
    for (seq, holds) in [(0u64, false), (1000, false), (1, true)] {
        let (mut st, tip, b) = cut_file();
        let r = chained(tip, b.s.id, seq, b"first");
        forge_sealed(&mut st, &r, &b.s);
        let rp = rep(&st);
        assert_eq!(rp.holds(), holds, "seq {seq}: {:?}", rp.counters);
    }
}
#[test]
fn db_twin_from_zero_is_broken() {
    let (mut st, _, b) = cut_file();
    let r = chained([0; 32], b.s.id, 1000, b"first");
    forge_sealed(&mut st, &r, &b.s);
    assert!(!rep(&st).holds());
}

fn stranger_split(body: &[u8], seq: u64, b_writes: usize) -> (Store, Store) {
    let mut st = Store::create_bytes(1 << 20).unwrap();
    let mut a = Dev::new(ToySigner { id: [0xA1; 32] });
    Doc::init_bytes(&mut st, 0, &[key_of(&a.s)]).unwrap();
    let x = chained([0; 32], [0xC3; 32], seq, body);
    forge_record(&mut st, &x);
    a.tip = x.id;
    a.commit(&mut st, b"one", None).unwrap();
    let mut b = admit_b(&mut st, &mut a);
    let bytes = st.to_bytes();
    b.tip = EvLog::tip_at(&st, Doc::open(&st).unwrap().log).unwrap();
    let (mut sa, mut sb) = (Store::from_bytes(&bytes), Store::from_bytes(&bytes));
    a.commit(&mut sa, b"ours", None).unwrap();
    for i in 0..b_writes {
        b.commit(&mut sb, format!("theirs-{i}").as_bytes(), None).unwrap();
    }
    (sa, sb)
}
fn merged(sa: &Store, sb: &Store) -> Result<(Store, bool), DocError> {
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let u = Doc::merge_into(sa, sb, &mut fresh, &ToyIds, &ToyVerifier, Trust::Genesis)?;
    Ok((fresh, u.wrote))
}

/// D-C: a REVOKE-bodied record of an actor with no KEY row is no REVOKE (`effective`): it must not
/// stop a device from merging.
#[test]
fn dc_a_non_effective_revoke_does_not_block_the_merge() {
    let body = KeyOp::Revoke { actor_id: [0xB2; 32], after_seq: 1, tip: [0; 32] }.body();
    for n in [1usize, 2] {
        let (sa, sb) = stranger_split(&body, 1, n);
        assert!(rep(&sa).holds() && rep(&sb).holds());
        let (u, _) = merged(&sa, &sb).unwrap();
        let r = rep(&u);
        assert!(r.holds() && r.counters.keys_disagree == 0, "{n}: {:?}", r.counters);
    }
}
#[test]
fn dc_twin_plain_body_merges() {
    let (sa, sb) = stranger_split(b"plain", 1, 2);
    assert!(rep(&merged(&sa, &sb).unwrap().0).holds());
}

/// `keyed` IS NOT MONOTONE ACROSS SIDES: an actor with no KEY row is not judged, and a side that
/// admits it makes its records on the other side judged in the union, where they are off their seq
/// (7) or above any seal of its (1). Each side holds; the union does not. Only the verify of the
/// union sees it: this is what reaches that guard.
#[test]
fn a_key_on_one_side_can_undo_the_other_sides_stranger() {
    for seq in [7u64, 1] {
        let (mut st, mut a) = doc_with(2);
        let mut b = admit_b(&mut st, &mut a);
        let bytes = st.to_bytes();
        let (mut sa, mut sb) = (Store::from_bytes(&bytes), Store::from_bytes(&bytes));
        let tip = EvLog::tip_at(&sb, Doc::open(&sb).unwrap().log).unwrap();
        let x = chained(tip, [0xC3; 32], seq, b"stranger");
        forge_record(&mut sb, &x);
        b.tip = x.id;
        b.commit(&mut sb, b"theirs", None).unwrap();
        let op = KeyOp::Admit(key_of(&ToySigner { id: [0xC3; 32] }));
        a.commit(&mut sa, &op.body(), Some(&op)).unwrap();
        assert!(rep(&sa).holds() && rep(&sb).holds(), "each side holds: {:?}", rep(&sb).counters);
        assert_eq!(merged(&sa, &sb).err(), Some(DocError::UnionNotWhole), "seq {seq}");
    }
}

/// One more seal of `s` over its newest tip: `over` true on top of the chain, false in place of the
/// top seal. A key holder can always sign this.
fn reseal(st: &mut Store, s: &ToySigner, ms: i64, over: bool) {
    let v = Doc::open(st).unwrap();
    let mut heads = read_heads(st, v.heads);
    let row = heads.iter_mut().find(|h| h.actor_id == s.id).unwrap();
    let top = read_seal(st, row.seal.unwrap()).unwrap();
    let (link, prev_tip) = if over { (Some(top.at), top.tip) } else { (top.prev, top.prev.map_or([0; 32], |p| read_seal(st, p).unwrap().tip)) };
    let sig = s.sign(&message(&s.id, top.seq, &top.tip, ms, &prev_tip));
    let mut tx = st.begin().unwrap();
    let seal = stage_seal(st, &mut tx, &s.id, top.seq, &top.tip, ms, link, s.alg(), &sig).unwrap();
    row.seal = Some(seal);
    let h = stage_heads(st, &mut tx, &mut heads).unwrap();
    let d = stage_doc(st, &mut tx, v.flags, Refs { log: v.log, heads: Some(h), keys: v.keys, seg: v.seg, anch: v.anch, ..Refs::default() }).unwrap();
    st.commit_bytes(&tx, d);
}

/// TWO SEAL CHAINS OVER ONE HISTORY: the records agree, so the signer signed the same tip twice
/// under different words. HEADS has one chain per device and a seal signs the tip before it, so one
/// of the two signatures would have to go. Refused, by its own name: it is no fork of the records.
#[test]
fn two_seal_chains_over_one_history_are_refused() {
    let (mut st, a) = doc_with(3);
    let bytes = st.to_bytes();
    let other = Store::from_bytes(&bytes);
    reseal(&mut st, &a.s, 42, false);
    assert!(rep(&st).holds() && rep(&other).holds(), "each is a whole file");
    assert_eq!(merged(&st, &other).err(), Some(DocError::SealFork { actor: a.s.id, seq: 3 }));
}

/// THEIR SEAL, NO NEW RECORD: taken in. A merge loses no record and no signature: a seal is a
/// statement of a key, and "nothing to take" must not mean "their last statement is gone".
#[test]
fn a_seal_with_no_new_record_is_carried() {
    let (st, a) = doc_with(3);
    let bytes = st.to_bytes();
    let mut more = Store::from_bytes(&bytes);
    reseal(&mut more, &a.s, 42, true);
    assert!(rep(&more).holds(), "{:?}", rep(&more).counters);
    let n = rep(&more).counters.seals_ok;
    let (u, wrote) = merged(&st, &more).unwrap();
    assert!(wrote, "their extra seal is something to take");
    assert_eq!((rep(&u).counters.seals_ok, rep(&u).holds()), (n, true));
    assert!(!merged(&more, &st).unwrap().1, "the other way there is nothing to take");
}
