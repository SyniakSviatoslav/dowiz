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

impl Hub {
    /// The records this log quarantines for a failed crc, newest first -- one crc pass,
    /// no decode. What the Worker's `image-quarantined:` log line names.
    pub fn crc_quarantined(&self) -> Vec<BadCrc> {
        EvLog::chain_scan(&self.store).map(|s| s.quarantined).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests;
