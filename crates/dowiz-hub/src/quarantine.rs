//! A RECORD WHOSE CRC FAILS IS QUARANTINED, NOT REFUSED (operator, 2026-10-05).
//!
//! Two policies, by image kind:
//!   * compacted KV images (catalogue, settings, posts, tables, roster) REFUSE the
//!     whole image on a failed crc (`crate::kv_load`): there is no "rest of the image"
//!     to serve around one bad array, and last night's copy is the remedy.
//!   * append logs (the order `Hub`, `LogImage`) QUARANTINE the record: it is left out
//!     of `events()` / `entries()`, named in `quarantined()` with reason `"crc"` (the
//!     law `len() == events().len() + quarantined().len()` holds), carried VERBATIM --
//!     failed crc included -- through every grow, rotation and redaction rebuild, and
//!     reaches the owner through `/api/owner/health`'s quarantine list.
//!     `StockLog` still refuses (it calls the strict `chain_is_whole`; its files belong
//!     to another lane -- hand-back in the W-CRC verdict).
//!
//! WHERE A BAD RECORD STILL REFUSES THE IMAGE is `EvLog::chain_scan`'s doc, exactly: a
//! failed ROOT crc; a chain that does not deliver the root's count or does not end; or a
//! bad record whose `prev` id does not name the record its `next` ref leads to (a changed
//! ref cannot be told from a changed `prev`, so both refuse).

use bebop_store::evlog::EvLog;
use bebop_store::{BadCrc, Store};

use crate::{Hub, HubError};

/// `chain_is_whole` for an append log under the quarantine policy: refuses what breaks
/// the walk, and hands back the records it quarantines (newest first).
pub(crate) fn chain_is_whole_quarantining(store: &Store) -> Result<Vec<BadCrc>, HubError> {
    let claimed = EvLog::len(store);
    let scan = EvLog::chain_scan(store).map_err(HubError::BadCrc)?;
    match scan.chained {
        Some(c) if c == claimed => Ok(scan.quarantined),
        chained => Err(HubError::Corrupt { claimed, chained }),
    }
}

/// WHAT THE LOAD-TIME SCAN FOUND, CARRIED (W-LOOPB, R-LOOPS rows 4/6). `chain_scan` hashed
/// every record when the image was loaded; a reader that walks it again (`Hub::events`,
/// `LogImage::recent_checked`) needs only to know WHICH records failed, not to hash them all
/// a second time. Positions count from the OLDEST record, because that number does not move
/// when a record is appended (an append seals a good one) or when the image grows (the copy
/// carries every failed crc to the same place). A rebuild that re-lays the chain (a
/// redaction, a rotation) cannot keep it: its owner drops it to `None` and the readers go
/// back to `walk_marked`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Seen {
    /// Records on the chain when it was scanned; every record past them was appended here.
    pub(crate) at: usize,
    /// The quarantined (failed-crc) records among those, oldest = 0, ascending.
    pub(crate) bad: Vec<usize>,
}

impl Seen {
    /// Is the record at `pos` of a newest-first walk of `n` records a failed crc? `None`
    /// when the walk is SHORTER than the scan -- this is not the chain that was scanned.
    pub(crate) fn is_bad(&self, n: usize, pos: usize) -> Option<bool> {
        let oldest = n.checked_sub(1 + pos)?;
        (n >= self.at).then(|| self.bad.binary_search(&oldest).is_ok())
    }
}

/// `chain_is_whole_quarantining`, keeping what it found as a `Seen`.
pub(crate) fn chain_is_whole_seen(store: &Store) -> Result<Seen, HubError> {
    let quarantined = chain_is_whole_quarantining(store)?;
    let at = EvLog::len(store);
    let mut bad = Vec::with_capacity(quarantined.len());
    if !quarantined.is_empty() {
        // Rare (a venue with a damaged record): one pointer walk to turn objects into places.
        let mut cur = store.root().and_then(|r| store.follow(r, 1));
        let mut pos = 0usize;
        while let (Some(obj), true) = (cur, pos < at) {
            if quarantined.iter().any(|b| b.obj == obj) {
                bad.push(at - 1 - pos);
            }
            pos += 1;
            cur = store.follow(obj, 2);
        }
        bad.reverse();
    }
    Ok(Seen { at, bad })
}

impl Hub {
    /// The records this log quarantines for a failed crc, newest first -- one crc pass,
    /// no decode. What the Worker's `image-quarantined:` log line names.
    pub fn crc_quarantined(&self) -> Vec<BadCrc> {
        EvLog::chain_scan(&self.store).map(|s| s.quarantined).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests;
