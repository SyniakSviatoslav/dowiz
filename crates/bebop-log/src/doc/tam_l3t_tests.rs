//! W-MG-TAM's L3t traces, split from `tam_tests` (file size): a forged TRUNCATE with no key reaches
//! status 0 and `holds` stays false, the FORGOTTEN shapes around it, and a cut that leaves a sync as the
//! oldest kept seal. Toy seams, as there; nothing here is proof of anything but these traces.

use super::super::cut::read_old;
use super::super::seg::Segment;
use super::{dev, only, relaid, store, D};
use super::super::tests::{ToyIds, ToyVerifier};
use super::super::verify::{verify, Trust};
use super::super::{forgotten_body, Doc, DocError, Erased, Ids};
use crate::{Record, Store};

/// L3t's file before the forgery: root A wrote 1 to 6, cut at 1, and nobody signed the cut, so
/// `verify` refuses it (75). `one_sync`: 3 to 6 went under one seal, so no seal names seq 3 or 4,
/// the two records L3t touches. Else one seal per commit. Gives the file, A, the segment and the
/// five kept records, oldest first, seq 2 to 6.
fn cut_undeclared(one_sync: bool) -> (Store, D, Segment, Vec<Record>) {
    let mut a = dev(0xA1);
    let mut st = store(&[&a]);
    a.commit(&mut st, b"a-1", None).unwrap();
    a.commit(&mut st, b"a-2", None).unwrap();
    let bodies = [&b"a-3"[..], b"a-4", b"a-5", b"a-6"];
    if one_sync {
        let sync: Vec<Record> = bodies.iter().map(|p| a.next(p)).collect();
        Doc::commit_many_bytes(&mut st, &sync, &a.s, 1_700_000_000_100).unwrap();
    } else {
        for p in bodies {
            a.commit(&mut st, p, None).unwrap();
        }
    }
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&st, &mut fresh, 5, &ToyIds).unwrap();
    let kept = read_old(&fresh).unwrap().recs.into_iter().map(|(_, r)| r).collect();
    (fresh, a, seg, kept)
}

/// A record whose id holds. The id needs no key: that is all a forger can write.
fn forged(prev: [u8; 32], actor: [u8; 32], seq: u64, payload: Vec<u8>) -> Record {
    let mut r = Record { id: [0; 32], prev, actor_pubkey: actor, actor_seq: seq, flags: 0, payload };
    r.id = ToyIds.chained(&r);
    r
}

/// `of` rewritten as a tombstone: its id kept, which then checks nothing, and its `prev` free.
fn stone(of: &Record, prev: [u8; 32]) -> Record {
    Record { prev, flags: 0, payload: vec![0x80], ..of.clone() }
}

fn row_of(z: &Record) -> Erased {
    Erased { id: z.id, digest: ToyIds.chained(z) }
}

fn erasing(rows: &[Erased]) -> Vec<u8> {
    forgotten_body(&[0x5B; 32], rows)
}

/// L3t, FORMAT s14 "Status 0 is not `holds`": no key at all. The record at seq 3 becomes a TRUNCATE
/// under the root's actor, and the genuine seq 4 a tombstone with `prev` on it. Nothing forks, the
/// forgery's id holds, the seal over 6 covers it, so the cut is declared and 75 becomes counters.
/// `holds` stays false on two terms: `tombs_foreign`, and `redacted` 1 against `declared` 0. The next
/// tests take the second away.
#[test]
fn tam_l3t_a_forged_truncate_reaches_status_0_and_holds_is_false() {
    let (fresh, a, seg, k) = cut_undeclared(true);
    let roots = [a.s.id];
    let undeclared = Err(DocError::CutUndeclared { root: seg.root });
    assert_eq!(verify(&fresh, &ToyIds, &ToyVerifier, Trust::Only(&roots)).map(|_| ()), undeclared, "the control: 75");
    let same = relaid(&fresh, &k);
    assert_eq!(verify(&same, &ToyIds, &ToyVerifier, Trust::Only(&roots)).map(|_| ()), undeclared, "the control: `relaid` alone changes nothing");

    let t = forged(k[0].id, a.s.id, 3, seg.truncate_body());
    let st = relaid(&fresh, &[k[0].clone(), t.clone(), stone(&k[2], t.id), k[3].clone(), k[4].clone()]);
    let r = only(&st, &[&a]);
    let c = r.counters;
    println!("L3t {c:?}");
    assert_eq!((c.records, c.chained, c.redacted, c.broken, c.forks), (5, 4, 1, 0, 0), "{c:?}");
    assert_eq!((c.seals_ok, c.seals_bad, c.unsealed, c.keys_disagree), (2, 0, 0, 0), "{c:?}");
    assert_eq!((c.segments, c.cut, c.declared, c.tombs_foreign), (1, 1, 0, 1), "{c:?}");
    assert!(!r.holds(), "L3t: {c:?}");
}

