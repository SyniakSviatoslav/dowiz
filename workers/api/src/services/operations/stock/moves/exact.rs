//! A MOVEMENT'S QUANTITY AND PRICE, exact (W-QTYHUB H3): what the person
//! typed in milli-units, the hub's millionths beside it, and a price per a
//! fraction of a piece as integers.

use super::{Bad, StockMoveIn};
use crate::recipe::Qty;

/// A quantity the shelf takes WHOLE (a prep's board, its output, a batch):
/// a fraction is refused by name, never rounded behind the owner's back.
pub fn whole(q: Qty) -> Result<i64, Bad> {
    q.as_whole().ok_or_else(|| (400, format!("{}: weigh this one in whole units of the base measure (g, ml, piece)", q.show())))
}

/// What the person typed, in the hub's millionths (`Meta::uq`): the hub's
/// write door books the whole units from it (`stock::carry::book_human`).
/// `None` for nothing positive -- the event's own `qty` then refuses it.
pub(super) fn uq_of(q: Option<Qty>) -> Option<i64> {
    q.map(Qty::uq).filter(|u| *u > 0)
}

/// The paper's price as `(unit_cost, per)`: the total for the whole quantity,
/// or a cost per basis. Half a price is refused, not guessed.
///
/// THE TOTAL FOR A FRACTION (W-QTYHUB H3): 750 for 2.5 pieces is stored as
/// `750 000 per 2 500` (milli-units), reduced -- the same price exactly, as
/// integers, since `per` counts whole base units.
pub(super) fn price(body: &StockMoveIn, qty: Qty) -> Result<(Option<i64>, Option<i64>), Bad> {
    let total_per = |t: i64| match qty.as_whole() {
        Some(w) => (t, w),
        None => {
            let (a, b) = (t.saturating_mul(crate::recipe::qty::MILLI), qty.milli.max(1));
            let g = gcd(a, b).max(1);
            (a / g, b / g)
        }
    };
    match (body.total, body.unit_cost, body.per) {
        (Some(t), None, None) if t >= 0 => {
            let (c, p) = total_per(t);
            Ok((Some(c), Some(p)))
        }
        (None, Some(c), Some(p)) if c >= 0 && p > 0 => Ok((Some(c), Some(p))),
        (None, None, None) => Ok((None, None)),
        _ => Err((400, "a price is the line's total, or a cost with what it is per -- not negative".into())),
    }
}

fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

