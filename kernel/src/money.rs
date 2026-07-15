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

/// M5 — currency identity. Money is integer minor units *in a specific currency*.
/// Two amounts in different currencies may NEVER be added/compared as raw ints —
/// the type carries the currency so a cross-currency operation is a caught error,
/// not a silent unit confusion (the M5 gap: `unit_price` was a bare `i64`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Currency {
    /// Albanian lek (the product's home currency).
    All,
    /// Euro.
    Eur,
    /// US dollar.
    Usd,
}

impl Currency {
    pub fn code(self) -> &'static str {
        match self {
            Currency::All => "ALL",
            Currency::Eur => "EUR",
            Currency::Usd => "USD",
        }
    }
    pub fn from_code(s: &str) -> Option<Currency> {
        match s {
            "ALL" => Some(Currency::All),
            "EUR" => Some(Currency::Eur),
            "USD" => Some(Currency::Usd),
            _ => None,
        }
    }
}

/// A currency-tagged money amount (integer minor units). Arithmetic between two
/// `Money` values is fail-closed on currency mismatch (M5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Money {
    pub minor: i64,
    pub currency: Currency,
}

impl Money {
    pub fn new(minor: i64, currency: Currency) -> Self {
        Money { minor, currency }
    }

    /// Add two amounts. Returns `Err` if the currencies differ (never silently
    /// mixes units) or if the sum overflows i64.
    pub fn checked_add(self, other: Money) -> Result<Money, String> {
        if self.currency != other.currency {
            return Err(format!(
                "cross-currency add rejected: {} + {}",
                self.currency.code(),
                other.currency.code()
            ));
        }
        let minor = self
            .minor
            .checked_add(other.minor)
            .ok_or("money add overflow")?;
        Ok(Money {
            minor,
            currency: self.currency,
        })
    }
}


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

// ── Order-total mirror (RW-03 authority surface) ──────────────────────────────
// 1:1 port of `packages/ui/src/lib/money.ts` `computeDeliveryFee` + `estimateOrderTotal`.
// The SERVER (apps/api orders.ts fee ladder) stays the single source of truth for what is
// CHARGED; this mirror only drives what the client SEES (Approach M / ADR-0005). All amounts
// are integer minor units. Returns `None` (fee unknown) when the fee is server-only
// (distance-tiered) or delivery is unconfigured — the caller must degrade, never show a number
// it can't back.

#[derive(Clone, Copy)]
pub struct FeeConfig {
    pub is_pickup: bool,
    pub free_delivery_threshold: Option<i64>,
    pub delivery_fee_flat: Option<i64>,
    /// Distance-tiered fees are RLS-hidden from /info — client cannot compute them.
    pub has_distance_tiers: bool,
}

#[derive(Clone, Copy)]
pub struct OrderTotalConfig {
    pub fee: FeeConfig,
    pub tax_rate: f64,
    pub price_includes_tax: bool,
    pub min_order_value: Option<i64>,
}

#[derive(Clone, Copy)]
pub struct OrderTotalEstimate {
    /// True only when the delivery fee is computable client-side (flat/free/pickup).
    pub fee_known: bool,
    pub delivery_fee: Option<i64>,
    pub tax_total: i64,
    /// Authoritative-by-construction total, or None when the fee is unknown.
    pub total: Option<i64>,
    /// Mirrors server MIN_ORDER_NOT_MET gate (applies to pickup AND delivery).
    pub min_not_met: bool,
}

/// Mirror of money.ts `computeDeliveryFee` (orders.ts:528-560 ladder).
pub fn compute_delivery_fee(subtotal: i64, cfg: &FeeConfig) -> Option<i64> {
    if cfg.is_pickup {
        return Some(0);
    }
    if let Some(thr) = cfg.free_delivery_threshold {
        if subtotal >= thr {
            return Some(0);
        }
    }
    if cfg.has_distance_tiers {
        return None; // distance-based — server-only
    }
    if let Some(flat) = cfg.delivery_fee_flat {
        return Some(flat);
    }
    None // delivery not configured
}

