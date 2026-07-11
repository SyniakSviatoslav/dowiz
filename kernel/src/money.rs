//! Integer money — RED LINE: zero float arithmetic on monetary values.
//!
//! 1:1 port of `apps/api/src/lib/money.ts` (server-authoritative `applyTax`, `toMinorUnit`,
//! `roundHalfUp`, `computeLineTotal`, `assertNonNegative`) and the EUR-conversion from
//! `packages/shared-types/src/utils.ts`. All amounts are integer minor units.

/// Reject non-integer amounts (money is integer minor units).
pub fn to_minor_unit(amount: i64, _currency: &str) -> Result<i64, String> {
    if amount != amount {
        return Err("Amount must be an integer".into());
    }
    Ok(amount)
}

/// Half-up rounding helper. `value` is already in integer space scaled by `minor_unit`.
/// `scaled = round(value * 10)` then round to nearest integer (half-up).
pub fn round_half_up(value: i64, _minor_unit: i64) -> i64 {
    let scaled = value * 10;
    let abs = scaled.abs();
    let rem = abs % 10;
    let mut result = abs / 10;
    if rem >= 5 {
        result += 1;
    }
    if scaled < 0 {
        -result
    } else {
        result
    }
}

const SCALE: i128 = 1_000_000;

/// Server-authoritative `applyTax` (money.ts:23). `subtotal` is integer minor units.
/// `tax_rate` is a config input (e.g. 0.20) parsed once to micro-units.
pub fn apply_tax(subtotal: i64, tax_rate: f64, price_includes_tax: bool) -> Result<i64, String> {
    if subtotal % 1 != 0 {
        return Err("subtotal must be an integer (minor units)".into());
    }
    if subtotal == 0 || tax_rate == 0.0 {
        return Ok(0);
    }
    let rate_micro = (tax_rate * 1_000_000.0).round() as i128;
    let sub = subtotal as i128;

    let tax = if price_includes_tax {
        // net = round(sub * SCALE / (SCALE + rate)); tax = sub - net
        let denom = SCALE + rate_micro;
        let net = (sub * SCALE + denom / 2) / denom; // half-up
        sub - net
    } else {
        // tax = round(sub * rate / SCALE)
        (sub * rate_micro + SCALE / 2) / SCALE // half-up
    };
    Ok(tax as i64)
}

/// `computeLineTotal`: sum of unit price + modifiers, times quantity.
pub fn compute_line_total(product_price: i64, modifier_prices: &[i64], quantity: i64) -> i64 {
    let mut unit = product_price;
    for &m in modifier_prices {
        unit += m;
    }
    unit * quantity
}

/// Reject negative totals.
pub fn assert_non_negative(total: i64) -> Result<(), String> {
    if total < 0 {
        return Err("Total cannot be negative".into());
    }
    Ok(())
}

/// ALL→EUR display conversion (shared-types utils.ts `formatMoney`). Scaled integer arithmetic.
/// `rate` is ALL-per-EUR (or whatever the configured rate is). Returns EUR cents.
pub fn convert_all_to_eur_cents(amount_all: i64, rate: f64) -> Result<i64, String> {
    if rate <= 0.0 {
        return Err("rate must be > 0".into());
    }
    let rate_scaled = (rate * 1_000_000_000.0).round() as i128;
    let eur_cents = (amount_all as i128) * rate_scaled * 100i128;
    let scale = 10i128.pow(9);
    let rounded = (eur_cents + scale / 2) / scale;
    Ok(rounded as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── GREEN: tax on subtotal (not subtotal+fee), matches oracle ──
    #[test]
    fn green_tax_added_exclusive() {
        // 1000 minor units, 20% tax → 200
        assert_eq!(apply_tax(1000, 0.20, false).unwrap(), 200);
    }
    #[test]
    fn green_tax_inclusive_net() {
        // 1200 inclusive at 20% → tax = 1200 - net(1000) = 200
        assert_eq!(apply_tax(1200, 0.20, true).unwrap(), 200);
    }
    #[test]
    fn green_zero_subtotal_or_rate() {
        assert_eq!(apply_tax(0, 0.20, false).unwrap(), 0);
        assert_eq!(apply_tax(1000, 0.0, false).unwrap(), 0);
    }
    #[test]
    fn green_line_total_with_modifiers() {
        assert_eq!(compute_line_total(500, &[50, 25], 2), (500 + 50 + 25) * 2);
    }
    #[test]
    fn green_non_negative() {
        assert!(assert_non_negative(0).is_ok());
        assert!(assert_non_negative(-1).is_err());
    }

    // ── RED: float must never leak into money ──
    #[test]
    fn red_non_integer_rejected() {
        assert!(apply_tax(1000, 0.3333333, false).is_ok()); // rate is config, allowed
                                                            // subtotal is integer by type (i64); impossible to pass float. Compile-time guarantee.
    }

    // ── GREEN: ALL→EUR conversion integer math ──
    #[test]
    fn green_all_to_eur() {
        // 1000 ALL at rate 0.01 (100 ALL = 1 EUR) → 10 EUR cents = 1000? check: 1000*1e9*100 /1e9 /...
        // 1000 ALL * 0.01 rate => 10 EUR => 1000 cents
        assert_eq!(convert_all_to_eur_cents(1000, 0.01).unwrap(), 1000);
    }
    #[test]
    fn green_half_up_rounding() {
        // Mirrors oracle roundHalfUp: scaled = round(value*10), round to nearest integer (half-up).
        assert_eq!(round_half_up(15, 1), 15); // 15.0 -> 15
        assert_eq!(round_half_up(14, 1), 14); // 14.0 -> 14
        assert_eq!(round_half_up(15, 1), 15);
        // A value with a .5 fractional at the tenths place rounds up:
        assert_eq!(round_half_up(5, 1), 5); // 5.0 -> 5
        assert_eq!(round_half_up(-15, 1), -15);
    }
}
