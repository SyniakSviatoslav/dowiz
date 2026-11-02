//! W-MG-TAM's traces of the symbolic model, against the real `verify`, FLIPPED by W-MG-MERGE2: each
//! once asserted the attack worked. Toy seams: every step below is one the attacker may take with
//! the keys named, so no forged signature is involved; each signature is made with a key the
//! scenario hands him. Nothing here is proof of anything but these traces.

use super::cut::{read_old, seals_of};
use super::keys::{key_rows, stage_key, stage_keys, Key, KeyOp};
use super::merge_body::{merge_body, MergeView, Row, STATE_COMMUTES};
use super::seal::{read_heads, stage_heads, stage_seal, Head};
use super::seg::{stage_seg, Segment};
use super::budget::Budget;
use super::tests::{key_of, Dev, ToyIds, ToySigner, ToyVerifier};
use super::verify::{verify, Report, Trust};
use super::write::{stage_doc, Refs};
use super::{forgotten_body, Doc, DocError, Erased, Ids};
use crate::{EvLog, Record, Store, Tx};
use std::collections::BTreeMap;

#[path = "tam_l3t_tests.rs"]
mod l3t;

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
    let heads = carried_heads(st, &mut out, &mut tx, &o.heads);
    let doc = stage_doc(&mut out, &mut tx, o.flags, Refs { log: Some(log), heads: Some(heads), keys: Some(keys), ..Refs::default() }).unwrap();
    out.commit_bytes(&tx, doc);
    out
}

/// HEADS and every seal of `st`, verbatim, in `out`: a seal is copied with no key.
fn carried_heads(st: &Store, out: &mut Store, tx: &mut Tx, rows: &[Head]) -> usize {
    let mut budget = Budget::of(st);
    let mut heads = Vec::new();
    for h in rows {
        let mut chain = seals_of(st, h, 64, &mut budget).unwrap();
        chain.reverse();
        let mut prev = None;
        for s in &chain {
            prev = Some(stage_seal(out, tx, &s.actor_id, s.seq, &s.tip, s.claimed_ms, prev, s.alg, &s.sig).unwrap());
        }
        heads.push(Head { seal: prev, ..*h });
    }
    stage_heads(out, tx, &mut heads).unwrap()
}

/// The cut file `st` with `recs` as its log and everything else verbatim: what a forger with the
/// file and no key writes. No seal, KEY or SEG row moves. Its keys are genesis keys: no ADMIT to remap.
fn relaid(st: &Store, recs: &[Record]) -> Store {
    let o = read_old(st).unwrap();
    let mut out = Store::create_bytes(1 << 20).unwrap();
    let mut tx = out.begin().unwrap();
    let mut log = EvLog::stage_init_in(&mut out, &mut tx).unwrap();
    for r in recs {
        log = EvLog::stage_append_in(&mut out, &mut tx, log, r, Some(&r.id)).unwrap();
    }
    let mut rows = Vec::new();
    for k in &o.keys {
        rows.push((k.actor_id, stage_key(&mut out, &mut tx, k).unwrap()));
    }
    let keys = stage_keys(&mut out, &mut tx, &mut rows).unwrap();
    let heads = carried_heads(st, &mut out, &mut tx, &o.heads);
    let seg = stage_seg(&mut out, &mut tx, &Segment { prev: None, ..o.segs[0].clone() }).unwrap();
    let doc = stage_doc(&mut out, &mut tx, o.flags, Refs { log: Some(log), heads: Some(heads), keys: Some(keys), seg: Some(seg), ..Refs::default() }).unwrap();
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
    let tip = Doc::tip_at(&st, &t.s.id, 1).unwrap();
    op(&mut st, &mut c, &KeyOp::Revoke { actor_id: t.s.id, after_seq: 1, tip });
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
/// because that sits on another device's chain, in an order nobody signed. Only a root's TRUNCATE
/// is effective, so the segment stands undeclared: refused, as a forger's would be.
#[test]
fn h1_a_non_root_key_cannot_cut_a_revoke_of_another_device_away() {
    let (st, a, c, mut t) = revoked_by_another_root();
    let re = reordered(&st, &[2, 0, 1, 3]);
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&re, &mut fresh, 3, &Bodiless).unwrap();
    zero_revoked(&mut fresh, &t);
    t.commit(&mut fresh, &seg.truncate_body(), None).unwrap();
    let roots = [a.s.id, c.s.id];
    let r = verify(&fresh, &ToyIds, &ToyVerifier, Trust::Only(&roots)).map(|r| r.counters);
    assert_eq!(r, Err(DocError::CutUndeclared { root: seg.root }));
}

/// THE TWIN: the same cut signed by a root declares it.
#[test]
fn h1_a_root_cuts() {
    let (mut a, mut t) = (dev(0xA1), dev(0xB2));
    let mut st = store(&[&a]);
    a.commit(&mut st, b"a-1", None).unwrap();
    op(&mut st, &mut a, &KeyOp::Admit(key_of(&t.s)));
    t.commit(&mut st, b"t-1", None).unwrap();
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&st, &mut fresh, 2, &ToyIds).unwrap();
    a.commit(&mut fresh, &seg.truncate_body(), None).unwrap();
    assert!(only(&fresh, &[&a]).holds());
}

