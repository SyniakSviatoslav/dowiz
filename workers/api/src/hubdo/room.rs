//! THE ROOM'S COMMANDS, executed by the venue's object in one turn each.
//!
//! The same shape as `place`, `advance` and `assign` in the parent module, and
//! for the same reason: the object holds every image in memory, so each command
//! runs its pure `decide` against copies and writes only once all of it has
//! succeeded. A refusal drops the copies and nothing was ever persisted.
//!
//! A CHILD MODULE of `hubdo` so it reads the object's private state the way the
//! parent's commands do, and so the parent file does not grow by a room.
//! The till's commands are its own child, `room/till.rs`, for the same reason.

use super::{HubImages, OrderView, LOG_IMAGE};
use crate::command::till::Cmd;
use crate::command::Refused;
use serde::Serialize;
use worker::*;

mod till;

/// A command's answer on the wire: its value, or its refusal with its status.
fn reply<T: Serialize>(r: std::result::Result<T, Refused>) -> Result<Response> {
    match r {
        Ok(v) => Response::from_json(&v),
        Err(r) => Response::error(r.message().to_string(), r.status()),
    }
}

impl HubImages {
    /// `POST /fold/room/<what>` — the Worker has authenticated the signer and
    /// checked the capability; what arrives here is that decision.
    pub(super) async fn room(&self, what: &str, mut req: Request) -> Result<Response> {
        match what {
            "amend" => reply(self.amend(req.json().await?).await?),
            "pay" => reply(self.pay(req.json().await?).await?),
            "transfer" => reply(self.transfer(req.json().await?).await?),
            "move_sitting" => reply(self.move_sitting(req.json().await?).await?),
            // THE TILL (`room/till.rs`). One segment each: `/fold/room/till_open`.
            "till_open" => reply(self.till(Cmd::Open(req.json().await?)).await?),
            "till_pay_in" => reply(self.till(Cmd::PayIn(req.json().await?)).await?),
            "till_pay_out" => reply(self.till(Cmd::PayOut(req.json().await?)).await?),
            "till_count" => reply(self.till(Cmd::Count(req.json().await?)).await?),
            "till_close" => reply(self.till(Cmd::Close(req.json().await?)).await?),
            "till_report" => reply(self.till_report(req.json().await?).await?),
            _ => Response::error("no such room command", 404),
        }
    }

    /// The log, as a hub, and the generation it was read at.
    pub(super) async fn log_hub(&self) -> Result<(i64, dowiz_hub::Hub)> {
        Ok(match self.image(LOG_IMAGE).await? {
            Some((meta, bytes)) => (
                meta.generation,
                dowiz_hub::Hub::load(&bytes).map_err(|_| Error::RustError("hub image is unreadable".into()))?,
            ),
            None => (0, dowiz_hub::Hub::create_sized(64 * 1024).map_err(|_| Error::RustError("cannot create hub image".into()))?),
        })
    }

    /// The wallet ledger's log (`wallet::IMAGE_LEDGER`), and its generation.
    async fn ledger_log(&self) -> Result<(i64, dowiz_hub::logimage::LogImage)> {
        Ok(match self.image(crate::wallet::IMAGE_LEDGER).await? {
            Some((meta, bytes)) => (
                meta.generation,
                dowiz_hub::logimage::LogImage::load(&bytes).map_err(|_| Error::RustError("ledger image is unreadable".into()))?,
            ),
            None => (0, dowiz_hub::logimage::LogImage::create().map_err(|_| Error::RustError("cannot create ledger image".into()))?),
        })
    }

    /// The stock ledger's log, and the generation it was read at.
    pub(super) async fn stock_log(&self) -> Result<(i64, dowiz_hub::stock::StockLog)> {
        Ok(match self.image(crate::hubstore::IMAGE_STOCK).await? {
            Some((meta, bytes)) => (
                meta.generation,
                dowiz_hub::stock::StockLog::load(&bytes).map_err(|_| Error::RustError("stock image is unreadable".into()))?,
            ),
            None => (0, dowiz_hub::stock::StockLog::create_sized(64 * 1024).map_err(|_| Error::RustError("cannot create stock image".into()))?),
        })
    }

    /// Write the log, then the stock if it moved — THE LOG FIRST, for the
    /// reason `place` gives: a lost second write then leaves an order whose
    /// shelf is wrong, which the audit sees, never a shelf with no order.
    pub(super) async fn write_both(
        &self,
        what: &str,
        log_gen: i64,
        hub: &dowiz_hub::Hub,
        stock: Option<(i64, &dowiz_hub::stock::StockLog)>,
    ) -> Result<std::result::Result<i64, Refused>> {
        let Some(next) = self.put_image(LOG_IMAGE, log_gen, &hub.to_bytes_trimmed()).await? else {
            return Ok(Err(Refused::Append(format!("the log generation moved during {what}"))));
        };
        if let Some((gen, s)) = stock {
            if self.put_image(crate::hubstore::IMAGE_STOCK, gen, &s.to_bytes_trimmed()).await?.is_none() {
                console_error!("stock: {what} was written and its shelf was NOT");
            }
        }
        Ok(Ok(next))
    }

