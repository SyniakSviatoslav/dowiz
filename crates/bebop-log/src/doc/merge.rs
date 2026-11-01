//! THE MERGE (W-MG-M2, M2-min of docs/design/M2-merge-design.md): two copies of one DOC become one
//! file with no server. `Doc::merge_into` rebuilds, as `cut.rs` and `copy.rs` do: every record,
//! seal and key of both sides, VERBATIM, into a FRESH store (K3 measured it: every real ML-DSA seal
//! survives a union), and nothing of either side is signed again. What this adds is the deciding:
//!
//!   * WHOLE. Each file passes `verify` first, `theirs` under OUR roots, never its own. The union
//!     passes it too, before the caller sees it. There is no verify exception for a merge: a MERGE
//!     is one more record under one more seal, and a union that needs a pass is refused.
//!   * ONE HISTORY PER DEVICE. Per actor, one side's chain is a prefix of the other's, id by id, or
//!     `Fork`. Dedupe by id follows from that. Records are ours as they stand, then their tail in
//!     their order: the order across devices is signed by nobody, and each device's is kept.
//!   * HEADS as a pointwise maximum: per actor the longer seal chain, which must extend the other,
//!     or `SealFork`: the records agree and the key signed them twice, so a merge would lose a
//!     signature. A seal of theirs that extends ours is carried with no new record: a merge
//!     loses no record and no signature.
//!   * `keyed` IS NOT MONOTONE: an actor one side has no key for is not judged there, and the other
//!     side's KEY row judges its records in the union. Only the union's `verify` sees that
//!     (`UnionNotWhole`, `lean_tests.rs`).
//!   * NAMED REFUSALS for what M2-min does not decide: a cut on one side (`CutOneSide`), an erasure
//!     on one side (`ForgetOneSide`), a REVOKE of a device that wrote past it (`RevokedTail`), a
//!     legacy tail (`LegacyTail`), other roots (`ForeignRoot`, `KeyClash`). Nothing is written.
//!
//! WHAT THE CALLER DOES NEXT. This writes no MERGE: it has no signer, and it does not know what a
//! conflict is. On `both_wrote` the caller names the conflicts over `their_tail` and `our_tail`,
//! and commits `merge_body` through `Doc::commit_bytes`. GLUING DETECTS AND NEVER RESOLVES: both
//! sides' records stay in the file, whatever the conflict. With only one side writing (or one side's
//! tail nothing but MERGEs) there is one history, and no MERGE: that is what stops two devices
//! answering each other's MERGE for ever.
//!
//! THE LIMIT, pinned in `merge/tests.rs`: a union with no MERGE holds, so a file holder may strip a
//! MERGE and its seal and leave a shorter true file (`settle.rs`, ROLLBACK). One file cannot know;
//! a merge against any copy that has the MERGE gets it back.

#[allow(unused_imports)]
use crate::prelude::*;
use alloc::collections::{BTreeMap, BTreeSet};
use crate::{EvLog, Record, Store, Tx};
use super::anch::{stage_anchor, Anchor};
use super::budget::Budget;
use super::cut::{read_old, seals_of, Old};
use super::keys::{narrower, stage_key, stage_keys, valid_at, Key};
use super::merge_body::{Row, MERGE_MARK};
use super::seal::{stage_heads, stage_seal, Head, Seal};
use super::seg::{parse_forgotten, stage_seg, Segment};
use super::verify::{verify, verify_with, Trust};
use super::write::{stage_doc, Refs};
use super::{Doc, DocError, Ids, Verifier};

type Actor = [u8; 32];

/// What a merge found. `wrote` false: `fresh` is untouched, this file already holds everything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Union {
    /// Each side's HEADS as they stood, in table order: the MERGE's two lists.
    pub ours: Vec<Row>,
    pub theirs: Vec<Row>,
    /// The newest shared SEG's root, 0^32 if the log was never cut.
    pub seg_root: [u8; 32],
    /// The records the other side lacked, oldest first.
    pub our_tail: Vec<Record>,
    pub their_tail: Vec<Record>,
    pub wrote: bool,
    /// Both sides wrote something other than MERGEs: two histories, so a MERGE is owed.
    pub both_wrote: bool,
}

