//! `verify_doc` (W-MG-M1): the library's `verify` of a DOC image FROM BYTES -- the same
//! last-commit rule `Db::verify` applies (FORMAT §7: bytes handed whole are never fallen
//! back past), then `einfile_log::doc::verify`'s counters. `Db` itself still refuses a DOC
//! root by name (`Refusal::UnknownVersion(5)`, `docverify/tests.rs`): teaching the KV
//! readers DOC is M5's, and this entry is how a caller verifies one until then.

use einfile_image::Store;
use einfile_log::doc::{self, DocError, Ids, Trust, Verifier};

use crate::refusal::Refusal;
use crate::sim::{recover, Via};

/// What `verify_doc` proved: the generation, how its last commit was proved, the counters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocReport {
    pub generation: i64,
    pub via: Via,
    pub doc: doc::Report,
}

fn refusal(e: DocError) -> Refusal {
    match e {
        DocError::BadCrc(b) => Refusal::BadCrc(b),
        DocError::Store(s) => Refusal::from(s),
        DocError::NotDoc => Refusal::Corrupt("the root is not a DOC root"),
        DocError::Corrupt(w) => Refusal::Corrupt(w),
        // W-MG-M3: a sealed segment left the file and no signed TRUNCATE names its root.
        DocError::CutUndeclared { root } => Refusal::CutUndeclared { root },
        DocError::Actor | DocError::UnknownActor(_) | DocError::Revoked { .. } | DocError::Seq { .. } | DocError::NothingToCut | DocError::RevokeTip { .. } | DocError::CutInsideSeal { .. } | DocError::CutsAdmit(_) | DocError::TakesRevoke(_) | DocError::Readmit(_) | DocError::NoTree | DocError::Carries(_) | DocError::Fork { .. } | DocError::ForeignRoot | DocError::KeyClash(_) | DocError::CutOneSide { .. } | DocError::ForgetOneSide | DocError::RevokedTail { .. } | DocError::LegacyTail { .. } | DocError::NotWhole { .. } | DocError::UnionNotWhole | DocError::SealFork { .. } => {
            Refusal::Corrupt("not a verify refusal")
        }
    }
}

/// Pick, PartTab crc, the anchor over the last commit (a torn one is `LastCommit`, never a
/// fallback), then the DOC's counters under `ids`, `v` and `trust`.
pub fn verify_doc(bytes: &[u8], ids: &dyn Ids, v: &dyn Verifier, trust: Trust<'_>) -> Result<DocReport, Refusal> {
    let st = Store::from_bytes(bytes);
    let rec = recover(&st)?;
    if let (Some(_), Via::Fallback { why, .. }) = (rec.hidden, rec.via) {
        return Err(Refusal::LastCommit(why));
    }
    let doc = doc::verify(&st, ids, v, trust).map_err(refusal)?;
    Ok(DocReport { generation: rec.sb.generation, via: rec.via, doc })
}

#[cfg(test)]
mod tests;