/// THE TWIN: with one seal per commit, the seal of the replaced seq 3 names a record that is gone. It
/// goes bad and the refusal falls all the same: the seal over 4 stands on the tombstone's kept id, and
/// its own `prev_tip` comes from the bad seal's tip, not from the record.
#[test]
fn tam_l3t_one_seal_per_commit_leaves_one_seal_bad_and_status_0() {
    let (fresh, a, seg, k) = cut_undeclared(false);
    let t = forged(k[0].id, a.s.id, 3, seg.truncate_body());
    let st = relaid(&fresh, &[k[0].clone(), t.clone(), stone(&k[2], t.id), k[3].clone(), k[4].clone()]);
    let r = only(&st, &[&a]);
    let c = r.counters;
    assert_eq!((c.seals_ok, c.seals_bad, c.broken, c.tombs_foreign), (4, 1, 0, 1), "{c:?}");
    assert!(!r.holds(), "{c:?}");
}

/// HOLDS IS NOT REACHED: a forged FORGOTTEN under the root's actor, at seq 4, listing the tombstone at
/// 5. It is effective, so `declared` balances `redacted` and `tombs_foreign` is the last term standing.
/// The digest it must carry covers the tombstone's `prev`, which is the FORGOTTEN's own id, which
/// covers that digest. No key helps and no key is needed: only a hash fixed point closes this, and
/// walking the loop gets no closer. Either the digest is stale, or the link is.
#[test]
fn tam_l3t_a_forgotten_under_the_tombstone_meets_the_hash_cycle() {
    let (fresh, a, seg, k) = cut_undeclared(true);
    let t = forged(k[0].id, a.s.id, 3, seg.truncate_body());
    let listing = |prev: [u8; 32]| forged(t.id, a.s.id, 4, erasing(&[row_of(&stone(&k[3], prev))]));
    let (mut below, mut g) = ([0u8; 32], listing([0; 32]));
    for _ in 0..64 {
        assert_ne!(g.id, below, "the cycle closed");
        below = g.id;
        g = listing(below);
    }
    assert_ne!(g.id, below, "the cycle closed");

    let st = relaid(&fresh, &[k[0].clone(), t.clone(), g.clone(), stone(&k[3], g.id), k[4].clone()]);
    let r = only(&st, &[&a]);
    let c = r.counters;
    println!("L3t forgotten {c:?}");
    assert_eq!((c.records, c.chained, c.redacted, c.broken, c.forks), (5, 4, 1, 0, 0), "{c:?}");
    assert_eq!((c.seals_ok, c.seals_bad, c.unsealed, c.keys_disagree), (2, 0, 0, 0), "{c:?}");
    assert_eq!((c.declared, c.tombs_foreign), (1, 1), "the link stands, the digest is stale: {c:?}");
    assert!(!r.holds(), "L3t closed: {c:?}");

    let st = relaid(&fresh, &[k[0].clone(), t.clone(), g.clone(), stone(&k[3], below), k[4].clone()]);
    let r = only(&st, &[&a]);
    assert_eq!(r.counters.broken, 1, "the digest stands, the link is stale: {:?}", r.counters);
    assert!(!r.holds(), "{:?}", r.counters);
}

