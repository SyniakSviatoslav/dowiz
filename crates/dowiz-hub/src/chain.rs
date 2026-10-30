//! THE CHAIN: content ids that commit to the record before them, the walk that
//! checks every one, the tip a witness writes down, and the load-time check that
//! an image holds every record it claims.
//!
//! Shared beyond the order log: `stock.rs`, `logimage.rs`, `forget.rs` and
//! `consent/forget.rs` address their own records with `chain_id`.
//!
//! ── TWO SCHEMES, AND THE RECORD SAYS WHICH (M0, 2026-10-30) ──
//!
//! A record written since M0 carries `FLAG_SHA256` in its flags cell, a 16-byte
//! nonce at the front of its payload, and `id = sha256(prev ‖ payload)` -- the
//! commitment `glue.rs` and `glue/wire.rs` had described since S0 while the code
//! under them hashed with four lanes of FNV-1a. Every record written before is
//! LEGACY: its id is `legacy_id_chained` (FNV over `prev ‖ payload`, 2026-09-20
//! to M0) or `legacy_id` (FNV over the payload alone, before that), its flags
//! cell is 0, and it stays exactly as it is -- `chain_check` counts it under
//! `legacy`, never rewrites it, and an old image serialises byte for byte.
//!
//! WHAT sha256 BUYS. FNV-1a is not a commitment: a forger who edits a payload
//! can choose the bytes so the four lanes land where they were, and nothing
//! downstream notices. With sha256 the id is bound to the bytes; what a forger
//! can still do is repair the id -- and `chain_check` now has one more rule for
//! that, under `walk_check`: the scheme may move from legacy to sha256 ONCE,
//! oldest to newest, and never back. A legacy record newer than any sha256
//! record is what "edited after M0, id repaired under the retired scheme"
//! looks like, and it is counted `broken`.
//!
//! WHAT IT DOES NOT BUY, stated so nobody reads more into it. Repairing a
//! record's id under sha256 leaves the record after it naming the OLD id in its
//! `prev`, and this walk COUNTS those gaps (`ChainCheck::gaps`) but cannot
//! refuse them: `rotate.rs` carries records verbatim past a checkpoint, so a
//! healthy rotated image has gaps by design. What binds the whole history is
//! the tip a witness wrote down (`Hub::tip`, `Hub::holds`) and, from M1, the
//! seal over it. The nonce is for M4 (R-MAINGOAL-A §2c): once a record is
//! redacted the nonce is erased with it, so its id cannot confirm a guessed
//! payload by recomputation. A writer with no entropy set derives the nonce
//! from the record itself and says so with `FLAG_NONCE_DERIVED` -- such a nonce
//! protects nothing, and `ChainCheck::derived` counts them so that is visible.

use bebop_store::evlog::{EvLog, Record, FLAG_NONCE_DERIVED};
use bebop_store::Store;

use crate::forget;
use crate::{Hub, HubError};

mod scheme;
pub use scheme::IdScheme;
pub(crate) use scheme::{body_of, chain_id, flags_for, id_for, legacy_id, legacy_id_chained, verify_id, with_nonce, Entropy, ERASED_NONCE, NONCE_LEN};

