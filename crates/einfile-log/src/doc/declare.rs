//! What the signed records DECLARE (W-MG-M3/M4, W-MG-REVOKE): the TRUNCATEs that vouch for
//! segments, the FORGOTTENs that vouch for redactions, and the REVOKEs that end a key
//! (`settle.rs` folds those; a redaction erases what its record declared). A declaration counts only when it is
//! EFFECTIVE -- its record chains (the id verifies), its actor is admitted, and an ok seal
//! of that actor covers its seq -- so a declaration a revoked key wrote after its revoke,
//! or one nobody sealed, declares nothing. That is M4's second kill test: a FORGOTTEN by a
//! revoked key leaves `redacted != declared`. A TRUNCATE asks more: its actor is a ROOT.

#[allow(unused_imports)]
use crate::prelude::*;
use alloc::collections::{BTreeMap, BTreeSet};
use crate::Record;
use super::frontier::Frontiers;
use super::keys::parse_revoke;
use super::merge_body::{walk_merge, MERGE_MARK};
use super::seg::{chain_hash, parse_forgotten, parse_truncate, Erased, Segment, Truncate};
use super::verify::Chains;
use super::{DocError, Ids};

type Actor = [u8; 32];

/// What one sound record declares, read once: `settle.rs` asks every round, and a round that
/// parsed and hashed the log again made a file of REVOKEs quadratic.
pub(crate) enum Body {
    Truncate(Truncate),
    Forgotten(Vec<Erased>),
    Revoke(Actor, i64, [u8; 32]),
    /// It carries the mark; `merges_bad` walks it (`merge_body.rs`).
    Merge,
}

/// `(index into recs, what it declares)`, oldest first, for each `sound` record that declares
/// anything and is no tombstone. `sound` is `count.rs`'s answer: an id that holds on a device's
/// chain, so no declaration stands on a record the counters call broken.
pub(crate) fn declarations(recs: &[(usize, Record)], sound: &[bool], ids: &dyn Ids) -> Vec<(usize, Body)> {
    let mut out = Vec::new();
    for (i, (_, r)) in recs.iter().enumerate() {
        if !sound.get(i).copied().unwrap_or(false) || ids.redacted(r.flags, &r.payload) {
            continue;
        }
        let body = ids.body(r.flags, &r.payload);
        if let Some(t) = parse_truncate(body) {
            out.push((i, Body::Truncate(t)));
        } else if let Some((_, list)) = parse_forgotten(body) {
            out.push((i, Body::Forgotten(list)));
        } else if let Some((a, after, tip)) = parse_revoke(body) {
            out.push((i, Body::Revoke(a, after, tip)));
        } else if body.starts_with(MERGE_MARK) {
            out.push((i, Body::Merge));
        }
    }
    out
}

/// The effective declarations of a log.
#[derive(Debug, Default)]
pub(crate) struct Declared {
    /// Σ `n_redacted` over effective FORGOTTEN records.
    pub forgotten: u64,
    /// Which of `declarations` are effective FORGOTTENs (`erased`).
    pub lists: Vec<u64>,
    pub truncates: Vec<Truncate>,
    /// `(target, after_seq)` of the effective REVOKEs, in file order.
    pub revokes: Vec<(Actor, i64)>,
    /// Effective REVOKEs of a key whose chain does not stand at the tip the revoker saw: it was
    /// rewritten at or below `after_seq` after the fact, or dropped. Counted in `broken`. `placed`'s
    /// reading: under a cut only the seq binds (FORMAT §14). `after_seq` <= 0 names no record, a
    /// REVOKE of no key changes nothing, and a key that revokes itself after this very record
    /// cannot name it: its id covers the body.
    pub off_chain: usize,
    /// Which of `declarations` are effective MERGEs (`merges_bad`).
    pub merges: Vec<u64>,
}

/// `recs` oldest first; `admitted` and `covered` are `verify`'s fixpoint (per actor, the
/// highest seq an ok seal signed).
///
/// `signatures` false (a verifier that knows no algorithm, `Verifier::checks_signatures`): no seal
/// can be judged, so a declaration is taken on its chain alone. The structure still binds: a
/// dropped segment stays refused, and `Report::holds` stays false because no seal is ok.
#[allow(clippy::too_many_arguments)]
pub(crate) fn effective(ch: &Chains, fr: &Frontiers, keyed: &BTreeMap<Actor, u64>, decls: &[(usize, Body)], admitted: &BTreeSet<Actor>, roots: &BTreeSet<Actor>, covered: &BTreeMap<Actor, u64>, signatures: bool) -> Declared {
    let recs = &ch.recs;
    let mut out = Declared::default();
    let mut n = 0u64;
    for (i, body) in decls {
        let at = n;
        n += 1;
        let Some((_, r)) = recs.get(*i) else { continue };
        if signatures && !(admitted.contains(&r.actor_pubkey) && covered.get(&r.actor_pubkey).is_some_and(|&s| s >= r.actor_seq)) {
            continue;
        }
        match body {
            // ONLY A ROOT CUTS (W-MG-MERGE2, W-MG-TAM's trace H1). A cut carries nothing but the
            // TRUNCATE's word for what left, and the REVOKE of another device can be in it: an
            // admitted key that could vouch for one could cut away its own revocation.
            Body::Truncate(t) if !signatures || roots.contains(&r.actor_pubkey) => out.truncates.push(*t),
            Body::Truncate(_) => {}
            Body::Forgotten(list) => {
                out.forgotten = out.forgotten.saturating_add(list.len() as u64);
                out.lists.push(at);
            }
            Body::Revoke(a, after, tip) => {
                out.revokes.push((*a, *after));
                let seq = *after as u64;
                if *after > 0 && keyed.contains_key(a) && !(r.actor_pubkey == *a && r.actor_seq == seq) && !placed(a, seq, tip, ch, fr) {
                    out.off_chain += 1;
                }
            }
            Body::Merge => out.merges.push(at),
        }
    }
    out
}