/// THE OTHER ORDER: a FORGOTTEN above a tombstone can list it, for that digest covers nothing of the
/// FORGOTTEN. But a seal names only a genuine id, so a forged record under a seal has a tombstone above
/// it with `prev` on the forged run, and that tombstone's digest covers the run. Each run needs its
/// list in another run, and such a chain of digests closes into a cycle however many runs there are.
/// The legacy door is shut too: a legacy id does not cover `prev`, but one above a chained record of
/// its device is broken (`count.rs`, `moved`). Here: two runs' worth, one tombstone left stale.
#[test]
fn tam_l3t_a_forgotten_above_a_tombstone_leaves_the_next_tombstone() {
    let (fresh, a, seg, k) = cut_undeclared(true);
    let t = forged(k[0].id, a.s.id, 3, seg.truncate_body());
    let z = stone(&k[2], t.id);
    let g = forged(z.id, a.s.id, 5, erasing(&[row_of(&z), row_of(&stone(&k[4], [0; 32]))]));
    let st = relaid(&fresh, &[k[0].clone(), t.clone(), z.clone(), g.clone(), stone(&k[4], g.id)]);
    let r = only(&st, &[&a]);
    let c = r.counters;
    assert_eq!((c.records, c.chained, c.redacted, c.broken), (5, 3, 2, 0), "{c:?}");
    assert_eq!((c.seals_ok, c.seals_bad, c.unsealed), (2, 0, 0), "{c:?}");
    assert_eq!((c.declared, c.tombs_foreign), (2, 1), "the top tombstone's digest is the one left: {c:?}");
    assert!(!r.holds(), "L3t closed: {c:?}");
}

/// Ids whose digest of one tombstone covers nothing, so a FORGOTTEN can list it before its `prev`
/// is known. No real `Ids` may be like this: `id_message` covers `prev`.
struct PrevUnbound {
    stone: [u8; 32],
    digest: [u8; 32],
}
impl Ids for PrevUnbound {
    fn chained(&self, r: &Record) -> [u8; 32] {
        if r.id == self.stone { self.digest } else { ToyIds.chained(r) }
    }
    fn legacy(&self, flags: i64, prev: &[u8; 32], payload: &[u8], id: &[u8; 32]) -> bool {
        ToyIds.legacy(flags, prev, payload, id)
    }
    fn redacted(&self, flags: i64, payload: &[u8]) -> bool {
        ToyIds.redacted(flags, payload)
    }
    fn hash(&self, b: &[u8]) -> Option<[u8; 32]> {
        ToyIds.hash(b)
    }
    fn tree_root(&self, ids: &[[u8; 32]]) -> Option<[u8; 32]> {
        ToyIds.tree_root(ids)
    }
}

/// THE CONTROL OF THE CYCLE: swap out only the digest's reach over `prev` and the very file holds, so
/// the cycle is what stops L3t and nothing else in the file was doing it by accident.
#[test]
fn tam_l3t_control_without_the_digest_over_prev_the_forgery_holds() {
    let (fresh, a, seg, k) = cut_undeclared(true);
    let t = forged(k[0].id, a.s.id, 3, seg.truncate_body());
    let unbound = PrevUnbound { stone: k[3].id, digest: [0x5D; 32] };
    let g = forged(t.id, a.s.id, 4, erasing(&[Erased { id: unbound.stone, digest: unbound.digest }]));
    let st = relaid(&fresh, &[k[0].clone(), t.clone(), g.clone(), stone(&k[3], g.id), k[4].clone()]);
    let roots = [a.s.id];
    let r = verify(&st, &unbound, &ToyVerifier, Trust::Only(&roots)).unwrap();
    assert!(r.holds(), "the control: {:?}", r.counters);
    assert!(!only(&st, &[&a]).holds(), "the same bytes under the real rule");
}

/// AN HONEST FILE, NO FORGER: a seal at 1, a sync 2 to 4 under one seal, a cut at 1 on the seal's edge,
/// and the root's TRUNCATE. The oldest kept seal signed `prev_tip` = the frontier tip, and its seq - 1
/// is not the frontier's seq.
#[test]
fn a_cut_that_leaves_a_sync_as_the_oldest_seal_holds() {
    let mut a = dev(0xA1);
    let mut st = store(&[&a]);
    a.commit(&mut st, b"a-1", None).unwrap();
    let sync: Vec<Record> = [&b"a-2"[..], b"a-3", b"a-4"].iter().map(|p| a.next(p)).collect();
    Doc::commit_many_bytes(&mut st, &sync, &a.s, 1_700_000_000_100).unwrap();
    assert!(only(&st, &[&a]).holds(), "the control: before the cut");
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&st, &mut fresh, 3, &ToyIds).unwrap();
    a.commit(&mut fresh, &seg.truncate_body(), None).unwrap();
    let r = only(&fresh, &[&a]);
    println!("sync after cut {:?}", r.counters);
    assert_eq!((r.counters.seals_ok, r.counters.seals_bad), (2, 0), "{:?}", r.counters);
    assert!(r.holds(), "{:?}", r.counters);
}
