//! THE CARRIED REMAINDER OF FRACTIONAL DRAWS, and the exact write door
//! (`docs/design/SPEC-SEMI-FINISHED-2026-09-29.md` §c).
//!
//! A dish expanded through a semi-finished card draws 0.773 810 g of salt per
//! roll. The shelf counts whole grams and stays that way (every record ever
//! written, every screen, every typed count). What changes is HOW WHOLE UNITS
//! ARE BOOKED: a `reserved`/`served` record for such a leaf carries `"uq"`,
//! its exact draw in millionths, and this fold keeps per item
//!
//!     carry = Σ uq − 10^6 · Σ qty          over the draws it has seen,
//!
//! so the next draw books `round_half_up((carry + uq) / 10^6)` whole units:
//! after N sales the booked total is `round(N·uq/10^6)`, exactly, with the
//! carry bounded in `(−0.5, 0.5]` units by construction. A record WITHOUT
//! `uq` counts as `uq = qty × 10^6`, moving the carry by zero -- every log
//! written before this file folds with carry 0 everywhere. A `released`
//! undoes the SAME `uq` and `qty` its `reserved` booked (the open hold is
//! remembered), so a cancelled order leaves no fraction behind; `consumed`
//! keeps the carry (the food was used); `unserved` undoes a `served`.
//!
//! A FOLD, NEVER A STORED COUNTER: rebuilt from the raw records beside the
//! cost book (`journal::step`, `checkpoint::fold_tail`) and carried through
//! checkpoints in their `X` section (written only when non-empty, so every
//! checkpoint already on a live log still verifies byte for byte).

use std::collections::BTreeMap;

use super::cost::CostBook;
use super::meta::Meta;
use super::{BomLine, Qty, StockError, StockEvent, StockLog};

/// Millionths of a base unit.
pub const MICRO: i64 = crate::prep::MICRO;

/// The remainder per item and the exact quantity of every open hold.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Carry {
    /// item -> Σ uq − 10^6 Σ qty, in millionths.
    pub(super) rem: BTreeMap<String, i64>,
    /// (order, item) -> (uq, qty) of a `reserved` not yet consumed or
    /// released, and of a `served` not yet reversed.
    pub(super) open: BTreeMap<(String, String), (i64, Qty)>,
}

impl Carry {
    /// The remainder owed on `item`, in millionths: what the next draw adds
    /// before rounding.
    pub fn of(&self, item: &str) -> i64 {
        self.rem.get(item).copied().unwrap_or(0)
    }

    /// The whole units to book for a draw of `uq` millionths of `item` now.
    pub fn book(&self, item: &str, uq: i64) -> Qty {
        let owed = i128::from(self.of(item)) + i128::from(uq);
        ((owed + i128::from(MICRO) / 2).div_euclid(i128::from(MICRO))) as Qty
    }

    fn shift(&mut self, item: &str, uq: i64, qty: Qty) {
        let e = self.rem.entry(item.to_string()).or_insert(0);
        *e = e.saturating_add(uq).saturating_sub(qty.saturating_mul(MICRO));
        if *e == 0 {
            self.rem.remove(item);
        }
    }