/// Does `r` name a record of this log, or a seq a cut of its actor took?
fn placed(actor: &Actor, seq: u64, tip: &[u8; 32], ch: &Chains, fr: &Frontiers) -> bool {
    ch.ids_at.get(&(*actor, seq)).is_some_and(|v| v.contains(tip)) || fr.of_actor(actor).is_some_and(|f| seq <= f.seq)
}

/// Effective MERGEs that are no MERGE, or that name a position this file does not hold
/// (`merge_body.rs`): a signed claim about a record nobody can show.
pub(crate) fn merges_bad(decls: &[(usize, Body)], d: &Declared, ch: &Chains, fr: &Frontiers, ids: &dyn Ids) -> usize {
    let good = |i: usize| ch.recs.get(i).is_some_and(|(_, r)| walk_merge(ids.body(r.flags, &r.payload), &mut |_, row| placed(&row.actor_id, row.seq, &row.tip, ch, fr), &mut |_| ()).is_some());
    d.merges.iter().filter(|n| !decls.get(**n as usize).is_some_and(|(i, _)| good(*i))).count()
}

/// What the effective FORGOTTENs list, with who signed each (`foreign_tombs`).
pub(crate) fn erased(recs: &[(usize, Record)], decls: &[(usize, Body)], d: &Declared) -> Vec<(Erased, Actor)> {
    let mut out = Vec::new();
    for n in &d.lists {
        if let Some((i, Body::Forgotten(list))) = decls.get(*n as usize) {
            if let Some((_, r)) = recs.get(*i) {
                out.extend(list.iter().map(|e| (*e, r.actor_pubkey)));
            }
        }
    }
    out
}

/// WHICH TOMBSTONE, BY WHOM (W-MG-REVOKE, W-MG-HOLES): a tombstone is one bit anyone can set and
/// it cannot say what it was, so law 9's sum balanced any record's erasure: a thief could erase
/// the REVOKE of its own key and sign the declaration, and a forger with no key could give an
/// erased record back and tombstone another in its place (`atk_holes` hole 3), or rewrite a
/// tombstone's bytes (hole 3b). Every live tombstone (`tombs`, indices into `recs`) must be listed
/// by an effective FORGOTTEN of its AUTHOR or of a ROOT (`trusted`), with the digest of what stands
/// there now; every listed id still in the log must be a tombstone. This counts what breaks that.
/// A listed id not in the log left under a cut. Note what a root is: under `Trust::Genesis` the
/// unsigned table names them, so this binds only a verifier that brings its roots (`Trust::Only`).
pub(crate) fn foreign_tombs(recs: &[(usize, Record)], tombs: &[usize], erased: &[(Erased, Actor)], roots: &BTreeSet<Actor>, ids: &dyn Ids) -> usize {
    let mut listed: BTreeMap<Actor, u64> = BTreeMap::new();
    let mut i = 0u64;
    for (e, _) in erased {
        listed.insert(e.id, i);
        i += 1;
    }
    let mut stones: BTreeSet<Actor> = BTreeSet::new();
    let mut foreign = 0usize;
    for (_, r) in tombs.iter().filter_map(|t| recs.get(*t)) {
        stones.insert(r.id);
        let ok = listed.get(&r.id).and_then(|n| erased.get(*n as usize)).is_some_and(|(e, by)| (*by == r.actor_pubkey || roots.contains(by)) && e.digest == ids.chained(r));
        foreign += usize::from(!ok);
    }
    foreign + recs.iter().filter(|(_, r)| listed.contains_key(&r.id) && !stones.contains(&r.id)).count()
}

/// Every segment must be vouched for, and one signature does it: an effective TRUNCATE names the
/// NEWEST exactly, six fields and `chain_hash` over the whole chain, so no number `verify` sums
/// from any segment is unsigned, and no record leaves a file without a key.
pub(crate) fn check_segments(segs: &[Segment], truncates: &[Truncate], ids: &dyn Ids) -> Result<(), DocError> {
    let Some(newest) = segs.first() else { return Ok(()) };
    let hash = chain_hash(segs, ids);
    if truncates.iter().any(|t| newest.is_named_by(t) && Some(t.seg_hash) == hash) {
        return Ok(());
    }
    Err(DocError::CutUndeclared { root: newest.root })
}
