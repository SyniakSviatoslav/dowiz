//! What the signed records DECLARE (W-MG-M3/M4, W-MG-REVOKE): the TRUNCATEs that vouch for
//! segments, the FORGOTTENs that vouch for redactions, and the REVOKEs that end a key
//! (`settle.rs` folds those; a redaction erases what its record declared). A declaration counts only when it is
//! EFFECTIVE -- its record chains (the id verifies), its actor is admitted, and an ok seal
//! of that actor covers its seq -- so a declaration a revoked key wrote after its revoke,
//! or one nobody sealed, declares nothing. That is M4's second kill test: a FORGOTTEN by a
//! revoked key leaves `redacted != declared`.

#[allow(unused_imports)]
use crate::prelude::*;
use alloc::collections::{BTreeMap, BTreeSet};
use crate::Record;
use super::keys::parse_revoke;
use super::seg::{parse_forgotten, parse_truncate, Segment, Truncate};
use super::{DocError, Ids};

type Actor = [u8; 32];

/// The effective declarations of a log.
#[derive(Debug, Default)]
pub(crate) struct Declared {
    /// Σ `n_redacted` over effective FORGOTTEN records.
    pub forgotten: u64,
    /// The same sum per signer (`foreign_tombs`).
    pub forgotten_by: BTreeMap<Actor, u64>,
    pub truncates: Vec<Truncate>,
    /// `(target, after_seq)` of the effective REVOKEs, in file order.
    pub revokes: Vec<(Actor, i64)>,
}

/// `recs` oldest first; `admitted` and `covered` are `verify`'s fixpoint (per actor, the
/// highest seq an ok seal signed).
///
/// `signatures` false (a verifier that knows no algorithm, `Verifier::checks_signatures`): no seal
/// can be judged, so a declaration is taken on its chain alone. The structure still binds: a
/// dropped segment stays refused, and `Report::holds` stays false because no seal is ok.
pub(crate) fn effective(recs: &[(usize, Record)], ids: &dyn Ids, admitted: &BTreeSet<Actor>, covered: &BTreeMap<Actor, u64>, signatures: bool) -> Declared {
    let mut out = Declared::default();
    for (_, r) in recs {
        let sealed = !signatures || admitted.contains(&r.actor_pubkey) && covered.get(&r.actor_pubkey).is_some_and(|&s| s >= r.actor_seq);
        if !sealed || ids.redacted(r.flags, &r.payload) || ids.chained(&r.prev, &r.payload) != r.id {
            continue;
        }
        let body = ids.body(r.flags, &r.payload);
        if let Some(t) = parse_truncate(body) {
            out.truncates.push(t);
        } else if let Some((n, _)) = parse_forgotten(body) {
            out.forgotten = out.forgotten.saturating_add(n);
            let by = out.forgotten_by.entry(r.actor_pubkey).or_insert(0);
            *by = by.saturating_add(n);
        } else if let Some(r) = parse_revoke(body) {
            out.revokes.push(r);
        }
    }
    out
}

/// WHO MAY ERASE WHOM (W-MG-REVOKE): law 9 alone is one sum, so any key's FORGOTTEN balanced any
/// record's tombstone, and a tombstone cannot say what it was: a thief could erase the REVOKE
/// of its own key and sign the declaration. Each live tombstone must be covered by a FORGOTTEN
/// of the record's AUTHOR or of a ROOT (`trusted`); this counts those that are not. Only the
/// live tombstones are owed: `Segment::redacted` carries no author, and a segment's root and
/// counts are already the TRUNCATE's claim. A root's sum is shared and may also have paid for
/// tombstones under a cut, so this can only under-count, on a root's signature. Note what a
/// root is: under `Trust::Genesis` the unsigned table names them, so this binds only a verifier
/// that brings its roots (`Trust::Only`).
pub(crate) fn foreign_tombs(tombs: &BTreeMap<Actor, u64>, by: &BTreeMap<Actor, u64>, roots: &BTreeSet<Actor>) -> u64 {
    let own = |x: &Actor| if roots.contains(x) { 0 } else { by.get(x).copied().unwrap_or(0) };
    let owed = tombs.iter().fold(0u64, |sum, (x, t)| sum.saturating_add(t.saturating_sub(own(x))));
    let from_roots = by.iter().filter(|(y, _)| roots.contains(*y)).fold(0u64, |sum, (_, n)| sum.saturating_add(*n));
    owed.saturating_sub(from_roots)
}

/// Every segment (newest first) must be declared: named exactly by an effective TRUNCATE,
/// or -- for an older segment whose own TRUNCATE a later cut took -- named by root as the
/// `prev_root` of the declared segment above it. The transitive case binds through the SEG
/// object's `prev_root`, which no signature covers: a forger who rewrites an OLDER segment
/// in the file is caught by the archive audit that recomputes each root (`cut/tests.rs`),
/// not here. The newest segment is always directly signed, so no record leaves a file
/// without a key.
pub(crate) fn check_segments(segs: &[Segment], truncates: &[Truncate]) -> Result<(), DocError> {
    let mut vouched: Option<[u8; 32]> = None;
    for s in segs {
        let direct = truncates.iter().any(|t| s.is_named_by(t));
        if !direct && vouched != Some(s.root) {
            return Err(DocError::CutUndeclared { root: s.root });
        }
        vouched = Some(s.prev_root);
    }
    Ok(())
}