    /// Fold one decoded record with its `uq`, if it carries one.
    pub fn apply(&mut self, ev: &StockEvent, uq: Option<i64>) {
        let key = |o: &str, i: &str| (o.to_string(), i.to_string());
        match ev {
            StockEvent::Reserved { item, qty, order_id } | StockEvent::Served { item, qty, order_id } => {
                // A whole line is remembered by nothing here: its release
                // undoes `(qty × 10^6, qty)`, which is exactly what it moved.
                // So a log with no fractional draw keeps BOTH maps empty and
                // its checkpoints keep their old bytes.
                let Some(uq) = uq.filter(|u| u % MICRO != 0) else { return };
                self.shift(item, uq, *qty);
                let e = self.open.entry(key(order_id, item)).or_insert((0, 0));
                *e = (e.0.saturating_add(uq), e.1.saturating_add(*qty));
            }
            StockEvent::Consumed { item, order_id, .. } => {
                self.open.remove(&key(order_id, item));
            }
            StockEvent::Released { item, qty, order_id } | StockEvent::Unserved { item, qty, order_id } => {
                // Undo exactly what was booked, as far as this record releases it.
                let (uq_held, q_held) = self.open.remove(&key(order_id, item)).unwrap_or((qty.saturating_mul(MICRO), *qty));
                let (back_uq, back_q) = if *qty >= q_held { (uq_held, q_held) } else { (qty.saturating_mul(MICRO), *qty) };
                self.shift(item, -back_uq, -back_q);
                if *qty < q_held {
                    self.open.insert(key(order_id, item), (uq_held - back_uq, q_held - back_q));
                }
            }
            StockEvent::Removed { item, .. } => {
                self.rem.remove(item);
                self.open.retain(|(_, i), _| i != item);
            }
            _ => {}
        }
    }
}

/// One leaf an order draws: `uq` millionths of `item` in all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draw {
    pub item: String,
    pub uq: i64,
    pub order_id: String,
}

/// What one order's lines draw, exactly: `lines` is `(product JSON, quantity
/// ordered)` as `reservations_for` takes it, the product JSON being
/// `prep::for_ledger`'s (leaf lines with `uq`) or a plain one. Quantities
/// multiply, one item's draws over the basket are summed, and the list is
/// sorted so two hubs replaying one basket write the same records.
pub fn draws_for(order_id: &str, lines: &[(String, i64)]) -> Vec<Draw> {
    let mut totals: BTreeMap<String, i64> = BTreeMap::new();
    for (product_json, ordered) in lines {
        if *ordered <= 0 {
            continue;
        }
        for BomLine { supply, uq, .. } in super::bom_of(product_json) {
            let e = totals.entry(supply).or_insert(0);
            *e = e.saturating_add(uq.saturating_mul(*ordered));
        }
    }
    totals.into_iter().map(|(item, uq)| Draw { item, uq, order_id: order_id.to_string() }).collect()
}

impl StockLog {
    /// THE EXACT DOOR for an order's reservations: fold the tail once, book
    /// each draw's whole units from the carry, decide the batch against the
    /// shelf as `append_all` does (all or nothing), write each `reserved`
    /// with its `uq`. Answers the cost book of the same fold and the log
    /// length it was folded at, as `append_all_costed` does (R4).
    pub fn append_draws(&mut self, draws: &[Draw]) -> Result<(CostBook, usize), StockError> {
        self.commit_drawn(draws, |item, qty, order_id| StockEvent::Reserved { item, qty, order_id })
    }

    /// The same door for a till import's sales (`served`, no reservation).
    pub fn append_served_draws(&mut self, draws: &[Draw]) -> Result<(CostBook, usize), StockError> {
        self.commit_drawn(draws, |item, qty, order_id| StockEvent::Served { item, qty, order_id })
    }

    fn commit_drawn(
        &mut self,
        draws: &[Draw],
        make: fn(String, Qty, String) -> StockEvent,
    ) -> Result<(CostBook, usize), StockError> {
        let (_, _, carry, _) = self.fold_tail(false, false)?;
        let mut trial = carry;
        let mut evs: Vec<(StockEvent, Meta)> = Vec::with_capacity(draws.len());
        for d in draws {
            if d.uq <= 0 {
                return Err(StockError::NotPositive { qty: d.uq });
            }
            let qty = trial.book(&d.item, d.uq);
            let ev = make(d.item.clone(), qty, d.order_id.clone());
            trial.apply(&ev, Some(d.uq));
            let whole = d.uq % MICRO == 0;
            evs.push((ev, Meta { uq: (!whole).then_some(d.uq), ..Meta::default() }));
        }
        self.commit(&evs, true)
    }
}

#[cfg(test)]
#[path = "carry/tests.rs"]
mod tests;
