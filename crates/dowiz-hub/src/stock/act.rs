//! THE PRODUCTION ACT (акт приготування, W-PF2 R2): "cooked N g of ПФ X".
//!
//! `planned` is what the card makes from what went in (the card scaled by
//! `planned / yield`); `out` is what came off the stove, WEIGHED. The card's
//! inputs leave the shelf exactly (`prep::stocked::plan`, a ready product
//! inside it taken first, one rounding per leaf, the carry books whole
//! units) as `Cooked` records; the batch lands as ONE `Made` record carrying
//! its value -- what the inputs cost at the weighted average when drawn --
//! so the ПФ's own average is the raw cost actually used (WAC). The loss on
//! cooking is `gross - out` in grams, and `out` against `planned` is the
//! measured yield against the card's.
//!
//! ONE DECISION: every record of the act is decided and written together
//! (`commit`), or none is.

use serde_json::Value;

use super::basket::ready_micro;
use super::carry::MICRO;
use super::meta::Meta;
use super::{Qty, StockError, StockEvent, StockLog};
use crate::import::recipes::num::Rat;
use crate::prep::stocked::{card_from, plan, World};
use crate::prep::{card_of, untracked};

/// What the kitchen says it did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Act {
    /// The semi-finished product made.
    pub prep: String,
    /// What the card makes from the inputs, in the ПФ's unit.
    pub planned: Qty,
    /// What came out, weighed.
    pub out: Qty,
    /// The act's id (one per request).
    pub act: String,
    /// The AUTHENTICATED signer.
    pub by: String,
    pub lot: Option<String>,
    pub expiry: Option<i64>,
}

/// What the act wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Done {
    /// `(item, whole units booked, exact millionths)` per input.
    pub inputs: Vec<(String, Qty, i64)>,
    /// The batch's cost in minor units; `None` when an input has no price.
    pub value: Option<i64>,
    /// Grams in (0 when a piece on the card has no weight).
    pub gross: Qty,
}

fn refused(why: impl std::fmt::Display) -> StockError {
    StockError::Linkage(why.to_string())
}

/// Grams of the card's lines at `planned / yield` (g and ml are grams, a
/// piece its `weightPerUnit`), rounded once; 0 when a piece has no weight.
fn gross_grams(lines: &[(String, i64)], planned: Qty, yield_qty: Qty, supply: &dyn Fn(&str) -> Option<String>) -> Qty {
    let mut g = 0.0;
    for (item, qty) in lines {
        let v: Value = supply(item).and_then(|j| serde_json::from_str(&j).ok()).unwrap_or(Value::Null);
        let per = match v.get("unit").and_then(Value::as_str).unwrap_or("g") {
            "unit" => match v.get("weightPerUnit").and_then(Value::as_f64).filter(|w| *w > 0.0) {
                Some(w) => w,
                None => return 0,
            },
            _ => 1.0,
        };
        g += per * *qty as f64;
    }
    (g * planned as f64 / yield_qty as f64).round() as Qty
}

impl StockLog {
    /// Record a production act. Refused, nothing written: an item that is
    /// not a semi-finished product with a card, a quantity that is not one,
    /// a card that does not expand, or the shelf's own refusal.
    pub fn cook(&mut self, a: &Act, supply: &dyn Fn(&str) -> Option<String>) -> Result<Done, StockError> {
        let card = supply(&a.prep).and_then(|j| card_of(&j)).ok_or_else(|| refused(format!("{} is not a semi-finished product with a card", a.prep)))?;
        if a.planned <= 0 || a.out <= 0 || a.planned > crate::prep::QTY_MAX || a.out > crate::prep::QTY_MAX {
            return Err(StockError::NotPositive { qty: a.planned.min(a.out) });
        }
        let lines: Vec<(String, i64)> = card.lines.iter().map(|l| (l.item.clone(), l.qty)).collect();
        let (led, book, carry, _) = self.fold_tail(true, true)?;
        let roots: Vec<(String, Rat)> =
            lines.iter().map(|(i, q)| (i.clone(), Rat::new(i128::from(*q) * i128::from(a.planned), i128::from(card.yield_qty)))).collect();
        let cards = |id: &str| card_from(supply, id);
        let quiet = |id: &str| supply(id).is_some_and(|j| untracked(&j));
        // The product being made is never taken from its own shelf.
        let ready = |id: &str| if id == a.prep { 0 } else { ready_micro(&led, &carry, id) };
        let leaves = plan(&roots, &World { card: &cards, untracked: &quiet, ready: &ready }).map_err(refused)?;

        let mut trial = carry;
        let mut evs: Vec<(StockEvent, Meta)> = Vec::with_capacity(leaves.len() + 1);
        let mut inputs = Vec::with_capacity(leaves.len());
        let mut sum: Option<i128> = Some(0);
        for l in &leaves {
            let qty = trial.book(&l.item, l.uq);
            let ev = StockEvent::Cooked { item: l.item.clone(), qty, into: a.prep.clone(), act: a.act.clone(), by: a.by.clone() };
            trial.apply(&ev, Some(l.uq));
            sum = sum.and_then(|s| Some(s + book.avg_micro(&l.item)? * i128::from(l.uq)));
            evs.push((ev, Meta { uq: (l.uq % MICRO != 0).then_some(l.uq), ..Meta::default() }));
            inputs.push((l.item.clone(), qty, l.uq));
        }
        let m2 = i128::from(MICRO) * i128::from(MICRO);
        let value = sum.and_then(|s| i64::try_from((s + m2 / 2) / m2).ok());
        let gross = gross_grams(&lines, a.planned, card.yield_qty, supply);
        let made = StockEvent::Made { item: a.prep.clone(), qty: a.out, planned: a.planned, gross, act: a.act.clone(), by: a.by.clone() };
        evs.push((made, Meta { value, lot: a.lot.clone(), expiry: a.expiry, ..Meta::default() }));
        self.commit(&evs, false)?;
        Ok(Done { inputs, value, gross })
    }
}

#[cfg(test)]
#[path = "act/tests.rs"]
pub(crate) mod tests;
