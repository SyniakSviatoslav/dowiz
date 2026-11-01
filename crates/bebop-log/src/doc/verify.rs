//! `verify` (W-MG-M1): the counters a DOC image answers with, from its bytes, the caller's
//! `Ids` (how ids are derived), its `Verifier` (how signatures are checked) and its trust.
//!
//! What the counters say, in words (R-MAINGOAL-A §2d, to be printed verbatim by any CLI):
//! a seal counted `ok` means the holder of key K signed this tip at seq s of its chain and
//! nothing under the tip has changed since. It does NOT say the event happened, that a
//! quantity is true, or that `claimed_ms` is right. Time is M6's.
//!
//! The rules, each one a counter:
//!   * records   every record the log delivers (the chain's crc is checked first; a crc
//!               failure is a refusal, not a count -- a reader that cannot trust the walk
//!               counts nothing).
//!   * chained / legacy / redacted / broken   one of the four per record. A record whose
//!               `prev` does not name the previous record OF ITS DEVICE is broken whatever
//!               its id says; then `Ids::chained`, then `Ids::legacy` (old images; a legacy
//!               id NEWER than a chained record of its device is broken -- M0's rule, the
//!               scheme moves forward once), then a redaction mark whose LINK holds (the
//!               next record of the device, or the device's head, names it: `forget.rs`'s
//!               rule), else broken.
//!               Behind all of it, a keyed device's seqs must climb by one (`count.rs`), since
//!               `valid_at` and "covers" believe a seq: a chained record off its seq is broken and
//!               no ok seal may name it.
//!   * forks     `(actor, seq)` pairs that occur more than once (§2a: a named refusal, not
//!               the death of the identity).
//!   * seals_ok / seals_bad   every seal reachable from HEADS through `ref PREV_SEAL`. A seal
//!               is ok when its actor's key is ADMITTED (below), belongs to that actor
//!               (`Verifier::actor_of`), valid at its seq (an effective REVOKE after k breaks
//!               k+1), the tip names a record at `(actor, seq)`, and the signature over `seal::message` verifies with the previous
//!               seal's tip. Anything else -- a head row that disagrees with its seal, a crc
//!               failure, an unknown algorithm -- is bad.
//!   * keys_disagree   keys whose table `revoked_at` is not what the REVOKE records fold to,
//!               either way. A COUNTER AND NOT A REFUSAL: the records answer in full without
//!               the table, so a refusal would only let one forged cell hide every other
//!               counter; `holds` is false on it, which is what a refusal would have meant.
//!   * revokes_unsettled   REVOKEs that contradict each other (`settle.rs`), held in force.
//!               Not a term of `holds`: a contradiction always leaves a seal bad, the one that
//!               made the REVOKE effective and that the REVOKEs in force then break.
//!   * tree_unsigned   the DOC names a TREE or sets `FLAG_TREE_IS_FOLD`. No signature covers a
//!               tree and `verify` never reads one, so `holds` speaks for the log alone and is
//!               false on such a file until a signature does. Nothing writes a TREE today.
//!   * unsealed  records no signed tip vouches for, plus chains a signed tip reaches that end
//!               on nothing, plus a key's records above its own seals (`unsealed` below). `prev` is walked from the tip INSIDE every ok
//!               seal, never from a HEADS row: HEADS is signed by nobody, so a row names where
//!               to look and a signature decides what was found. Reachability and not "a seal
//!               at this seq": one seal over a tip vouches for every record under it, across
//!               devices (a device's first record chains onto the log) and across seq gaps
//!               between seals. A redaction keeps its id and `prev`, so it stays on the path;
//!               a chain ends at 0^32 or at a frontier tip a segment recorded. THE LIMIT: a
//!               whole tail gone with its seals and its row is a shorter true file, the
//!               rollback `settle.rs` names; this counter sees only what stays in the file.
//!   * tombs_foreign   live tombstones no FORGOTTEN of their author or of a root lists as they
//!               stand, plus listed ids still in the log that are no tombstone (`declare.rs`
//!               `foreign_tombs`): nobody but those two erases a record, and none is swapped.
//!
//! THE BOUND (`budget.rs`). Every walk pays from one budget of the image's cells before it copies,
//! each object once. One `verify` holds at most 8x the file of heap above the image, measured
//! worst 3x; time is O(n log n) plus rounds x passes x seals, each a lookup, each signature once.
//!
//! ADMISSION AND REVOCATION are one fixpoint over the signed records (`settle.rs`), never the
//! KEYS table: a key is admitted when it is trusted (`Trust`), or when its `ref ADMIT_REC`
//! names a record whose body is an ADMIT of exactly this key, written by an admitted actor at
//! a seq that actor's ok seals cover, while that actor's key was valid. A key is revoked by
//! its effective REVOKE records, and by nothing else: the table's `revoked_at` is an index.
//!
//! SEGMENTS AND DECLARATIONS (W-MG-M3/M4, `seg.rs`, `declare.rs`): `segments` / `cut` /
//! `cut_redacted` count what left the file under a root; every segment must be declared by
//! the newest's effective TRUNCATE, which signs the hash of the whole chain, or the file is REFUSED
//! by name (`DocError::CutUndeclared`). A
//! device's oldest kept record must continue the frontier the segment recorded for it
//! (seq + 1, `prev` = the frontier tip) or it is broken; a seal with no previous seal reads
//! its `prev_tip` from that same frontier. `declared` sums the effective FORGOTTENs (plus
//! what the segments carry), and `holds` requires `redacted + cut_redacted == declared`.

