//! A QUANTITY ON THE WIRE AND IN A RECIPE LINE (W-QTY, TZ 2026-10-12 item 1;
//! `/root/lanes/CONTRACT-QTY.md` §1).
//!
//! A kitchen puts half a sheet of nori on a roll and a fifth of a gram of
//! spice in a sauce. The console sends such a number as it was typed --
//! `40.5`, `"40,5"`, `40` -- and this file reads it from its TEXT into
//! integer MILLI-UNITS (thousandths of the supply's base unit), never through
//! an f64 that sums with drift. Three decimals is the precision: 1 mg, 1 µl,
//! a thousandth of a piece. The hub ledger books recipe draws in millionths
//! (`dowiz_hub::prep::MICRO`, `stock::carry`), so `uq = milli x 1000` is
//! exact there too.
//!
//! A quantity compares with a WHOLE number of base units (`qty > QTY_MAX`,
//! `qty <= 0`) so a bound written before fractions existed still reads.

use serde::de::{self, Deserializer, Visitor};
use serde::Deserialize;
use serde_json::Value;
use std::fmt;

/// Milli-units per base unit.
pub const MILLI: i64 = 1000;
/// Decimals a quantity may carry.
pub const DECIMALS: usize = 3;

/// A quantity in milli-units of its supply's base unit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Qty {
    pub milli: i64,
}

impl Qty {
    /// `n` whole base units.
    pub const fn whole(n: i64) -> Qty {
        Qty { milli: n.saturating_mul(MILLI) }
    }
    pub const fn milli(milli: i64) -> Qty {
        Qty { milli }
    }
    /// From the hub's millionths, rounded half up to a thousandth.
    pub fn of_micro(uq: i64) -> Qty {
        Qty { milli: (uq + MILLI / 2).div_euclid(MILLI) }
    }
    /// The hub's millionths (`uq`).
    pub fn uq(self) -> i64 {
        self.milli.saturating_mul(MILLI)
    }
    pub fn is_whole(self) -> bool {
        self.milli % MILLI == 0
    }
    /// Whole base units when the quantity is one; `None` for a fraction.
    pub fn as_whole(self) -> Option<i64> {
        self.is_whole().then_some(self.milli / MILLI)
    }
    /// Whole base units, half up: what an integer-grams reader takes.
    pub fn rounded(self) -> i64 {
        (self.milli + MILLI / 2).div_euclid(MILLI)
    }
    /// The quantity as a number of base units, f64, for nutrition scaling
    /// (kcal are f64 already); never for money.
    pub fn units_f64(self) -> f64 {
        self.milli as f64 / MILLI as f64
    }
    /// `"40"`, `"40.5"`, `"0.001"`: the shortest exact text.
    pub fn show(self) -> String {
        let (neg, m) = (self.milli < 0, self.milli.unsigned_abs());
        let (w, f) = (m / MILLI as u64, m % MILLI as u64);
        let mut s = if neg { format!("-{w}") } else { w.to_string() };
        if f != 0 {
            let frac = format!("{f:03}");
            s.push('.');
            s.push_str(frac.trim_end_matches('0'));
        }
        s
    }
    /// The JSON number the console reads: an integer for a whole quantity,
    /// else the decimal with up to three places (serde_json prints the
    /// shortest text that reads back to the same number).
    pub fn json(self) -> Value {
        match self.as_whole() {
            Some(w) => Value::from(w),
            None => self.show().parse::<serde_json::Number>().map(Value::Number).unwrap_or(Value::Null),
        }
    }
    /// A stored recipe line: `{"qty": 40}` (whole), `{"uq": 40500000}`
    /// (millionths, exact) -- `None` when it carries neither as a number.
    pub fn from_stored(line: &Value) -> Option<Qty> {
        if let Some(uq) = line.get("uq").and_then(Value::as_i64) {
            return Some(Qty::of_micro(uq));
        }
        line.get("qty").and_then(Value::as_i64).map(Qty::whole)
    }
}

/// Why a text is not a quantity.
fn bad(s: &str) -> String {
    format!("{s:?} is not a quantity: a number with at most {DECIMALS} decimals")
}

