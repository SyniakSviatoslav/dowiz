//! Integer money — RED LINE: zero float arithmetic on monetary values.
//!
//! 1:1 port of `apps/api/src/lib/money.ts` (server-authoritative `applyTax`, `toMinorUnit`,
//! `computeLineTotal`, `assertNonNegative`) and the EUR-conversion from
//! `packages/shared-types/src/utils.ts`. All amounts are integer minor units.
//!
//! BP-17: all arithmetic is now OVERFLOW-SAFE (checked ops → `Err`, never panic/wrap),
//! `i128 → i64` casts are range-checked via `i64::try_from` (no silent truncation),
//! and the two dead guards (NaN-fossil `amount != amount`, identity `round_half_up`)
//! are removed. Money invariants are not weakened.

/// Reject non-integer amounts (money is integer minor units).
///
/// NOTE (BP-17): the old code guarded `amount != amount` (a NaN check) — dead on `i64`,
/// which can never be NaN. Removed. `i64` is already integer by type; this is a passthrough
/// that exists for interface fidelity with the oracle's `toMinorUnit`.
pub fn to_minor_unit(amount: i64, _currency: &str) -> Result<i64, String> {
    Ok(amount)
}

const SCALE: i128 = 1_000_000;

/// Server-authoritative `applyTax` (money.ts:23). `subtotal` is integer minor units.
/// `tax_rate` is a config input (e.g. 0.20) parsed once to micro-units.
///
/// BP-17: `i128 → i64` cast is range-checked (`i64::try_from`) — an overflowing tax
/// (pathological rate × huge subtotal) returns `Err` instead of silently truncating.
pub fn apply_tax(subtotal: i64, tax_rate: f64, price_includes_tax: bool) -> Result<i64, String> {
    if subtotal == 0 || tax_rate == 0.0 {
        return Ok(0);
    }
    // OLD (dead guard, removed): `if subtotal % 1 != 0` — i64 % 1 is always 0.
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
    i64::try_from(tax).map_err(|_| "tax overflow: subtotal * rate exceeds i64".into())
}

/// `computeLineTotal`: sum of unit price + modifiers, times quantity.
///
/// BP-17: overflow-safe. The OLD code did `unit * quantity` unchecked → panic (debug) /
/// wrap (release) on `unit_price = i64::MAX, quantity = 2`. Now returns `Err(Overflow)`
/// via checked arithmetic.
pub fn compute_line_total(
    product_price: i64,
    modifier_prices: &[i64],
    quantity: i64,
) -> Result<i64, String> {
    let mut unit = product_price;
    for &m in modifier_prices {
        unit = unit
            .checked_add(m)
            .ok_or("line unit price overflow (modifier sum)")?;
    }
    unit.checked_mul(quantity)
        .ok_or_else(|| "line total overflow (unit_price * quantity)".to_string())
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
///
/// BP-17: `i128 → i64` cast is range-checked (`i64::try_from`) — a huge conversion result
/// returns `Err` instead of silently truncating.
pub fn convert_all_to_eur_cents(amount_all: i64, rate: f64) -> Result<i64, String> {
    if rate <= 0.0 {
        return Err("rate must be > 0".into());
    }
    let rate_scaled = (rate * 1_000_000_000.0).round() as i128;
    let eur_cents = (amount_all as i128) * rate_scaled * 100i128;
    let scale = 10i128.pow(9);
    let rounded = (eur_cents + scale / 2) / scale;
    i64::try_from(rounded).map_err(|_| "EUR conversion overflow".into())
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
        assert_eq!(
            compute_line_total(500, &[50, 25], 2).unwrap(),
            (500 + 50 + 25) * 2
        );
    }
    #[test]
    fn green_non_negative() {
        assert!(assert_non_negative(0).is_ok());
        assert!(assert_non_negative(-1).is_err());
    }

    // ── RED→GREEN: overflow must return Err, never panic/wrap ──
    #[test]
    fn red_line_total_overflow_is_err() {
        // unit_price = i64::MAX, quantity = 2 → would overflow i64.
        assert!(matches!(compute_line_total(i64::MAX, &[], 2), Err(_)));
        // modifier sum overflow also Err
        assert!(matches!(compute_line_total(i64::MAX, &[1], 1), Err(_)));
    }

    #[test]
    fn red_tax_overflow_is_err() {
        // pathological: huge subtotal × rate=2.0 → tax ≈ i64::MAX*2 exceeds i64.
        let r = apply_tax(i64::MAX, 2.0, false);
        assert!(r.is_err(), "tax overflow must be Err, got {:?}", r);
    }

    #[test]
    fn red_eur_conversion_overflow_is_err() {
        let r = convert_all_to_eur_cents(i64::MAX, 1.0);
        assert!(r.is_err(), "EUR overflow must be Err, got {:?}", r);
    }

    // ── GREEN: ALL→EUR conversion integer math ──
    #[test]
    fn green_all_to_eur() {
        // 1000 ALL at rate 0.01 (100 ALL = 1 EUR) → 10 EUR => 1000 cents
        assert_eq!(convert_all_to_eur_cents(1000, 0.01).unwrap(), 1000);
    }
}