#[allow(unused_imports)]
use crate::prelude::*;
use alloc::collections::{BTreeMap, BTreeSet};
use crate::{EvLog, Record, Store};
use super::budget::{at_key, set_of, Budget};
use super::count::count_records;
use super::declare::{check_segments, declarations, erased, foreign_tombs};
use super::keys::read_keys_in;
use super::seal::{read_heads, Head};
use super::seals::load;
use super::frontier::Frontiers;
use super::seg::{read_segs_in, Segment};
use super::settle::{settle, Cx};
use super::{Doc, DocError, Ids, Verifier, FLAG_TREE_IS_FOLD};

#[cfg(test)]
mod tests;

/// The counters: M1's eight, M3/M4's four, W-MG-REVOKE's three, and W-MG-SEAL's one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counters {
    pub records: usize,
    pub chained: usize,
    pub legacy: usize,
    pub redacted: usize,
    pub broken: usize,
    pub seals_ok: usize,
    pub seals_bad: usize,
    pub forks: usize,
    /// Segments at the DOC's `ref SEG`, and the records they say left the file.
    pub segments: usize,
    pub cut: u64,
    /// Tombstones under the segments, as their TRUNCATEs declared them.
    pub cut_redacted: u64,
    /// What effective FORGOTTENs (live, and under the segments) declare.
    pub declared: u64,
    /// Keys whose table `revoked_at` differs from the fold of the REVOKE records.
    pub keys_disagree: usize,
    /// REVOKEs that contradict each other; held in force, never settled here.
    pub revokes_unsettled: usize,
    /// Live tombstones no FORGOTTEN of their author or of a root lists as they stand, plus listed
    /// ids still in the log that are no tombstone.
    pub tombs_foreign: usize,
    /// Records no signed tip reaches by `prev`, plus chains that end on an unknown id, plus
    /// records of a key above that key's own seals.
    pub unsealed: usize,
    /// 1 when the DOC names a TREE or sets `FLAG_TREE_IS_FOLD`: no signature covers either.
    pub tree_unsigned: usize,
}

/// A sum of cells of the file, so of any u64: saturating at `i64::MAX`, the largest number every
/// reader can print, and the same answer on 32 and 64 bits.
pub(super) fn sum_of(ns: &[u64]) -> u64 {
    ns.iter().fold(0u64, |a, n| a.saturating_add(*n)).min(i64::MAX as u64)
}

/// What `verify` found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub counters: Counters,
    pub heads: Vec<Head>,
    /// Keys in the table, and how many of them admission reached.
    pub keys: usize,
    pub admitted: usize,
}

