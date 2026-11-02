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
//! THREE WAYS TO HIDE A REVOKE THAT ARE PINNED, under `Trust::Only`, by a named test: a thief's
//! tombstone over it needs its author's or a root's FORGOTTEN listing it (`foreign_tombs`,
//! `a_thief_does_not_erase_the_revoke_of_its_own_key`); a thief's cut through it declares nothing,
//! because only a root's TRUNCATE is effective (`declare.rs`; the older sentence here, "a cut takes
//! the thief's own ADMIT", was false: that ADMIT can sit on another device's chain, and the file's
//! order is signed by nobody, `tam_tests::h1`); a forger with no key and the file: `atk_holes` holes
//! 2-6 name the doors tried. Nothing here says those are all the doors.
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
use super::budget::{at_key, set_of};
use super::declare::{effective, Body, Declared};
use super::frontier::Frontiers;
use super::keys::{admit_view, narrower, valid_at, Key};
use super::seal::message;
use super::seals::Seals;
use super::verify::Chains;
use super::{Ids, Verifier};

type Actor = [u8; 32];

/// What every round reads. Each of these is computed once, in `verify`: a round is a question
/// about who is admitted, and nothing about a seal or a record changes between rounds.
pub(super) struct Cx<'a> {
    pub ch: &'a Chains,
    pub fr: &'a Frontiers,
    pub seals: &'a Seals,
    pub decls: &'a [(usize, Body)],
    /// `count.rs`: the records whose id, link and seq stand.
    pub sound: &'a [bool],
    pub off_seq: &'a BTreeSet<Actor>,
    /// Per actor, its KEY's index in the table.
    pub key_ix: &'a BTreeMap<Actor, u64>,
    /// Per key, `actor_of(pk)` names it.
    pub bound: &'a [bool],
    pub ids: &'a dyn Ids,
    pub v: &'a dyn Verifier,
}

/// Per seal, in `Seals::all`'s order, the signature's answer once asked: the rounds of a hostile
/// file do not multiply the ML-DSA checks.
type Sigs = Vec<Option<bool>>;

/// One pass over the seals' ok, their bad, per actor the highest seq an ok seal signed, and the
/// tips `verify`'s `unsealed` walks from.
type Passes = (usize, usize, BTreeMap<Actor, u64>, Vec<[u8; 32]>);

/// One round: the key state with some REVOKEs in force, and what it makes effective.
pub(super) struct Round {
    /// Per key, `revoked_at` as the fold has it; nothing of the table's own survives.
    pub revoked: Vec<i64>,
    pub admitted: BTreeSet<Actor>,
    pub ok: usize,
    pub bad: usize,
    /// The tip inside every ok seal; a reader that judges no seal, inside every seal it read.
    /// Never a HEADS row's `tip`: no signature covers that cell.
    pub tips: Vec<[u8; 32]>,
    /// Per actor, the highest seq an ok seal signed.
    pub covered: BTreeMap<Actor, u64>,
    pub declared: Declared,
}

/// Every seal, judged under `revoked`. `s.seq` is the seal's own, and `names_record` ties it to a
/// record whose id holds that seq; `count.rs` is what makes that seq a place on the device's chain.
fn seals(cx: &Cx<'_>, keys: &[Key], revoked: &[i64], admitted: &BTreeSet<Actor>, sigs: &mut Sigs) -> Passes {
    let (mut ok, mut bad) = (0usize, cx.seals.bad);
    let mut covered: BTreeMap<Actor, u64> = BTreeMap::new();
    let mut tips: Vec<[u8; 32]> = Vec::new();
    let blind = !cx.v.checks_signatures();
    for (it, answer) in cx.seals.all.iter().zip(sigs.iter_mut()) {
        let s = &it.seal;
        let at = cx.key_ix.get(&s.actor_id).map(|i| *i as usize);
        let key = at.and_then(|i| keys.get(i));
        let names_record = cx.ch.ids_at.get(&(s.actor_id, s.seq)).is_some_and(|ids| ids.contains(&s.tip)) && !cx.off_seq.contains(&s.tip);
        let good = it.agrees
            && names_record
            && at.is_some_and(|i| cx.bound.get(i).copied().unwrap_or(false))
            && admitted.contains(&s.actor_id)
            && at.is_some_and(|i| revoked.get(i).is_some_and(|r| valid_at(*r, s.seq)))
            && key.is_some_and(|k| k.alg == s.alg && *answer.get_or_insert_with(|| cx.v.verify(s.alg, &k.pk, &message(&s.actor_id, s.seq, &s.tip, s.claimed_ms, &it.prev_tip), &s.sig)));
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
    }
    (ok, bad, covered, tips)
}

