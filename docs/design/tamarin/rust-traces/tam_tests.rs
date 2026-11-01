//! W-MG-TAM: the traces the symbolic model predicts, run against the real `verify`.
//! PRIVATE COPY of bebop-log, under /root/lanes/w-tam/.scratch. Toy seams: every step below is one
//! the attacker may take with the keys named, so no forged signature is involved; each signature
//! is made with a key the scenario hands him. Nothing here is proof of anything but these traces.

use super::cut::{read_old, seals_of};
use super::keys::{key_rows, stage_key, stage_keys, Key, KeyOp};
use super::merge_body::{merge_body, MergeView, Row, STATE_COMMUTES};
use super::seal::{read_heads, stage_heads, stage_seal, Head};
use super::budget::Budget;
use super::tests::{key_of, Dev, ToyIds, ToySigner, ToyVerifier};
use super::verify::{verify, Report, Trust};
use super::write::{stage_doc, Refs};
use super::{Doc, DocError, Ids};
use crate::{EvLog, Record, Store};
use std::collections::BTreeMap;

type D = Dev<ToySigner>;

/// The attacker's cut writer: `cut_into` then sees no REVOKE mark, as his own writer would not look.
struct Bodiless;
impl Ids for Bodiless {
    fn chained(&self, r: &Record) -> [u8; 32] {
        ToyIds.chained(r)
    }
    fn hash(&self, b: &[u8]) -> Option<[u8; 32]> {
        ToyIds.hash(b)
    }
    fn body<'a>(&self, _f: i64, _p: &'a [u8]) -> &'a [u8] {
        &[]
    }
    fn tree_root(&self, ids: &[[u8; 32]]) -> Option<[u8; 32]> {
        ToyIds.tree_root(ids)
    }
}

fn dev(b: u8) -> D {
    Dev::new(ToySigner { id: [b; 32] })
}
fn store(devs: &[&D]) -> Store {
    let mut st = Store::create_bytes(1 << 20).unwrap();
    let keys: Vec<Key> = devs.iter().map(|d| key_of(&d.s)).collect();
    Doc::init_bytes(&mut st, 0, &keys).unwrap();
    st
}
fn op(st: &mut Store, by: &mut D, o: &KeyOp) {
    by.commit(st, &o.body(), Some(o)).unwrap();
}
fn zero_revoked(st: &mut Store, who: &D) {
    let rows = key_rows(st, Doc::open(st).unwrap().keys);
    let at = rows.iter().find(|r| r.0 == who.s.id).unwrap().1;
    st.put_cell(at, 6, 0);
    st.seal(at);
}
fn only(st: &Store, roots: &[&D]) -> Report {
    let ids: Vec<[u8; 32]> = roots.iter().map(|d| d.s.id).collect();
    verify(st, &ToyIds, &ToyVerifier, Trust::Only(&ids)).unwrap()
}

/// The file with its records in `order` (indices into its file order), everything else verbatim,
/// signing nothing: the order across devices is signed by nobody (FORMAT s14).
fn reordered(st: &Store, order: &[usize]) -> Store {
    let o = read_old(st).unwrap();
    let mut out = Store::create_bytes(1 << 20).unwrap();
    let mut tx = out.begin().unwrap();
    let mut log = EvLog::stage_init_in(&mut out, &mut tx).unwrap();
    let id_at: BTreeMap<usize, [u8; 32]> = o.recs.iter().map(|(at, r)| (*at, r.id)).collect();
    let mut at: BTreeMap<[u8; 32], usize> = BTreeMap::new();
    for i in order {
        let r = &o.recs[*i].1;
        log = EvLog::stage_append_in(&mut out, &mut tx, log, r, Some(&r.id)).unwrap();
        at.insert(r.id, out.follow(log, 1).unwrap());
    }
    let mut rows = Vec::new();
    for k in &o.keys {
        let admit_rec = k.admit_rec.map(|a| at[&id_at[&a]]);
        rows.push((k.actor_id, stage_key(&mut out, &mut tx, &Key { admit_rec, ..k.clone() }).unwrap()));
    }
    let keys = stage_keys(&mut out, &mut tx, &mut rows).unwrap();
    let mut budget = Budget::of(st);
    let mut heads = Vec::new();
    for h in &o.heads {
        let mut chain = seals_of(st, h, 64, &mut budget).unwrap();
        chain.reverse();
        let mut prev = None;
        for s in &chain {
            prev = Some(stage_seal(&mut out, &mut tx, &s.actor_id, s.seq, &s.tip, s.claimed_ms, prev, s.alg, &s.sig).unwrap());
        }
        heads.push(Head { seal: prev, ..*h });
    }
    let heads = stage_heads(&mut out, &mut tx, &mut heads).unwrap();
    let doc = stage_doc(&mut out, &mut tx, o.flags, Refs { log: Some(log), heads: Some(heads), keys: Some(keys), ..Refs::default() }).unwrap();
    out.commit_bytes(&tx, doc);
    out
}

