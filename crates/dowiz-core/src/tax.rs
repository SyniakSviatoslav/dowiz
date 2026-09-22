//! The rate is an INTEGER — `RatePpm`, `tax_of`, `summarise`.
//!
//! `money.rs:1` has said "RED LINE: zero float arithmetic on monetary values"
//! since the day it was written, and `money.rs:267` took an `f64`. This module
//! is the side of the line the rate belongs on.
//!
//! **Parts per million, and why not basis points.** 20 % = `200_000`.
//! Basis points cannot hold 8.875 % (New York City's combined sales tax:
//! 887.5 bp is not an integer) and per-mille cannot hold it either (88.75 ‰).
//! ppm holds every published rate this platform can face, and it is the basis
//! the repo already uses three times: `money::MONEY_SCALE_MICRO`,
//! `eqc_gen`'s `rate_micro`, and `services/ordering/rates.rs`'s FX ppm.
//! Binary fixed point is NOT an answer: measured, 20 % of 100 000 000 lek in
//! Q16 is 19 999 695 — 305 lek short. A tax rate is a DECIMAL fraction and
//! needs a decimal scale.
//!
//! **The arithmetic is the generated organ.** `tax_of` calls
//! `crate::eqc_gen::apply_tax_{exclusive,inclusive}_int`, the integer-exact
//! artifacts emitted by `tools/eqc-rs` from the equation of truth. That is the
//! authority flip `eqc_gen.rs:18-20` recorded as "NOT done": the organ is now
//! the law and `money::apply_tax(_, f64, _)` is a thin adapter over it.
//!
//! **Rounding, with the rounded quantity NAMED** (blueprint §3.2 equation 4):
//! inclusive rounds the NET, exclusive rounds the TAX, both half-up. Three
//! rules that each sound like "round half up" give two different answers on a
//! tie, and a fiscal message carries the base, the VAT and the gross of the
//! same line, so they have to add up. See `tax/tests.rs`, example 2.
//!
//! No `f64` appears in this file, and `tools/gates/float-money.sh` counts it.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::{format, vec};

/// One million. The ppm basis, as `i64` (`money::MONEY_SCALE_MICRO` is the
/// `i128` spelling of the same constant).
pub const PPM: i64 = 1_000_000;

/// A tax rate in parts per million. 20 % = `RatePpm(200_000)`. Never a float,
/// never a decimal string.
///
/// `u32` is deliberate: a negative rate is the one input both the law and the
/// organ have to guard against (`money.rs`'s `denom <= 0`, `eqc_gen.rs:74-76`),
/// and an unsigned type turns that runtime guard into a PARSE-TIME refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RatePpm(pub u32);

impl RatePpm {
    /// 100 %. A rate above this is refused at parse: it is a typo
    /// (`20` meant as a percentage, `20000000` meant as ppm), not a tax.
    pub const MAX: u32 = 1_000_000;

    /// Parse a rate from the wire. **A float is refused**, which is the whole
    /// point: `"tax_rate": 0.20` arrives as the text `0.20` and is rejected;
    /// `"tax.default_ppm": "200000"` is accepted.
    ///
    /// ASCII digits only — no sign, no decimal point, no exponent, no
    /// separator, no surrounding space. Every refusal names its reason.
    pub fn parse(s: &str) -> Result<RatePpm, &'static str> {
        if s.is_empty() {
            return Err("rate: empty; a rate is an integer in parts per million (20% = 200000)");
        }
        let b = s.as_bytes();
        if b[0] == b'-' {
            return Err("rate: a negative rate is refused (a rate is unsigned ppm)");
        }
        for &c in b {
            if !c.is_ascii_digit() {
                return Err(
                    "rate: not an integer in parts per million — a float, sign, exponent or separator on the wire is refused (20% is 200000, never 0.20)",
                );
            }
        }
        let mut v: u32 = 0;
        for &c in b {
            v = v
                .checked_mul(10)
                .and_then(|x| x.checked_add(u32::from(c - b'0')))
                .ok_or("rate: too large for u32 parts per million")?;
        }
        if v > Self::MAX {
            return Err("rate: above 100% (1000000 ppm); a rate that high is a typo, not a tax");
        }
        Ok(RatePpm(v))
    }
}

/// The tax on `base` at `rate`. **This IS the generated organ** — the authority
/// flip. `inclusive` selects which quantity is rounded: the NET (gross given)
/// or the TAX (net given).
pub fn tax_of(base: i64, rate: RatePpm, inclusive: bool) -> Result<i64, String> {
    tax_micro(base, i64::from(rate.0), inclusive).map_err(|e| format!("tax_of: {e}"))
}

/// The organ, addressed by a raw `rate_micro` so the `f64` adapter
/// (`money::apply_tax`) can reach rates outside `RatePpm`'s domain that its two
/// remaining callers still pass. Crate-private: `tax_of` is the front door.
pub(crate) fn tax_micro(base: i64, rate_micro: i64, inclusive: bool) -> Result<i64, &'static str> {
    if inclusive {
        crate::eqc_gen::apply_tax_inclusive_int(base, rate_micro)
    } else {
        crate::eqc_gen::apply_tax_exclusive_int(base, rate_micro)
    }
}

/// One priced amount and the rate it carries. `amount` is as PRICED: gross when
/// the venue's prices include tax, net when they do not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaxLine {
    pub amount: i64,
    pub rate: RatePpm,
}

/// One rate group: the level the fiscal message states (`SameTax`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaxGroup {
    pub rate_ppm: u32,
    /// The DISCOUNTED base (equation 3).
    pub base: i64,
    pub tax: i64,
    pub lines: u32,
}