/// Keys the records admit, given who is already admitted and what their ok seals cover. The ADMIT
/// must be `sound`: a number a forger picked, on a record the counters call broken, admits nobody.
fn admit_more(cx: &Cx<'_>, keys: &[Key], revoked: &[i64], admitted: &mut BTreeSet<Actor>, covered: &BTreeMap<Actor, u64>) -> bool {
    let mut grew = false;
    for k in keys {
        if admitted.contains(&k.actor_id) {
            continue;
        }
        let Some(at) = k.admit_rec else { continue };
        let Some(i) = cx.ch.by_obj.get(&at_key(at)).map(|i| *i as usize) else { continue };
        let Some((_, r)) = cx.ch.recs.get(i) else { continue };
        if !cx.sound.get(i).copied().unwrap_or(false) || cx.ids.redacted(r.flags, &r.payload) {
            continue;
        }
        let Some((actor, alg, valid_from, pk)) = admit_view(cx.ids.body(r.flags, &r.payload)) else { continue };
        let same = actor == k.actor_id && alg == k.alg && valid_from == k.valid_from && pk == k.pk.as_slice();
        let by = r.actor_pubkey;
        let by_valid = cx.key_ix.get(&by).and_then(|i| revoked.get(*i as usize)).is_some_and(|c| valid_at(*c, r.actor_seq));
        let authorised = admitted.contains(&by) && covered.get(&by).is_some_and(|&s| s >= r.actor_seq) && by_valid;
        if same && authorised {
            admitted.insert(k.actor_id);
            grew = true;
        }
    }
    grew
}

/// Per key, `revoked_at` as these REVOKEs say; nothing of the table's own survives.
fn narrowed(keys: &[Key], in_force: &[(Actor, i64)]) -> Vec<i64> {
    let mut cell: BTreeMap<Actor, u64> = BTreeMap::new();
    for (a, after) in in_force {
        let e = cell.entry(*a).or_insert(0);
        *e = narrower(*e as i64, *after) as u64;
    }
    keys.iter().map(|k| cell.get(&k.actor_id).map_or(0, |c| *c as i64)).collect()
}

/// The admission fixpoint under `in_force`, and what it makes effective.
fn round(cx: &Cx<'_>, table: &[Key], trusted: &BTreeSet<Actor>, in_force: &[(Actor, i64)], sigs: &mut Sigs) -> Round {
    let revoked = narrowed(table, in_force);
    let mut admitted = set_of(trusted.iter().copied());
    let (mut ok, mut bad, mut covered, mut tips) = seals(cx, table, &revoked, &admitted, sigs);
    for _ in 0..=table.len() {
        if !admit_more(cx, table, &revoked, &mut admitted, &covered) {
            break;
        }
        (ok, bad, covered, tips) = seals(cx, table, &revoked, &admitted, sigs);
    }
    let declared = effective(cx.ch, cx.fr, cx.key_ix, cx.decls, &admitted, trusted, &covered, cx.v.checks_signatures());
    Round { revoked, admitted, ok, bad, tips, covered, declared }
}

/// The alternating fixpoint: the final round, and how many REVOKEs stayed unsettled. It ends
/// because `sure` only grows (round-of-round is monotone), and no further than the log's
/// REVOKEs: a pass that adds nothing returns.
pub(super) fn settle(cx: &Cx<'_>, table: &[Key], trusted: &BTreeSet<Actor>) -> (Round, usize) {
    let mut sigs: Sigs = vec![None; cx.seals.all.len()];
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
