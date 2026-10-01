//! THE CHAIN: content ids that commit to the record before them, the walk that
//! checks every one, the tip a witness writes down, and the load-time check that
//! an image holds every record it claims.
//!
//! Shared beyond the order log: `stock.rs`, `logimage.rs`, `forget.rs` and
//! `consent/forget.rs` address their own records with `content_id_chained`.

use bebop_store::{evlog::EvLog, Store};

use crate::forget;
use crate::{Hub, HubError};

impl Hub {
    /// Walk the chain and check every id against the payload it names.
    ///
    /// THE POINT IS THAT IT CAN FAIL. An append-only log whose ids are never
    /// recomputed is an append-only log by assertion; this is the assertion
    /// being checked, and it is the half of I4 that was claimed in a comment
    /// and not delivered by any code.
    pub fn chain_check(&self) -> ChainCheck {
        let mut out = ChainCheck::default();
        let walked = EvLog::walk(&self.store);
        let tip = EvLog::tip(&self.store);
        for (at, r) in walked.iter().enumerate() {
            out.records += 1;
            if r.id == content_id_chained(&r.prev, &r.payload) {
                out.chained += 1;
            } else if r.id == content_id(&r.payload) {
                out.legacy += 1;
            } else if forget::tombstone_holds(&walked, at, tip) {
                out.redacted += 1;
            } else {
                out.broken += 1;
            }
        }
        out
    }

    /// The newest record's chain id, in hex. `None` for a log with no records.
    ///
    /// THE ONE VALUE A WITNESS NEEDS. Every id commits to the id before it, so
    /// the tip commits to the whole history: two logs with the same tip are the
    /// same log, and a log that no longer holds last night's tip has had its
    /// end rewritten. That is the failure `chain_check` cannot see — a
    /// truncation leaves a shorter chain that is perfectly valid — and it is
    /// why this is published rather than kept inside the check.
    pub fn tip(&self) -> Option<String> {
        EvLog::tip(&self.store).map(|t| hex32(&t))
    }

    /// Is this chain id anywhere in this log?
    ///
    /// The question a witness asks the next night: the tip I wrote down is
    /// still in there, so nothing between it and the start was rewritten. A
    /// `false` after a rotation is not yet an accusation — the record may have
    /// moved to an archive, verbatim and with the same id — so the caller asks
    /// the archives before it says anything out loud.
    pub fn holds(&self, id_hex: &str) -> bool {
        EvLog::walk(&self.store).iter().any(|r| hex32(&r.id) == id_hex)
    }
}

/// Does this image hold every record it says it holds?
///
/// WHY AT LOAD AND NOT AT USE. `load` is on the path of every request, and the
/// cost was the argument against checking anything here -- but the check is a
/// pointer walk over a chain of a few hundred records, while `load` has
/// already decoded the whole image into cells. It is a fraction of a cost
/// already paid, and D0's `reliability-over-latency` is not a slogan: the
/// alternative is every reader above deciding for itself whether a short
/// answer was the truth, which is the decision that was already got wrong.
///
/// WHAT IT DOES NOT DO is verify the ids -- that is `chain_check`, it hashes
/// every record, and it belongs in the nightly job. This asserts what a caller
/// is promised: the chain is as long as the root claims, it ends, and every
/// record on it reads back.
///
/// AND IT IS ABOUT THE IMAGE, NOT ABOUT ONE RECORD. An image that arrived
/// incomplete is refused, because the caller can re-fetch it. A single record
/// this build cannot parse is a different failure and gets the opposite
/// answer: it is QUARANTINED, counted and served around, because one bad
/// record must not close the restaurant. See `Hub::quarantined`.
pub(crate) fn chain_is_whole(store: &Store) -> Result<(), HubError> {
    let claimed = EvLog::len(store);
    let Some(chained) = EvLog::chain_len(store) else {
        return Err(HubError::Corrupt { claimed, chained: None });
    };
    if chained != claimed {
        return Err(HubError::Corrupt { claimed, chained: Some(chained) });
    }
    Ok(())
}

pub(crate) fn hex32(b: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for x in b {
        s.push(char::from_digit((x >> 4) as u32, 16).unwrap_or('0'));
        s.push(char::from_digit((x & 15) as u32, 16).unwrap_or('0'));
    }
    s
}

/// Content id over the PREVIOUS ID AND THE PAYLOAD — the cascade that makes
/// the chain tamper-evident.
///
/// WHAT THIS FIXES. `content_id` hashes the payload alone, so editing an event
/// in an image changed that event's id and NOTHING ELSE: the records after it
/// still verified, and `stock.rs` said in as many words that "editing any
/// event changes every content id after it", which was not true of the code
/// under the comment. Folding the previous id in makes it true: rewriting
/// event N breaks N's own id, and repairing that id breaks N+1's `prev` and
/// therefore N+1's id, all the way to the tip.
///
/// Still FNV rather than sha256, for the reason below: this crate has no
/// dependencies and the cryptographic commitment lives in the kernel, where
/// the keys are. What this gives is detection of an EDIT, not resistance to a
/// determined forger.
pub(crate) fn content_id_chained(prev: &[u8; 32], payload: &[u8]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(32 + payload.len());
    buf.extend_from_slice(prev);
    buf.extend_from_slice(payload);
    content_id(&buf)
}

/// What a walk of the chain found. `legacy` records were written before the
/// cascade and can only be checked against the old scheme, which is a fact
/// about them rather than a fault: pretending otherwise would make every image
/// in production look broken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChainCheck {
    pub records: usize,
    /// Verified against `content_id_chained(prev, payload)`.
    pub chained: usize,
    /// Verified only against the old `content_id(payload)`.
    pub legacy: usize,
    /// Matched neither. An edited event, or a damaged one.
    pub broken: usize,
    /// Redacted in place by `Hub::forget`: verified by LINK, not content.
    /// Must equal what the `Forgotten` declarations name (conservation law 9).
    pub redacted: usize,
}

impl ChainCheck {
    /// Nothing in this log fails BOTH schemes.
    pub fn intact(&self) -> bool {
        self.broken == 0
    }
}

/// Content id over the payload. Not a cryptographic commitment — it is the
/// chain's own addressing, and it is derived from the bytes so two identical
/// events are indistinguishable by construction.
pub(crate) fn content_id(payload: &[u8]) -> [u8; 32] {
    // FNV-1a over 4 lanes, spread to 32 bytes. Deliberately NOT sha256: this
    // crate has zero dependencies, and the cryptographic chain commitment lives
    // in the kernel where the keys are.
    let mut out = [0u8; 32];
    for lane in 0..4u64 {
        let mut h: u64 = 0xcbf29ce484222325 ^ lane.wrapping_mul(0x9E3779B97F4A7C15);
        for b in payload {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        out[lane as usize * 8..lane as usize * 8 + 8].copy_from_slice(&h.to_le_bytes());
    }
    out
}
