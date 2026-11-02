//! THE SECOND DOOR (W-MG-P1, docs/design/MESH-TRANSPORT-OVER-EINFILE.md §3): `Doc::tail_into` lays the
//! same union as `Doc::merge_into` from a `Tail` in place of a whole file, through the same
//! `stage_union`, in the same order. Kill test K-T1 holds the two to the byte (`tail/tests.rs`): if
//! they differ, there are two ways into a file and this door goes.
//!
//! WHAT IS WEAKER, BY EXACTLY ONE CHECK. `merge_into` runs `verify` on `theirs` alone. A tail is not a
//! file, so it cannot: only the union is judged, under our roots (`UnionNotWhole`). A file that
//! fails on its own by something it does not send is therefore refused by the first door and
//! taken by this one, and what lands is the part the union can vouch for.
//!
//! WHAT `merge_into` READS FROM `theirs`, AND WHERE IT COMES FROM HERE:
//!
//!   * their tail: `new_records`, which also walks each device on from our end (`Seq`, `Fork`).
//!     A record we hold must be the same record: `Fork` for another id, `ForgetOneSide` for other
//!     bytes under one id. So a tail built against an older position of ours lands the same.
//!   * our tail: our records past the sender's row. The rows are checked against the union (`Fork`,
//!     `Seq`), because a sender that is BEHIND on a fork sends no record to compare.
//!   * KEYS: built from the ADMITs in their tail, the newest per actor, as the writer keeps it, then
//!     through `keys_of` itself, so `KeyClash` is one rule. Sorted by actor, as a KEYS table is.
//!   * THE REVOKES ITS `verify` FOUND EFFECTIVE (`theirs_revokes`). A side that holds has no keyed
//!     record outside an ok seal, so every REVOKE on a keyed device's chain, no tombstone, is
//!     effective there. `keyed` is the side's own, which is why a key only we admitted does not
//!     count: the union's `verify` judges that (`merge.rs`, `keyed` IS NOT MONOTONE). If the side does
//!     not hold, the rule overreads a REVOKE, and then the table and the fold disagree: refused.
//!   * seals: ours, then theirs past them; `agree` first, on both sides of the wire, and the fold at
//!     the top after, so one key's two chains are `SealFork` wherever they part.
//!
//! COST BEFORE TRUST (§11): everything a hash can refuse comes before the first signature of ours.

#[allow(unused_imports)]
use crate::prelude::*;
use alloc::collections::{BTreeMap, BTreeSet};
use crate::{Record, Store};
use super::anch::Anchor;
use super::cut::{read_old, Old};
use super::front::{agree, folds, front_of, seal_map, Place, Tail};
use super::keys::{parse_admit, parse_revoke, Key};
use super::merge::{anchor_key, check_tail, genesis, keys_of, revoked, rows_of, same_seal, stage_union, Actor, Union};
use super::merge_body::MERGE_MARK;
use super::seal::Seal;
use super::verify::{verify, verify_with, Trust};
use super::{Doc, DocError, Ids, Verifier};

/// The records of `tail` this file lacks, in the tail's order.
fn new_records(o: &Old, tail: &Tail) -> Result<Vec<Record>, DocError> {
    let mut held: BTreeMap<(Actor, u64), &Record> = BTreeMap::new();
    let mut top: BTreeMap<Actor, (u64, Actor)> = BTreeMap::new();
    for (_, r) in &o.recs {
        held.insert((r.actor_pubkey, r.actor_seq), r);
        top.insert(r.actor_pubkey, (r.actor_seq, r.id));
    }
    let mut out = Vec::new();
    for r in &tail.recs {
        let (actor, seq) = (r.actor_pubkey, r.actor_seq);
        if let Some(x) = held.get(&(actor, seq)) {
            if x.id != r.id {
                return Err(DocError::Fork { actor, seq, ours: x.id, theirs: r.id });
            }
            if **x != *r {
                return Err(DocError::ForgetOneSide);
            }
            continue;
        }
        if let Some((s, id)) = top.get(&actor).copied() {
            let want = s.saturating_add(1);
            if seq != want {
                return Err(DocError::Seq { want, got: seq });
            }
            if r.prev != id {
                return Err(DocError::Fork { actor, seq: s, ours: id, theirs: r.prev });
            }
        }
        top.insert(actor, (seq, r.id));
        out.push(r.clone());
    }
    Ok(out)
}