impl Hub {
    /// Walk the chain and check every id against the payload it names.
    ///
    /// THE POINT IS THAT IT CAN FAIL. An append-only log whose ids are never
    /// recomputed is an append-only log by assertion; this is the assertion
    /// being checked, and it is the half of I4 that was claimed in a comment
    /// and not delivered by any code. The rules are `walk_check`'s.
    pub fn chain_check(&self) -> ChainCheck {
        let walked = EvLog::walk(&self.store);
        let tip = EvLog::tip(&self.store);
        walk_check(&walked, tip, forget::tombstone_holds)
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

    /// THE ENTROPY SEAM. The nonce in every record written after this call is drawn from
    /// `seed` (sha256 over the seed, a counter and the previous id), never from a clock or a
    /// counter alone. This crate has no randomness of its own on wasm32, so the caller
    /// hands it one seed per request, as it hands the clock to the stock log. Without one
    /// the nonce is derived from the record and flagged `FLAG_NONCE_DERIVED`.
    pub fn set_entropy(&mut self, seed: [u8; 32]) {
        self.entropy.set(seed);
    }
}

/// THE WALK'S RULES, shared by the order log and `LogImage`. `walked` is newest first;
/// `tombstone(walked, at, tip)` says whether a record that verifies under no scheme is a
/// redaction whose LINK holds (the order log's `forget::tombstone_holds`; a log with no
/// redaction answers `false`).
pub(crate) fn walk_check(walked: &[Record], tip: Option<[u8; 32]>, tombstone: impl Fn(&[Record], usize, Option<[u8; 32]>) -> bool) -> ChainCheck {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Kind {
        Chained,
        Legacy,
        Redacted,
        Broken,
    }
    let mut kinds: Vec<Kind> = walked
        .iter()
        .enumerate()
        .map(|(at, r)| match verify_id(r.flags, &r.prev, &r.payload, &r.id) {
            Some(IdScheme::Sha256) => Kind::Chained,
            Some(_) => Kind::Legacy,
            None if tombstone(walked, at, tip) => Kind::Redacted,
            None => Kind::Broken,
        })
        .collect();
    // THE SCHEME MOVES FORWARD ONCE. Newest first, so everything before the oldest sha256
    // record was written after it: a legacy id there was repaired under a scheme this log
    // had already left, which is the one forgery FNV lets through and sha256 does not.
    if let Some(oldest_sha) = kinds.iter().rposition(|k| *k == Kind::Chained) {
        for k in kinds.iter_mut().take(oldest_sha) {
            if *k == Kind::Legacy {
                *k = Kind::Broken;
            }
        }
    }
    let mut out = ChainCheck { records: walked.len(), ..ChainCheck::default() };
    for (at, (r, k)) in walked.iter().zip(&kinds).enumerate() {
        match k {
            Kind::Chained => out.chained += 1,
            Kind::Legacy => out.legacy += 1,
            Kind::Redacted => out.redacted += 1,
            Kind::Broken => out.broken += 1,
        }
        if r.flags & FLAG_NONCE_DERIVED != 0 {
            out.derived += 1;
        }
        if walked.get(at + 1).is_some_and(|below| below.id != r.prev) {
            out.gaps += 1;
        }
    }
    out
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
    // AND EVERY RECORD'S CRC (W-CRC): the same walk, hashing the root and each record it
    // counts, so a changed byte is refused here by name instead of folded into an answer.
    let Some(chained) = EvLog::chain_crc(store).map_err(HubError::BadCrc)? else {
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

/// What a walk of the chain found. `legacy` records were written before M0 (or before the
/// 2026-09-20 cascade) and can only be checked against their own scheme, which is a fact
/// about them rather than a fault: pretending otherwise would make every image in
/// production look broken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChainCheck {
    pub records: usize,
    /// Verified as `sha256(prev ‖ payload)` (M0).
    pub chained: usize,
    /// Verified only under a pre-M0 FNV scheme, and older than every sha256 record.
    pub legacy: usize,
    /// Matched no scheme, or a legacy id newer than a sha256 record. An edited event, or a
    /// damaged one.
    pub broken: usize,
    /// Redacted in place by `Hub::forget`: verified by LINK, not content.
    /// Must equal what the `Forgotten` declarations name (conservation law 9).
    pub redacted: usize,
    /// Records whose `prev` is not the id of the record below them. A rotation leaves these
    /// by design (`rotate.rs`); a nightly that knows how many rotations an image has had can
    /// say whether the number is the expected one. Counted, never refused here.
    pub gaps: usize,
    /// Records whose nonce was derived, not drawn (`FLAG_NONCE_DERIVED`): the writer had no
    /// entropy set, so their ids offer no cover to a redacted payload (M4).
    pub derived: usize,
}

impl ChainCheck {
    /// Nothing in this log fails every scheme it may be under.
    pub fn intact(&self) -> bool {
        self.broken == 0
    }
}
