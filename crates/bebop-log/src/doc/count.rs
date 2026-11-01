//! THE RECORD COUNTERS (`verify.rs` says what each means), and THE DENSE SEQ (W-MG-HOLES2).
//!
//! A device's seqs are 1, 2, 3 ... along its chain, and the writer has always refused anything else
//! (`Doc::commit_bytes`). `verify` did not, and every rule that reads a seq trusts it: `valid_at`
//! (a revoked key may seal at or below `after_seq`), `covered` (a seal at s vouches for the
//! device's records up to s), so a record at a seq nobody used, 0 above all, passed all of them. It
//! was what a key holder writes after its REVOKE, and with no key at all it was an ADMIT in the
//! owner's name that the forger's own seal then reached (`atk2_seq`).
//!
//! With seqs dense and the id over the seq, `seq <= covered` means "at or below that seal on this
//! device's chain", and the only way to a used seq is a fork, which is named.
//!
//! WHOSE. An actor with a KEY row. A pre-seal record's actor signed nothing and may carry any
//! number (`rotate.rs` writes 0, a stock log its position); dropping a KEY row to escape this
//! leaves that key's seals bad.
//! WHAT. A chained record and a tombstone: their seq is bound (id; FORGOTTEN digest). A legacy id
//! does not cover its seq, so a legacy record is not judged here, and the first chained record after
//! one may start at 1 (what `Doc::commit_bytes` gives a device's first seal) or go on from it. THE
//! LIMIT: a device with no chained record has no bound seq at all; that is the open legacy row.
//! ONE DEFECT, ONE COUNT. Behind a cut the frontier row decides (`first_ok`: its seq + 1). A seq held
//! by two records is a FORK: counted by name, `holds` false, and not `broken` as well. A device's
//! first record whose `prev` left the file is `unsealed`'s, and a broken record's own seq still
//! places the next one, so `holds` is false in each case and one hole reads as one.

#[allow(unused_imports)]
use crate::prelude::*;
use alloc::collections::BTreeSet;
use crate::Record;
use super::seal::Head;
use super::frontier::Frontiers;
use super::seg::SCHEME_CHAINED;
use super::verify::{Chains, Counters};
use super::Ids;

type Actor = [u8; 32];

/// What the per-record count leaves behind it.
pub(super) struct Counted {
    /// Indices into `recs` of the live tombstones.
    pub tombs: Vec<usize>,
    /// Per record: its id holds and its link and seq stand. Only these may declare or admit.
    pub sound: Vec<bool>,
    /// Ids of chained records at a seq their device never reached in order. A seal over one vouches
    /// for nothing: its seq is the number `valid_at` and `covered` would believe.
    pub off_seq: BTreeSet<Actor>,
}

pub(super) fn count_records(ch: &Chains, heads: &[Head], fr: &Frontiers, keyed: &BTreeSet<Actor>, log_tip: Option<[u8; 32]>, ids: &dyn Ids, c: &mut Counters) -> Counted {
    let mut out = Counted { tombs: Vec::new(), sound: vec![false; ch.recs.len()], off_seq: BTreeSet::new() };
    c.records = ch.recs.len();
    c.forks = ch.ids_at.values().filter(|v| v.len() > 1).count();
    // The first row of an actor answers for it.
    let mut head_of: alloc::collections::BTreeMap<Actor, u64> = alloc::collections::BTreeMap::new();
    let mut n = 0u64;
    for h in heads {
        head_of.entry(h.actor_id).or_insert(n);
        n += 1;
    }
    for (actor, idx) in &ch.by_actor {
        let head_tip = head_of.get(actor).and_then(|i| heads.get(*i as usize)).map(|h| h.tip).or(log_tip);
        let rec_at = |i: usize| ch.recs.get(i).map(|(_, r)| r);
        let row = fr.of_actor(actor);
        // The device's oldest kept record continues the frontier a cut recorded for it.
        let first_ok = |r: &Record| match fr.at(actor, r.actor_seq.wrapping_sub(1)) {
            Some(f) => f.tip == r.prev,
            None => row.is_none(),
        };
        let judged = keyed.contains(actor);
        let forked = |r: &Record| ch.ids_at.get(&(*actor, r.actor_seq)).is_some_and(|v| v.len() > 1);
        // M0's rule (dowiz-hub `chain.rs` `walk_check`), per device: the id scheme may move
        // from legacy to chained ONCE, oldest to newest, and never back. A legacy id newer
        // than a chained record of the same device is a record edited after the move and
        // repaired under the scheme the log had left -- the one forgery the old scheme lets
        // through -- and it is broken, whatever its id says. Across a cut the rule continues
        // from the frontier row's `scheme` (W-MG-M3b): the kept part does not start over.
        let mut moved = row.is_some_and(|f| f.scheme == SCHEME_CHAINED);
        // The seq of the device's previous chained record or tombstone on this chain.
        let mut last: Option<u64> = None;
        // The seq cell of the legacy record just before, which nothing binds.
        let mut soft: Option<u64> = None;
        for (j, &i) in idx.iter().enumerate() {
            let Some(r) = rec_at(i) else { continue };
            let prev_ok = if j == 0 { first_ok(r) } else { idx.get(j.wrapping_sub(1)).and_then(|&p| rec_at(p)).is_some_and(|p| p.id == r.prev) };
            let next_names_me = match idx.get(j + 1).and_then(|&n| rec_at(n)) {
                Some(n) => n.prev == r.id,
                None => head_tip == Some(r.id),
            };
            let dense = !judged
                || forked(r)
                || match last {
                    Some(l) => l.checked_add(1) == Some(r.actor_seq),
                    None => row.is_some() || r.actor_seq == 1 || soft.and_then(|l| l.checked_add(1)) == Some(r.actor_seq) || (j == 0 && r.prev != [0u8; 32] && !ch.ids.contains(&r.prev)),
                };
            if !prev_ok {
                c.broken += 1;
                last = Some(r.actor_seq);
            } else if r.id == ids.chained(r) {
                moved = true;
                last = Some(r.actor_seq);
                if dense {
                    c.chained += 1;
                    if let Some(s) = out.sound.get_mut(i) {
                        *s = true;
                    }
                } else {
                    c.broken += 1;
                    out.off_seq.insert(r.id);
                }
            } else if ids.legacy(r.flags, &r.prev, &r.payload, &r.id) {
                last = None;
                soft = Some(r.actor_seq);
                if moved {
                    c.broken += 1;
                } else {
                    c.legacy += 1;
                }
            } else if ids.redacted(r.flags, &r.payload) && next_names_me && (dense || last.is_none()) {
                last = Some(r.actor_seq);
                c.redacted += 1;
                out.tombs.push(i);
            } else {
                last = Some(r.actor_seq);
                c.broken += 1;
            }
        }
    }
    out
}