/// A sender's rows are a claim its tail must make good: a row at a seq the union holds names the record
/// there, and no row stands past what the union holds. A MERGE copies these rows, and a row no record
/// holds would be sealed as `merges_bad`.
fn check_rows(o: &Old, their_tail: &[Record], places: &[Place]) -> Result<(), DocError> {
    let mut at: BTreeMap<(Actor, u64), Actor> = BTreeMap::new();
    let mut top: BTreeMap<Actor, u64> = BTreeMap::new();
    for r in o.recs.iter().map(|(_, r)| r).chain(their_tail.iter()) {
        at.insert((r.actor_pubkey, r.actor_seq), r.id);
        top.insert(r.actor_pubkey, r.actor_seq);
    }
    for p in places {
        let (actor, seq) = (p.row.actor_id, p.row.seq);
        if let Some(id) = at.get(&(actor, seq)).filter(|id| **id != p.row.tip) {
            return Err(DocError::Fork { actor, seq, ours: *id, theirs: p.row.tip });
        }
        let end = top.get(&actor).copied().unwrap_or(0);
        if seq > end {
            return Err(DocError::Seq { want: end.saturating_add(1), got: seq });
        }
    }
    Ok(())
}

/// Our records past the sender's place, in file order. Only a device with a HEADS row has a place.
fn our_tail_of(o: &Old, places: &[Place], theirs: &[Record]) -> Vec<Record> {
    let hosted: BTreeSet<Actor> = o.heads.iter().map(|h| h.actor_id).collect();
    let mut at: BTreeMap<Actor, u64> = places.iter().map(|p| (p.row.actor_id, p.row.seq)).collect();
    for r in theirs {
        let e = at.entry(r.actor_pubkey).or_insert(0);
        *e = (*e).max(r.actor_seq);
    }
    o.recs.iter().map(|(_, r)| r).filter(|r| hosted.contains(&r.actor_pubkey) && r.actor_seq > at.get(&r.actor_pubkey).copied().unwrap_or(0)).cloned().collect()
}

/// `merge_into`'s `t`, as far as `keys_of` reads it: the ADMITs of their tail as KEY rows, the newest
/// per actor, each pointing at its place in `their_tail`.
fn their_keys(o: &Old, their_tail: &[Record], ids: &dyn Ids) -> Old {
    let mut rows: BTreeMap<Actor, Key> = BTreeMap::new();
    let mut recs = Vec::with_capacity(their_tail.len());
    for (i, r) in their_tail.iter().enumerate() {
        recs.push((i + 1, r.clone()));
        if let Some((actor_id, alg, valid_from, pk)) = parse_admit(ids.body(r.flags, &r.payload)) {
            rows.insert(actor_id, Key { actor_id, alg, valid_from, revoked_at: 0, admit_rec: Some(i + 1), pk });
        }
    }
    Old { flags: o.flags, recs, heads: Vec::new(), keys: rows.into_values().collect(), segs: Vec::new(), anchors: Vec::new() }
}

/// The REVOKEs `verify` would find effective on the sender's side (header).
fn theirs_revokes(o: &Old, our_tail: &[Record], their_tail: &[Record], keys: &[(Key, Option<Actor>)], t: &Old, ids: &dyn Ids) -> Vec<(Actor, i64)> {
    let only_ours: BTreeSet<Actor> = our_tail.iter().map(|r| r.id).collect();
    let mut keyed: BTreeSet<Actor> = keys.iter().filter(|(_, admit)| admit.is_none_or(|id| !only_ours.contains(&id))).map(|(k, _)| k.actor_id).collect();
    keyed.extend(t.keys.iter().map(|k| k.actor_id));
    let shared = o.recs.iter().map(|(_, r)| r).filter(|r| !only_ours.contains(&r.id));
    shared
        .chain(their_tail.iter())
        .filter(|r| keyed.contains(&r.actor_pubkey) && !ids.redacted(r.flags, &r.payload))
        .filter_map(|r| parse_revoke(ids.body(r.flags, &r.payload)).map(|(target, after, _)| (target, after)))
        .collect()
}

