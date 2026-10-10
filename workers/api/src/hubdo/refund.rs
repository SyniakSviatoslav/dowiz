//! THE REFUND COMMAND, in the venue's object, in one turn: the order's events
//! and the shelf's release decided in memory (`command::refund::decide`),
//! then written log first, stock second, then broadcast.

use super::{HubImages, OrderView};
use crate::command::refund::returned::{ReturnedIn, ReturnedOut};
use crate::command::refund::{RefundIn, RefundOut};
use crate::command::Refused;
use worker::*;

impl HubImages {
    pub(super) async fn refund(
        &self,
        input: RefundIn,
        claim: Option<crate::idempotency::commit::Claim>,
    ) -> Result<std::result::Result<RefundOut, Refused>> {
        let (log_gen, listed) = self.orders_view().await?;
        let current: Option<OrderView> = listed.into_iter().find(|o| o.order_id == input.order_id);
        let (_, mut hub) = self.log_hub().await?;
        let (stock_gen, mut stock) = self.stock_log_at(input.now_ms).await?;
        let venue_currency = self.venue_currency().await?;
        let before = stock.len();
        // THE CARD'S SHARE (W-REFUND): planned in the same turn; a Stripe key
        // on this Worker (read by the route, `RefundIn::stripe_on`) is what
        // turns a card refund from by-hand into queued.
        let stripe_on = input.stripe_on;
        let (merged, written, job) = match crate::command::refund::card::decide_card(&mut hub, &mut stock, current.as_ref(), &input, &venue_currency, stripe_on) {
            Ok(v) => v,
            // NOTHING HAS BEEN WRITTEN.
            Err(r) => return Ok(Err(r)),
        };
        let moved = (stock.len() != before).then_some((stock_gen, &stock));
        let next = match self.write_both("a refund", log_gen, &hub, moved).await? {
            Ok(n) => n,
            Err(r) => return Ok(Err(r)),
        };
        // THE REFUND IS IN THE LOG: its claim is marked committed now (W-O2).
        let seq = written.last().map_or(0, |w| w.2);
        let out = RefundOut { merged: merged.to_string(), seq, generation: next };
        self.commit_claim(claim.as_ref(), &serde_json::to_string(&out).unwrap_or_default()).await;
        for (kind, body, _) in &written {
            self.broadcast(*kind as u8, &input.order_id, body, next);
        }
        // THE CARD REFUND IS OWED TO STRIPE: into the outbox beside the event
        // (the drain sends it, `stripe/refund_io.rs`). A queue write that
        // fails is said out loud; the order shows the attempt `queued` and the
        // owner's "try again" is refused only while it is in flight.
        if let Some(job) = &job {
            if let Err(e) = self.enqueue_card(job, input.now_ms).await {
                log_error!("refund: order {} card refund {} was recorded and NOT queued: {e}", input.order_id, job.key);
            }
        }
        // D12 (G5): THE WALLET'S SHARE GOES BACK when the refund is completed.
        // The log first, then the ledger, for `write_both`'s reason (as `pay`
        // writes its leg). A lost reversal is named on the console, and the
        // law-12 repair writes it (`refund::wallet::hand_back`, W-FIX O3).
        if input.complete {
            self.hand_back_wallets(&merged, &input).await?;
        }
        // THE EXCEPTION ALERT (P1-5): a refund is an exception row; the
        // alert is evidence about it and never fails the refund.
        self.exceptions_after(&input.location_id, input.now_ms).await;
        Ok(Ok(out))
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
                log_error!("wallet: order {} was refunded and its wallet was NOT credited ({})", input.order_id, r.message());
                return Ok(());
            }
        };
        if back.is_empty() {
            return Ok(());
        }
        let appended = back.iter().all(|d| log.append(crate::wallet::K_TX, &d.tx_id, &d.record).is_ok());
        if !appended || self.put_derived(crate::wallet::IMAGE_LEDGER, gen, &log.to_bytes()).await?.is_none() {
            log_error!("wallet: order {} was refunded and its wallet credit was NOT written", input.order_id);
        }
        Ok(())
    }

    /// One card refund into the venue's outbox; the write arms the alarm.
    async fn enqueue_card(&self, job: &crate::stripe::refund::Job, now_ms: i64) -> Result<()> {
        use crate::outbox::{IMAGE_OUTBOX, KIND, OUTBOX_BYTES};
        let (generation, mut table) = match self.image(IMAGE_OUTBOX).await? {
            Some((meta, b)) => (meta.generation, dowiz_hub::table::Table::load(&b, OUTBOX_BYTES).map_err(|_| Error::RustError("outbox image is unreadable".into()))?),
            None => (0, dowiz_hub::table::Table::create(OUTBOX_BYTES).map_err(|_| Error::RustError("cannot create outbox image".into()))?),
        };
        let e = crate::stripe::refund::entry(job, now_ms);
        let rec = serde_json::to_string(&e).map_err(|x| Error::RustError(x.to_string()))?;
        table.put(KIND, &e.id, &rec, &[], &[]).map_err(|x| Error::RustError(format!("outbox: {x:?}")))?;
        let bytes = table.to_bytes().map_err(|x| Error::RustError(format!("outbox will not serialise: {x:?}")))?;
        if self.put_image(IMAGE_OUTBOX, generation, &bytes).await?.is_none() {
            return Err(Error::RustError("the outbox generation moved".into()));
        }
        Ok(())
    }

    /// THE FOOD THAT CAME BACK: one choice per order, written to the stock log
    /// only. The marker is the `Returned` events themselves.
    pub(super) async fn returned(&self, input: ReturnedIn) -> Result<std::result::Result<ReturnedOut, Refused>> {
        let (_, listed) = self.orders_view().await?;
        let current: Option<OrderView> = listed.into_iter().find(|o| o.order_id == input.order_id);
        let (stock_gen, mut stock) = self.stock_log_at(input.now_ms).await?;
        let out = match crate::command::refund::returned::decide(&mut stock, current.as_ref(), &input) {
            Ok(v) => v,
            // NOTHING HAS BEEN WRITTEN.
            Err(r) => return Ok(Err(r)),
        };
        if self.put_derived(crate::hubstore::IMAGE_STOCK, stock_gen, &stock.to_bytes_trimmed()).await?.is_none() {
            return Ok(Err(Refused::Append("the stock generation moved during the returned-food choice".into())));
        }
        Ok(Ok(out))
    }
}

/// P3: every stock write site in the object's turns is clocked.
#[cfg(test)]
#[path = "refund/clock_tests.rs"]
mod clock_tests;