    /// AMEND A ROUND: the delta, the shelf, the broadcast, in one turn.
    async fn amend(
        &self,
        input: crate::command::amend::AmendIn,
    ) -> Result<std::result::Result<crate::command::amend::AmendOut, Refused>> {
        let (log_gen, listed) = self.orders_view().await?;
        let current: Option<OrderView> = listed.into_iter().find(|o| o.order_id == input.order_id);
        let (_, mut hub) = self.log_hub().await?;
        let (stock_gen, mut stock) = self.stock_log().await?;
        let before = stock.len();
        let (round, body, seq) = match crate::command::amend::decide(&mut hub, &mut stock, current.as_ref(), &input) {
            Ok(v) => v,
            Err(r) => return Ok(Err(r)),
        };
        let moved = (stock.len() != before).then_some((stock_gen, &stock));
        let next = match self.write_both("an amendment", log_gen, &hub, moved).await? {
            Ok(n) => n,
            Err(r) => return Ok(Err(r)),
        };
        self.broadcast(dowiz_hub::EventKind::Amended as u8, &input.order_id, &body, next);
        // THE EXCEPTION ALERT (P1-5): a void after the kitchen, a comp and a
        // late amendment are all AMENDMENTS, and until 2026-09-24 only the
        // refund and the till asked -- so the three kinds the alert exists
        // for never reached the owner's chat. Never fails the amendment.
        self.exceptions_after(&input.location_id, input.now_ms).await;
        // AN ADDED LINE IS RUNG AT ITS STATION (§2.7), in the turn that wrote it.
        // A failure to queue is loud and does not undo the amendment.
        let added = crate::bell_route::added_lines(&input.ops);
        if !added.is_empty() {
            let head = crate::bell_route::amend_header(&input.order_id, &round);
            if let Err(e) = self.enqueue_bell(&input.order_id, &head, &added, Some(seq), input.now_ms).await {
                console_error!("outbox: order {} was amended and the bell was NOT queued: {e}", input.order_id);
            }
        }
        Ok(Ok(crate::command::amend::AmendOut { merged: round.to_string(), seq, generation: next }))
    }

    /// TAKE A PAYMENT: one `Paid` event, the broadcast, in one turn. The
    /// till is read for cash only; the venue's currency always, because an
    /// order that does not name its own is in it.
    async fn pay(
        &self,
        input: crate::command::pay::PayIn,
    ) -> Result<std::result::Result<crate::command::pay::PayOut, Refused>> {
        let (log_gen, listed) = self.orders_view().await?;
        let current: Option<OrderView> = listed.into_iter().find(|o| o.order_id == input.order_id);
        let (_, mut hub) = self.log_hub().await?;
        let open = if input.method == "cash" {
            match self.till_state().await? {
                Ok((_, _, periods)) => periods.last().filter(|p| p.closed_at.is_none()).map(|p| p.till_id.clone()),
                Err(r) => return Ok(Err(r)),
            }
        } else {
            None
        };
        let venue_currency = self.venue_currency().await?;
        let room = crate::command::pay::Room { open_till: open.as_deref(), venue_currency: &venue_currency };
        // A WALLET TENDER reads the ledger image in the same turn and refuses a
        // debit its balance does not cover (`command::pay::wallet`).
        let ledger = if input.method == "wallet" { Some(self.ledger_log().await?) } else { None };
        let rows: Vec<String> = ledger.as_ref().map_or_else(Vec::new, |(_, log)| {
            let mut es = log.about(crate::wallet::K_TX, None, usize::MAX);
            es.reverse();
            es.into_iter().map(|e| e.json).collect()
        });
        let (round, body, seq, debit) =
            match crate::command::pay::wallet::pay(&mut hub, current.as_ref(), &input, &room, &rows) {
                Ok(v) => v,
                Err(r) => return Ok(Err(r)),
            };
        let next = match self.write_both("a payment", log_gen, &hub, None).await? {
            Ok(n) => n,
            Err(r) => return Ok(Err(r)),
        };
        // THE LOG FIRST, then the wallet's leg, for `write_both`'s reason; a
        // lost leg is a `Paid` naming a wallet with no debit, which the ledger
        // statement and this console line both show.
        if let (Some(d), Some((gen, mut log))) = (debit, ledger) {
            let wrote = match log.append(crate::wallet::K_TX, &d.tx_id, &d.record) {
                Ok(()) => self.put_image(crate::wallet::IMAGE_LEDGER, gen, &log.to_bytes()).await?.is_some(),
                Err(_) => false,
            };
            if !wrote {
                self.retry_leg(&round, &input, &venue_currency, &d.tx_id).await?;
            }
        }
        self.broadcast(dowiz_hub::EventKind::Paid as u8, &input.order_id, &body, next);
        // A payment can be an exception row (cash outside a till) or a
        // wallet-leg finding (law 12, told on first sight): same hook.
        self.exceptions_after(&input.location_id, input.now_ms).await;
        Ok(Ok(crate::command::pay::PayOut { merged: round.to_string(), seq, generation: next }))
    }

