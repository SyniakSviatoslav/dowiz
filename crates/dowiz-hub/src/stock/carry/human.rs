//! THE HUMAN MOVEMENTS' FRACTION (W-QTYHUB H3, CONTRACT-QTY §1): a delivery,
//! a write-off and a count carry what was typed as `uq` (millionths) beside
//! the whole units the shelf books; the carry keeps the difference, so the
//! EXACT shelf is `on_hand x 10^6 - carry`.

use super::super::cost::CostBook;
use super::super::meta::Meta;
use super::{Carry, MICRO};
use crate::stock::{Qty, StockError, StockEvent, StockLedger, StockLog};

impl Carry {
    /// The whole units a DELIVERY of `uq` millionths books now (W-QTYHUB
    /// H3): the exact shelf after it, rounded half up, less the booked shelf
    /// before -- `round_half_up((uq - carry) / 10^6)`. AT LEAST ONE: a
    /// delivery is "always > 0" (`decide`), and the carry keeps the rest, so
    /// the exact shelf is still exact.
    pub fn book_in(&self, item: &str, uq: i64) -> Qty {
        let owed = i128::from(uq) - i128::from(self.of(item));
        ((owed + i128::from(MICRO) / 2).div_euclid(i128::from(MICRO))).max(1) as Qty
    }

    /// THE EXACT SHELF, in millionths: what the whole-unit `on_hand` stands
    /// for once the carried remainder is taken off (`on_hand x 10^6 - carry`).
    /// Equal to `on_hand x 10^6` for every item no fraction ever touched.
    pub fn exact(&self, item: &str, on_hand: Qty) -> i64 {
        on_hand.saturating_mul(MICRO).saturating_sub(self.of(item))
    }
}

impl CostBook {
    /// `value_of` for an exact quantity in millionths: 0.25 of a piece at
    /// the average, rounded once, half up.
    pub fn value_of_micro(&self, item: &str, uq: i64) -> Option<i64> {
        let (a, m2) = (self.avg_micro(item)?, i128::from(MICRO) * i128::from(MICRO));
        i64::try_from((a * i128::from(uq) + m2 / 2).div_euclid(m2)).ok()
    }
}

impl StockLog {
    /// The shelf and its carried remainders NOW (from the newest checkpoint):
    /// `carry.exact(item, ledger.level(item).on_hand)` is the exact level in
    /// millionths -- 2.5 pieces received reads 2 500 000, not the 3 booked.
    pub fn shelf_exact(&self) -> Result<(StockLedger, Carry), StockError> {
        self.fold_tail(true, false).map(|(l, _, c, _)| (l, c))
    }
}

/// BOOK A HUMAN MOVEMENT EXACTLY (W-QTYHUB H3, CONTRACT-QTY §1): a delivery,
/// a write-off or a count whose `meta.uq` holds what the person typed, in
/// millionths. The whole `qty` / `observed` the shelf counts is booked HERE,
/// from the carry folded at the write, and `carry` advances as the fold will.
///
/// A WHOLE quantity on an item with no remainder writes no `uq`: the record is
/// byte for byte the one written before fractions existed. Any other event
/// (an order's draw, a batch's input) is left as its own door booked it.
pub(in crate::stock) fn book_human(carry: &mut Carry, ev: &mut StockEvent, meta: &mut Meta) {
    let Some(uq) = meta.uq else { return };
    let whole = uq % MICRO == 0;
    match ev {
        StockEvent::Received { item, qty } => match whole {
            true => (*qty, meta.uq) = (uq / MICRO, None),
            false => *qty = carry.book_in(item, uq),
        },
        StockEvent::Wasted { item, qty, .. } => match whole {
            true => (*qty, meta.uq) = (uq / MICRO, None),
            false => *qty = carry.book(item, uq).max(1),
        },
        StockEvent::Stocktake { item, observed, .. } => {
            *observed = (uq + MICRO / 2).div_euclid(MICRO);
            if whole && carry.of(item) == 0 {
                meta.uq = None;
            }
        }
        _ => return,
    }
    carry.apply(ev, meta.uq);
}