/// H2: A revokes T after 2, naming T's record there. T's holder rewrites T's seq 2 into an ADMIT of
/// a fresh key E, after the REVOKE, with A's records and A's signatures not moved by one byte. The
/// chain at `after_seq` no longer ends at the tip A signed: broken, and it does not hold.
#[test]
fn h2_a_revoked_key_cannot_backdate_an_admit_under_after_seq() {
    let build = |second: Option<(&D, [u8; 32])>| -> (Store, D, D) {
        let (mut a, mut t) = (dev(0xA1), dev(0xB2));
        let mut st = store(&[&a]);
        op(&mut st, &mut a, &KeyOp::Admit(key_of(&t.s)));
        t.commit(&mut st, b"t-1", None).unwrap();
        let tip = match second {
            None => {
                t.commit(&mut st, b"t-2", None).unwrap();
                Doc::tip_at(&st, &t.s.id, 2).unwrap()
            }
            Some((_, seen)) => seen,
        };
        let revoke = KeyOp::Revoke { actor_id: t.s.id, after_seq: 2, tip };
        // A's record as A signed it. The writer would refuse this file, so it is laid by hand.
        a.commit(&mut st, &revoke.body(), if second.is_none() { Some(&revoke) } else { None }).unwrap();
        if let Some((e, _)) = second {
            let rows = key_rows(&st, Doc::open(&st).unwrap().keys);
            st.put_cell(rows.iter().find(|r| r.0 == t.s.id).unwrap().1, 6, 2);
            st.seal(rows.iter().find(|r| r.0 == t.s.id).unwrap().1);
            op(&mut st, &mut t, &KeyOp::Admit(key_of(&e.s)));
        }
        (st, a, t)
    };
    let (honest, a, t) = build(None);
    assert!(only(&honest, &[&a]).holds(), "the twin: the REVOKE that names its tip holds");
    let seen = Doc::tip_at(&honest, &t.s.id, 2).unwrap();

    let mut e = dev(0xEE);
    let (mut st, a2, _) = build(Some((&e, seen)));
    e.commit(&mut st, b"e-1 long after the revoke", None).unwrap();
    let r = only(&st, &[&a2]);
    let tip = |s: &Store| read_heads(s, Doc::open(s).unwrap().heads).iter().find(|h| h.actor_id == a.s.id).unwrap().tip;
    assert_eq!(tip(&honest), tip(&st), "the REVOKE's record did not move");
    assert_eq!((r.counters.broken, r.counters.seals_bad), (1, 0), "{:?}", r.counters);
    assert!(!r.holds(), "H2: {:?}", r.counters);
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let roots = [a.s.id];
    let m = Doc::merge_into(&honest, &st, &mut fresh, &ToyIds, &ToyVerifier, Trust::Only(&roots));
    assert_eq!(m.err(), Some(DocError::NotWhole { ours: false }));
}

/// The writer refuses a tip that is not the target's record, and a seq its file does not hold.
#[test]
fn h2_the_writer_names_only_a_tip_it_holds() {
    let (mut a, mut t) = (dev(0xA1), dev(0xB2));
    let mut st = store(&[&a]);
    op(&mut st, &mut a, &KeyOp::Admit(key_of(&t.s)));
    t.commit(&mut st, b"t-1", None).unwrap();
    let wrong = KeyOp::Revoke { actor_id: t.s.id, after_seq: 1, tip: [0x77; 32] };
    assert_eq!(a.commit(&mut st, &wrong.body(), Some(&wrong)), Err(DocError::RevokeTip { actor: t.s.id, seq: 1 }));
    let blind = KeyOp::Revoke { actor_id: t.s.id, after_seq: 5, tip: [0; 32] };
    assert_eq!(a.commit(&mut st, &blind.body(), Some(&blind)), Err(DocError::RevokeTip { actor: t.s.id, seq: 5 }));
}

/// H3: NO KEY AT ALL. A forged TRUNCATE at a seq already used: a fork is dense so that it counts
/// once, and the victim's seal covers that seq. A record at a forked seq declares nothing, so the
/// refusal stands: 75, not a status 0 with a fork in it.
#[test]
fn h3_a_forger_with_no_key_does_not_escape_the_refusal_by_a_fork() {
    let mut a = dev(0xA1);
    let mut st = store(&[&a]);
    for p in [&b"a-1"[..], b"a-2", b"a-3"] {
        a.commit(&mut st, p, None).unwrap();
    }
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&st, &mut fresh, 1, &ToyIds).unwrap();
    let roots = [a.s.id];
    let undeclared = Err(DocError::CutUndeclared { root: seg.root });
    assert_eq!(verify(&fresh, &ToyIds, &ToyVerifier, Trust::Only(&roots)).map(|_| ()), undeclared, "the control");

    let mut r = Record { id: [0; 32], prev: a.tip, actor_pubkey: a.s.id, actor_seq: 3, flags: 0, payload: seg.truncate_body() };
    r.id = ToyIds.chained(&r);
    let v = Doc::open(&fresh).unwrap();
    let mut tx = fresh.begin().unwrap();
    let log = EvLog::stage_append_in(&mut fresh, &mut tx, v.log.unwrap(), &r, None).unwrap();
    let d = stage_doc(&mut fresh, &mut tx, v.flags, Refs { log: Some(log), heads: v.heads, keys: v.keys, seg: v.seg, ..Refs::default() }).unwrap();
    fresh.commit_bytes(&tx, d);
    assert_eq!(verify(&fresh, &ToyIds, &ToyVerifier, Trust::Only(&roots)).map(|_| ()), undeclared);
}

/// H4, A LIMIT AND NOT A HOLE (main's decision, FORMAT §14): under a cut a MERGE row fixes the seq and
/// not the tip. PINNED: this stays true until the file carries what left.
#[test]
fn h4_a_merge_row_under_a_cut_fixes_the_seq_and_not_the_tip() {
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
    assert!(r.holds(), "the limit moved: {:?}", r.counters);
}