fn rows_of(o: &Old) -> Vec<Row> {
    o.heads.iter().map(|h| Row { actor_id: h.actor_id, seq: h.seq, tip: h.tip }).collect()
}

fn genesis(o: &Old) -> BTreeSet<Actor> {
    o.keys.iter().filter(|k| k.admit_rec.is_none()).map(|k| k.actor_id).collect()
}

/// A segment as a fact: its offsets are this file's.
fn plain(segs: &[Segment]) -> Vec<Segment> {
    segs.iter().map(|s| Segment { prev: None, ..s.clone() }).collect()
}

fn chains(o: &Old) -> BTreeMap<Actor, Vec<&Record>> {
    let mut m: BTreeMap<Actor, Vec<&Record>> = BTreeMap::new();
    for (_, r) in &o.recs {
        m.entry(r.actor_pubkey).or_default().push(r);
    }
    m
}

/// The ids each side has and the other lacks. Same id on both sides and other bytes can only be a
/// tombstone in a file that holds: the id stays and the payload left.
fn missing(o: &Old, t: &Old) -> Result<(BTreeSet<Actor>, BTreeSet<Actor>), DocError> {
    let (mo, mt) = (chains(o), chains(t));
    let (mut ours, mut theirs) = (BTreeSet::new(), BTreeSet::new());
    let none: Vec<&Record> = Vec::new();
    for actor in mo.keys().chain(mt.keys()) {
        let (a, b) = (mo.get(actor).unwrap_or(&none), mt.get(actor).unwrap_or(&none));
        for (x, y) in a.iter().zip(b.iter()) {
            if x.id != y.id {
                return Err(DocError::Fork { actor: *actor, seq: x.actor_seq, ours: x.id, theirs: y.id });
            }
            if x != y {
                return Err(DocError::ForgetOneSide);
            }
        }
        let n = a.len().min(b.len());
        ours.extend(a.iter().skip(n).map(|r| r.id));
        theirs.extend(b.iter().skip(n).map(|r| r.id));
    }
    Ok((ours, theirs))
}

/// A tail record M2-min may not take in.
fn check_tail(tail: &[Record], ids: &dyn Ids) -> Result<(), DocError> {
    for r in tail {
        if ids.redacted(r.flags, &r.payload) || parse_forgotten(ids.body(r.flags, &r.payload)).is_some() {
            return Err(DocError::ForgetOneSide);
        }
        if ids.chained(r) != r.id {
            return Err(DocError::LegacyTail { actor: r.actor_pubkey, seq: r.actor_seq });
        }
    }
    Ok(())
}

/// The REVOKEs each side's `verify` found effective, folded per target (`settle.rs`'s rule: the
/// earliest wins), and `RevokedTail` for any record past one. A record that merely reads like a
/// REVOKE (an actor with no key, an unsound record) declares nothing, here as there.
fn revoked(recs: &[&Record], revokes: &[(Actor, i64)]) -> Result<BTreeMap<Actor, i64>, DocError> {
    let mut cell: BTreeMap<Actor, i64> = BTreeMap::new();
    for (target, after) in revokes {
        let c = cell.entry(*target).or_insert(0);
        *c = narrower(*c, *after);
    }
    for r in recs {
        if let Some(c) = cell.get(&r.actor_pubkey).filter(|c| !valid_at(**c, r.actor_seq)) {
            return Err(DocError::RevokedTail { actor: r.actor_pubkey, after_seq: *c });
        }
    }
    Ok(cell)
}

