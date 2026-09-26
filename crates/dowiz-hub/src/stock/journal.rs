//! ONE PASS OVER THE RAW LOG (research 2026-09-26 §3.7.1): every record
//! decoded once, folded into the shelf, the cost book and the lots together,
//! and kept as a dated, valued row for the reports.
//!
//! Before this, a report walked the log twice with two decoders -- `events()`
//! for the shelf, `raw()` for the cost -- and a lot fold would have been a
//! third. Here the shelf is the SAME `StockLedger::apply` the write door runs,
//! so the journal's levels cannot disagree with `ledger()` (a test holds them
//! equal), and each row's `value` is read off the cost book AT THAT POINT of
//! history, before the row moved it: a write-off in March is valued at
//! March's average, not today's.

use super::cost::CostBook;
use super::lots::Lots;
use super::meta::{meta_of, Meta};
use super::{decode, Qty, StockError, StockEvent, StockLedger, StockLog};

/// One record, as a report reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Position in the log, oldest first, counting only records that decode.
    pub seq: usize,
    pub ev: StockEvent,
    pub meta: Meta,
    /// `on_hand` of the event's item just before this record.
    pub before: Qty,
    /// Minor units. A priced receipt: what the paper says it cost. A draw
    /// (consumed, served, wasted, the input of a prep): its quantity at the
    /// average then. A count: the drift (observed - expected) at the average,
    /// signed. `None` where no price has reached the supply yet.
    pub value: Option<i64>,
}

impl Entry {
    /// What the ledger held when a count was written: the stored `expected`
    /// when the record carries one, else the fold's own `before`.
    pub fn expected(&self) -> Qty {
        self.meta.expected.unwrap_or(self.before)
    }
}

/// The whole log, folded once.
#[derive(Debug, Clone, Default)]
pub struct Journal {
    pub entries: Vec<Entry>,
    pub ledger: StockLedger,
    pub book: CostBook,
    pub lots: Lots,
}

/// Minor units for `qty` at `unit_cost` per `per`, rounded half up.
pub fn priced(qty: Qty, unit_cost: i64, per: Qty) -> Option<i64> {
    if per <= 0 {
        return None;
    }
    let n = i128::from(qty) * i128::from(unit_cost);
    let d = i128::from(per);
    i64::try_from((n + d / 2) / d).ok()
}

impl Journal {
    fn value_of(&self, ev: &StockEvent, meta: &Meta, before: Qty) -> Option<i64> {
        if meta.value.is_some() {
            return meta.value;
        }
        match ev {
            StockEvent::Received { qty, .. } => priced(*qty, meta.unit_cost?, meta.per?),
            StockEvent::Consumed { item, qty, .. }
            | StockEvent::Served { item, qty, .. }
            | StockEvent::Wasted { item, qty, .. }
            | StockEvent::Produced { item, qty, .. } => self.book.value_of(item, *qty),
            StockEvent::Returned { item, qty, resell: false, .. } => self.book.value_of(item, *qty),
            StockEvent::Stocktake { item, observed, .. } => {
                let drift = observed - meta.expected.unwrap_or(before);
                self.book.value_of(item, drift.abs()).map(|v| if drift < 0 { -v } else { v })
            }
            _ => None,
        }
    }

    /// Fold one raw record. A record that does not decode is skipped, as
    /// `events()` skips it; one the ledger refuses stops the fold, as
    /// `ledger()` does.
    fn step(&mut self, rec: &str) -> Result<(), StockError> {
        let Some(ev) = decode(rec) else { return Ok(()) };
        let meta = meta_of(rec);
        let before = self.ledger.level(ev.item()).on_hand;
        let value = self.value_of(&ev, &meta, before);
        self.ledger.apply(&ev)?;
        self.book.apply_event(&ev, rec);
        let entry = Entry { seq: self.entries.len(), ev, meta, before, value };
        let ledger = &self.ledger;
        self.lots.step(&entry, |i| ledger.level(i).on_hand);
        self.entries.push(entry);
        Ok(())
    }
}

impl StockLog {
    /// Every record folded ONCE: shelf, cost, lots and the dated rows.
    pub fn journal(&self) -> Result<Journal, StockError> {
        let mut j = Journal::default();
        for rec in self.raw() {
            j.step(&rec)?;
        }
        Ok(j)
    }
}

#[cfg(test)]
#[path = "journal/tests.rs"]
mod tests;