impl Report {
    /// Every record chains or is a declared redaction, every seal holds, no fork, every
    /// tombstone -- live or cut -- is counted by a declaration (law 9), every live one listed by
    /// someone who may, the table and the REVOKE records tell one story, a signed tip vouches
    /// for every record, and no tree stands outside the signatures. About the log alone.
    pub fn holds(&self) -> bool {
        let c = &self.counters;
        c.broken == 0 && c.seals_bad == 0 && c.forks == 0 && c.seals_ok > 0 && sum_of(&[c.redacted as u64, c.cut_redacted]) == c.declared && c.keys_disagree == 0 && c.tombs_foreign == 0 && c.unsealed == 0 && c.tree_unsigned == 0
    }
}

/// Whose keys the verifier believes without a record saying so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trust<'a> {
    /// The file's own genesis keys (`admit_rec` = none): "signed by the keys this file
    /// names as its roots" -- all a self-contained read can say.
    Genesis,
    /// Exactly these actor ids, from outside the file.
    Only(&'a [[u8; 32]]),
}

type Actor = [u8; 32];

pub(super) struct Chains {
    /// Oldest first: the record and its object offset.
    pub recs: Vec<(usize, Record)>,
    /// Per device, the indices into `recs`, in file order.
    pub by_actor: BTreeMap<Actor, Vec<usize>>,
    /// Per record object (`budget::at_key`), its index in `recs`.
    pub by_obj: BTreeMap<Actor, u64>,
    /// Every id the log holds.
    pub ids: BTreeSet<Actor>,
    /// Every id seen at `(actor, seq)` (more than one = a fork).
    pub ids_at: BTreeMap<(Actor, u64), Vec<[u8; 32]>>,
}

fn chains(st: &Store, log: Option<usize>) -> Chains {
    let objs = EvLog::objs_at(st, log);
    let newest_first = EvLog::walk_until_at(st, log, |_| false);
    let mut recs: Vec<(usize, Record)> = objs.into_iter().zip(newest_first).collect();
    recs.reverse();
    let mut by_actor: BTreeMap<Actor, Vec<usize>> = BTreeMap::new();
    let mut ids_at: BTreeMap<(Actor, u64), Vec<[u8; 32]>> = BTreeMap::new();
    let mut by_obj: BTreeMap<Actor, u64> = BTreeMap::new();
    let ids = set_of(recs.iter().map(|(_, r)| r.id));
    for (i, (o, r)) in recs.iter().enumerate() {
        by_obj.insert(at_key(*o), i as u64);
        by_actor.entry(r.actor_pubkey).or_default().push(i);
        ids_at.entry((r.actor_pubkey, r.actor_seq)).or_default().push(r.id);
    }
    Chains { recs, by_actor, by_obj, ids, ids_at }
}

/// `unsealed`: walk `prev` from `tips`, over ids and not over positions, so the file's order
/// plays no part. Each record no walk visits counts one; so does each walk that leaves the file
/// anywhere but at 0^32 or a frontier tip, because a signature over a tip commits to the whole
/// chain below it, and a record dropped from under it must not read as a shorter history. So does
/// a visited record for which `unowned` is true: a device's first record chains onto the log, so
/// another device's tip reaches records their own author's seals may not cover, and such a record
/// stays in the file while declaring nothing (a REVOKE disowned by rolling back a HEADS row,
/// `atk_holes` hole 4).
/// THE TWO COLLECTIONS ARE `covered`'s AND `admitted`'s TYPES, ON PURPOSE: a sort or a new map
/// type here cost the reader's wasm 6 KB (measured); these two are already in the binary.
fn unsealed(ch: &Chains, tips: &[[u8; 32]], fr: &Frontiers, unowned: &dyn Fn(&Record) -> bool) -> usize {
    let mut at: BTreeMap<Actor, u64> = BTreeMap::new();
    let mut i = 0u64;
    for (_, r) in &ch.recs {
        *at.entry(r.id).or_insert(0) = i;
        i += 1;
    }
    let mut seen: BTreeSet<Actor> = BTreeSet::new();
    let mut dangling = 0usize;
    for tip in tips {
        let mut id = *tip;
        loop {
            match at.get(&id).and_then(|n| ch.recs.get(*n as usize)) {
                Some((_, r)) => {
                    if !seen.insert(id) {
                        break;
                    }
                    id = r.prev;
                }
                None => {
                    if id != [0u8; 32] && !fr.tips.contains(&id) {
                        dangling += 1;
                    }
                    break;
                }
            }
        }
    }
    ch.recs.len().saturating_sub(seen.len()) + dangling + ch.recs.iter().filter(|(_, r)| seen.contains(&r.id) && unowned(r)).count()
}

