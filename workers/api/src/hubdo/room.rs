//! THE ROOM'S COMMANDS, executed by the venue's object in one turn each.
//!
//! The same shape as `place`, `advance` and `assign` in the parent module, and
//! for the same reason: the object holds every image in memory, so each command
//! runs its pure `decide` against copies and writes only once all of it has
//! succeeded. A refusal drops the copies and nothing was ever persisted.
//!
//! A CHILD MODULE of `hubdo` so it reads the object's private state the way the
//! parent's commands do, and so the parent file does not grow by a room.

use super::{HubImages, OrderView, LOG_IMAGE};
use crate::command::Refused;
use serde::Serialize;
use worker::*;

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
        Ok(Ok(crate::command::amend::AmendOut { merged: round.to_string(), seq, generation: next }))
    }

    /// TAKE A PAYMENT: one `Paid` event, the broadcast, in one turn.
    async fn pay(
        &self,
        input: crate::command::pay::PayIn,
    ) -> Result<std::result::Result<crate::command::pay::PayOut, Refused>> {
        let (log_gen, listed) = self.orders_view().await?;
        let current: Option<OrderView> = listed.into_iter().find(|o| o.order_id == input.order_id);
        let (_, mut hub) = self.log_hub().await?;
        let (round, body, seq) = match crate::command::pay::decide(&mut hub, current.as_ref(), &input) {
            Ok(v) => v,
            Err(r) => return Ok(Err(r)),
        };
        let next = match self.write_both("a payment", log_gen, &hub, None).await? {
            Ok(n) => n,
            Err(r) => return Ok(Err(r)),
        };
        self.broadcast(dowiz_hub::EventKind::Paid as u8, &input.order_id, &body, next);
        Ok(Ok(crate::command::pay::PayOut { merged: round.to_string(), seq, generation: next }))
    }
}
