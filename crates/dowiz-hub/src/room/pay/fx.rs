//! A PAYMENT IN ANOTHER CURRENCY (BLUEPRINT-OPERATIONAL-BLIND-SPOTS P3-2).
//!
//! Guests in Durrës pay euro and lek mixed; the invoice stays in the order's
//! currency, and the payment states what was handed over, in what, and at
//! what rate. The bill's arithmetic (Σ ≤ total, "paid") is ALWAYS in the
//! order's currency; the drawer's is ALWAYS in the currency paid
//! (`command::till`), because that is the pile the note went into.
//!
//! THE RATE UNIT, precisely. `rate_ppm` is:
//!
//!   the number of ORDER-currency MINOR units that ONE PAYMENT-currency MINOR
//!   unit is worth, multiplied by 1 000 000.
//!
//! Minor units on both sides, so the two currencies' decimal places
//! (`Currency::minor_units`: ALL 0, EUR 2) are inside the number and never a
//! second step someone can forget. Worked, order in ALL, paid in EUR at
//! 1 EUR = 97.50 ALL: one euro CENT is 0.975 lek, so `rate_ppm = 975_000`,
//! and €20.00 (2 000 cents) is `(2000 · 975000 + 500000) / 1000000 = 1950`
//! lek. The other way, order in EUR, paid in ALL at the same rate: one lek is
//! 1.025641… cents, `rate_ppm = 1_025_641`.
//!
//! WHY NOT `dowiz_core::tax::RatePpm`. It is a TAX rate and is capped at
//! 1 000 000 (100 %) at parse; the lek→euro-cent rate above is over that,
//! so the type would refuse a correct exchange. Same scale, different bound.
//!
//! ROUNDING is the kernel's, copied from `crates/dowiz-core/src/eqc_gen.rs`
//! (`apply_tax_exclusive_int`): half-up with an INTEGER `b / 2`, in i128,
//! every step checked. Amount and rate are both ≥ 1 here, so half-up has one
//! reading.
//!
//! MOVED FROM `workers/api/src/command/pay/fx.rs` (D7 phase 1); the Worker
//! re-exports it and keeps its tests.

use super::super::Refused;
use dowiz_core::money::Currency;

/// The divisor of `rate_ppm`.
pub const RATE_SCALE: i64 = 1_000_000;

/// What a payment settles, once its currency is resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settled {
    /// The currency handed over.
    pub currency: Currency,
    /// Present only when that is not the order's currency.
    pub rate_ppm: Option<i64>,
    /// What it pays off the bill, in the order's currency.
    pub in_order_currency: i64,
}

/// `amount` payment-currency minor units in order-currency minor units.
pub fn convert(amount: i64, rate_ppm: i64) -> Result<i64, Refused> {
    if rate_ppm < 1 {
        return Err(Refused::Invalid("a rate is at least 1 (order minor units per payment minor unit, × 1 000 000)".into()));
    }
    let b = RATE_SCALE as i128;
    let n = (amount as i128)
        .checked_mul(rate_ppm as i128)
        .and_then(|x| x.checked_add(b / 2))
        .ok_or_else(|| Refused::Invalid("amount × rate overflows".into()))?
        / b;
    i64::try_from(n).map_err(|_| Refused::Invalid("the converted amount overflows".into()))
}

/// Resolve a payment's currency against the order's.
///
/// No currency named → the order's. Same currency → no rate, and a rate sent
/// anyway is refused (a number recorded that means nothing is a number
/// someone later believes). Another currency → a rate is required.
pub fn settle(order_currency: &str, paid_in: Option<&str>, rate_ppm: Option<i64>, amount: i64) -> Result<Settled, Refused> {
    let order = Currency::from_code(order_currency)
        .ok_or_else(|| Refused::Invalid(format!("the order's currency {order_currency} is not one this platform knows")))?;
    let paid = match paid_in {
        None => order,
        Some(c) => Currency::from_code(c).ok_or_else(|| Refused::Invalid(format!("{c}: not a currency this venue can take")))?,
    };
    if paid == order {
        if rate_ppm.is_some() {
            return Err(Refused::Invalid(format!("a payment in {} on a {} order has no rate", paid.code(), order.code())));
        }
        return Ok(Settled { currency: paid, rate_ppm: None, in_order_currency: amount });
    }
    let rate = rate_ppm.ok_or_else(|| {
        Refused::Invalid(format!("a payment in {} on a {} order states its rate (rate_ppm)", paid.code(), order.code()))
    })?;
    let in_order_currency = convert(amount, rate)?;
    if in_order_currency < 1 {
        return Err(Refused::Invalid("that payment is worth less than one minor unit of the order's currency".into()));
    }
    Ok(Settled { currency: paid, rate_ppm: Some(rate), in_order_currency })
}

#[cfg(test)]
mod tests;
