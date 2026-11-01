//! One genuine cut is enough: what the TRUNCATE does not name, nobody signed.
use super::*;
use bebop_store::evlog::doc::keys::{KeyOp, REVOKE_MARK};
use bebop_store::evlog::doc::seg::{Frontier, SCHEME_CHAINED};
use bebop_store::nodekey::sha256;

/// HOLE 5. Frontier rows are signed by nobody, and a kept seal reads its `prev_tip` from one. The
/// forger moves A's row up past two kept records: the next seal's message comes out right, the
/// next record's `prev` is the row's tip, and two records are gone with `cut` still saying 2.
#[test]
fn hole_5_a_moved_frontier_row_excuses_a_hole_in_the_kept_log() {
    let a = key(0xA1);
    let mut sh = SealedHub::create(&a).unwrap();
    sh.set_entropy([1; 32]);
    orders(&mut sh, &a, "c", 6);
    sh.cut(4, &a, 7).unwrap();
    let bytes = sh.to_bytes_trimmed();
    let roots = [a.id()];
    let mut s = round_trip(&bytes, &roots);
    let gone = s.recs.drain(..2).last().unwrap();
    let chain = chain_of(&mut s, &a.id());
    chain.retain(|x| x.seq > gone.actor_seq);
    s.segs[0].frontier[0].seq = gone.actor_seq;
    s.segs[0].frontier[0].tip = gone.id;
    must_not_hold("h5-row-moved", &write(&s), &roots);
}

/// HOLE 5b. The same row, ADDED, and the record it excuses is a REVOKE: the cut refuses to take
/// a REVOKE, the forger does not ask it. C's REVOKE leaves with its seal, C's next seal still
/// verifies from the row. The cut never touched C: the genuine segment has no row for it.
#[test]
fn hole_5b_an_added_frontier_row_erases_a_revoke() {
    let (a, b, c) = (key(0xA1), key(0xB2), key(0xC3));
    let mut sh = SealedHub::create(&a).unwrap();
    sh.set_entropy([1; 32]);
    orders(&mut sh, &a, "c", 1);
    sh.commit_keyop(&KeyOp::Admit(b.key()), &a, 1).unwrap();
    sh.commit_keyop(&KeyOp::Admit(c.key()), &a, 2).unwrap();
    orders(&mut sh, &b, "b", 2);
    sh.commit_keyop(&KeyOp::Revoke { actor_id: b.id(), after_seq: 1 }, &c, 3).unwrap();
    orders(&mut sh, &c, "n", 1);
    let keep = sh.len() - 1;
    sh.cut(keep, &a, 4).unwrap();
    let bytes = sh.to_bytes_trimmed();
    let roots = [a.id()];
    let mut s = round_trip(&bytes, &roots);
    assert!(!only("h5b-control", &bytes, &roots).unwrap().holds());
    let at = pos_of(&s, REVOKE_MARK);
    let gone = s.recs.remove(at);
    chain_of(&mut s, &c.id()).retain(|x| x.seq > gone.actor_seq);
    s.segs[0].frontier.push(Frontier { actor_id: c.id(), seq: gone.actor_seq, tip: gone.id, scheme: SCHEME_CHAINED });
    zero_table(&mut s);
    must_not_hold("h5b-row-added", &write(&s), &roots);
}

/// Cut, forget one kept record, cut past the first TRUNCATE with the FORGOTTEN left live, two
/// more orders: two segments, the older one named only by root.
fn twice_cut(a: &DeviceKey) -> SealedHub {
    let mut sh = SealedHub::create(a).unwrap();
    sh.set_entropy([1; 32]);
    orders(&mut sh, a, "c", 5);
    sh.cut(3, a, 6).unwrap();
    let n = sh.forget(|e| (e.order_id == "c2").then(|| "{}".to_string()), &sha256(b"who"), a, 7).unwrap();
    assert_eq!(n, 1);
    sh.cut(1, a, 8).unwrap();
    orders(&mut sh, a, "z", 2);
    sh
}

/// HOLE 6. The older segment's counts are nobody's claim: `check_segments` asks it for a root
/// only, `verify` sums its counts anyway. `declared` + 1 there pays for a tombstone laid by the
/// forger on a live record, and the root's live FORGOTTEN satisfies `tombs_foreign`.
#[test]
fn hole_6_an_older_segments_counts_pay_for_a_forgers_tombstone() {
    let a = key(0xA1);
    let bytes = twice_cut(&a).to_bytes_trimmed();
    let roots = [a.id()];
    let mut s = round_trip(&bytes, &roots);
    assert!(only("h6-control", &bytes, &roots).unwrap().holds());
    assert_eq!(s.segs.len(), 2);
    let target = s.recs.len() - 2;
    tombstone(&mut s.recs[target]);
    s.segs[1].declared += 1;
    must_not_hold("h6-old-seg-declared", &write(&s), &roots);
}

/// HOLE 6b. Below the oldest segment anything may be ADDED: its root is vouched for by the
/// `prev_root` read from its own `ref PREV_SEG`. Here a segment claiming a thousand records.
#[test]
fn hole_6b_a_segment_added_under_the_oldest_is_counted() {
    let a = key(0xA1);
    let bytes = twice_cut(&a).to_bytes_trimmed();
    let roots = [a.id()];
    let mut s = round_trip(&bytes, &roots);
    let mut extra = s.segs[1].clone();
    extra.root = [0x11; 32];
    extra.first = 0;
    extra.last = 999;
    s.segs.push(extra);
    must_not_hold("h6b-seg-added", &write(&s), &roots);
}
