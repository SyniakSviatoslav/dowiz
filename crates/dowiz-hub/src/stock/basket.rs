//! A BASKET THAT REACHES A SEMI-FINISHED PRODUCT (W-PF2 R2): the tree the
//! Worker embedded in each dish (`prep::for_ledger` -> `"tree"`), read back
//! in the venue's object, where the shelf is -- so a batch cooked ahead is
//! taken first, as far as it goes, and the rest is expanded to raw
//! (`prep::stocked`, whose header says why the rest is not refused).
//!
//! A basket with no tree in it is never one of these (`of` answers `None`)
//! and books exactly what `draws_for` always booked.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::carry::{Carry, Draw, MICRO};
use super::{bom_of, StockError, StockLedger};
use crate::import::recipes::num::Rat;
use super::cost::CostBook;
use crate::prep::stocked::{plan_split, portion, Shelf, World};
use crate::prep::tree::{Card, Edge};
use crate::prep::Refusal;

/// The basket's roots (what its dishes name, times how many were ordered)
/// and every card under them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Basket {
    pub roots: Vec<(String, Rat)>,
    pub cards: BTreeMap<String, Card>,
    pub untracked: BTreeSet<String>,
}

fn card_of(v: &Value) -> Option<Card> {
    let lines = v.get("lines")?.as_array()?.iter().map(|l| Some(Edge { item: l.get("item")?.as_str()?.to_string(), qty: Rat::new(i128::from(l.get("qty")?.as_i64()?), 1) })).collect::<Option<Vec<_>>>()?;
    Some(Card { lines, batch: Rat::new(i128::from(v.get("yield")?.as_i64().filter(|y| *y > 0)?), 1) })
}

/// The basket of `lines` (`(product JSON, ordered)`, as `draws_for` takes
/// them), or `None` when no dish in it carries a tree.
pub fn of(lines: &[(String, i64)]) -> Option<Basket> {
    if !lines.iter().any(|(j, _)| j.contains(r#""tree""#)) {
        return None;
    }
    let mut b = Basket { roots: Vec::new(), cards: BTreeMap::new(), untracked: BTreeSet::new() };
    let mut any = false;
    for (j, ordered) in lines {
        if *ordered <= 0 {
            continue;
        }
        let tree = serde_json::from_str::<Value>(j).ok().and_then(|v| v.get("tree").cloned()).filter(Value::is_object);
        let Some(t) = tree else {
            // A dish with no tree: its leaves as they are, exactly.
            for l in bom_of(j) {
                b.roots.push((l.supply, Rat::new(i128::from(l.uq) * i128::from(*ordered), i128::from(MICRO))));
            }
            continue;
        };
        any = true;
        for l in t.get("lines").and_then(Value::as_array).into_iter().flatten() {
            let (Some(s), Some(q)) = (l.get("supply").and_then(Value::as_str), l.get("qty").and_then(Value::as_i64)) else { continue };
            b.roots.push((s.to_string(), Rat::new(i128::from(q) * i128::from(*ordered), 1)));
        }
        for (id, c) in t.get("cards").and_then(Value::as_object).into_iter().flatten() {
            if let Some(c) = card_of(c) {
                b.cards.insert(id.clone(), c);
            }
        }
        for u in t.get("untracked").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str) {
            b.untracked.insert(u.to_string());
        }
    }
    any.then_some(b)
}

/// Millionths of a ready semi-finished product the shelf can give NOW,
/// booked whole units included: the most whose booking (`Carry::book`)
/// stays within what is available. 0 for a product never made (uncounted)
/// or with nothing left.
pub fn ready_micro(led: &StockLedger, carry: &Carry, id: &str) -> i64 {
    let a = led.available(id);
    if !led.is_counted(id) || a <= 0 {
        return 0;
    }
    a.saturating_mul(MICRO).saturating_add(MICRO / 2 - 1).saturating_sub(carry.of(id)).max(0)
}

impl Basket {
    /// What the order draws, the shelf asked first.
    pub fn draws(&self, order_id: &str, led: &StockLedger, carry: &Carry) -> Result<Vec<Draw>, StockError> {
        self.draws_split(order_id, led, carry).map(|(d, _)| d)
    }

    /// [`Basket::draws`], and the share of each semi-finished product the
    /// shelf gave (`prep::stocked::Shelf`), for the cost stamp (W-PF3 T1).
    pub fn draws_split(&self, order_id: &str, led: &StockLedger, carry: &Carry) -> Result<(Vec<Draw>, Shelf), StockError> {
        let card = |id: &str| -> Result<Option<Card>, Refusal> { Ok(self.cards.get(id).cloned()) };
        let quiet = |id: &str| self.untracked.contains(id);
        let ready = |id: &str| ready_micro(led, carry, id);
        let (leaves, shelf) = plan_split(&self.roots, &World { card: &card, untracked: &quiet, ready: &ready }).map_err(|r| StockError::Linkage(r.to_string()))?;
        Ok((leaves.into_iter().map(|l| Draw { item: l.item, uq: l.uq, order_id: order_id.to_string(), via: None, stations: Vec::new() }).collect(), shelf))
    }

    /// THE COST THE LEDGER BOOKS for this basket (one portion of one dish,
    /// `of(&[(dish, 1)])`) when each semi-finished product gives the share
    /// `shelf` says: the part from a ready batch at the BATCH's average, the
    /// rest at the raw averages, exact inside and rounded ONCE to minor
    /// units. `None` when the shelf gave nothing this dish reaches (the
    /// caller prices the all-raw `bom` as before); `Some(None)` when anything
    /// it takes has no price (a partial sum is not a cost: no stamp).
    pub fn portion_cost(&self, shelf: &Shelf, book: &CostBook) -> Option<Option<i64>> {
        if !self.cards.keys().any(|k| shelf.contains_key(k)) {
            return None;
        }
        Some(self.priced(shelf, book))
    }

    fn priced(&self, shelf: &Shelf, book: &CostBook) -> Option<i64> {
        let card = |id: &str| -> Result<Option<Card>, Refusal> { Ok(self.cards.get(id).cloned()) };
        let quiet = |id: &str| self.untracked.contains(id);
        let takes = portion(&self.roots, &World { card: &card, untracked: &quiet, ready: &|_| 0 }, shelf).ok()?;
        // avg_micro is micro-minor per unit: sum(avg * q) / MICRO minor units.
        let mut sum = Rat::new(0, 1);
        for (item, q) in &takes {
            sum = sum.checked_add(q.checked_mul(Rat::new(book.avg_micro(item)?, 1))?)?;
        }
        let den = sum.den.checked_mul(i128::from(MICRO))?;
        i64::try_from((sum.num.checked_mul(2)?.checked_add(den)?).div_euclid(den.checked_mul(2)?)).ok()
    }
}

#[cfg(test)]
#[path = "basket/tests.rs"]
mod tests;
