//! COST THAT FOLLOWS PURCHASES (BLUEPRINT-OPERATIONAL-BLIND-SPOTS §2.10,
//! P2-4 / P3-4) -- the smallest slice with a CHECK.
//!
//! A delivery may carry what it cost. The price rides ON THE SAME `received`
//! record, as extra fields (`unit_cost`, `per`, `supplier`, `doc`), so there
//! is one log, one chain and one write: a priced delivery cannot land on the
//! shelf without its price or the other way round. `stock::decode` reads a
//! `received` by its `item` and `qty` and ignores the rest, so the shelf's fold
//! -- and every reader written before prices existed -- sees exactly the
//! `Received { item, qty }` it always did.
//!
//! THE COST IS A FOLD, never a stored number: a weighted moving average per
//! supply over the log, oldest first. FIFO needs lots and is refused for now
//! (§4). What moves the average:
//! - a PRICED receipt joins the pool at its price;
//! - an UNPRICED receipt, a resold return and a voided sale join at the
//!   current average (they do not move it);
//! - a draw (`consumed`, `wasted`, `served`) leaves at the current average;
//! - a `stocktake` sets the costed quantity to what was counted, at the average.
//!
//! A reservation moves nothing: the food is still on the shelf.
//!
//! STAMPED, NOT LOOKED UP (§2.10): the cost of a dish at placement is a
//! [`Stamp`] -- the cost AND the log length it was folded at -- so last March's
//! margin shows last March's cost, and [`rebuild`] (law 8) folds the same
//! prefix and must say the same number.
//!
//! Money is integer minor units. Internally a supply's cost is held in
//! MILLIONTHS of a minor unit per base unit (i128), so 1,000 per kg is exactly
//! 1,000,000 per gram and no average is taken through a float.

use super::{decode, encode, BomLine, EvLog, Qty, StockError, StockEvent, StockLog};
use crate::minijson::{esc, int_field};

const MICRO: i128 = 1_000_000;

/// What a delivery cost: `unit_cost` minor units per `per` base units
/// (1,000 per 1,000 g = 1,000 lek per kg), and who sold it on which paper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Price {
    pub unit_cost: i64,
    pub per: Qty,
    pub supplier: Option<String>,
    pub doc: Option<String>,
}

/// A dish's cost at placement, and the log length it was folded at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    pub cost: i64,
    pub at: usize,
}

/// One supply's pool: the costed quantity and its value, in millionths.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Pool {
    qty: i128,
    value: i128,
    /// The average when the pool last held something, so a shelf drawn to
    /// zero still has a cost to stamp.
    last_avg: Option<i128>,
}

impl Pool {
    fn avg(&self) -> Option<i128> {
        if self.qty > 0 {
            Some(half_up(self.value, self.qty))
        } else {
            self.last_avg
        }
    }
    fn add_at_avg(&mut self, q: i128) {
        if let Some(a) = self.avg() {
            self.value += q * a;
            self.qty += q;
        }
    }
    fn draw(&mut self, q: i128) {
        if self.qty <= 0 {
            return;
        }
        self.last_avg = self.avg();
        let d = q.min(self.qty);
        // Proportional, so drawing everything leaves exactly nothing.
        self.value = self.value * (self.qty - d) / self.qty;
        self.qty -= d;
    }
}

fn half_up(n: i128, d: i128) -> i128 {
    (n + d / 2) / d
}

/// The cost of every supply, as folded over a log prefix.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CostBook {
    pools: Vec<(String, Pool)>,
}

impl CostBook {
    fn pool(&mut self, item: &str) -> &mut Pool {
        if let Some(p) = self.pools.iter().position(|(i, _)| i == item) {
            return &mut self.pools[p].1;
        }
        self.pools.push((item.to_string(), Pool::default()));
        &mut self.pools.last_mut().expect("just pushed").1
    }

    fn avg_micro(&self, item: &str) -> Option<i128> {
        self.pools.iter().find(|(i, _)| i == item).and_then(|(_, p)| p.avg())
    }

    /// The weighted average, in minor units per `per` base units; `None` for
    /// a supply no priced delivery has ever reached.
    pub fn wac(&self, item: &str, per: Qty) -> Option<i64> {
        let a = self.avg_micro(item)?;
        i64::try_from(half_up(a * i128::from(per), MICRO)).ok()
    }

    /// One portion's cost in minor units; `None` unless EVERY line's supply
    /// has a cost (a partial sum is not a cost, as `recipe::derive` holds).
    pub fn dish_cost(&self, bom: &[BomLine]) -> Option<i64> {
        if bom.is_empty() {
            return None;
        }
        let mut sum: i128 = 0;
        for l in bom {
            sum += self.avg_micro(&l.supply)? * i128::from(l.qty);
        }
        i64::try_from(half_up(sum, MICRO)).ok()
    }