/// `"40"`, `"40.5"`, `"40,5"`, `" 1500 "`, `"-2"` -> milli-units, exactly.
pub fn parse(text: &str) -> Result<Qty, String> {
    let s = text.trim();
    let (neg, body) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    // A space groups thousands ("1 500"); a comma is the DECIMAL comma, never
    // a thousands separator -- "2,000" is two, as every stock screen reads it.
    let grouped = body.replace(' ', "");
    let body = grouped.as_str();
    let (whole, frac) = match body.find(['.', ',']) {
        Some(i) => (&body[..i], &body[i + 1..]),
        None => (body, ""),
    };
    if whole.is_empty() && frac.is_empty() || !whole.chars().all(|c| c.is_ascii_digit()) || !frac.chars().all(|c| c.is_ascii_digit()) {
        return Err(bad(text));
    }
    if frac.len() > DECIMALS {
        return Err(format!("{text:?}: at most {DECIMALS} decimals"));
    }
    let w: i64 = if whole.is_empty() { 0 } else { whole.parse().map_err(|_| bad(text))? };
    let f: i64 = if frac.is_empty() { 0 } else { format!("{frac:0<3}").parse().map_err(|_| bad(text))? };
    let milli = w.checked_mul(MILLI).and_then(|m| m.checked_add(f)).ok_or_else(|| bad(text))?;
    Ok(Qty { milli: if neg { -milli } else { milli } })
}

/// A JSON number as the console may send it: an integer, a float with at
/// most three decimals (checked by reading it back), or a string.
fn of_f64(x: f64) -> Result<Qty, String> {
    if !x.is_finite() || x.abs() > 9.0e15 {
        return Err(bad(&x.to_string()));
    }
    let milli = (x * MILLI as f64).round();
    if (milli / MILLI as f64 - x).abs() > f64::EPSILON * x.abs().max(1.0) {
        return Err(format!("{x}: at most {DECIMALS} decimals"));
    }
    Ok(Qty { milli: milli as i64 })
}

struct QtyVisitor;

impl<'de> Visitor<'de> for QtyVisitor {
    type Value = Qty;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a quantity: a number with at most {DECIMALS} decimals, or its text")
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Qty, E> {
        v.checked_mul(MILLI).map(Qty::milli).ok_or_else(|| E::custom(bad(&v.to_string())))
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Qty, E> {
        i64::try_from(v).ok().and_then(|v| v.checked_mul(MILLI)).map(Qty::milli).ok_or_else(|| E::custom(bad(&v.to_string())))
    }
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Qty, E> {
        of_f64(v).map_err(E::custom)
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Qty, E> {
        parse(v).map_err(E::custom)
    }
}

impl<'de> Deserialize<'de> for Qty {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Qty, D::Error> {
        d.deserialize_any(QtyVisitor)
    }
}

impl serde::Serialize for Qty {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.json().serialize(s)
    }
}

/// A quantity against a count of WHOLE base units: `qty > QTY_MAX`.
impl PartialEq<i64> for Qty {
    fn eq(&self, other: &i64) -> bool {
        self.milli == other.saturating_mul(MILLI)
    }
}
impl PartialOrd<i64> for Qty {
    fn partial_cmp(&self, other: &i64) -> Option<std::cmp::Ordering> {
        Some(self.milli.cmp(&other.saturating_mul(MILLI)))
    }
}

impl From<i64> for Qty {
    fn from(n: i64) -> Qty {
        Qty::whole(n)
    }
}

impl fmt::Display for Qty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.show())
    }
}

/// THE SHELF COUNTS WHOLE BASE UNITS (`dowiz_hub::stock::Qty`; hand-back H3
/// of the contract): a stock movement's quantity is read as a decimal and
/// must come out whole -- `"1500"`, `"1500.000"` -- or it is refused by
/// name, never rounded behind the owner's back. `1,5 kg` is the screen's to
/// turn into `1500` before sending.
pub fn whole_opt<'de, D: Deserializer<'de>>(d: D) -> Result<Option<i64>, D::Error> {
    match Option::<Qty>::deserialize(d)? {
        None => Ok(None),
        Some(q) => q.as_whole().map(Some).ok_or_else(|| {
            de::Error::custom(format!("{}: the shelf counts whole units of the base measure (g, ml, piece); type 1,5 kg as 1500", q.show()))
        }),
    }
}

/// [`whole_opt`] for a required field.
pub fn whole<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    let q = Qty::deserialize(d)?;
    q.as_whole().ok_or_else(|| de::Error::custom(format!("{}: the shelf counts whole units of the base measure (g, ml, piece); type 1,5 kg as 1500", q.show())))
}

/// HAND-BACK H2 (`/root/lanes/CONTRACT-QTY.md` §5): the hub's `prep::card_of`
/// reads a card line's `qty` and the `yield` as integers, so a fraction on a
/// semi-finished card has no stored form yet. Read as a decimal and refused
/// by name when it is not whole, never rounded; main replaces this with
/// `Qty` once the hub reads `uq` on a card line.
pub fn whole_card<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    let q = Qty::deserialize(d)?;
    q.as_whole().ok_or_else(|| de::Error::custom(format!("{}: a semi-finished card counts whole base units for now", q.show())))
}

#[cfg(test)]
#[path = "qty/tests.rs"]
mod tests;