/// Every key of both sides once, `KeyClash` when one actor names two; `ours` comes first, and each
/// key's ADMIT record named by id, because the offsets are the side's.
fn keys_of(o: &Old, t: &Old) -> Result<Vec<(Key, Option<Actor>)>, DocError> {
    let id_at = |s: &Old| -> BTreeMap<usize, Actor> { s.recs.iter().map(|(at, r)| (*at, r.id)).collect() };
    let (ido, idt) = (id_at(o), id_at(t));
    let mut out: Vec<(Key, Option<Actor>)> = Vec::new();
    for (k, map) in o.keys.iter().map(|k| (k, &ido)).chain(t.keys.iter().map(|k| (k, &idt))) {
        let admit = match k.admit_rec {
            Some(at) => Some(*map.get(&at).ok_or(DocError::Corrupt("a KEY's ADMIT record is not in the log"))?),
            None => None,
        };
        match out.iter().find(|(old, _)| old.actor_id == k.actor_id) {
            Some((old, _)) if old.pk != k.pk || old.alg != k.alg || old.valid_from != k.valid_from => return Err(DocError::KeyClash(k.actor_id)),
            Some(_) => {}
            None => out.push((k.clone(), admit)),
        }
    }
    Ok(out)
}

fn same_seal(a: &Seal, b: &Seal) -> bool {
    a.seq == b.seq && a.tip == b.tip && a.claimed_ms == b.claimed_ms && a.alg == b.alg && a.sig == b.sig
}

/// Per actor, the longer seal chain, oldest first; the shorter must be a prefix of it. And whether
/// `theirs` held a seal `ours` did not.
fn seal_chains(o: &Old, ours: &Store, t: &Old, theirs: &Store) -> Result<(BTreeMap<Actor, Vec<Seal>>, bool), DocError> {
    let mut out: BTreeMap<Actor, Vec<Seal>> = BTreeMap::new();
    let mut more = false;
    for (s, st, side) in [(o, ours, false), (t, theirs, true)] {
        let mut budget = Budget::of(st);
        for h in &s.heads {
            let mut c = seals_of(st, h, s.recs.len() + 1, &mut budget)?;
            c.reverse();
            let e = out.entry(h.actor_id).or_default();
            if let Some((x, _)) = e.iter().zip(c.iter()).find(|(x, y)| !same_seal(x, y)) {
                return Err(DocError::SealFork { actor: h.actor_id, seq: x.seq });
            }
            if c.len() > e.len() {
                more |= side;
                *e = c;
            }
        }
    }
    Ok((out, more))
}

/// One anchor's identity: unsigned, and carried like any other, judged by whoever checks it.
fn anchor_key(a: &Anchor) -> (i64, Actor, i64, Vec<u8>) {
    (a.kind, a.over, a.time_ms, a.proof.clone())
}

impl Doc {
    /// Glue `theirs` onto `ours` into `fresh` (an empty store the caller sized; `DocError::Store`
    /// for room comes back for the caller to retry larger). `trust` is OUR roots: `Trust::Genesis`
    /// means this file's own genesis keys, and `theirs` is judged by the same keys. On any `Err`,
    /// and on `wrote` false, the caller's file stands as it was. On `both_wrote` the caller still
    /// owes the MERGE (header).
    pub fn merge_into(ours: &Store, theirs: &Store, fresh: &mut Store, ids: &dyn Ids, v: &dyn Verifier, trust: Trust<'_>) -> Result<Union, DocError> {
        let (o, t) = (read_old(ours)?, read_old(theirs)?);
        if genesis(&o) != genesis(&t) || o.flags != t.flags {
            return Err(DocError::ForeignRoot);
        }
        let roots: Vec<Actor> = match trust {
            Trust::Only(list) => list.to_vec(),
            Trust::Genesis => genesis(&o).into_iter().collect(),
        };
        let (ro, decl_o) = verify_with(ours, ids, v, Trust::Only(&roots))?;
        if !ro.holds() {
            return Err(DocError::NotWhole { ours: true });
        }
        let (rt, decl_t) = verify_with(theirs, ids, v, Trust::Only(&roots))?;
        if !rt.holds() {
            return Err(DocError::NotWhole { ours: false });
        }
        if plain(&o.segs) != plain(&t.segs) {
            return Err(DocError::CutOneSide { root: o.segs.first().or(t.segs.first()).map_or([0u8; 32], |s| s.root) });
        }
        let (not_ours, not_theirs) = missing(&o, &t)?;
        let tail = |s: &Old, set: &BTreeSet<Actor>| -> Vec<Record> { s.recs.iter().map(|(_, r)| r).filter(|r| set.contains(&r.id)).cloned().collect() };
        let (our_tail, their_tail) = (tail(&o, &not_ours), tail(&t, &not_theirs));
        check_tail(&our_tail, ids)?;
        check_tail(&their_tail, ids)?;
        let all: Vec<&Record> = o.recs.iter().map(|(_, r)| r).chain(their_tail.iter()).collect();
        let cell = revoked(&all, &[decl_o.revokes, decl_t.revokes].concat())?;
        let keys = keys_of(&o, &t)?;
        let (seals, more_seals) = seal_chains(&o, ours, &t, theirs)?;
        let real = |tail: &[Record]| tail.iter().any(|r| !ids.body(r.flags, &r.payload).starts_with(MERGE_MARK));
        let seen: BTreeSet<_> = o.anchors.iter().map(anchor_key).collect();
        let new_anchors: Vec<&Anchor> = t.anchors.iter().filter(|a| !seen.contains(&anchor_key(a))).collect();
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
        Ok(Union { ours: rows_of(&o), theirs: rows_of(&t), seg_root: o.segs.first().map_or([0u8; 32], |s| s.root), our_tail, their_tail, wrote, both_wrote })
    }
}

