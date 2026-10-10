//! `LogImage::load`, through a mark (W-HUBCRC follow-up, 2026-10-09): the wallet ledger,
//! the till and the other entry logs are append logs like the order `Hub`, loaded on
//! every turn that writes them, so they take the same `LogMark` (`Hub::load_since`,
//! `bebop_store::verify::since`). `None` is the full check `LogImage::load` always made.

use super::LogImage;
use crate::{HubError, LogMark};
use bebop_store::evlog::EvLog;
use bebop_store::Store;

impl LogImage {
    /// `load`, not re-hashing the records `mark` covers; hands back the mark for these bytes.
    pub fn load_since(bytes: &[u8], mark: Option<&LogMark>) -> Result<(Self, Option<LogMark>), HubError> {
        let store = Store::from_bytes(bytes);
        if store.pick().is_none() {
            return Err(HubError::NotAHub);
        }
        // A store with no log root is not this kind of image. Refusing is the
        // point: reading it as empty is a failure dressed as an absence, which
        // is the `.ok().flatten()` defect that would have orphaned a venue's
        // whole order log.
        if EvLog::len(&store) == 0 && store.root().is_none() {
            return Err(HubError::NotAHub);
        }
        // AND IT IS NOT THIS IMAGE EITHER IF IT LOST RECORDS ON THE WAY HERE.
        // The superblock is fifteen cells at the front, so it survives a
        // truncation that takes half the records with it -- this image loaded
        // clean and then said `len() == 40` while `entries()` gave two. That
        // is a venue quietly losing thirty-eight of its errors, messages or
        // ledger postings, and the one place it can still be said out loud is
        // here. See `crate::chain_is_whole`.
        // A record whose crc fails is QUARANTINED, not refused (`crate::quarantine`).
        // ONE walk: the scan's findings are kept (`crate::Seen`, W-LOOPB) and give the mark.
        let (seen, next) = crate::chain_is_whole_seen(&store, mark)?;
        Ok((LogImage { store, seen }, next))
    }
}

#[cfg(test)]
mod tests;
