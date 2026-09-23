//! THE REFUND COMMAND, in the venue's object, in one turn: the order's events
//! and the shelf's release decided in memory (`command::refund::decide`),
//! then written log first, stock second, then broadcast.

use super::{HubImages, OrderView};
use crate::command::refund::returned::{ReturnedIn, ReturnedOut};
use crate::command::refund::{RefundIn, RefundOut};
use crate::command::Refused;
use worker::*;

impl HubImages {
    pub(super) async fn refund(&self, input: RefundIn) -> Result<std::result::Result<RefundOut, Refused>> {
        let (log_gen, listed) = self.orders_view().await?;
        let current: Option<OrderView> = listed.into_iter().find(|o| o.order_id == input.order_id);
        let (_, mut hub) = self.log_hub().await?;
        let (stock_gen, mut stock) = self.stock_log().await?;
        let before = stock.len();
        let (merged, written) = match crate::command::refund::decide(&mut hub, &mut stock, current.as_ref(), &input) {
            Ok(v) => v,
            // NOTHING HAS BEEN WRITTEN.
            Err(r) => return Ok(Err(r)),
        };
        let moved = (stock.len() != before).then_some((stock_gen, &stock));
        let next = match self.write_both("a refund", log_gen, &hub, moved).await? {
            Ok(n) => n,
            Err(r) => return Ok(Err(r)),
        };
        for (kind, body, _) in &written {
            self.broadcast(*kind as u8, &input.order_id, body, next);
        }
        let seq = written.last().map_or(0, |w| w.2);
        Ok(Ok(RefundOut { merged: merged.to_string(), seq, generation: next }))
    }

    /// THE FOOD THAT CAME BACK: one choice per order, written to the stock log
    /// only. The marker is the `Returned` events themselves.
    pub(super) async fn returned(&self, input: ReturnedIn) -> Result<std::result::Result<ReturnedOut, Refused>> {
        let (_, listed) = self.orders_view().await?;
        let current: Option<OrderView> = listed.into_iter().find(|o| o.order_id == input.order_id);
        let (stock_gen, mut stock) = self.stock_log().await?;
        let out = match crate::command::refund::returned::decide(&mut stock, current.as_ref(), &input) {
            Ok(v) => v,
            // NOTHING HAS BEEN WRITTEN.
            Err(r) => return Ok(Err(r)),
        };
        if self.put_image(crate::hubstore::IMAGE_STOCK, stock_gen, &stock.to_bytes_trimmed()).await?.is_none() {
            return Ok(Err(Refused::Append("the stock generation moved during the returned-food choice".into())));
        }
        Ok(Ok(out))
    }
}
