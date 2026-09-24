//! THE FISCAL QUEUE, in the venue's object: the one write `hubdo::place` owes
//! when an order that takes money lands at a venue that fiscalises. The rules
//! are `fiscal::wire`, pure; this file reads the images, runs them, and writes
//! the entry into the venue's own `fiscal` image.
//!
//! IN THE SAME TURN AS THE ORDER, for the reason the bell is (`enqueue_bell`):
//! "the order took money" and "a document is owed" are one fact. A failure to
//! queue does NOT fail the order -- it is in the log and the money is taken --
//! it is loud, and conservation law N names the order until it is fixed.
//!
//! NOTHING IS SENT FROM HERE: sending is the armed venue's cron firing
//! (`fiscal/rail.rs`), whose object half is `fiscal/send.rs`.

/// The eBills sender's commands (`/fold/ebills/fiscal_*`, card L70).
mod send;

use super::HubImages;
use crate::fiscal::queue::KIND;
use crate::fiscal::wire::{self, AtPlacement};
use dowiz_hub::table::Table;
use worker::*;

impl HubImages {
    /// Queue the fiscal document for the order just written as `stored`.
    ///
    /// A VENUE THAT DOES NOT FISCALISE COSTS ONE CACHED SETTINGS READ: the
    /// catalogue (for the invoicing currency) and the `fiscal` image are only
    /// touched once `fiscal.since_ms` is set.
    pub(super) async fn enqueue_fiscal(&self, stored: &str, now_ms: i64) -> Result<AtPlacement> {
        let cfg = match self.image(crate::hubstore::IMAGE_SETTINGS).await? {
            Some((_, b)) => dowiz_hub::settings::Settings::load(&b)
                .map(|s| wire::config(&s.known(wire::SETTING)))
                .map_err(|_| Error::RustError("settings image is unreadable".into()))?,
            None => wire::Config::Off,
        };
        if !matches!(cfg, wire::Config::From(_)) {
            return Ok(AtPlacement::NotConfigured);
        }
        let currency = match self.image(super::CATALOG_IMAGE).await? {
            Some((_, b)) => crate::services::venue::currency_of(
                &dowiz_hub::catalog::Catalog::load(&b)
                    .map_err(|_| Error::RustError("catalog image is unreadable".into()))?,
            ),
            None => "ALL".into(),
        };
        let order: serde_json::Value = serde_json::from_str(stored)
            .map_err(|e| Error::RustError(format!("the stored order is not json: {e}")))?;
        let out = wire::at_placement(&cfg, &order, &currency, now_ms);
        let AtPlacement::Queued(entry) = &out else { return Ok(out) };

        let (generation, mut table) = match self.image(wire::IMAGE).await? {
            Some((meta, bytes)) => (
                meta.generation,
                Table::load(&bytes, wire::CEILING)
                    .map_err(|_| Error::RustError("fiscal image is unreadable".into()))?,
            ),
            None => (0, Table::create(wire::CEILING).map_err(|_| Error::RustError("cannot create fiscal image".into()))?),
        };
        // A REPLAYED PLACEMENT FINDS ITS OWN ENTRY and leaves it alone: its
        // `queued_at_ms` IS the issue instant the 48 h run from, and writing
        // over it would move the deadline.
        if table.has(KIND, &entry.id) {
            return Ok(out);
        }
        let rec = serde_json::to_string(entry).unwrap_or_default();
        table.put(KIND, &entry.id, &rec, &[], &[]).map_err(|e| Error::RustError(format!("fiscal: {e:?}")))?;
        let bytes = table.to_bytes().map_err(|e| Error::RustError(format!("fiscal will not serialise: {e:?}")))?;
        if self.put_image(wire::IMAGE, generation, &bytes).await?.is_none() {
            return Err(Error::RustError("the fiscal generation moved".into()));
        }
        Ok(out)
    }
}