    /// Fold one raw record.
    fn apply(&mut self, rec: &str) {
        let Some(ev) = decode(rec) else { return };
        match &ev {
            StockEvent::Received { item, qty } => match price_of(rec) {
                Some((unit_cost, per)) => {
                    let p = self.pool(item);
                    let micro = half_up(i128::from(unit_cost) * MICRO, i128::from(per));
                    p.value += i128::from(*qty) * micro;
                    p.qty += i128::from(*qty);
                }
                None => self.pool(item).add_at_avg(i128::from(*qty)),
            },
            StockEvent::Returned { item, qty, resell: true, .. } | StockEvent::Unserved { item, qty, .. } => {
                self.pool(item).add_at_avg(i128::from(*qty))
            }
            StockEvent::Consumed { item, qty, .. }
            | StockEvent::Wasted { item, qty, .. }
            | StockEvent::Served { item, qty, .. } => self.pool(item).draw(i128::from(*qty)),
            StockEvent::Stocktake { item, observed, .. } => {
                let p = self.pool(item);
                if let Some(a) = p.avg() {
                    p.last_avg = Some(a);
                    p.qty = i128::from(*observed);
                    p.value = p.qty * a;
                }
            }
            StockEvent::Reserved { .. } | StockEvent::Released { .. } | StockEvent::Returned { .. } => {}
        }
    }
}

/// The price fields of a `received` record, if it carries a valid one.
fn price_of(rec: &str) -> Option<(i64, Qty)> {
    let (c, per) = (int_field(rec, "unit_cost")?, int_field(rec, "per")?);
    (c >= 0 && per > 0).then_some((c, per))
}

/// The record a priced delivery writes: `encode`'s own bytes with the price
/// appended inside the object, so the shelf's decoder is untouched.
fn priced_payload(ev: &StockEvent, price: &Price) -> String {
    let base = encode(ev);
    let mut s = base.strip_suffix('}').unwrap_or(&base).to_string();
    s.push_str(&format!(r#","unit_cost":{},"per":{}"#, price.unit_cost, price.per));
    if let Some(v) = &price.supplier {
        s.push_str(&format!(r#","supplier":"{}""#, esc(v)));
    }
    if let Some(v) = &price.doc {
        s.push_str(&format!(r#","doc":"{}""#, esc(v)));
    }
    s.push('}');
    s
}

impl StockLog {
    /// Every record's payload, OLDEST FIRST, undecoded: the price lives in
    /// fields `StockEvent` does not carry.
    fn raw(&self) -> Vec<String> {
        let mut out: Vec<String> =
            EvLog::walk(&self.store).into_iter().map(|r| String::from_utf8_lossy(&r.payload).into_owned()).collect();
        out.reverse();
        out
    }

    /// A DELIVERY WITH ITS PRICE. Decided by the shelf exactly as a plain
    /// `Received` is, and refused -- nothing written -- for a price that is not
    /// one (negative cost, a basis that is not positive).
    pub fn receive_priced(&mut self, item: &str, qty: Qty, price: &Price) -> Result<(), StockError> {
        if price.per <= 0 {
            return Err(StockError::NotPositive { qty: price.per });
        }
        if price.unit_cost < 0 {
            return Err(StockError::NotPositive { qty: price.unit_cost });
        }
        let ev = StockEvent::Received { item: item.to_string(), qty };
        self.ledger()?.decide(&ev)?;
        self.write_payload(priced_payload(&ev, price).into_bytes())
    }

    /// The cost book folded over the first `at` records (the whole log when
    /// `at` is past its end).
    pub fn cost_book_at(&self, at: usize) -> CostBook {
        let mut book = CostBook::default();
        for rec in self.raw().iter().take(at) {
            book.apply(rec);
        }
        book
    }

    pub fn cost_book(&self) -> CostBook {
        self.cost_book_at(usize::MAX)
    }
}

/// The stamp for one portion of a dish, as of NOW (the log's current length).
pub fn stamp(log: &StockLog, bom: &[BomLine]) -> Option<Stamp> {
    let at = log.len();
    log.cost_book_at(at).dish_cost(bom).map(|cost| Stamp { cost, at })
}

/// LAW 8: the cost a stamp claims, re-folded from the log alone.
pub fn rebuild(log: &StockLog, s: &Stamp, bom: &[BomLine]) -> Option<i64> {
    log.cost_book_at(s.at).dish_cost(bom)
}

#[cfg(test)]
mod tests;
