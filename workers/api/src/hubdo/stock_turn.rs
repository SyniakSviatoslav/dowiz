//! ONE STOCK MOVEMENT, IN THE VENUE'S OBJECT, IN ONE TURN (W0a): the ledger
//! decided in memory (`stock/turn.rs`), the stock image written, then -- in
//! the same turn -- what the movement tells the Telegram groups, into the
//! `outbox` (`routed.rs`). A refusal writes nothing at all.
//!
//! `stock.expiring` is told on the first movement of the venue's day: the
//! day is marked in the outbox (`produce::EXPIRING_MARK`) and yesterday's
//! mark is dropped, so it is one message a day at most, and none to a venue
//! whose groups do not hear it.

use super::routed::Owed;
use super::HubImages;
use crate::notify::route::produce;
use crate::services::operations::stock::turn::{self, StockTurnIn};
use worker::*;
// The plain-Rust request/response (W-COV C2): these bodies run under `cargo test`.
use crate::wire::Reply as Response;

impl HubImages {
    /// THE OWNER'S INGREDIENTS RESET (`/fold/stock_reset`): the stock image is
    /// replaced by an empty ledger, under the generation guard like any other
    /// write. Answers how many records the old ledger held.
    pub(super) async fn stock_reset(&self) -> Result<Response> {
        let (gen, old) = self.stock_log().await?;
        let dropped = old.len();
        let fresh = dowiz_hub::stock::StockLog::create_sized(64 * 1024)
            .map_err(|_| Error::RustError("cannot create stock image".into()))?;
        if self.put_image(crate::hubstore::IMAGE_STOCK, gen, &fresh.to_bytes_trimmed()).await?.is_none() {
            return Response::error("the stock generation moved during the reset", 409);
        }
        Response::from_json(&serde_json::json!({ "dropped": dropped }))
    }

    pub(super) async fn stock_move(&self, input: StockTurnIn) -> Result<Response> {
        // THE CATALOGUE'S PART IS READ HERE (BN1): the supplies, the venue's
        // day and its currency come from the image this object holds, never
        // across the hop (`turn::from_catalogue`).
        let cat = self.catalogue().await?; // decoded ONCE: the expiry forecast below reads it too (W-LOOPB)
        let input = input.from_catalogue(&cat);
        let (gen, mut log) = self.stock_log().await?;
        let before = log.len();
        let routing = self.routing_groups().await.map(|(_, g)| g);
        let day = input.today.to_string();
        let expiring_marks = match &routing {
            Some(g) if produce::wants(g, "stock.expiring") => Some(self.marks(produce::EXPIRING_MARK).await),
            _ => None,
        };
        let due = expiring_marks.as_ref().is_some_and(|m| !m.contains(&day));
        let (shown, mut told) = match turn::run(&mut log, &input, due) {
            Ok(v) => v,
            // NOTHING HAS BEEN WRITTEN: the log copy is dropped here.
            Err((status, said)) => return Response::error(said, status),
        };
        if log.len() != before && self.put_derived(crate::hubstore::IMAGE_STOCK, gen, &log.to_bytes_trimmed()).await?.is_none() {
            return Response::error("the stock generation moved during a movement", 409);
        }
        // P7 (W-PREP): what the forecast will not use before each lot's date.
        self.expiring_surplus(&mut told, &log, &input, &cat).await;
        if let Some(groups) = routing {
            let base = format!("stock/{}", log.len());
            let heard: Vec<_> = told.into_iter().filter(|(ev, _)| produce::wants(&groups, ev)).collect();
            let mut owed = Owed { entries: produce::entries(&base, heard, input.now_ms), ..Owed::default() };
            if let Some(marks) = expiring_marks.filter(|_| due) {
                owed.mark.push((produce::EXPIRING_MARK, day.clone()));
                owed.unmark.extend(marks.into_iter().filter(|m| *m != day).map(|m| (produce::EXPIRING_MARK, m)));
            }
            self.route_owed(owed, "a stock movement").await;
        }
        Response::from_json(&shown)
    }
}
