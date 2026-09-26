//! GROSS, NET, OUT -- брутто, нетто, вихід (research 2026-09-26 §3.2, row R6).
//!
//! A recipe line's `qty` is GROSS in the supply's base unit: what the ledger
//! reserves and consumes, what the kitchen paid for. Two losses follow it:
//! CLEANING (whole salmon -> fillet: `cleanPm` of the gross remains) and
//! COOKING (fillet -> seared, rice dry -> cooked: `cookPm` of the net
//! remains; above 1000 when it grows, as rice does). Both are per-mille
//! defaults on the SUPPLY; a line may override its net or its out with a
//! weighed number, because a nigiri slice and a tartare trim the same fish
//! differently.
//!
//! EVERY WEIGHT HERE IS INTEGER GRAMS. For a supply counted in pieces the
//! gross weight is `weightPerUnit x qty`; for millilitres a millilitre is a
//! gram (`ML_TO_G`). A dish's weight is the sum of its food lines' OUT -- what
//! is on the plate -- which is what the storefront prints.

use serde_json::Value;

/// Per mille: 1000 = nothing lost.
pub const PM: i64 = 1000;
/// Cleaning can only lose.
pub const CLEAN_MAX: i64 = PM;
/// Cooking can grow a thing five-fold at most (dry rice is about 2.2x).
pub const COOK_MAX: i64 = 5 * PM;
/// A weighed override is bounded like a quantity.
pub const GRAMS_MAX: i64 = 500_000;

/// One line's weights and where they came from.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Weights {
    pub gross: Option<i64>,
    pub net: Option<i64>,
    pub out: Option<i64>,
    pub clean_pm: i64,
    pub cook_pm: i64,
    /// The line's own weighed numbers, when the owner typed them.
    pub net_set: Option<i64>,
    pub out_set: Option<i64>,
}

/// A supply's per-mille default, or 1000 when absent or out of range.
pub fn pm_of(supply: &Value, key: &str, max: i64) -> i64 {
    supply.get(key).and_then(Value::as_i64).filter(|v| (1..=max).contains(v)).unwrap_or(PM)
}

/// `x * pm / 1000`, rounded half up, in integers.
pub fn scale(x: i64, pm: i64) -> i64 {
    (x.saturating_mul(pm) + PM / 2).div_euclid(PM)
}

/// The gross weight of `qty` base units, in grams: grams are grams, a
/// millilitre is a gram, a piece weighs `weightPerUnit`.
pub fn gross_g(unit: &str, qty: i64, supply: &Value) -> Option<i64> {
    match unit {
        "g" | "ml" => Some(qty),
        _ => supply.get("weightPerUnit").and_then(Value::as_f64).map(|w| (w * qty as f64).round() as i64),
    }
}

/// The line's gross, net and out from the supply's defaults and the line's
/// own overrides.
pub fn weights(unit: &str, qty: i64, supply: &Value, net: Option<i64>, out: Option<i64>) -> Weights {
    let clean_pm = pm_of(supply, "cleanPm", CLEAN_MAX);
    let cook_pm = pm_of(supply, "cookPm", COOK_MAX);
    let gross = gross_g(unit, qty, supply);
    let n = net.or_else(|| gross.map(|g| scale(g, clean_pm)));
    let o = out.or_else(|| n.map(|v| scale(v, cook_pm)));
    Weights { gross, net: n, out: o, clean_pm, cook_pm, net_set: net, out_set: out }
}

impl Weights {
    /// Of the gross, what reaches the plate, per mille. `None` without a gross.
    pub fn yield_pm(&self) -> Option<i64> {
        let g = self.gross.filter(|g| *g > 0)?;
        Some(self.out? * PM / g)
    }
    /// Lost between the gross and the plate, per mille (negative: it grew).
    pub fn loss_pm(&self) -> Option<i64> {
        self.yield_pm().map(|y| PM - y)
    }
    /// The edible share of the gross, for nutrition declared on the RAW
    /// weight: net / gross, else the supply's cleaning default.
    pub fn edible(&self) -> f64 {
        match (self.gross, self.net) {
            (Some(g), Some(n)) if g > 0 => n as f64 / g as f64,
            _ => self.clean_pm as f64 / PM as f64,
        }
    }
}

/// A line's typed net/out, refused when it is not a weight: negative, past
/// the bound, or a net heavier than the gross it was cut from.
pub fn check(net: Option<i64>, out: Option<i64>, gross: Option<i64>) -> Result<(), String> {
    for (k, v) in [("net", net), ("out", out)] {
        if v.is_some_and(|x| !(0..=GRAMS_MAX).contains(&x)) {
            return Err(format!("{k} is 0 to {GRAMS_MAX} g"));
        }
    }
    if let (Some(n), Some(g)) = (net, gross) {
        if n > g {
            return Err(format!("net {n} g is more than the gross {g} g"));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "weights/tests.rs"]
mod tests;