/// The union, one transaction: records, keys, seals, HEADS, anchors, segments, the DOC last.
#[allow(clippy::too_many_arguments)]
fn stage_union(st: &mut Store, tx: &mut Tx, o: &Old, recs: &[&Record], keys: &[(Key, Option<Actor>)], cell: &BTreeMap<Actor, i64>, seals: &BTreeMap<Actor, Vec<Seal>>, new_anchors: &[&Anchor]) -> Result<usize, DocError> {
    let mut log = EvLog::stage_init_in(st, tx)?;
    let mut at: BTreeMap<Actor, usize> = BTreeMap::new();
    for r in recs {
        log = EvLog::stage_append_in(st, tx, log, r, Some(&r.id))?;
        at.insert(r.id, st.follow(log, 1).ok_or(DocError::Corrupt("the copied record is not reachable"))?);
    }
    let mut rows = Vec::with_capacity(keys.len());
    for (k, admit) in keys {
        let admit_rec = match admit {
            Some(id) => Some(*at.get(id).ok_or(DocError::Corrupt("a KEY's ADMIT record is not in the union"))?),
            None => None,
        };
        let revoked_at = cell.get(&k.actor_id).copied().unwrap_or(0);
        rows.push((k.actor_id, stage_key(st, tx, &Key { admit_rec, revoked_at, ..k.clone() })?));
    }
    let keys_root = stage_keys(st, tx, &mut rows)?;
    let mut heads: Vec<Head> = Vec::with_capacity(seals.len());
    for (actor, chain) in seals {
        let mut prev = None;
        for s in chain {
            prev = Some(stage_seal(st, tx, &s.actor_id, s.seq, &s.tip, s.claimed_ms, prev, s.alg, &s.sig)?);
        }
        if let Some(top) = chain.last() {
            heads.push(Head { actor_id: *actor, seq: top.seq, tip: top.tip, seal: prev });
        }
    }
    let heads_root = stage_heads(st, tx, &mut heads)?;
    let mut anch = None;
    for a in o.anchors.iter().rev().chain(new_anchors.iter().rev().copied()) {
        anch = Some(stage_anchor(st, tx, a.kind, &a.over, a.time_ms, anch, &a.proof)?);
    }
    let mut seg = None;
    for s in o.segs.iter().rev() {
        seg = Some(stage_seg(st, tx, &Segment { prev: seg, ..s.clone() })?);
    }
    Ok(stage_doc(st, tx, o.flags, Refs { log: Some(log), tree: None, heads: Some(heads_root), keys: Some(keys_root), spec: None, anch, seg })?)
}

#[cfg(test)]
mod tests;
