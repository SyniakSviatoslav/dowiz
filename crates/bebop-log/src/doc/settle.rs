//! THE KEY STATE (W-MG-REVOKE): who is admitted and who is revoked, folded from the signed
//! records alone. KEY rows are signed by nobody, so a holder of a stolen key and the file could
//! zero `revoked_at` and keep sealing; here the table decides nothing, and one that differs
//! from the fold, either way, is counted (`keys_disagree`, `verify.rs` says why a count).
//!
//! A REVOKE is EFFECTIVE by `declare.rs`'s rule: its record chains and is no tombstone, its
//! actor is admitted, and an ok seal of that actor covers its seq (so the key was valid
//! there). The effective `revoked_at` of a key is the earliest `after_seq` of the REVOKEs
//! naming it; a REVOKE naming no key in KEYS changes nothing.
//!
//! WHO MAY REVOKE WHOM is the writer's rule, unchanged: ANY key valid at its seq may revoke any
//! key, itself and the owner included. There are no roles; a stolen key can do this too.
//!
//! THE ORDER. A REVOKE reaches back: `after_seq` may sit below the seqs its target already
//! sealed, and those seals may have carried an ADMIT or another REVOKE, which then fall with
//! them. So no walk of the log gives the answer, and the file's order across devices is signed
//! by nobody anyway: the answer may depend only on each device's own seqs.
//! `round(in_force)` runs the admission fixpoint with the REVOKEs `in_force` applied and
//! returns the REVOKEs that are effective under it. More in force means fewer seals, fewer
//! admitted, fewer effective: the round is antitone, so round-of-round is monotone and has a
//! least fixpoint from nothing. That is the alternating fixpoint: `sure` holds under every
//! reading, `maybe` = round(`sure`) holds under some.
//!
//! When the two meet, the fold is settled and the order of the file played no part. When they
//! do not, the records contradict themselves: two keys that revoked each other back past
//! their REVOKEs, or a key that declared itself dead before its own seal. That is not solved
//! here (the Keyhive design calls it unresolved, R-MAINGOAL-A §2e): every REVOKE of `maybe`
//! stays IN FORCE, the narrowest reading, `revokes_unsettled` counts those not in `sure`, and
//! a seal is bad (the one the contradiction turns on), so `holds` is false and a person decides.
//!
//! WHAT THE FOLD MEANS, BY TRUST. Under `Trust::Genesis` the roots are whatever the unsigned
//! table says, so a forger writes his own key in as a root (or zeroes a `ref ADMIT_REC`) and
//! nothing here binds him: it is all a self-contained read can say. Everything below assumes a
//! verifier that brings its roots (`Trust::Only`), which a thief cannot edit.
//!
//! A REVOKE IS NOT HIDDEN INSIDE THE FILE by a thief holding an admitted key: its tombstone
//! needs its author's or a root's FORGOTTEN (`declare.rs` `foreign_tombs`), and a cut through
//! a REVOKE of an admitted key takes that key's ADMIT with it, an older record, so the
//! key is admitted by nothing and its TRUNCATE declares nothing (`CutUndeclared`).
//!
//! WHAT THE FILE CANNOT DECIDE (pinned in `settle/tests.rs`). ROLLBACK: the file as it stood
//! before the REVOKE, or a revoker's newest records dropped with their HEADS row: one file
//! cannot know a newer one exists (`unsealed` in `verify.rs` catches a row dropped alone, never
//! the records going with it), and that is what level 1 anchors are for (an anchor over a
//! dropped record names nothing, `anch.rs`), with M2's comparison of two files. A STOLEN ROOT:
//! roots have no rank between them and nothing outranks one, so a thief of a revoked root
//! tombstones or cuts the REVOKE and signs it; a witness or the audit of the archive, which
//! holds the record under the root the TRUNCATE names, is what sees that.
//!
//! A reader with no signature algorithm judges no seal: a REVOKE counts on its chain alone
//! (`declare.rs`), `keys_disagree` then compares the table with what the records CLAIM, and
//! `holds` stays false because no seal is ok.

#[allow(unused_imports)]
use crate::prelude::*;
use alloc::collections::{BTreeMap, BTreeSet};
use crate::Store;
use super::declare::{effective, Declared};
use super::keys::{narrower, parse_admit, Key};
use super::seal::{message, read_seal, Head};
use super::seg::{frontier_at, Segment};
use super::verify::Chains;
use super::{Ids, Verifier};

type Actor = [u8; 32];

/// What every round reads.
pub(super) struct Cx<'a> {
    pub st: &'a Store,
    pub ch: &'a Chains,
    pub heads: &'a [Head],
    pub segs: &'a [Segment],
    pub ids: &'a dyn Ids,
    pub v: &'a dyn Verifier,
}

/// Each seal's signature answer by offset, sorted (`seals`).
type Sigs = Vec<(usize, bool)>;

/// One pass over the seals' ok, their bad, per actor the highest seq an ok seal signed, and the
/// tips `verify`'s `unsealed` walks from.
type Passes = (usize, usize, BTreeMap<Actor, u64>, Vec<[u8; 32]>);

/// One round: the key state with some REVOKEs in force, and what it makes effective.
pub(super) struct Round {
    /// The table with `revoked_at` replaced by the fold's.
    pub keys: Vec<Key>,
    pub admitted: BTreeSet<Actor>,
    pub ok: usize,
    pub bad: usize,
    /// The tip inside every ok seal; a reader that judges no seal, inside every seal it read.
    /// Never a HEADS row's `tip`: no signature covers that cell.
    pub tips: Vec<[u8; 32]>,
    pub declared: Declared,
}

