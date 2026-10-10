//! THE OFFLINE SALE'S COMMANDS, in the venue's object (`/fold/room/offline_*`).
//! The rules are `services::orders::offline_sale::{sync, ledger, overdue}`,
//! pure; this file reads the images, runs them, and writes -- the log first,
//! the shelf second (`write_both`), the fiscal entry third, as `place` does.
//!
//! A CHILD of `hubdo/room.rs`, reached through its default arm, so neither
//! `room.rs` nor `hubdo.rs` grows by this feature.

use super::super::HubImages;
use crate::command::Refused;
use crate::exceptions::alert::Voice;
use crate::fiscal::wire::{AtPlacement, Config, CEILING, IMAGE};
use crate::outbox::{Entry, IMAGE_OUTBOX, KIND, OUTBOX_BYTES};
use crate::services::orders::offline_sale::{ledger, overdue, sync, SyncIn, SyncOut};
use dowiz_hub::table::Table;
use serde_json::Value;
use worker::*;
use crate::wire::{Call as Request, Reply as Response};

/// A synced sale's id, kept in the `fiscal` image after the sale (`offline_mark_seen`).
const SEEN_KIND: &str = "offline.seen";

fn bad(e: impl std::fmt::Display) -> Error {
    Error::RustError(format!("offline sale: {e}"))
}

impl HubImages {
    /// `POST /fold/room/offline_<what>`; anything else is the room's 404.
    pub(super) async fn offline(&self, what: &str, mut req: Request) -> Result<Response> {
        match what {
            "offline_sync" => super::reply(self.offline_sync(crate::body::parse(&mut req).await?).await?),
            "offline_list" => {
                let at: Value = crate::body::parse(&mut req).await?;
                Response::from_json(&self.offline_list(at["now_ms"].as_i64().unwrap_or(0)).await?)
            }
            _ => Response::error("no such room command", 404),
        }
    }

    /// The `fiscal` image's entries (none when it does not exist yet).
    async fn fiscal_entries(&self) -> Result<Vec<Entry>> {
        Ok(match self.image(IMAGE).await? {
            Some((_, b)) => Table::load(&b, CEILING)
                .map_err(|_| bad("the fiscal image is unreadable"))?
                .all(crate::fiscal::queue::KIND)
                .into_iter()
                .filter_map(|(_, j)| serde_json::from_str(&j).ok())
                .collect(),
            None => Vec::new(),
        })
    }

    /// The `fiscal` table and its generation (a fresh one when absent).
    async fn fiscal_table(&self) -> Result<(i64, Table)> {
        Ok(match self.image(IMAGE).await? {
            Some((meta, b)) => (meta.generation, Table::load(&b, CEILING).map_err(|_| bad("the fiscal image is unreadable"))?),
            None => (0, Table::create(CEILING).map_err(|_| bad("cannot create the fiscal image"))?),
        })
    }

    /// THE KEY OUTLIVES THE LOG. The live log is rotated into archives, and the
    /// idempotency record expires; a sale replayed after both would find no
    /// order and append a second one. Its id is kept in the `fiscal` image
    /// (`SEEN_KIND`), which nothing rotates, in a write of its own after the sale.
    async fn offline_mark_seen(&self, order_id: &str, sold_at: i64) -> Result<()> {
        let (generation, mut t) = self.fiscal_table().await?;
        if t.has(SEEN_KIND, order_id) {
            return Ok(());
        }
        t.put(SEEN_KIND, order_id, &sold_at.to_string(), &[], &[]).map_err(|x| bad(format!("fiscal: {x:?}")))?;
        let bytes = t.to_bytes().map_err(|x| bad(format!("fiscal will not serialise: {x:?}")))?;
        match self.put_image(IMAGE, generation, &bytes).await? {
            Some(_) => Ok(()),
            None => Err(bad("the fiscal generation moved")),
        }
    }

    /// The outbox table and its generation (a fresh one when absent).
    async fn offline_outbox(&self) -> Result<(i64, Table)> {
        Ok(match self.image(IMAGE_OUTBOX).await? {
            Some((meta, b)) => (meta.generation, Table::load(&b, OUTBOX_BYTES).map_err(|_| bad("the outbox image is unreadable"))?),
            None => (0, Table::create(OUTBOX_BYTES).map_err(|_| bad("cannot create the outbox image"))?),
        })
    }