/// Mirror of money.ts `estimateOrderTotal` (orders.ts:518-565). Tax is on the subtotal
/// (not subtotal+fee), matching the server.
pub fn estimate_order_total(subtotal: i64, cfg: &OrderTotalConfig) -> OrderTotalEstimate {
    let delivery_fee = compute_delivery_fee(subtotal, &cfg.fee);
    let tax_total = apply_tax(subtotal, cfg.tax_rate, cfg.price_includes_tax).unwrap_or(0);
    let min_not_met = match cfg.min_order_value {
        Some(min) => subtotal < min,
        None => false,
    };
    let fee_known = delivery_fee.is_some();
    let total = delivery_fee.map(|fee| subtotal + fee + tax_total);
    OrderTotalEstimate {
        fee_known,
        delivery_fee,
        tax_total,
        total,
        min_not_met,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── M5: currency guard — cross-currency add is fail-closed ──
    #[test]
    fn green_same_currency_add() {
        let a = Money::new(1000, Currency::All);
        let b = Money::new(250, Currency::All);
        assert_eq!(a.checked_add(b).unwrap(), Money::new(1250, Currency::All));
    }

    #[test]
    fn red_cross_currency_add_is_err() {
        let all = Money::new(1000, Currency::All);
        let eur = Money::new(1000, Currency::Eur);
        assert!(all.checked_add(eur).is_err(), "ALL + EUR must be rejected");
    }

    #[test]
    fn red_money_add_overflow_is_err() {
        let a = Money::new(i64::MAX, Currency::Usd);
        let b = Money::new(1, Currency::Usd);
        assert!(a.checked_add(b).is_err());
    }

    #[test]
    fn green_currency_code_roundtrip() {
        for c in [Currency::All, Currency::Eur, Currency::Usd] {
            assert_eq!(Currency::from_code(c.code()), Some(c));
        }
        assert_eq!(Currency::from_code("XXX"), None);
    }

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

    // ── RW-03 parity: kernel estimate_order_total == packages/ui/src/lib/money.ts ──
    // These mirror money.ts's documented behavior (fee ladder + min-order + tax on subtotal).
    // They are RED→GREEN: they must FAIL if the kernel ever diverges from the JS port,
    // proving the kernel is a safe authority before money.ts is deleted.
    fn cfg(
        is_pickup: bool,
        free_thr: Option<i64>,
        flat: Option<i64>,
        distance: bool,
        tax_rate: f64,
        incl: bool,
        min: Option<i64>,
    ) -> OrderTotalConfig {
        OrderTotalConfig {
            fee: FeeConfig {
                is_pickup,
                free_delivery_threshold: free_thr,
                delivery_fee_flat: flat,
                has_distance_tiers: distance,
            },
            tax_rate,
            price_includes_tax: incl,
            min_order_value: min,
        }
    }

    // Flat fee + 20% tax exclusive: 1000 + 200 fee + 200 tax = 1400
    #[test]
    fn green_parity_flat_fee_exclusive() {
        let r = estimate_order_total(1000, &cfg(false, None, Some(200), false, 0.20, false, None));
        assert!(r.fee_known);
        assert_eq!(r.delivery_fee, Some(200));
        assert_eq!(r.tax_total, 200);
        assert_eq!(r.total, Some(1400));
        assert!(!r.min_not_met);
    }

    // Free-over-threshold boundary (threshold 2000, subtotal 2000 → fee 0)
    #[test]
    fn green_parity_free_threshold_boundary() {
        let r = estimate_order_total(
            2000,
            &cfg(false, Some(2000), Some(200), false, 0.10, false, None),
        );
        assert_eq!(r.delivery_fee, Some(0));
        assert_eq!(r.tax_total, 200);
        assert_eq!(r.total, Some(2200));
    }

    // Pickup → fee 0, tax still applies
    #[test]
    fn green_parity_pickup() {
        let r = estimate_order_total(1500, &cfg(true, None, Some(200), false, 0.20, false, None));
        assert_eq!(r.delivery_fee, Some(0));
        assert_eq!(r.total, Some(1500 + 300));
    }

    // Distance-tiered → fee unknown → total None (caller must degrade)
    #[test]
    fn green_parity_distance_unknown() {
        let r = estimate_order_total(1000, &cfg(false, None, Some(200), true, 0.20, false, None));
        assert!(!r.fee_known);
        assert_eq!(r.delivery_fee, None);
        assert_eq!(r.total, None);
    }

    // Min-order gate (min 500, subtotal 400 → min_not_met)
    #[test]
    fn green_parity_min_not_met() {
        let r = estimate_order_total(
            400,
            &cfg(false, None, Some(200), false, 0.20, false, Some(500)),
        );
        assert!(r.min_not_met);
        assert_eq!(r.total, Some(400 + 200 + 80));
    }

    // Inclusive tax: 1200 inclusive at 20% → tax 200, total = 1200 + fee(0)
    #[test]
    fn green_parity_inclusive_tax() {
        let r = estimate_order_total(
            1200,
            &cfg(false, Some(9999), Some(0), false, 0.20, true, None),
        );
        assert_eq!(r.tax_total, 200);
        assert_eq!(r.total, Some(1400)); // money.ts always adds tax_total to subtotal+fee
    }
}