/// What `summarise` is given.
#[derive(Debug, Clone, Copy)]
pub struct TaxInput<'a> {
    pub lines: &'a [TaxLine],
    /// The whole-basket cut already decided (`promo::Promo::discount`).
    pub discount: i64,
    pub inclusive: bool,
    /// The delivery fee is its own group at its own rate (equation 5), and is
    /// NOT discounted.
    pub fee: Option<TaxLine>,
    /// A voluntary gratuity: outside the taxable base, inside the order total.
    pub tip: i64,
}

/// The `tax` block of an order (blueprint §3.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxSummary {
    pub inclusive: bool,
    /// Sorted by `rate_ppm` ascending — a canonical order, so the block is
    /// byte-identical wherever it is recomputed (MANIFESTO C2).
    pub groups: Vec<TaxGroup>,
    /// Parallel to `groups`: what each group absorbed of the discount. Sums to
    /// `discount` EXACTLY (largest remainder).
    pub discount_allocated: Vec<i64>,
    pub fee: Option<TaxGroup>,
    /// Σ tax over the groups AND the fee. Serialised as `tax.total`.
    pub tax_total: i64,
    /// Equation 6.
    pub order_total: i64,
}

/// Group by rate, allocate the discount, round ONCE per group, sum.
///
/// Per-line VAT is derived for display and never summed: with whole-lek prices,
/// per-line rounding drifts by up to ⌈n/2⌉ lek, and a fiscal message has no
/// field for a rounding difference. Three 250-lek coffees at 20 % inclusive are
/// 125 lek of VAT, not 126.
pub fn summarise(inp: &TaxInput) -> Result<TaxSummary, String> {
    if inp.discount < 0 {
        return Err("summarise: a negative discount is refused".into());
    }
    if inp.tip < 0 {
        return Err("summarise: a negative tip is refused".into());
    }
    if let Some(f) = inp.fee {
        if f.amount < 0 {
            return Err("summarise: a negative fee is refused".into());
        }
    }

    // (1) group by rate. Linear scan, not a map: the groups are few and the
    // order must not depend on a hasher.
    let mut acc: Vec<(u32, i64, u32)> = vec![];
    for l in inp.lines {
        if l.amount < 0 {
            return Err("summarise: a negative line amount is refused".into());
        }
        match acc.iter_mut().find(|g| g.0 == l.rate.0) {
            Some(g) => {
                g.1 = g
                    .1
                    .checked_add(l.amount)
                    .ok_or("summarise: rate group overflows i64")?;
                g.2 += 1;
            }
            None => acc.push((l.rate.0, l.amount, 1)),
        }
    }
    acc.sort_by(|a, b| a.0.cmp(&b.0));

    let mut gross: i64 = 0;
    for g in &acc {
        gross = g
            .1
            .checked_add(gross)
            .ok_or("summarise: priced subtotal overflows i64")?;
    }
    if inp.discount > gross {
        return Err(format!(
            "summarise: discount {} exceeds the priced subtotal {}",
            inp.discount, gross
        ));
    }

    // (2) allocate the discount, largest remainder, so Σ d_r == D exactly.
    let d = i128::from(inp.discount);
    let total = i128::from(gross);
    let mut alloc: Vec<i64> = vec![0; acc.len()];
    if d > 0 && total > 0 {
        let mut rems: Vec<(i128, u32, usize)> = vec![];
        let mut given: i128 = 0;
        for (i, g) in acc.iter().enumerate() {
            let num = d * i128::from(g.1);
            let q = num / total;
            alloc[i] = q as i64;
            given += q;
            rems.push((num % total, g.0, i));
        }
        // Ties in the remainder go to the LOWER rate first — deterministic,
        // and reproducible from the stored order alone.
        rems.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut left = d - given;
        for r in &rems {
            if left <= 0 {
                break;
            }
            alloc[r.2] += 1;
            left -= 1;
        }
        if left != 0 {
            return Err("summarise: the discount could not be allocated exactly".into());
        }
    }

    // (3)(4) base and tax, once per group.
    let mut groups: Vec<TaxGroup> = vec![];
    let mut base_sum: i64 = 0;
    let mut tax_total: i64 = 0;
    for (i, g) in acc.iter().enumerate() {
        let base = g.1 - alloc[i];
        let tax = tax_of(base, RatePpm(g.0), inp.inclusive)?;
        base_sum = base_sum
            .checked_add(base)
            .ok_or("summarise: base sum overflows i64")?;
        tax_total = tax_total
            .checked_add(tax)
            .ok_or("summarise: tax sum overflows i64")?;
        groups.push(TaxGroup {
            rate_ppm: g.0,
            base,
            tax,
            lines: g.2,
        });
    }

    // (5) the fee is its own group.
    let fee = match inp.fee {
        Some(f) => {
            let tax = tax_of(f.amount, f.rate, inp.inclusive)?;
            tax_total = tax_total
                .checked_add(tax)
                .ok_or("summarise: tax sum overflows i64 (fee)")?;
            Some(TaxGroup {
                rate_ppm: f.rate.0,
                base: f.amount,
                tax,
                lines: 1,
            })
        }
        None => None,
    };

    // (6) totals. Inclusive prices already contain their tax; exclusive add it.
    let fee_amount = inp.fee.map_or(0, |f| f.amount);
    let mut order_total = base_sum;
    if !inp.inclusive {
        order_total = order_total
            .checked_add(tax_total)
            .ok_or("summarise: order total overflows i64")?;
    }
    order_total = order_total
        .checked_add(fee_amount)
        .and_then(|t| t.checked_add(inp.tip))
        .ok_or("summarise: order total overflows i64")?;

    Ok(TaxSummary {
        inclusive: inp.inclusive,
        groups,
        discount_allocated: alloc,
        fee,
        tax_total,
        order_total,
    })
}

#[cfg(test)]
mod tests;