    /// ONE SYNCED SALE: decide, write, queue its fiscal document from the SALE's instant.
    async fn offline_sync(&self, mut input: SyncIn) -> Result<std::result::Result<SyncOut, Refused>> {
        // The drawer open at the sale, from the till log this object holds.
        let till = match self.image(crate::command::till::IMAGE_TILL).await? {
            Some((_, b)) => dowiz_hub::logimage::LogImage::load(&b).map_err(|_| bad("the till image is unreadable"))?.entries(),
            None => Vec::new(),
        };
        // A till log that does not fold must not lose a sale: no drawer is named, and it is said.
        let periods = crate::command::till::periods(&till).unwrap_or_else(|e| {
            log_error!("offline sale {}: the till log does not fold, no drawer named: {e}", input.order_id);
            Vec::new()
        });
        sync::into_till(&mut input.envelope, sync::till_at(&periods, input.sold_at_ms));
        let (log_gen, listed) = self.orders_view().await?;
        // SEEN BEFORE, AND NO LONGER IN THE LIVE LOG: rotated out. Nothing is appended.
        if !listed.iter().any(|o| o.order_id == input.order_id) && self.fiscal_table().await?.1.has(SEEN_KIND, &input.order_id) {
            return Ok(Ok(SyncOut {
                order_id: input.order_id.clone(), replayed: true, sold_at_ms: input.sold_at_ms,
                fiscal_deadline_ms: crate::services::orders::offline_sale::rules::deadline(input.sold_at_ms),
                fiscal: "rotated: the sale is in the venue's archive".into(), conflicts: Vec::new(),
            }));
        }
        let (_, mut hub) = self.log_hub().await?;
        let (stock_gen, mut stock) = self.stock_log_at(input.now_ms).await?;
        let before = stock.len();
        // THE VENUE'S TAX AT THE SALE'S INSTANT, not the sync's.
        let tax = match self.image(crate::hubstore::IMAGE_SETTINGS).await? {
            Some((_, b)) => match dowiz_hub::settings::Settings::load(&b) {
                Ok(s) => crate::services::ordering::tax_cfg::resolve(|k| s.known(k), input.sold_at_ms),
                Err(_) => Err("tax: the settings image is unreadable".into()),
            },
            None => Ok(None),
        };
        let currency = self.venue_currency().await?;
        let d = match sync::decide(&mut hub, &mut stock, &listed, &tax, &currency, &input) {
            Ok(d) => d,
            Err(r) => return Ok(Err(r)),
        };
        if !d.replayed {
            let moved = (stock.len() != before).then_some((stock_gen, &stock));
            let next = match self.write_both("an offline sale", log_gen, &hub, moved).await? {
                Ok(n) => n,
                Err(r) => return Ok(Err(r)),
            };
            self.broadcast(dowiz_hub::EventKind::Placed as u8, &input.order_id, &d.stored, next);
        }
        // A REPLAY ANSWERS FROM THE ORDER AS STORED: its instant is the first
        // sync's, whatever clock this copy of the request was checked against.
        let sold_at = serde_json::from_str::<Value>(&d.stored).ok().and_then(|o| o["created_at_ms"].as_i64()).unwrap_or(input.sold_at_ms);
        // EVERY OFFLINE SALE OWES A DOCUMENT (the receipt promised it), whether
        // or not the venue set `fiscal.since_ms`; a replay finds its own entry.
        let fiscal = match self.enqueue_fiscal_with(Config::From(0), &d.stored, sold_at).await {
            Ok(AtPlacement::Queued(_)) => "queued".to_string(),
            Ok(AtPlacement::Refused(r)) => format!("refused: {r:?}"),
            Ok(_) => "not_owed".to_string(),
            Err(e) => {
                log_error!("fiscal: offline sale {} was written and its document was NOT queued: {e}", input.order_id);
                format!("error: {e}")
            }
        };
        // Loud, never fatal: the sale is written; a lost marker only narrows the rotation guard.
        if let Err(e) = self.offline_mark_seen(&input.order_id, sold_at).await {
            log_error!("offline sale {}: its key was not kept beside the fiscal queue: {e}", input.order_id);
        }
        Ok(Ok(SyncOut {
            order_id: input.order_id.clone(),
            replayed: d.replayed,
            sold_at_ms: sold_at,
            fiscal_deadline_ms: crate::services::orders::offline_sale::rules::deadline(sold_at),
            fiscal,
            conflicts: d.conflicts,
        }))
    }

