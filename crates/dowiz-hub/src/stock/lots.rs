//! LOTS ON HAND, FIRST-EXPIRY-FIRST-OUT (research 2026-09-26 §3.5, row R8).
//!
//! A lot is not a table: it is what the fold makes of the receipts that name
//! one and the draws after them. Each delivery opens a lot -- its label's code,
//! or `#<seq>` when the paper had none -- with its expiry, supplier, invoice
//! and price. Draws take from the lot that EXPIRES FIRST (then the oldest),
//! which is what HACCP already asks a kitchen to do, unless the record names
//! its lot (an explicit pick: the fish on the board is from THAT box).
//!
//! THE LOTS ALWAYS ADD UP TO THE SHELF. After every record the item's lots are
//! reconciled to the ledger's `on_hand` (never below zero): a shortfall comes
//! off the first-expiring lots, a surplus (a count above the book, a resold
//! return) joins an unlabelled lot. So the lot view can never claim food the
//! shelf does not have, and a count lands its drift on what should already
//! have been used.

use super::journal::Entry;
use super::meta::valid_day;
use super::{moved_into, Qty, StockEvent};

/// One lot of one supply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lot {
    pub item: String,
    /// The label's code, or `#<seq>` for a delivery that named none.
    pub code: String,
    /// `yyyymmdd`, local.
    pub expiry: Option<i64>,
    pub supplier: Option<String>,
    pub doc: Option<String>,
    /// ms, when it came in (absent on records from before dates).
    pub at: Option<i64>,
    pub unit_cost: Option<i64>,
    pub per: Option<Qty>,
    pub received: Qty,
    pub left: Qty,
    /// The record that opened it.
    pub seq: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lots {
    lots: Vec<Lot>,
}

/// The lot code a count's surplus or a put-back lands in.
pub const UNLABELLED: &str = "-";

impl Lots {
    /// Open lots of `item`, in the order draws take them: expiry first (a lot
    /// with no date last), then the order they arrived.
    pub fn of(&self, item: &str) -> Vec<&Lot> {
        let mut v: Vec<&Lot> = self.lots.iter().filter(|l| l.item == item && l.left > 0).collect();
        v.sort_by_key(|l| (l.expiry.is_none(), l.expiry.unwrap_or(0), l.seq));
        v
    }

    /// Every open lot, grouped by item, each group in draw order.
    pub fn open(&self) -> Vec<&Lot> {
        let mut items: Vec<&str> = self.lots.iter().filter(|l| l.left > 0).map(|l| l.item.as_str()).collect();
        items.sort_unstable();
        items.dedup();
        items.into_iter().flat_map(|i| self.of(i)).collect()
    }

    fn open_lot(&mut self, item: &str, qty: Qty, e: &Entry) {
        let code = e.meta.lot.clone().unwrap_or_else(|| format!("#{}", e.seq));
        if let Some(l) = self.lots.iter_mut().find(|l| l.item == item && l.code == code) {
            l.received += qty;
            l.left += qty;
            return;
        }
        self.lots.push(Lot {
            item: item.to_string(),
            code,
            expiry: e.meta.expiry.filter(|d| valid_day(*d)),
            supplier: e.meta.supplier.clone(),
            doc: e.meta.doc.clone(),
            at: e.meta.at,
            unit_cost: e.meta.unit_cost,
            per: e.meta.per,
            received: qty,
            left: qty,
            seq: e.seq,
        });
    }

    /// Take `qty` of `item`, from the lot named `first` if it is open, then
    /// first-expiry-first-out. Takes what there is and no more.
    fn take(&mut self, item: &str, mut qty: Qty, first: Option<&str>) {
        let mut order: Vec<usize> = Vec::new();
        if let Some(code) = first {
            order.extend(self.lots.iter().position(|l| l.item == item && l.code == code && l.left > 0));
        }
        let mut rest: Vec<usize> = (0..self.lots.len()).filter(|i| self.lots[*i].item == item && self.lots[*i].left > 0).collect();
        rest.sort_by_key(|i| {
            let l = &self.lots[*i];
            (l.expiry.is_none(), l.expiry.unwrap_or(0), l.seq)
        });
        let first_pick = order.first().copied();
        order.extend(rest.into_iter().filter(|i| Some(*i) != first_pick));
        for i in order {
            if qty <= 0 {
                break;
            }
            let d = qty.min(self.lots[i].left);
            self.lots[i].left -= d;
            qty -= d;
        }
    }

    fn total(&self, item: &str) -> Qty {
        self.lots.iter().filter(|l| l.item == item).map(|l| l.left).sum()
    }

    /// Bring `item`'s lots to exactly `on_hand` (or zero when it is negative).
    fn reconcile(&mut self, item: &str, on_hand: Qty, e: &Entry) {
        let target = on_hand.max(0);
        let have = self.total(item);
        if have > target {
            self.take(item, have - target, None);
        } else if have < target {
            let mut m = e.clone();
            m.meta.lot = Some(UNLABELLED.to_string());
            m.meta.expiry = None;
            self.open_lot(item, target - have, &m);
        }
    }

    /// Fold one journal row. `on_hand` answers the ledger AFTER the row.
    pub fn step(&mut self, e: &Entry, on_hand: impl Fn(&str) -> Qty) {
        let item = e.ev.item().to_string();
        match &e.ev {
            StockEvent::Received { qty, .. } => self.open_lot(&item, *qty, e),
            StockEvent::Consumed { qty, .. } | StockEvent::Served { qty, .. } | StockEvent::Wasted { qty, .. } => {
                self.take(&item, *qty, e.meta.lot.as_deref())
            }
            StockEvent::Produced { qty, out, into, .. } => {
                if let Some(to) = moved_into(&item, into).map(str::to_string) {
                    // The output keeps the input's date unless the cook gave
                    // one: the lot it came from, else the first to expire.
                    let src = e.meta.lot.as_deref().and_then(|c| self.of(&item).into_iter().find(|l| l.code == c).cloned());
                    let src_expiry = src.or_else(|| self.of(&item).first().map(|l| (*l).clone())).and_then(|l| l.expiry);
                    self.take(&item, *qty, e.meta.lot.as_deref());
                    // The output's lot: named, or minted from this record.
                    let mut m = e.clone();
                    m.meta.lot = e.meta.lot.as_ref().map(|l| format!("{l}>{to}"));
                    m.meta.expiry = e.meta.expiry.or(src_expiry);
                    self.open_lot(&to, *out, &m);
                    self.reconcile(&to, on_hand(&to), e);
                }
            }
            _ => {}
        }
        self.reconcile(&item, on_hand(&item), e);
    }
}

#[cfg(test)]
#[path = "lots/tests.rs"]
mod tests;
