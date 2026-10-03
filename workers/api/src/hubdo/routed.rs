//! THE GROUPS' MESSAGES, WRITTEN IN THE TURN THAT CAUSED THEM (W0a/W0b).
//!
//! The rules are pure (`notify::route::produce`, `stock/tell.rs`); this file
//! only reads the settings image the object already holds, asks whether any
//! group hears the event, and puts the routed entries -- and the marks that
//! make `order.late` / `stock.expiring` "once" -- into the `outbox` image in
//! ONE write, as `enqueue_bell` does for the kitchen's ticket.
//!
//! A LEGACY VENUE (no `notify.tg.groups`) NEVER GETS A ROUTED ENTRY: its
//! messages are the old tickets, byte for byte (`notify::route::bell`).
//!
//! A MESSAGE THAT CANNOT BE QUEUED NEVER UNDOES THE EVENT: loud, not fatal.

use super::HubImages;
use crate::notify::route::{groups, produce, Group};
use crate::outbox::{Entry, IMAGE_OUTBOX, KIND, OUTBOX_BYTES};
use dowiz_hub::table::Table;
use worker::*;

/// What a turn owes the outbox: entries, marks to set and marks to clear.
#[derive(Default)]
pub(super) struct Owed {
    pub entries: Vec<Entry>,
    pub mark: Vec<(&'static str, String)>,
    pub unmark: Vec<(&'static str, String)>,
}

impl HubImages {
    /// The venue's groups, when it has written some. `None` = a legacy venue,
    /// no settings, or groups nobody can read (said out loud).
    pub(super) async fn routing_groups(&self) -> Option<(dowiz_hub::settings::Settings, Vec<Group>)> {
        let bytes = match self.image(crate::hubstore::IMAGE_SETTINGS).await {
            Ok(Some((_, b))) => b,
            _ => return None,
        };
        let s = dowiz_hub::settings::Settings::load(&bytes).ok()?;
        s.get(groups::KEY_GROUPS)?;
        match crate::notify::route::groups_of(&s, "en") {
            Ok(g) => Some((s, g.list)),
            Err(e) => {
                log_error!("telegram groups unreadable, nothing routed: {e}");
                None
            }
        }
    }

    /// The marks of one kind already in the outbox.
    pub(super) async fn marks(&self, kind: &str) -> Vec<String> {
        match self.image(IMAGE_OUTBOX).await {
            Ok(Some((_, b))) => Table::load(&b, OUTBOX_BYTES).map(|t| t.all(kind).into_iter().map(|(id, _)| id).collect()).unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    /// Write what a turn owes, in one outbox write. Never fails the caller.
    pub(super) async fn route_owed(&self, owed: Owed, what: &str) {
        if owed.entries.is_empty() && owed.mark.is_empty() && owed.unmark.is_empty() {
            return;
        }
        if let Err(e) = self.write_owed(&owed).await {
            log_error!("outbox: {what} happened and its group messages were NOT queued: {e}");
        }
    }

    async fn write_owed(&self, owed: &Owed) -> Result<()> {
        let (generation, mut table) = match self.image(IMAGE_OUTBOX).await? {
            Some((meta, b)) => (meta.generation, Table::load(&b, OUTBOX_BYTES).map_err(|_| Error::RustError("outbox image is unreadable".into()))?),
            None => (0, Table::create(OUTBOX_BYTES).map_err(|_| Error::RustError("cannot create outbox image".into()))?),
        };
        for e in &owed.entries {
            let rec = serde_json::to_string(e).map_err(|x| Error::RustError(x.to_string()))?;
            table.put(KIND, &e.id, &rec, &[], &[]).map_err(|x| Error::RustError(format!("outbox: {x:?}")))?;
        }
        for (kind, id) in &owed.mark {
            table.put(kind, id, "1", &[], &[]).map_err(|x| Error::RustError(format!("outbox: {x:?}")))?;
        }
        for (kind, id) in &owed.unmark {
            table.remove(kind, id);
        }
        let bytes = table.to_bytes().map_err(|x| Error::RustError(format!("outbox will not serialise: {x:?}")))?;
        if self.put_image(IMAGE_OUTBOX, generation, &bytes).await?.is_none() {
            return Err(Error::RustError("the outbox generation moved".into()));
        }
        Ok(())
    }

    /// AFTER AN ORDER TURN (W0b): its new status, when a group hears it, and
    /// every open order that has waited past the venue's late threshold and
    /// was not told yet. `orders` are the orders as this turn left them.
    pub(super) async fn tell_orders(&self, status: Option<(&str, &str)>, orders: &[(String, String)], now_ms: i64) {
        self.tell_push_orders(status, orders, now_ms).await; // W-PUSH: the phones first (`hubdo/push_turn.rs`)
        let Some((settings, groups)) = self.routing_groups().await else { return };
        let mut owed = Owed::default();
        if let (Some((id, next)), true) = (status, produce::wants(&groups, "order.status")) {
            owed.entries.push(produce::status(id, next, now_ms));
        }
        if produce::wants(&groups, "order.late") {
            let late_ms = produce::late_ms(settings.get(produce::LATE_KEY).as_deref());
            let marked = self.marks(produce::LATE_MARK).await;
            let (entries, mark, drop) = produce::late(orders, &marked, now_ms, late_ms);
            owed.entries.extend(entries);
            owed.mark.extend(mark.into_iter().map(|m| (produce::LATE_MARK, m)));
            owed.unmark.extend(drop.into_iter().map(|m| (produce::LATE_MARK, m)));
        }
        self.route_owed(owed, "an order turn").await;
    }

    /// The shelf before an order turn, when a group hears `stock.low`; `None`
    /// otherwise, so a venue nobody told pays no second fold.
    pub(super) async fn low_watch(&self, stock: &dowiz_hub::stock::StockLog) -> Option<dowiz_hub::stock::StockLedger> {
        let (_, groups) = self.routing_groups().await?;
        if !produce::wants(&groups, "stock.low") || stock.len() == 0 {
            return None;
        }
        stock.ledger().ok()
    }

    /// AFTER AN ORDER TURN THAT HELD STOCK (W0a): the supplies whose free
    /// quantity crossed their threshold in it, told once, by the crossing.
    pub(super) async fn tell_low(&self, before: &dowiz_hub::stock::StockLedger, stock: &dowiz_hub::stock::StockLog, now_ms: i64) {
        let Ok(after) = stock.ledger() else { return };
        let items: Vec<String> = after
            .items()
            .into_iter()
            .filter(|(id, lv)| before.available(id) != lv.available())
            .map(|(id, _)| id)
            .collect();
        if items.is_empty() {
            return;
        }
        let supplies = match self.image(super::CATALOG_IMAGE).await {
            Ok(Some((_, b))) => match dowiz_hub::catalog::Catalog::load(&b) {
                Ok(c) => crate::services::operations::stock::turn::supplies_of(c.supplies()),
                Err(_) => return,
            },
            _ => return,
        };
        let supply = |id: &str| supplies.get(id).map(|s| produce::Supply { name: s.name.clone(), unit: s.unit.clone(), low_at: s.low_at });
        let was = |id: &str| produce::Shelf { free: before.available(id), counted: before.is_counted(id) };
        let now = |id: &str| produce::Shelf { free: after.available(id), counted: after.is_counted(id) };
        let Some(data) = produce::low(&items, &was, &now, &supply) else { return };
        let owed = Owed { entries: produce::entries(&format!("stock/{}", stock.len()), vec![("stock.low", data)], now_ms), ..Owed::default() };
        self.route_owed(owed, "an order that held stock").await;
    }
}
