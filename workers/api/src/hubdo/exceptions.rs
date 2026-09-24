//! THE EXCEPTION ALERT, in the venue's object (BLUEPRINT-OPERATIONAL-BLIND-SPOTS
//! §2.5): after a turn that may have written an exception — an amendment, a
//! payment, a refund, a till pay-out or close — fold the rows (pure,
//! `crate::exceptions::fold`), ask `alert::due` whether a venue-period count
//! crossed the owner's threshold, and put what it returns into the `outbox`
//! beside the event, as `enqueue_bell` does for the kitchen's message.
//!
//! THE WALLET LEGS (law 12) alert on FIRST sight, not on a count: every
//! finding `exceptions::legs` lays out that has no marker yet is told once,
//! and its marker (outbox record kind `legs::MARK_KIND`, which the drain never
//! reads) is written in the same image write as the message.
//!
//! EVERY IMAGE IS ALREADY IN THE OBJECT'S MEMORY, so this costs no request.
//!
//! AN ALERT THAT CANNOT BE QUEUED DOES NOT UNDO THE EVENT. The amendment or
//! pay-out has landed; the alert is evidence about it. A failure is said out
//! loud (`console_error!`) and the report still lists the rows.

use super::HubImages;
use crate::exceptions::{alert, fold, legs};
use crate::outbox::{IMAGE_OUTBOX, KIND, OUTBOX_BYTES};
use dowiz_hub::logimage::LogImage;
use dowiz_hub::table::Table;
use worker::*;

impl HubImages {
    /// Queue the exception alerts this turn owes. Never fails the caller.
    pub(super) async fn exceptions_after(&self, venue: &str, now_ms: i64) {
        if let Err(e) = self.exceptions_alert(venue, now_ms).await {
            console_error!("exceptions alert for {venue} not queued: {e}");
        }
    }

    async fn exceptions_alert(&self, venue: &str, now_ms: i64) -> Result<()> {
        let Some((_, sb)) = self.image(crate::hubstore::IMAGE_SETTINGS).await? else { return Ok(()) };
        let settings = dowiz_hub::settings::Settings::load(&sb)
            .map_err(|_| Error::RustError("settings image is unreadable".into()))?;
        let threshold = alert::threshold(settings.get(alert::THRESHOLD_KEY).as_deref());
        let chat = settings.known("notify.telegram.chat");
        // No chat: nobody to tell. A threshold of 0 turns the COUNT alerts off,
        // never the wallet-leg ones (money integrity, first sight).
        if chat.trim().is_empty() {
            return Ok(());
        }
        let (_, hub) = self.log_hub().await?;
        let till = match self.image(crate::command::till::IMAGE_TILL).await? {
            Some((_, b)) => LogImage::load(&b).map_err(|_| Error::RustError("till image is unreadable".into()))?.entries(),
            None => Vec::new(),
        };
        let periods = crate::command::till::periods(&till).map_err(Error::RustError)?;
        let late = alert::late_ms(settings.get(alert::LATE_KEY).as_deref());
        let mut rows = fold::order_rows(&hub.events_oldest_first(), late);
        rows.extend(fold::till_rows(&till, &periods));
        let since = match periods.last().filter(|p| p.closed_at.is_none()) {
            Some(p) => p.opened_at,
            None => now_ms - alert::WINDOW_MS,
        };
        // The venue's own record (the catalogue image): its zone for the
        // period start, and its `default_locale` for the words.
        // Its currency is the wallet legs' default (`services::venue::currency_of`).
        let catalog = match self.image(super::CATALOG_IMAGE).await? {
            Some((_, b)) => Some(dowiz_hub::catalog::Catalog::load(&b).map_err(|_| Error::RustError("catalog image is unreadable".into()))?),
            None => None,
        };
        let record: Option<serde_json::Value> =
            catalog.as_ref().and_then(|c| c.location()).and_then(|j| serde_json::from_str(&j).ok());
        let lang = record.as_ref().and_then(|r| r.get("default_locale")).and_then(|v| v.as_str()).unwrap_or("en");
        let voice = alert::Voice { venue, zone: crate::hubstore::zone_of(record.as_ref()), lang };
        let mut due = alert::due(&rows, since, now_ms, threshold, &voice, &chat);
        let currency = catalog.as_ref().map_or_else(|| "ALL".to_string(), crate::services::venue::currency_of);
        // A ledger that does not replay is said out loud and does not silence
        // the count alerts; the report refuses it in the owner's face.
        let legs = self.leg_rows(venue, &currency).await.unwrap_or_else(|e| {
            console_error!("exceptions: the wallet legs of {venue} do not fold: {e}");
            Vec::new()
        });
        let (generation, mut table) = match self.image(IMAGE_OUTBOX).await? {
            Some((meta, b)) => (
                meta.generation,
                Table::load(&b, OUTBOX_BYTES).map_err(|_| Error::RustError("outbox image is unreadable".into()))?,
            ),
            None => (0, Table::create(OUTBOX_BYTES).map_err(|_| Error::RustError("cannot create outbox image".into()))?),
        };
        let (first, marks) = legs::first(&legs, &|id| table.get(legs::MARK_KIND, id).is_some(), now_ms, &voice, &chat);
        due.extend(first);
        if due.is_empty() {
            return Ok(());
        }
        for m in &marks {
            table.put(legs::MARK_KIND, m, &now_ms.to_string(), &[], &[]).map_err(|x| Error::RustError(format!("outbox: {x:?}")))?;
        }
        for e in &due {
            let rec = serde_json::to_string(e).map_err(|x| Error::RustError(x.to_string()))?;
            table.put(KIND, &e.id, &rec, &[], &[]).map_err(|x| Error::RustError(format!("outbox: {x:?}")))?;
        }
        let bytes = table.to_bytes().map_err(|x| Error::RustError(format!("outbox will not serialise: {x:?}")))?;
        if self.put_image(IMAGE_OUTBOX, generation, &bytes).await?.is_none() {
            return Err(Error::RustError("the outbox generation moved".into()));
        }
        Ok(())
    }

    /// The wallet-leg findings (law 12) from the images in memory: the order
    /// fold and the ledger, oldest first. A ledger that does not replay is an
    /// error, said by the caller, never "no findings".
    async fn leg_rows(&self, venue: &str, currency: &str) -> Result<Vec<fold::Row>> {
        let rows: Vec<String> = match self.image(crate::wallet::IMAGE_LEDGER).await? {
            Some((_, b)) => {
                let log = LogImage::load(&b).map_err(|_| Error::RustError("ledger image is unreadable".into()))?;
                let mut es = log.about(crate::wallet::K_TX, None, usize::MAX);
                es.reverse();
                es.into_iter().map(|e| e.json).collect()
            }
            None => Vec::new(),
        };
        let (_, listed) = self.orders_view().await?;
        legs::leg_rows(&legs::orders_json(&listed), &rows, venue, currency).map_err(Error::RustError)
    }
}