/// Root A admits T; root C revokes T after 1 and goes on writing. Both roots are honest and keep
/// their keys. File order: A1 T1 C1(REVOKE) C2.
fn revoked_by_another_root() -> (Store, D, D, D) {
    let (mut a, mut c, mut t) = (dev(0xA1), dev(0xC3), dev(0xB2));
    let mut st = store(&[&a, &c]);
    op(&mut st, &mut a, &KeyOp::Admit(key_of(&t.s)));
    t.commit(&mut st, b"t-1", None).unwrap();
    op(&mut st, &mut c, &KeyOp::Revoke { actor_id: t.s.id, after_seq: 1 });
    c.commit(&mut st, b"c-2", None).unwrap();
    (st, a, c, t)
}

/// CONTROL for H1: with the REVOKE standing, T's next commit is a bad seal.
#[test]
fn h1_control_the_revoke_stands_without_the_cut() {
    let (mut st, a, c, mut t) = revoked_by_another_root();
    assert!(only(&st, &[&a, &c]).holds(), "the honest file holds");
    zero_revoked(&mut st, &t);
    t.commit(&mut st, b"t-2 after the revoke", None).unwrap();
    let r = only(&st, &[&a, &c]);
    println!("H1 control {:?}", r.counters);
    assert!(!r.holds());
}

/// H1: T, an admitted NON-ROOT key, cuts away ONE record, the REVOKE, and keeps its own ADMIT
/// because that sits on another device's chain. Two roots stay honest. C's newer record stays, so
/// this is no rollback: the file is NOT a shorter true file, it holds C2 and lacks the C1 under it.
#[test]
fn h1_a_non_root_key_cuts_a_revoke_of_another_device_away() {
    let (st, a, c, mut t) = revoked_by_another_root();
    let re = reordered(&st, &[2, 0, 1, 3]);
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&re, &mut fresh, 3, &Bodiless).unwrap();
    zero_revoked(&mut fresh, &t);
    t.commit(&mut fresh, &seg.truncate_body(), None).unwrap();
    let r = only(&fresh, &[&a, &c]);
    println!("H1 {:?} admitted={}", r.counters, r.admitted);
    assert_eq!((r.counters.segments, r.counters.keys_disagree, r.counters.seals_bad), (1, 0, 0));
    assert!(r.holds(), "H1 closed: {:?}", r.counters);
}

/// H2: A revokes T after 2, naming a seq and no tip. T's holder rewrites T's seq 2 into an ADMIT of
/// a fresh key E, after the REVOKE, and A's records and A's signatures do not change by one byte.
/// E is not revoked: T's power outlives the REVOKE. No fork in one file.
#[test]
fn h2_a_revoked_key_backdates_an_admit_under_after_seq() {
    let build = |second: Option<&D>| -> (Store, D, D) {
        let (mut a, mut t) = (dev(0xA1), dev(0xB2));
        let mut st = store(&[&a]);
        op(&mut st, &mut a, &KeyOp::Admit(key_of(&t.s)));
        t.commit(&mut st, b"t-1", None).unwrap();
        if second.is_none() {
            t.commit(&mut st, b"t-2", None).unwrap();
        }
        op(&mut st, &mut a, &KeyOp::Revoke { actor_id: t.s.id, after_seq: 2 });
        if let Some(e) = second {
            // After the REVOKE in file order, in signing order, in fact: the key is revoked here.
            op(&mut st, &mut t, &KeyOp::Admit(key_of(&e.s)));
        }
        (st, a, t)
    };
    let (honest, a, _) = build(None);
    assert!(only(&honest, &[&a]).holds());

    let mut e = dev(0xEE);
    let (mut st, a2, t2) = build(Some(&e));
    e.commit(&mut st, b"e-1 long after the revoke", None).unwrap();
    let r = only(&st, &[&a2]);
    println!("H2 {:?} admitted={}", r.counters, r.admitted);
    // The revoker's own head is the same id with the same signature in both files.
    let tip = |s: &Store| read_heads(s, Doc::open(s).unwrap().heads).iter().find(|h| h.actor_id == a.s.id).unwrap().tip;
    assert_eq!(tip(&honest), tip(&st), "the REVOKE's seal binds nothing of T's chain");
    assert!(r.holds(), "H2 closed: {:?}", r.counters);
    assert_eq!(r.admitted, 3, "T's successor is admitted");
    // CONTROL: a second file sees it, as a fork of T's chain.
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let roots = [a.s.id];
    let m = Doc::merge_into(&honest, &st, &mut fresh, &ToyIds, &ToyVerifier, Trust::Only(&roots));
    println!("H2 merge against the honest copy: {:?}", m.as_ref().err());
    assert!(matches!(m, Err(DocError::Fork { .. })));
    let _ = t2;
}

