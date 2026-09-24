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
        let venue_currency = self.venue_currency().await?;
        let before = stock.len();
        let (merged, written) = match crate::command::refund::decide(&mut hub, &mut stock, current.as_ref(), &input, &venue_currency) {
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
        // D12 (G5): THE WALLET'S SHARE GOES BACK when the refund is completed.
        // The log first, then the ledger, for `write_both`'s reason (as `pay`
        // writes its leg). A lost reversal is named on the console; nothing
        // re-attempts it yet (OPEN in G5's verdict).
        if input.complete {
            self.hand_back_wallets(&merged, &input).await?;
        }
        // THE EXCEPTION ALERT (P1-5): a refund is an exception row; the
        // alert is evidence about it and never fails the refund.
        self.exceptions_after(&input.location_id, input.now_ms).await;
        let seq = written.last().map_or(0, |w| w.2);
        Ok(Ok(RefundOut { merged: merged.to_string(), seq, generation: next }))
    }

    /// Append the REFUND records for every wallet payment on the round
    /// (`command::refund::wallet::reversals`). Never fails the refund.
    async fn hand_back_wallets(&self, order: &serde_json::Value, input: &RefundIn) -> Result<()> {
        if !order.get("payments").and_then(serde_json::Value::as_array).into_iter().flatten()
            .any(|p| p.get("method").and_then(serde_json::Value::as_str) == Some("wallet"))
        {
            return Ok(());
        }
        let (gen, mut log) = self.ledger_log().await?;
        let mut rows: Vec<String> = log.about(crate::wallet::K_TX, None, usize::MAX).into_iter().map(|e| e.json).collect();
        rows.reverse();
        let back = match crate::command::refund::wallet::reversals(order, &input.location_id, &rows, input.now_ms) {
            Ok(b) => b,
            Err(r) => {
                console_error!("wallet: order {} was refunded and its wallet was NOT credited ({})", input.order_id, r.message());
                return Ok(());
            }
        };
        if back.is_empty() {
            return Ok(());
        }
        let appended = back.iter().all(|d| log.append(crate::wallet::K_TX, &d.tx_id, &d.record).is_ok());
        if !appended || self.put_image(crate::wallet::IMAGE_LEDGER, gen, &log.to_bytes()).await?.is_none() {
            console_error!("wallet: order {} was refunded and its wallet credit was NOT written", input.order_id);
        }
        Ok(())
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
