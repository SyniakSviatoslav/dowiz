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

use super::carry::Carry;
use super::cost::CostBook;
use super::lots::Lots;
use super::meta::{meta_of, Meta};
use super::storages::{Moved, Stores};
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

/// Rows no report can date: no `at` of their own, and -- for the ones with an
/// order -- dated only if that order's placement is known to the reader.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Undated {
    /// No `at` and no order: never datable.
    pub plain: usize,
    /// No `at`, with an order: how many rows per order, first-seen order.
    pub by_order: Vec<(String, usize)>,
}

impl Undated {
    fn count(&mut self, e: &Entry) {
        if e.meta.at.is_some() || matches!(e.ev, StockEvent::Reserved { .. } | StockEvent::Released { .. }) {
            return;
        }
        match e.ev.order_id() {
            None => self.plain += 1,
            Some(o) => match self.by_order.iter_mut().find(|(k, _)| k == o) {
                Some(p) => p.1 += 1,
                None => self.by_order.push((o.to_string(), 1)),
            },
        }
    }

    /// How many of these rows a reader who knows `placed` still cannot date.
    pub fn left(&self, placed: impl Fn(&str) -> bool) -> usize {
        self.plain + self.by_order.iter().filter(|(o, _)| !placed(o)).map(|(_, n)| n).sum::<usize>()
    }
}

/// The whole log, folded once -- or, from [`StockLog::journal_since`], the
/// newest checkpoint older than a report's window plus every row after it.
#[derive(Debug, Clone, Default)]
pub struct Journal {
    /// The rows folded HERE; a checkpoint's rows are in its state, not here.
    pub entries: Vec<Entry>,
    pub ledger: StockLedger,
    pub book: CostBook,
    pub lots: Lots,
    /// The carried remainder of fractional draws (`carry.rs`).
    pub carry: Carry,
    /// Where each supply is: kitchen, bar, freezer (`storages.rs`, P12).
    pub stores: Stores,
    /// Rows folded so far, counting the checkpoint's: the next row's `seq`.
    pub seen: usize,
    /// The newest `at` any folded row carried.
    pub max_at: Option<i64>,
    /// Every undatable row folded so far, counting the checkpoint's.
    pub undated: Undated,
    /// The part of `undated` that is NOT in `entries` (the checkpoint's), so
    /// a report over `entries` adds it once. Empty for a fold from genesis.
    pub before: Undated,
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
            | StockEvent::Produced { item, qty, .. }
            | StockEvent::Cooked { item, qty, .. } => self.book.value_of(item, *qty),
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
    pub(super) fn step(&mut self, rec: &str) -> Result<(), StockError> {
        // A TRANSFER moves stock between storages and nothing else (P12).
        if let Some(m) = Moved::of(rec) {
            self.stores.apply_move(&m, &self.ledger);
            return Ok(());
        }
        // A STATION BOUND TO A STORAGE (W-STORE2): the storages' fold only.
        if let Some((station, store)) = super::storages::bind::bound_of(rec) {
            self.stores.bind(&station, &store);
            return Ok(());
        }
        let Some(ev) = decode(rec) else { return Ok(()) };
        let meta = meta_of(rec);
        let before = self.ledger.level(ev.item()).on_hand;
        let value = self.value_of(&ev, &meta, before);
        if meta.store.is_some() || meta.drawn.is_some() {
            self.stores.materialise(&self.ledger);
        }
        let to = match &ev {
            StockEvent::Produced { item, into, .. } => super::moved_into(item, into).map(str::to_string),
            _ => None,
        };
        let before_to = to.as_deref().map_or(0, |t| self.ledger.level(t).on_hand);
        self.ledger.apply(&ev)?;
        self.stores.step(&ev, meta.store.as_deref(), meta.drawn.as_deref(), before, before_to, &self.ledger);
        self.book.apply_event(&ev, rec);
        self.carry.apply(&ev, meta.uq);
        let entry = Entry { seq: self.seen, ev, meta, before, value };
        let ledger = &self.ledger;
        self.lots.step(&entry, |i| ledger.level(i).on_hand);
        self.seen += 1;
        self.max_at = self.max_at.max(entry.meta.at);
        self.undated.count(&entry);
        self.entries.push(entry);
        Ok(())
    }
}

impl StockLog {
    /// Every record folded ONCE, from the first: shelf, cost, lots and the
    /// dated rows. Checkpoints are not read (they do not decode).
    pub fn journal(&self) -> Result<Journal, StockError> {
        let mut j = Journal::default();
        for rec in self.raw() {
            j.step(&rec)?;
        }
        Ok(j)
    }

    /// The journal a report over `[since_ms, ...)` needs: the state of the
    /// newest checkpoint EVERY row of which is dated before `since_ms`, and
    /// the rows after it. Its `ledger`, `book` and `lots` are today's, as
    /// [`StockLog::journal`]'s are; its `entries` start at that checkpoint,
    /// and `before` carries the undatable rows behind it.
    ///
    /// Exact for a window starting at `since_ms` on one assumption, stated:
    /// a row without its own `at` belongs to an order placed no later than
    /// the row was written (a reservation, a draw, a till sale).
    pub fn journal_since(&self, since_ms: i64) -> Result<Journal, StockError> {
        let t = self.tail(|_, newest| newest.is_some_and(|n| n < since_ms));
        let mut j = t.base.unwrap_or_default();
        for rec in &t.recs {
            j.step(rec)?;
        }
        Ok(j)
    }
}

#[cfg(test)]
#[path = "journal/tests.rs"]
mod tests;