    /// A WALLET LEG THAT LOST ITS WRITE (the ledger moved between our read
    /// and our put: a top-up, or an owner's repair). The `Paid` is already on
    /// the log, so the leg is decided ONCE MORE against the ledger as it is
    /// now, by law 12's own repair (`command::pay::legs::repair`): the id is
    /// derived, so a leg someone else already wrote is found and nothing is
    /// written; a leg the wallet no longer covers is refused, never posted.
    /// Only a retry that fails, or a refusal, is left for the owner's audit
    /// (`GET /api/owner/wallet/legs`), with the console line naming it.
    async fn retry_leg(&self, round: &serde_json::Value, input: &crate::command::pay::PayIn, currency: &str, tx_id: &str) -> Result<()> {
        let (gen, mut log) = self.ledger_log().await?;
        let mut rows: Vec<String> = log.about(crate::wallet::K_TX, None, usize::MAX).into_iter().map(|e| e.json).collect();
        rows.reverse();
        let plan = match crate::command::pay::legs::repair(std::slice::from_ref(round), &rows, &input.location_id, currency) {
            Ok(p) => p,
            Err(r) => {
                console_error!("wallet: order {} was paid and its debit {tx_id} was NOT written ({})", input.order_id, r.message());
                return Ok(());
            }
        };
        for (leg, why) in &plan.refused {
            console_error!("wallet: order {} was paid and its debit {} is REFUSED on retry: {why}", leg.order_id, leg.tx_id);
        }
        if plan.write.is_empty() {
            return Ok(());
        }
        let appended = plan.write.iter().all(|d| log.append(crate::wallet::K_TX, &d.tx_id, &d.record).is_ok());
        if !appended || self.put_image(crate::wallet::IMAGE_LEDGER, gen, &log.to_bytes()).await?.is_none() {
            console_error!("wallet: order {} was paid and its debit {tx_id} was NOT written after one retry", input.order_id);
        }
        Ok(())
    }

    /// TRANSFER LINES BETWEEN ROUNDS (§2.9): both rounds' deltas, the shelf
    /// following the lines, and both broadcasts, in one turn.
    async fn transfer(
        &self,
        input: crate::command::transfer::TransferIn,
    ) -> Result<std::result::Result<crate::command::transfer::TransferOut, Refused>> {
        let (log_gen, listed) = self.orders_view().await?;
        let from = listed.iter().find(|o| o.order_id == input.from_order_id);
        let to = listed.iter().find(|o| o.order_id == input.to_order_id);
        let (_, mut hub) = self.log_hub().await?;
        let (stock_gen, mut stock) = self.stock_log().await?;
        let before = stock.len();
        let d = match crate::command::transfer::decide(&mut hub, &mut stock, from, to, &input) {
            Ok(v) => v,
            Err(r) => return Ok(Err(r)),
        };
        let moved = (stock.len() != before).then_some((stock_gen, &stock));
        let next = match self.write_both("a transfer", log_gen, &hub, moved).await? {
            Ok(n) => n,
            Err(r) => return Ok(Err(r)),
        };
        self.broadcast(dowiz_hub::EventKind::Amended as u8, &input.from_order_id, &d.from_body, next);
        self.broadcast(dowiz_hub::EventKind::Amended as u8, &input.to_order_id, &d.to_body, next);
        Ok(Ok(crate::command::transfer::TransferOut {
            from_merged: d.moved.from.to_string(),
            from_seq: d.from_seq,
            to_merged: d.moved.to.to_string(),
            to_seq: d.to_seq,
            generation: next,
        }))
    }

    /// MOVE A SITTING TO ANOTHER TABLE (§2.9): one `Amended` per round still
    /// in the room, one write, one broadcast each. The shelf is not read.
    async fn move_sitting(
        &self,
        input: crate::command::transfer::sitting::MoveIn,
    ) -> Result<std::result::Result<crate::command::transfer::sitting::MoveOut, Refused>> {
        let (log_gen, listed) = self.orders_view().await?;
        let (_, mut hub) = self.log_hub().await?;
        let moved = match crate::command::transfer::sitting::decide(&mut hub, &listed, &input) {
            Ok(v) => v,
            Err(r) => return Ok(Err(r)),
        };
        let next = match self.write_both("a table move", log_gen, &hub, None).await? {
            Ok(n) => n,
            Err(r) => return Ok(Err(r)),
        };
        for m in &moved {
            self.broadcast(dowiz_hub::EventKind::Amended as u8, &m.order_id, &m.body, next);
        }
        Ok(Ok(crate::command::transfer::sitting::MoveOut {
            sitting_id: input.sitting_id,
            table: input.table.trim().to_string(),
            moved: moved.into_iter().map(|m| (m.order_id, m.seq)).collect(),
            generation: next,
        }))
    }
}
