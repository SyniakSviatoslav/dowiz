//! What the signed records DECLARE (W-MG-M3/M4): the TRUNCATEs that vouch for segments and
//! the FORGOTTENs that vouch for redactions. A declaration counts only when it is
//! EFFECTIVE -- its record chains (the id verifies), its actor is admitted, and an ok seal
//! of that actor covers its seq -- so a declaration a revoked key wrote after its revoke,
//! or one nobody sealed, declares nothing. That is M4's second kill test: a FORGOTTEN by a
//! revoked key leaves `redacted != declared`.

#[allow(unused_imports)]
use crate::prelude::*;
use alloc::collections::{BTreeMap, BTreeSet};
use crate::Record;
use super::seg::{parse_forgotten, parse_truncate, Segment, Truncate};
use super::{DocError, Ids};

type Actor = [u8; 32];

/// The effective declarations of a log.
#[derive(Debug, Default)]
pub(crate) struct Declared {
    /// Σ `n_redacted` over effective FORGOTTEN records.
    pub forgotten: u64,
    pub truncates: Vec<Truncate>,
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
        }
    }
    out
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
