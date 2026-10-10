//! THE LOG'S CRC, ONCE PER RECORD, IN THE OBJECT (W-HUBCRC, 2026-10-09).
//!
//! Every turn that writes the order log (`append`, `place`, `advance`, `assign`, the room
//! commands) loaded it with `Hub::load`, which since W-CRC hashes EVERY record: FRESH
//! 866-872 us of a 2 000-event log's ~1.2 ms load, on every order, on a 10 ms CPU plan.
//! The log is append-only and this object holds its bytes in `mem` between turns, so the
//! records it already checked are the same cells next turn. `Hub::load_since` re-hashes
//! only the records newer than a `LogMark` (`bebop_store::verify::since`); the root, the
//! whole walk, the count and the `prev` links are checked on every load as before.
//!
//! THE SAME FOR EVERY APPEND LOG THE OBJECT WRITES PER TURN (follow-up, 2026-10-09): the
//! stock log (`StockLog::load_since`: a mark only for a log with nothing quarantined) and
//! the entry logs -- wallet ledger, till (`LogImage::load_since`). One mark per image id.
//!
//! WHEN THE OBJECT HOLDS A MARK, and the one rule that keeps it honest: a mark describes
//! bytes that never left this object's memory since a full or marked scan passed on them.
//!   * TAKEN by `load_log` / `load_stock` / `load_entries`, keyed by image id and by the
//!     generation the bytes were read at.
//!   * CARRIED across a write only when the write is this object's own derivation of the
//!     bytes the mark describes: for the order log `Written::Appended` (`append`) or
//!     `Written::Log` / `Written::Tail` (`put_log`: a command's hub, loaded from `mem` at
//!     `expected`; `Tail` is W-LOOPB's appended-only form of `Log`); for the
//!     other logs a write through `put_derived`, which their writers call with a log they
//!     loaded through these helpers in the same turn. The anchor stays where it was, so the
//!     records the write added are HASHED by the next load, not trusted.
//!   * DROPPED whenever bytes enter `mem` any other way: the cold read from storage in
//!     `image()` and every other write (a Worker's `/img/<id>` put, a rotation, an import,
//!     a stock reset), at the same two points W-ZC drops the catalogue's `Checked`. A cold
//!     object has none. So storage is always read with a full scan.
//! `rebuild` keeps `Hub::load` (a fresh full scan, by design), as do the archive readers
//! and the rarer entry logs (consent, the catalogue's journal).

use super::{HubImages, LOG_IMAGE};
use crate::fold::projection::Written;
use dowiz_hub::logimage::LogImage;
use dowiz_hub::stock::StockLog;
use dowiz_hub::{Hub, HubError, LogMark};

impl HubImages {
    /// Bytes are entering `mem` under `id` from storage: a mark no longer describes them.
    pub(super) fn logmark_forget(&self, id: &str) {
        self.log_marks.borrow_mut().remove(id);
    }

    /// A write to `id` landed at `next`: its mark is carried or dropped (`carried`).
    pub(super) fn logmark_written(&self, id: &str, how: &Written, expected: i64, next: i64) {
        let held = self.log_marks.borrow_mut().remove(id);
        if let Some(m) = carried(held, how, expected, next) {
            self.log_marks.borrow_mut().insert(id.to_string(), m);
        }
    }

    /// A write of a log this turn loaded through `load_stock` / `load_entries` and appended
    /// to: `put_image`, saying the bytes are the object's own derivation (module doc).
    /// For an image that is not the order log, `how` says nothing else.
    pub(super) async fn put_derived(&self, id: &str, expected: i64, bytes: &[u8]) -> worker::Result<Option<i64>> {
        self.put_image_as(id, expected, bytes, Written::Log(Vec::new())).await
    }

    /// The mark for `id`'s bytes at `generation`, if the object holds one.
    fn mark_at(&self, id: &str, generation: i64) -> Option<LogMark> {
        self.log_marks.borrow().get(id).filter(|(g, _)| *g == generation).map(|(_, m)| *m)
    }

    /// What a load proved becomes the mark for `id` at `generation` (or none).
    fn mark_now(&self, id: &str, generation: i64, next: Option<LogMark>) {
        match next {
            Some(m) => self.log_marks.borrow_mut().insert(id.to_string(), (generation, m)),
            None => self.log_marks.borrow_mut().remove(id),
        };
    }

    /// The order log as held in `mem` at `generation`, loaded through the mark when it
    /// describes these bytes and with a full scan when not; the mark is then moved to them.
    /// ONLY for the bytes `image(LOG_IMAGE)` returned for `generation`.
    pub(super) fn load_log(&self, generation: i64, bytes: &[u8]) -> Result<Hub, HubError> {
        let (hub, next) = Hub::load_since(bytes, self.mark_at(LOG_IMAGE, generation).as_ref())?;
        self.mark_now(LOG_IMAGE, generation, next);
        Ok(hub)
    }

    /// `load_log` for the stock log (`IMAGE_STOCK`).
    pub(super) fn load_stock(&self, generation: i64, bytes: &[u8]) -> Result<StockLog, HubError> {
        let id = crate::hubstore::IMAGE_STOCK;
        let (log, next) = StockLog::load_since(bytes, self.mark_at(id, generation).as_ref())?;
        self.mark_now(id, generation, next);
        Ok(log)
    }

    /// `load_log` for an entry log held under `id` (the wallet ledger, the till).
    pub(super) fn load_entries(&self, id: &str, generation: i64, bytes: &[u8]) -> Result<LogImage, HubError> {
        let (log, next) = LogImage::load_since(bytes, self.mark_at(id, generation).as_ref())?;
        self.mark_now(id, generation, next);
        Ok(log)
    }
}

/// What a log's mark becomes after a write from `expected` to `next`.
pub(super) fn carried(mark: Option<(i64, LogMark)>, how: &Written, expected: i64, next: i64) -> Option<(i64, LogMark)> {
    match (mark, how) {
        (Some((g, m)), Written::Appended(_) | Written::Log(_) | Written::Tail(_)) if g == expected => Some((next, m)),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
