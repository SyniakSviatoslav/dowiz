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
//!   * forks     `(actor, seq)` pairs that occur more than once (§2a: a named refusal, not
//!               the death of the identity).
//!   * seals_ok / seals_bad   every seal reachable from HEADS through `ref PREV_SEAL`. A seal
//!               is ok when its actor's key is ADMITTED (below), valid at its seq (an
//!               effective REVOKE after k breaks k+1), the tip names a record at `(actor,
//!               seq)`, and the signature over `seal::message` verifies with the previous
//!               seal's tip. Anything else -- a head row that disagrees with its seal, a crc
//!               failure, an unknown algorithm -- is bad.
//!   * keys_disagree   keys whose table `revoked_at` is not what the REVOKE records fold to,
//!               either way. A COUNTER AND NOT A REFUSAL: the records answer in full without
//!               the table, so a refusal would only let one forged cell hide every other
//!               counter; `holds` is false on it, which is what a refusal would have meant.
//!   * revokes_unsettled   REVOKEs that contradict each other (`settle.rs`), held in force.
//!               Not a term of `holds`: a contradiction always leaves a seal bad, the one that
//!               made the REVOKE effective and that the REVOKEs in force then break.
//!   * unsealed  records no signed tip vouches for, plus chains a signed tip reaches that end
//!               on nothing (`unsealed` below). `prev` is walked from the tip INSIDE every ok
//!               seal, never from a HEADS row: HEADS is signed by nobody, so a row names where
//!               to look and a signature decides what was found. Reachability and not "a seal
//!               at this seq": one seal over a tip vouches for every record under it, across
//!               devices (a device's first record chains onto the log) and across seq gaps
//!               between seals. A redaction keeps its id and `prev`, so it stays on the path;
//!               a chain ends at 0^32 or at a frontier tip a segment recorded. THE LIMIT: a
//!               whole tail gone with its seals and its row is a shorter true file, the
//!               rollback `settle.rs` names; this counter sees only what stays in the file.
//!   * tombs_foreign   live tombstones no FORGOTTEN of their author or of a root covers
//!               (`declare.rs` `foreign_tombs`): a thief must not erase a REVOKE and declare it.
//!
//! ADMISSION AND REVOCATION are one fixpoint over the signed records (`settle.rs`), never the
//! KEYS table: a key is admitted when it is trusted (`Trust`), or when its `ref ADMIT_REC`
//! names a record whose body is an ADMIT of exactly this key, written by an admitted actor at
//! a seq that actor's ok seals cover, while that actor's key was valid. A key is revoked by
//! its effective REVOKE records, and by nothing else: the table's `revoked_at` is an index.
//!
//! SEGMENTS AND DECLARATIONS (W-MG-M3/M4, `seg.rs`, `declare.rs`): `segments` / `cut` /
//! `cut_redacted` count what left the file under a root; every segment must be declared by
//! an effective TRUNCATE or the file is REFUSED by name (`DocError::CutUndeclared`). A
//! device's oldest kept record must continue the frontier the segment recorded for it
//! (seq + 1, `prev` = the frontier tip) or it is broken; a seal with no previous seal reads
//! its `prev_tip` from that same frontier. `declared` sums the effective FORGOTTENs (plus
//! what the segments carry), and `holds` requires `redacted + cut_redacted == declared`.

#[allow(unused_imports)]
use crate::prelude::*;
use alloc::collections::{BTreeMap, BTreeSet};
use crate::{EvLog, Record, Store};
use super::declare::{check_segments, foreign_tombs};
use super::keys::read_keys;
use super::seal::{read_heads, Head};
use super::seg::{frontier_at, frontier_of, read_segs, Segment, SCHEME_CHAINED};
use super::settle::{settle, Cx};
use super::{Doc, DocError, Ids, Verifier};

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
    pub cut: usize,
    /// Tombstones under the segments, as their TRUNCATEs declared them.
    pub cut_redacted: usize,
    /// What effective FORGOTTENs (live, and under the segments) declare.
    pub declared: usize,
    /// Keys whose table `revoked_at` differs from the fold of the REVOKE records.
    pub keys_disagree: usize,
    /// REVOKEs that contradict each other; held in force, never settled here.
    pub revokes_unsettled: usize,
    /// Live tombstones covered by no FORGOTTEN of their author or of a root.
    pub tombs_foreign: usize,
    /// Records no signed tip reaches by `prev`, plus chains that end on an unknown id.
    pub unsealed: usize,
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
    /// tombstone -- live or cut -- is declared (law 9) by someone who may, the table and the
    /// REVOKE records tell one story, and a signed tip vouches for every record.
    pub fn holds(&self) -> bool {
        let c = &self.counters;
        c.broken == 0 && c.seals_bad == 0 && c.forks == 0 && c.seals_ok > 0 && c.redacted + c.cut_redacted == c.declared && c.keys_disagree == 0 && c.tombs_foreign == 0 && c.unsealed == 0
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
    by_actor: BTreeMap<Actor, Vec<usize>>,
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
    for (i, (_, r)) in recs.iter().enumerate() {
        by_actor.entry(r.actor_pubkey).or_default().push(i);
        ids_at.entry((r.actor_pubkey, r.actor_seq)).or_default().push(r.id);
    }
    Chains { recs, by_actor, ids_at }
}

