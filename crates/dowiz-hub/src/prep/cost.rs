//! WHAT ONE UNIT OF A ПФ COSTS -- derived on read, exact inside, rounded once
//! (SPEC §d). The price of a raw leaf is whatever the caller answers in
//! millionths of a minor unit per base unit: the catalogue's list price for
//! the owner's screens, the ledger's weighted average for a sale's stamp.

use super::{leaves_exact, tree::Stop, untracked, Refusal};
use crate::import::recipes::num::Rat;

/// The cost of ONE base unit of `lines`' root, in millionths of a minor unit
/// (`stock::cost`'s scale): Σ leaf quantity × `price_micro(leaf)` (millionths
/// of a minor unit per base unit of the leaf), rounded once. `None` unless
/// every tracked leaf has a price -- a partial sum is not a cost.
pub fn cost_micro(
    lines: &[(String, i64)],
    supply: &dyn Fn(&str) -> Option<String>,
    price_micro: &dyn Fn(&str) -> Option<i128>,
) -> Result<Option<i64>, Refusal> {
    let walked = leaves_exact("cost", lines, supply)?;
    let mut sum = Rat::new(0, 1);
    for l in &walked.leaves {
        let p = match price_micro(&l.item) {
            Some(p) => p,
            None if supply(&l.item).is_some_and(|j| untracked(&j)) => 0,
            None => return Ok(None),
        };
        sum = sum.checked_add(l.qty.checked_mul(Rat::new(p, 1)).ok_or(Refusal::Tree(Stop::Overflow))?).ok_or(Refusal::Tree(Stop::Overflow))?;
    }
    let n = sum.num.checked_add(sum.den / 2).ok_or(Refusal::Tree(Stop::Overflow))?;
    Ok(Some(i64::try_from(n.div_euclid(sum.den)).map_err(|_| Refusal::Tree(Stop::Overflow))?))
}