/// The counters of the DOC at the image's root. `Err` = the image could not be counted (not
/// a DOC, a crc failure on the root, a table or the log chain, a chain that never ends).
pub fn verify(st: &Store, ids: &dyn Ids, v: &dyn Verifier, trust: Trust<'_>) -> Result<Report, DocError> {
    let view = Doc::open(st)?;
    if EvLog::chain_crc_at(st, view.log)?.is_none() {
        return Err(DocError::Corrupt("the log chain never ends"));
    }
    // One budget for every walk (`budget.rs`): an honest file's objects are disjoint and cannot
    // run it out. Everything from here is paid for before it is copied.
    let mut budget = Budget::of(st);
    for o in EvLog::objs_at(st, view.log) {
        budget.spend(st, o)?;
    }
    for t in [view.heads, view.keys].into_iter().flatten() {
        budget.spend(st, t)?;
        st.check_obj(t)?;
    }
    let segs = read_segs_in(st, view.seg, EvLog::step_cap(st), &mut budget)?;
    let fr = Frontiers::of(&segs);
    let heads = read_heads(st, view.heads);
    let keys = read_keys_in(st, view.keys, &mut budget)?;
    let seals = load(st, &heads, &fr, &mut budget)?;
    let ch = chains(st, view.log);
    let keyed = set_of(keys.iter().map(|k| k.actor_id));
    let mut c = Counters::default();
    let counted = count_records(&ch, &heads, &fr, &keyed, EvLog::tip_at(st, view.log), ids, &mut c);
    let decls = declarations(&ch.recs, &counted.sound, ids);
    // The first row of an actor answers for it, as the table's order always did.
    let mut key_ix: BTreeMap<Actor, u64> = BTreeMap::new();
    let mut i = 0u64;
    for k in &keys {
        key_ix.entry(k.actor_id).or_insert(i);
        i += 1;
    }
    let bound: Vec<bool> = keys.iter().map(|k| v.actor_of(&k.pk).map_or(true, |id| id == k.actor_id)).collect();

    let trusted: BTreeSet<Actor> = match trust {
        Trust::Genesis => set_of(keys.iter().filter(|k| k.admit_rec.is_none()).map(|k| k.actor_id)),
        Trust::Only(list) => set_of(list.iter().copied()),
    };
    let cx = Cx { ch: &ch, seals: &seals, decls: &decls, sound: &counted.sound, off_seq: &counted.off_seq, key_ix: &key_ix, bound: &bound, ids, v };
    let (end, unsettled) = settle(&cx, &keys, &trusted);
    c.seals_ok = end.ok;
    c.seals_bad = end.bad;
    c.revokes_unsettled = unsettled;
    c.keys_disagree = keys.iter().zip(&end.revoked).filter(|(table, fold)| table.revoked_at != **fold).count();
    // A key's records must sit at or below its own highest ok seal; a reader that judges no seal
    // cannot say, and a pre-seal record's actor has no KEY row (`seal_hub`).
    let unowned = |r: &Record| v.checks_signatures() && keyed.contains(&r.actor_pubkey) && end.covered.get(&r.actor_pubkey).map_or(true, |&s| r.actor_seq > s);
    c.unsealed = unsealed(&ch, &end.tips, &fr, &unowned);
    c.tree_unsigned = usize::from(view.tree.is_some() || view.flags & FLAG_TREE_IS_FOLD != 0);
    let decl = end.declared;
    c.tombs_foreign = foreign_tombs(&ch.recs, &counted.tombs, &erased(&ch.recs, &decls, &decl), &trusted, ids);
    check_segments(&segs, &decl.truncates, ids)?;
    c.segments = segs.len();
    let sum = |f: &dyn Fn(&Segment) -> u64| sum_of(&segs.iter().map(f).collect::<Vec<u64>>());
    c.cut = sum(&|s: &Segment| s.count());
    c.cut_redacted = sum(&|s: &Segment| s.redacted);
    c.declared = sum_of(&[sum(&|s: &Segment| s.declared), decl.forgotten]);
    Ok(Report { counters: c, heads, keys: keys.len(), admitted: end.admitted.len() })
}
