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
//!               is ok when its actor's key is ADMITTED (below), valid at its seq (REVOKE
//!               after k breaks k+1), the tip names a record at `(actor, seq)`, and the
//!               signature over `seal::message` verifies with the previous seal's tip.
//!               Anything else -- a head row that disagrees with its seal, a crc failure,
//!               an unknown algorithm -- is bad.
//!
//! ADMISSION is a fixpoint over the signed records, never the KEYS table alone: a key is
//! admitted when it is trusted (`Trust`), or when its `ref ADMIT_REC` names a record whose
//! body is an ADMIT of exactly this key, written by an admitted actor at a seq that actor's
//! ok seals cover, while that actor's key was valid. The table's `revoked_at` is believed
//! as a claim that only NARROWS (a forger can make a key less valid, never more); proving
//! the REVOKE record is M2's merge event's business.
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
use super::declare::{check_segments, effective};
use super::keys::{parse_admit, read_keys, Key};
use super::seal::{message, read_heads, read_seal, Head};
use super::seg::{frontier_at, frontier_of, read_segs, Segment, SCHEME_CHAINED};
use super::{Doc, DocError, Ids, Verifier};

/// The counters: M1's eight, and M3/M4's four.
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
    /// Every record chains or is a declared redaction, every seal holds, no fork, and every
    /// tombstone -- live or cut -- is declared (law 9).
    pub fn holds(&self) -> bool {
        let c = &self.counters;
        c.broken == 0 && c.seals_bad == 0 && c.forks == 0 && c.seals_ok > 0 && c.redacted + c.cut_redacted == c.declared
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

struct Chains {
    /// Oldest first: the record and its object offset.
    recs: Vec<(usize, Record)>,
    /// Per device, the indices into `recs`, in file order.
    by_actor: BTreeMap<Actor, Vec<usize>>,
    /// Every id seen at `(actor, seq)` (more than one = a fork).
    ids_at: BTreeMap<(Actor, u64), Vec<[u8; 32]>>,
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

fn count_records(ch: &Chains, heads: &[Head], segs: &[Segment], log_tip: Option<[u8; 32]>, ids: &dyn Ids, c: &mut Counters) {
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
            } else {
                c.broken += 1;
            }
        }
    }
}

/// One pass over every seal reachable from HEADS. Returns `(ok, bad, covered)`: `covered`
/// is, per actor, the highest seq an ok seal signed.
fn seals(st: &Store, ch: &Chains, heads: &[Head], segs: &[Segment], keys: &[Key], admitted: &BTreeSet<Actor>, v: &dyn Verifier) -> (usize, usize, BTreeMap<Actor, u64>) {
    let (mut ok, mut bad) = (0usize, 0usize);
    let mut covered: BTreeMap<Actor, u64> = BTreeMap::new();
    let cap = ch.recs.len() + 1;
    for h in heads {
        let mut cur = h.seal;
        let mut first = true;
        let mut steps = 0usize;
        while let Some(at) = cur {
            steps += 1;
            if steps > cap {
                bad += 1;
                break;
            }
            let Some(s) = st.check_obj(at).ok().and_then(|_| read_seal(st, at)) else {
                bad += 1;
                break;
            };
            let head_agrees = !first || (s.actor_id == h.actor_id && s.seq == h.seq && s.tip == h.tip);
            first = false;
            let key = keys.iter().find(|k| k.actor_id == s.actor_id);
            // The previous seal's tip; a seal with no previous seal (the device's first, or
            // its oldest kept after a cut) reads it from the segment's frontier, else 0^32.
            let prev_tip = s
                .prev
                .and_then(|p| read_seal(st, p))
                .map(|p| p.tip)
                .or_else(|| frontier_at(segs, &s.actor_id, s.seq.wrapping_sub(1)).map(|f| f.tip))
                .unwrap_or([0u8; 32]);
            let names_record = ch.ids_at.get(&(s.actor_id, s.seq)).is_some_and(|ids| ids.contains(&s.tip));
            let good = head_agrees
                && names_record
                && admitted.contains(&s.actor_id)
                && key.is_some_and(|k| k.valid_at(s.seq) && k.alg == s.alg && v.verify(s.alg, &k.pk, &message(&s.actor_id, s.seq, &s.tip, s.claimed_ms, &prev_tip), &s.sig));
            if good {
                ok += 1;
                let e = covered.entry(s.actor_id).or_insert(0);
                *e = (*e).max(s.seq);
            } else {
                bad += 1;
            }
            cur = s.prev;
        }
    }
    (ok, bad, covered)
}

/// Keys the records admit, given who is already admitted and what their ok seals cover.
fn admit_more(ch: &Chains, keys: &[Key], ids: &dyn Ids, admitted: &mut BTreeSet<Actor>, covered: &BTreeMap<Actor, u64>) -> bool {
    let mut grew = false;
    for k in keys {
        if admitted.contains(&k.actor_id) {
            continue;
        }
        let Some(at) = k.admit_rec else { continue };
        let Some((_, r)) = ch.recs.iter().find(|(o, _)| *o == at) else { continue };
        let Some((actor, alg, valid_from, pk)) = parse_admit(ids.body(r.flags, &r.payload)) else { continue };
        let same = actor == k.actor_id && alg == k.alg && valid_from == k.valid_from && pk == k.pk;
        let by = r.actor_pubkey;
        let by_key = keys.iter().find(|x| x.actor_id == by);
        let authorised = admitted.contains(&by)
            && covered.get(&by).is_some_and(|&s| s >= r.actor_seq)
            && by_key.is_some_and(|x| x.valid_at(r.actor_seq));
        if same && authorised {
            admitted.insert(k.actor_id);
            grew = true;
        }
    }
    grew
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
    count_records(&ch, &heads, &segs, EvLog::tip_at(st, view.log), ids, &mut c);

    let mut admitted: BTreeSet<Actor> = match trust {
        Trust::Genesis => keys.iter().filter(|k| k.admit_rec.is_none()).map(|k| k.actor_id).collect(),
        Trust::Only(list) => list.iter().copied().collect(),
    };
    let (mut ok, mut bad, mut covered) = seals(st, &ch, &heads, &segs, &keys, &admitted, v);
    for _ in 0..=keys.len() {
        if !admit_more(&ch, &keys, ids, &mut admitted, &covered) {
            break;
        }
        (ok, bad, covered) = seals(st, &ch, &heads, &segs, &keys, &admitted, v);
    }
    c.seals_ok = ok;
    c.seals_bad = bad;
    let decl = effective(&ch.recs, ids, &admitted, &covered, v.checks_signatures());
    check_segments(&segs, &decl.truncates)?;
    c.segments = segs.len();
    c.cut = segs.iter().map(|s| s.count() as usize).sum();
    c.cut_redacted = segs.iter().map(|s| s.redacted as usize).sum();
    c.declared = decl.forgotten as usize + segs.iter().map(|s| s.declared as usize).sum::<usize>();
    Ok(Report { counters: c, heads, keys: keys.len(), admitted: admitted.len() })
}
