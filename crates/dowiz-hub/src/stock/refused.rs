//! LOST SALES (research 2026-10-03 P8, row A13, W-LOST): every basket the
//! shelf refused at checkout leaves ONE `refused` record in the stock chain,
//! so the kitchen's numbers can say what the stock-outs cost.
//!
//! A NOTE, NOT AN EVENT. The record is a note of kind [`REFUSED`]
//! (`notes.rs`): `decode` answers `None` for it, so the ledger, the journal,
//! the cost book, the lots, the carry, the storages and every checkpoint walk
//! past it. It cannot move a shelf number by construction, and a Worker
//! rolled back past this file reads a log WITH refusals as the shelf it
//! always was (`refused/tests.rs` holds both, against the P12 golden).
//!
//! NO PERSON. The only writer is [`StockLog::append_lost`] and it takes a
//! [`LostSale`]: the supply that ran out, the dish, portions, the dish's
//! unit price -- no phone, no order id, no guest, no name. A personal field
//! cannot be written because the type has nowhere to put one, and the
//! record's keys are pinned by a test.
//!
//! RATE-LIMITED: one per supply per [`WINDOW_MS`]. A guest who retries a
//! refused basket five times is one lost sale, not five; so the count is a
//! LOWER BOUND, said in the screen's own words.

use super::notes::{at_of, is_note};
use super::{StockError, StockLog};
use crate::minijson::{esc, int_field, str_field};
use bebop_store::evlog::EvLog;

/// The note kind of a refused basket.
pub const REFUSED: &str = "refused";
/// One record per supply per this many milliseconds (10 minutes).
pub const WINDOW_MS: i64 = 10 * 60 * 1000;
/// The rate check reads at most this many newest records: ten minutes of a
/// busy kitchen is far fewer, and an old log without clocks is not walked whole.
pub const SCAN_MAX: usize = 4096;

/// What one refusal records. THESE FOUR FIELDS AND NOTHING ELSE.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LostSale {
    /// The supply the shelf did not have.
    pub item: String,
    /// The dish (catalogue product id) whose line needed it.
    pub dish: String,
    /// Portions of that dish in the refused basket.
    pub qty: i64,
    /// The dish's unit price then, integer minor units (lek).
    pub price: i64,
}

/// A refusal read back, with its clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LostRow {
    pub sale: LostSale,
    pub at: i64,
}

impl StockLog {
    /// Record one refused basket, unless `sale.item` already has a refusal in
    /// the last [`WINDOW_MS`]. `Ok(true)`: written; `Ok(false)`: rate-limited.
    /// Refused, nothing written: no request clock (a refusal with no time
    /// cannot be limited or put on a day), an empty item or dish, a quantity
    /// or price that is not one.
    pub fn append_lost(&mut self, sale: &LostSale) -> Result<bool, StockError> {
        let now = self.clock.ok_or(StockError::Malformed)?;
        if sale.item.trim().is_empty() || sale.dish.trim().is_empty() {
            return Err(StockError::Malformed);
        }
        if sale.qty <= 0 {
            return Err(StockError::NotPositive { qty: sale.qty });
        }
        if sale.price < 0 {
            return Err(StockError::NotPositive { qty: sale.price });
        }
        if self.refused_within(&sale.item, now) {
            return Ok(false);
        }
        let body = format!(
            r#"{{"item":"{}","dish":"{}","qty":{},"price":{}}}"#,
            esc(&sale.item),
            esc(&sale.dish),
            sale.qty,
            sale.price
        );
        self.append_note(REFUSED, &body)?;
        Ok(true)
    }

    /// Has `item` a refusal less than [`WINDOW_MS`] before `now`? Walks newest
    /// first and stops at the first record older than the window.
    fn refused_within(&self, item: &str, now: i64) -> bool {
        let floor = now.saturating_sub(WINDOW_MS);
        let mut seen = 0usize;
        let mut hit = false;
        EvLog::walk_until(&self.store, |r| {
            seen += 1;
            let rec = String::from_utf8_lossy(&r.payload);
            let at = at_of(&rec);
            if is_note(&rec, REFUSED) && str_field(&rec, "item").as_deref() == Some(item) && at.is_some_and(|a| a > floor) {
                hit = true;
            }
            hit || seen >= SCAN_MAX || at.is_some_and(|a| a <= floor)
        });
        hit
    }

    /// Every refusal, OLDEST FIRST. A record that does not read is skipped.
    pub fn lost_rows(&self) -> Vec<LostRow> {
        self.notes(REFUSED).iter().filter_map(|r| row_of(r)).collect()
    }
}

/// One refusal record, read.
pub fn row_of(rec: &str) -> Option<LostRow> {
    Some(LostRow {
        sale: LostSale {
            item: str_field(rec, "item")?,
            dish: str_field(rec, "dish")?,
            qty: int_field(rec, "qty")?,
            price: int_field(rec, "price")?,
        },
        at: at_of(rec)?,
    })
}

/// WHICH DISH LOST THE SALE: the first line of the basket (`(product JSON,
/// portions)`, as the placement passes them) whose recipe -- or an option's
/// recipe on it -- names `item`. Its price is the record's `price` plus the
/// chosen options' `delta` (`modifiers::bom::option_line`). `None` when no
/// line names the item (the basket's tree refused a semi-finished product):
/// then the first line is the one charged, so the refusal is still counted.
pub fn lost_of(lines: &[(String, i64)], item: &str) -> Option<LostSale> {
    // Top-level keys through a real parser: a nested `id` (a modifier group's)
    // must never be read as the dish's.
    let parsed: Vec<(serde_json::Value, i64)> =
        lines.iter().map(|(j, q)| (serde_json::from_str(j).unwrap_or(serde_json::Value::Null), *q)).collect();
    let key = |v: &serde_json::Value, k: &str| v.get(k).and_then(serde_json::Value::as_str).map(str::to_string);
    let names = |j: &str| super::bom_of(j).iter().any(|l| l.supply == item) || j.contains(&format!("\"{}\"", esc(item)));
    let at = lines
        .iter()
        .position(|(j, q)| *q > 0 && names(j))
        .or_else(|| parsed.iter().position(|(v, q)| *q > 0 && key(v, "id").is_some()))?;
    let (v, qty) = &parsed[at];
    let dish = key(v, "id").or_else(|| key(v, "of"))?;
    let num = |v: &serde_json::Value, k: &str| v.get(k).and_then(serde_json::Value::as_i64);
    let base = parsed.iter().find(|(p, _)| key(p, "id").as_deref() == Some(dish.as_str())).and_then(|(p, _)| num(p, "price"));
    let delta = parsed.iter().find(|(p, _)| key(p, "of").as_deref() == Some(dish.as_str())).and_then(|(p, _)| num(p, "delta"));
    let price = base.unwrap_or(0).saturating_add(delta.unwrap_or(0)).max(0);
    Some(LostSale { item: item.to_string(), dish, qty: *qty, price })
}

/// THE PLACEMENT'S REFUSAL PATH: on an `OutOfStock`, record the lost sale in
/// `log` (in memory -- the caller writes the image) and hand the error back
/// unchanged. Any other refusal records nothing. Recording never turns a
/// refusal into something else: a failed write is the same refusal.
pub fn on_refusal(log: &mut StockLog, lines: &[(String, i64)], e: StockError) -> StockError {
    if let StockError::OutOfStock { item, .. } = &e {
        if let Some(sale) = lost_of(lines, item) {
            let _ = log.append_lost(&sale);
        }
    }
    e
}

#[cfg(test)]
#[path = "refused/tests.rs"]
mod tests;