    /// The owner's pane, from the orders and the fiscal image in memory.
    async fn offline_list(&self, now_ms: i64) -> Result<Value> {
        let (_, listed) = self.orders_view().await?;
        let orders: Vec<Value> = listed
            .iter()
            .filter(|o| o.order_id.starts_with(crate::services::orders::offline_sale::PREFIX))
            .filter_map(|o| serde_json::from_str(&o.order_json).ok())
            .collect();
        let entries = self.fiscal_entries().await?;
        let (_, outbox) = self.offline_outbox().await?;
        let marked = |id: &str| outbox.get(overdue::MARK_KIND, id).is_some();
        let pane = ledger::pane(&orders, &entries, &marked, now_ms, crate::fiscal::SEND_ENABLED);
        serde_json::to_value(&pane).map_err(bad)
    }

    /// When the next overdue alert is owed (`hubdo/timer.rs` arms the alarm for it).
    pub(in crate::hubdo) async fn offline_overdue_next(&self) -> Result<Option<i64>> {
        let entries = self.fiscal_entries().await?;
        if overdue::offline(&entries).is_empty() {
            return Ok(None);
        }
        let (_, outbox) = self.offline_outbox().await?;
        Ok(overdue::next_alert(&entries, &|id| outbox.get(overdue::MARK_KIND, id).is_some()))
    }

    /// Queue the overdue alert this alarm owes, with its markers, in ONE outbox
    /// write. Never fails the alarm: it is said out loud instead.
    pub(in crate::hubdo) async fn offline_overdue(&self, venue: &str, now_ms: i64) {
        if let Err(e) = self.offline_overdue_turn(venue, now_ms).await {
            log_error!("offline sales: the overdue alert for {venue} was not queued: {e}");
        }
    }

    async fn offline_overdue_turn(&self, venue: &str, now_ms: i64) -> Result<()> {
        let entries = self.fiscal_entries().await?;
        if overdue::offline(&entries).is_empty() {
            return Ok(());
        }
        let settings = match self.image(crate::hubstore::IMAGE_SETTINGS).await? {
            Some((_, b)) => Some(dowiz_hub::settings::Settings::load(&b).map_err(|_| bad("the settings image is unreadable"))?),
            None => None,
        };
        let chat = settings.as_ref().map(crate::notify::route::alert_target).unwrap_or_default();
        let record: Option<Value> = match self.cat_location().await? {
            Some(l) => l.ok().flatten().and_then(|j| serde_json::from_str(&j).ok()),
            None => None,
        };
        let lang = record.as_ref().and_then(|r| r.get("default_locale")).and_then(Value::as_str).unwrap_or("en").to_string();
        let voice = Voice { venue, zone: crate::hubstore::zone_of(record.as_ref()), lang: &lang };
        let (generation, mut table) = self.offline_outbox().await?;
        let (due, marks) = overdue::due(&entries, &|id| table.get(overdue::MARK_KIND, id).is_some(), now_ms, &voice, &chat);
        if marks.is_empty() {
            return Ok(());
        }
        for m in &marks {
            table.put(overdue::MARK_KIND, m, &now_ms.to_string(), &[], &[]).map_err(|x| bad(format!("outbox: {x:?}")))?;
        }
        for e in &due {
            let rec = serde_json::to_string(e).map_err(bad)?;
            table.put(KIND, &e.id, &rec, &[], &[]).map_err(|x| bad(format!("outbox: {x:?}")))?;
        }
        let bytes = table.to_bytes().map_err(|x| bad(format!("outbox will not serialise: {x:?}")))?;
        if self.put_image(IMAGE_OUTBOX, generation, &bytes).await?.is_none() {
            return Err(bad("the outbox generation moved"));
        }
        Ok(())
    }
}