/// H3: NO KEY AT ALL. A forged TRUNCATE at a seq already used: `forked` makes it dense
/// (count.rs:95), so it is sound, and the victim's seal covers its seq, so it is effective.
#[test]
fn h3_a_forger_with_no_key_escapes_the_refusal_by_a_fork() {
    let mut a = dev(0xA1);
    let mut st = store(&[&a]);
    for p in [&b"a-1"[..], b"a-2", b"a-3"] {
        a.commit(&mut st, p, None).unwrap();
    }
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&st, &mut fresh, 1, &ToyIds).unwrap();
    let roots = [a.s.id];
    let refused = verify(&fresh, &ToyIds, &ToyVerifier, Trust::Only(&roots)).map(|_| ());
    println!("H3 control {refused:?}");
    assert_eq!(refused, Err(DocError::CutUndeclared { root: seg.root }));

    let mut r = Record { id: [0; 32], prev: a.tip, actor_pubkey: a.s.id, actor_seq: 3, flags: 0, payload: seg.truncate_body() };
    r.id = ToyIds.chained(&r);
    let v = Doc::open(&fresh).unwrap();
    let mut tx = fresh.begin().unwrap();
    let log = EvLog::stage_append_in(&mut fresh, &mut tx, v.log.unwrap(), &r, None).unwrap();
    let d = stage_doc(&mut fresh, &mut tx, v.flags, Refs { log: Some(log), heads: v.heads, keys: v.keys, seg: v.seg, ..Refs::default() }).unwrap();
    fresh.commit_bytes(&tx, d);
    let rep = verify(&fresh, &ToyIds, &ToyVerifier, Trust::Only(&roots)).unwrap();
    println!("H3 {:?} holds={}", rep.counters, rep.holds());
    assert_eq!(rep.counters.segments, 1, "status 0, not 75");
    assert!(rep.counters.forks > 0 && !rep.holds(), "the fork is what still names it: {:?}", rep.counters);
}

/// H4: under a cut a MERGE row's tip is not read (declare.rs:100): a signer names a tip nothing signed.
#[test]
fn h4_a_merge_row_under_a_cut_names_a_tip_nobody_signed() {
    let mut a = dev(0xA1);
    let mut st = store(&[&a]);
    for p in [&b"a-1"[..], b"a-2", b"a-3"] {
        a.commit(&mut st, p, None).unwrap();
    }
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&st, &mut fresh, 1, &ToyIds).unwrap();
    a.commit(&mut fresh, &seg.truncate_body(), None).unwrap();
    let row = Row { actor_id: a.s.id, seq: 1, tip: [0xEE; 32] };
    let body = merge_body(&MergeView { ours: vec![row], theirs: vec![row], state: STATE_COMMUTES, seg_root: seg.root, conflicts: Vec::new() });
    a.commit(&mut fresh, &body, None).unwrap();
    let r = only(&fresh, &[&a]);
    println!("H4 {:?}", r.counters);
    assert_eq!((r.counters.merges, r.counters.merges_bad), (1, 0));
    assert!(r.holds(), "H4 closed: {:?}", r.counters);
}