/// One pass over every seal reachable from HEADS. `sigs` keeps each signature's answer by the
/// seal's offset, so the rounds of a hostile file do not multiply the ML-DSA checks: a seal's
/// key and message do not change between rounds. A sorted Vec and not a map: the reader's
/// wasm ratchet pays for every map type, and this one holds one entry per seal.
fn seals(cx: &Cx<'_>, keys: &[Key], admitted: &BTreeSet<Actor>, sigs: &mut Sigs) -> Passes {
    let (mut ok, mut bad) = (0usize, 0usize);
    let mut covered: BTreeMap<Actor, u64> = BTreeMap::new();
    let mut tips: Vec<[u8; 32]> = Vec::new();
    let blind = !cx.v.checks_signatures();
    let cap = cx.ch.recs.len() + 1;
    for h in cx.heads {
        let mut cur = h.seal;
        let mut first = true;
        let mut steps = 0usize;
        while let Some(at) = cur {
            steps += 1;
            if steps > cap {
                bad += 1;
                break;
            }
            let Some(s) = cx.st.check_obj(at).ok().and_then(|_| read_seal(cx.st, at)) else {
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
                .and_then(|p| read_seal(cx.st, p))
                .map(|p| p.tip)
                .or_else(|| frontier_at(cx.segs, &s.actor_id, s.seq.wrapping_sub(1)).map(|f| f.tip))
                .unwrap_or([0u8; 32]);
            let names_record = cx.ch.ids_at.get(&(s.actor_id, s.seq)).is_some_and(|ids| ids.contains(&s.tip));
            let good = head_agrees
                && names_record
                && admitted.contains(&s.actor_id)
                && key.is_some_and(|k| k.valid_at(s.seq) && k.alg == s.alg && match sigs.binary_search_by_key(&at, |e| e.0) {
                    Ok(i) => sigs.get(i).is_some_and(|e| e.1),
                    Err(i) => {
                        let yes = cx.v.verify(s.alg, &k.pk, &message(&s.actor_id, s.seq, &s.tip, s.claimed_ms, &prev_tip), &s.sig);
                        sigs.insert(i, (at, yes));
                        yes
                    }
                });
            if good || blind {
                tips.push(s.tip);
            }
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
    (ok, bad, covered, tips)
}

/// Keys the records admit, given who is already admitted and what their ok seals cover.
fn admit_more(cx: &Cx<'_>, keys: &[Key], admitted: &mut BTreeSet<Actor>, covered: &BTreeMap<Actor, u64>) -> bool {
    let mut grew = false;
    for k in keys {
        if admitted.contains(&k.actor_id) {
            continue;
        }
        let Some(at) = k.admit_rec else { continue };
        let Some((_, r)) = cx.ch.recs.iter().find(|(o, _)| *o == at) else { continue };
        let Some((actor, alg, valid_from, pk)) = parse_admit(cx.ids.body(r.flags, &r.payload)) else { continue };
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

/// The table with `revoked_at` as these REVOKEs say; nothing of the table's own survives.
fn narrowed(keys: &[Key], in_force: &[(Actor, i64)]) -> Vec<Key> {
    let cell = |k: &Key| in_force.iter().filter(|(a, _)| *a == k.actor_id).fold(0, |c, (_, after)| narrower(c, *after));
    keys.iter().map(|k| Key { revoked_at: cell(k), ..k.clone() }).collect()
}

/// The admission fixpoint under `in_force`, and what it makes effective.
fn round(cx: &Cx<'_>, table: &[Key], trusted: &BTreeSet<Actor>, in_force: &[(Actor, i64)], sigs: &mut Sigs) -> Round {
    let keys = narrowed(table, in_force);
    let mut admitted: BTreeSet<Actor> = trusted.iter().copied().collect();
    let (mut ok, mut bad, mut covered, mut tips) = seals(cx, &keys, &admitted, sigs);
    for _ in 0..=keys.len() {
        if !admit_more(cx, &keys, &mut admitted, &covered) {
            break;
        }
        (ok, bad, covered, tips) = seals(cx, &keys, &admitted, sigs);
    }
    let declared = effective(&cx.ch.recs, cx.ids, &admitted, &covered, cx.v.checks_signatures());
    Round { keys, admitted, ok, bad, tips, declared }
}

/// The alternating fixpoint: the final round, and how many REVOKEs stayed unsettled. It ends
/// because `sure` only grows (round-of-round is monotone), and no further than the log's
/// REVOKEs: a pass that adds nothing returns.
pub(super) fn settle(cx: &Cx<'_>, table: &[Key], trusted: &BTreeSet<Actor>) -> (Round, usize) {
    let mut sigs = Sigs::new();
    let mut sure: Vec<(Actor, i64)> = Vec::new();
    loop {
        let first = round(cx, table, trusted, &sure, &mut sigs);
        if first.declared.revokes == sure {
            return (first, 0);
        }
        let maybe = first.declared.revokes;
        let second = round(cx, table, trusted, &maybe, &mut sigs);
        if second.declared.revokes.len() <= sure.len() {
            return (second, maybe.len() - sure.len());
        }
        sure = second.declared.revokes;
    }
}

#[cfg(test)]
mod tests;