/// Our chains with their seals on top, and whether any landed. A seal at a seq we have is the same
/// seal; one below our top at a seq we do not have is a second chain.
fn seal_union(mut map: BTreeMap<Actor, Vec<Seal>>, tail: &Tail, ids: &dyn Ids) -> Result<(BTreeMap<Actor, Vec<Seal>>, bool), DocError> {
    let none: Vec<Seal> = Vec::new();
    for p in &tail.front.places {
        agree(map.get(&p.row.actor_id).unwrap_or(&none), p, ids)?;
    }
    let mut more = false;
    for s in &tail.seals {
        let e = map.entry(s.actor_id).or_default();
        match e.iter().find(|x| x.seq == s.seq) {
            Some(x) if same_seal(x, s) => {}
            Some(_) => return Err(DocError::SealFork { actor: s.actor_id, seq: s.seq }),
            None if e.last().is_some_and(|top| s.seq < top.seq) => return Err(DocError::SealFork { actor: s.actor_id, seq: s.seq }),
            None => {
                e.push(s.clone());
                more = true;
            }
        }
    }
    for p in &tail.front.places {
        let Some(chain) = map.get(&p.row.actor_id).filter(|c| c.last().is_some_and(|top| top.seq == p.row.seq)) else { continue };
        if folds(&p.row.actor_id, chain, ids)?.last() != Some(&p.seals) {
            return Err(DocError::SealFork { actor: p.row.actor_id, seq: p.row.seq });
        }
    }
    Ok((map, more))
}

impl Doc {
    /// Glue `tail` onto `ours` into `fresh`: `merge_into`'s contract, word for word, with `tail` in
    /// the place of `theirs`. On any `Err`, and on `wrote` false, the caller's file stands as it was;
    /// on `both_wrote` the caller owes the MERGE.
    pub fn tail_into(ours: &Store, tail: &Tail, fresh: &mut Store, ids: &dyn Ids, v: &dyn Verifier, trust: Trust<'_>) -> Result<Union, DocError> {
        let o = read_old(ours)?;
        let map = seal_map(&o, ours)?;
        let mine = front_of(&o, &map, ids)?;
        if mine.doc != tail.front.doc {
            return Err(DocError::ForeignRoot);
        }
        if mine.seg_hash != tail.front.seg_hash {
            return Err(DocError::CutOneSide { root: o.segs.first().map_or(tail.front.seg_root, |s| s.root) });
        }
        let their_tail = new_records(&o, tail)?;
        check_rows(&o, &their_tail, &tail.front.places)?;
        let our_tail = our_tail_of(&o, &tail.front.places, &their_tail);
        check_tail(&our_tail, ids)?;
        check_tail(&their_tail, ids)?;
        let roots: Vec<Actor> = match trust {
            Trust::Only(list) => list.to_vec(),
            Trust::Genesis => genesis(&o).into_iter().collect(),
        };
        let (ro, decl_o) = verify_with(ours, ids, v, Trust::Only(&roots))?;
        if !ro.holds() {
            return Err(DocError::NotWhole { ours: true });
        }
        let t = their_keys(&o, &their_tail, ids);
        let keys = keys_of(&o, &t)?;
        let all: Vec<&Record> = o.recs.iter().map(|(_, r)| r).chain(their_tail.iter()).collect();
        let revokes = [decl_o.revokes, theirs_revokes(&o, &our_tail, &their_tail, &keys, &t, ids)].concat();
        let cell = revoked(&all, &revokes)?;
        let (seals, more_seals) = seal_union(map, tail, ids)?;
        let real = |tail: &[Record]| tail.iter().any(|r| !ids.body(r.flags, &r.payload).starts_with(MERGE_MARK));
        let seen: BTreeSet<_> = o.anchors.iter().map(anchor_key).collect();
        let new_anchors: Vec<&Anchor> = tail.anchors.iter().filter(|a| !seen.contains(&anchor_key(a))).collect();
        let both_wrote = real(&our_tail) && real(&their_tail);
        let wrote = !their_tail.is_empty() || !new_anchors.is_empty() || more_seals;
        if wrote {
            let mut tx = fresh.begin()?;
            let doc = stage_union(fresh, &mut tx, &o, &all, &keys, &cell, &seals, &new_anchors)?;
            fresh.commit_bytes(&tx, doc);
            if !verify(fresh, ids, v, Trust::Only(&roots))?.holds() {
                return Err(DocError::UnionNotWhole);
            }
        }
        let theirs = tail.front.places.iter().map(|p| p.row).collect();
        Ok(Union { ours: rows_of(&o), theirs, seg_root: o.segs.first().map_or([0u8; 32], |s| s.root), our_tail, their_tail, wrote, both_wrote })
    }
}

#[cfg(test)]
mod tests;