fn count_records(ch: &Chains, heads: &[Head], segs: &[Segment], log_tip: Option<[u8; 32]>, ids: &dyn Ids, c: &mut Counters) -> BTreeMap<Actor, u64> {
    let mut tombs: BTreeMap<Actor, u64> = BTreeMap::new();
    c.records = ch.recs.len();
    c.forks = ch.ids_at.values().filter(|v| v.len() > 1).count();
    for (actor, idx) in &ch.by_actor {
        let head_tip = heads.iter().find(|h| h.actor_id == *actor).map(|h| h.tip).or(log_tip);
        let rec_at = |i: usize| ch.recs.get(i).map(|(_, r)| r);
        // The device's oldest kept record continues the frontier a cut recorded for it.
        let first_ok = |r: &Record| match frontier_at(segs, actor, r.actor_seq.wrapping_sub(1)) {
            Some(f) => f.tip == r.prev,
            None => segs.iter().all(|s| s.frontier.iter().all(|f| f.actor_id != *actor)),
        };
        // M0's rule (dowiz-hub `chain.rs` `walk_check`), per device: the id scheme may move
        // from legacy to chained ONCE, oldest to newest, and never back. A legacy id newer
        // than a chained record of the same device is a record edited after the move and
        // repaired under the scheme the log had left -- the one forgery the old scheme lets
        // through -- and it is broken, whatever its id says. Across a cut the rule continues
        // from the frontier row's `scheme` (W-MG-M3b): the kept part does not start over.
        let mut moved = frontier_of(segs, actor).is_some_and(|f| f.scheme == SCHEME_CHAINED);
        for (j, &i) in idx.iter().enumerate() {
            let Some(r) = rec_at(i) else { continue };
            let prev_ok = if j == 0 { first_ok(r) } else { idx.get(j.wrapping_sub(1)).and_then(|&p| rec_at(p)).is_some_and(|p| p.id == r.prev) };
            let next_names_me = match idx.get(j + 1).and_then(|&n| rec_at(n)) {
                Some(n) => n.prev == r.id,
                None => head_tip == Some(r.id),
            };
            if !prev_ok {
                c.broken += 1;
            } else if r.id == ids.chained(&r.prev, &r.payload) {
                c.chained += 1;
                moved = true;
            } else if ids.legacy(r.flags, &r.prev, &r.payload, &r.id) {
                if moved {
                    c.broken += 1;
                } else {
                    c.legacy += 1;
                }
            } else if ids.redacted(r.flags, &r.payload) && next_names_me {
                c.redacted += 1;
                *tombs.entry(*actor).or_insert(0) += 1;
            } else {
                c.broken += 1;
            }
        }
    }
    tombs
}

/// `unsealed`: walk `prev` from `tips`, over ids and not over positions, so the file's order
/// plays no part. Each record no walk visits counts one; so does each walk that leaves the file
/// anywhere but at 0^32 or a frontier tip, because a signature over a tip commits to the whole
/// chain below it, and a record dropped from under it must not read as a shorter history.
/// THE TWO COLLECTIONS ARE `covered`'s AND `admitted`'s TYPES, ON PURPOSE: a sort or a new map
/// type here cost the reader's wasm 6 KB (measured); these two are already in the binary.
fn unsealed(ch: &Chains, tips: &[[u8; 32]], segs: &[Segment]) -> usize {
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
                    if id != [0u8; 32] && !segs.iter().any(|s| s.frontier.iter().any(|f| f.tip == id)) {
                        dangling += 1;
                    }
                    break;
                }
            }
        }
    }
    ch.recs.len().saturating_sub(seen.len()) + dangling
}

/// The counters of the DOC at the image's root. `Err` = the image could not be counted (not
/// a DOC, a crc failure on the root, a table or the log chain, a chain that never ends).
pub fn verify(st: &Store, ids: &dyn Ids, v: &dyn Verifier, trust: Trust<'_>) -> Result<Report, DocError> {
    let view = Doc::open(st)?;
    if EvLog::chain_crc_at(st, view.log)?.is_none() {
        return Err(DocError::Corrupt("the log chain never ends"));
    }
    for t in [view.heads, view.keys].into_iter().flatten() {
        st.check_obj(t)?;
    }
    let segs = read_segs(st, view.seg, EvLog::step_cap(st))?;
    let ch = chains(st, view.log);
    let heads = read_heads(st, view.heads);
    let keys = read_keys(st, view.keys);
    let mut c = Counters::default();
    let tombs = count_records(&ch, &heads, &segs, EvLog::tip_at(st, view.log), ids, &mut c);

    let trusted: BTreeSet<Actor> = match trust {
        Trust::Genesis => keys.iter().filter(|k| k.admit_rec.is_none()).map(|k| k.actor_id).collect(),
        Trust::Only(list) => list.iter().copied().collect(),
    };
    let cx = Cx { st, ch: &ch, heads: &heads, segs: &segs, ids, v };
    let (end, unsettled) = settle(&cx, &keys, &trusted);
    c.seals_ok = end.ok;
    c.seals_bad = end.bad;
    c.revokes_unsettled = unsettled;
    c.keys_disagree = keys.iter().zip(&end.keys).filter(|(table, fold)| table.revoked_at != fold.revoked_at).count();
    c.unsealed = unsealed(&ch, &end.tips, &segs);
    let decl = end.declared;
    c.tombs_foreign = foreign_tombs(&tombs, &decl.forgotten_by, &trusted) as usize;
    check_segments(&segs, &decl.truncates)?;
    c.segments = segs.len();
    c.cut = segs.iter().map(|s| s.count() as usize).sum();
    c.cut_redacted = segs.iter().map(|s| s.redacted as usize).sum();
    c.declared = decl.forgotten as usize + segs.iter().map(|s| s.declared as usize).sum::<usize>();
    Ok(Report { counters: c, heads, keys: keys.len(), admitted: end.admitted.len() })
}
