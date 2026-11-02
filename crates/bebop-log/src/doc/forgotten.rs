//! FORGOTTEN (W-MG-M4, W-MG-HOLES): the signed declaration of an erasure, and the list that binds
//! it to the tombstones it covers. Why a list and not a count: `seg.rs`.

#[allow(unused_imports)]
use crate::prelude::*;

pub const FORGOTTEN_MARK: &[u8] = b"ein-forgotten/1";

/// One tombstone a FORGOTTEN covers: its id, and `Ids::chained` over it as it now stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Erased {
    pub id: [u8; 32],
    pub digest: [u8; 32],
}

/// The FORGOTTEN declaration: the records `erased` lists, of `subject` (a hash of the person's
/// key, never the key), were redacted in place.
pub fn forgotten_body(subject: &[u8; 32], erased: &[Erased]) -> Vec<u8> {
    let mut b = Vec::with_capacity(FORGOTTEN_MARK.len() + 40 + erased.len() * 64);
    b.extend_from_slice(FORGOTTEN_MARK);
    b.extend_from_slice(&(erased.len() as u64).to_le_bytes());
    b.extend_from_slice(subject);
    for e in erased {
        b.extend_from_slice(&e.id);
        b.extend_from_slice(&e.digest);
    }
    b
}

/// `(subject, the list)` of a FORGOTTEN body; `None` when it is not one, or when `n_redacted` is
/// not what the body holds.
pub fn parse_forgotten(body: &[u8]) -> Option<([u8; 32], Vec<Erased>)> {
    let r = body.strip_prefix(FORGOTTEN_MARK)?;
    let n = u64::from_le_bytes(r.get(..8)?.try_into().ok()?);
    let subject: [u8; 32] = r.get(8..40)?.try_into().ok()?;
    let list = r.get(40..)?;
    if list.len() % 64 != 0 || (list.len() / 64) as u64 != n {
        return None;
    }
    Some((subject, list.chunks_exact(64).filter_map(|e| Some(Erased { id: e.get(..32)?.try_into().ok()?, digest: e.get(32..)?.try_into().ok()? })).collect()))
}
