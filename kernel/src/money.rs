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

// Single scale authority (promoted from private `SCALE`, BLUEPRINT-P-A §2/A3).
pub const MONEY_SCALE_MICRO: i128 = 1_000_000;

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

    /// Additive inverse — the compensating credit of a debit (P07 reversal primitive).
    /// Fail-closed on `i64::MIN` (its negation overflows i64): `checked_neg` returns `Err`
    /// rather than wrapping to `i64::MIN` (UB-adjacent; S5 fail-closed).
    pub fn checked_neg(self) -> Result<Money, String> {
        let minor = self
            .minor
            .checked_neg()
            .ok_or("money neg overflow (i64::MIN has no additive inverse)")?;
        Ok(Money {
            minor,
            currency: self.currency,
        })
    }

    /// Subtract two amounts. Cross-currency fail-closed (same M5 guard as `checked_add`),
    /// then `checked_sub` on `minor` (never wraps).
    pub fn checked_sub(self, other: Money) -> Result<Money, String> {
        if self.currency != other.currency {
            return Err(format!(
                "cross-currency sub rejected: {} - {}",
                self.currency.code(),
                other.currency.code()
            ));
        }
        let minor = self
            .minor
            .checked_sub(other.minor)
            .ok_or("money sub overflow")?;
        Ok(Money {
            minor,
            currency: self.currency,
        })
    }
}

/// ── P07 double-entry ledger + reversal primitive ──────────────────────────────
/// RED LINE: money movements are modelled as a per-order double-entry ledger. Every
/// `Earn` (debit/credit) leg has an exact compensation (`Reversal`) produced by
/// `reversed_leg`, so a compensated order's entries sum to EXACTLY zero by construction:
///
///   entry(m).amount.checked_add(reversed_leg(m).amount).unwrap() == Money::new(0, m.currency)
///
/// Conservation invariant: `ledger_sum(entries)` (Σ of `amount.minor` for entries that are
/// NOT reversed) must equal 0 at every compensated terminal state. The compensating credit
/// of a debit is defined as the `checked_neg` of the original — a debit and its reversal net
/// to zero. This is the kernel's money-correctness falsifier.

/// The kind of a ledger entry (one side of a double entry).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// Money earned into the platform (a debit from customer / credit to platform).
    Earn,
    /// The exact compensation of a prior `Earn` leg — amount = `checked_neg` of the original.
    /// Conserves the ledger (Σ == 0 with its paired earn leg).
    Reversal,
}

/// A single ledger entry. `id` is a caller-assigned stable key (used for idempotency + for a
/// `Reversal` to name the `Earn` it reverses via `reverses`). Fail-closed: a `Reversal` MUST
/// name an existing `Earn` id, and a given earn leg may be reversed at most once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedgerEntry {
    pub id: u64,
    pub kind: EntryKind,
    pub amount: Money,
    /// For `Reversal`: the `id` of the `Earn` leg being compensated. `None` for an `Earn`.
    pub reverses: Option<u64>,
}

/// Build the exact compensating `Reversal` of an `Earn` leg. The reversal's amount is the
/// `checked_neg` of the earn's amount, so the pair nets to exactly zero. The reversal references
/// the earn's `id` so it can be replayed/idempotently matched.
///
/// Fail-closed: an `Earn` leg with `amount.minor == i64::MIN` cannot be reversed
/// (`checked_neg` overflows) → this returns `Err` rather than fabricating a non-cancelling credit.
pub fn reversed_leg(earn: &LedgerEntry, reversal_id: u64) -> Result<LedgerEntry, String> {
    if earn.kind != EntryKind::Earn {
        return Err("reversed_leg requires an Earn leg".into());
    }
    let neg = earn.amount.checked_neg()?;
    Ok(LedgerEntry {
        id: reversal_id,
        kind: EntryKind::Reversal,
        amount: neg,
        reverses: Some(earn.id),
    })
}

/// Append `entry` to `ledger`, enforcing the reversal Law fail-closed:
/// * cross-currency / arithmetic overflow inside `Money` is rejected (S5).
/// * a `Reversal` must name an existing `Earn` id present in the ledger.
/// * an `Earn` leg may be reversed at most once (a second reversal is rejected → idempotent
///   replay is a no-op at the caller, but a *distinct* second reversal is refused here).
/// * duplicate `id` insertions are rejected (replay protection at the ledger level).
///
/// Returns the (possibly extended) ledger. `ledger` is owned so callers thread it through.
pub fn ledger_append(
    mut ledger: Vec<LedgerEntry>,
    entry: LedgerEntry,
) -> Result<Vec<LedgerEntry>, String> {
    // Duplicate id → reject (replay must not re-append; caller should detect first).
    if ledger.iter().any(|e| e.id == entry.id) {
        return Err(format!("ledger: duplicate entry id {}", entry.id));
    }
    match entry.kind {
        EntryKind::Earn => {
            // Earn is always accepted (its amount was already checked by the caller via Money).
            ledger.push(entry);
        }
        EntryKind::Reversal => {
            let target = entry
                .reverses
                .ok_or("reversal must name the earn leg it compensates")?;
            let earn = ledger
                .iter()
                .find(|e| e.id == target && e.kind == EntryKind::Earn)
                .ok_or_else(|| format!("reversal targets unknown earn leg {target}"))?;
            // Fail-closed: reversal amount must be the exact negation of the earn (no silent drift).
            let expected = earn.amount.checked_neg()?;
            if entry.amount != expected {
                return Err(format!(
                    "reversal amount {} != -earn {} (conservation violated)",
                    entry.amount.minor, expected.minor
                ));
            }
            // At most one reversal per earn leg.
            if ledger.iter().any(|e| e.reverses == Some(target)) {
                return Err(format!(
                    "earn leg {target} already reversed (idempotent once)"
                ));
            }
            ledger.push(entry);
        }
    }
    Ok(ledger)
}

/// Sum the `minor` units of all entries that are NOT themselves reversed. A compensated terminal
/// order (Earn + its Reversal) sums to exactly 0; an uncompensated earn sums to its amount.
///
/// This is the conservation probe: returns `Ok(0)` iff the ledger nets to zero.
pub fn ledger_sum(ledger: &[LedgerEntry]) -> i64 {
    ledger
        .iter()
        // A reversed earn leg is excluded from the live balance (its credit cancelled it);
        // the Reversal entry itself nets the same amount to zero, so it is also excluded.
        .filter(|e| !matches!(e.kind, EntryKind::Reversal))
        .filter(|e| {
            if e.kind == EntryKind::Earn {
                !ledger.iter().any(|r| r.reverses == Some(e.id))
            } else {
                true
            }
        })
        .map(|e| e.amount.minor)
        .sum()
}

/// Fail-closed compensation driver: given a ledger and an earn leg id, produce and append the
/// exact reversal. Idempotent — calling on an already-reversed earn leg returns `Err`
/// ("already reversed"), which the caller treats as a no-op (replay = no-op).
///
/// Rejects: unknown earn leg, overflow on `checked_neg`, or an already-reversed leg.
pub fn reverse_transfer(
    ledger: Vec<LedgerEntry>,
    earn_id: u64,
    reversal_id: u64,
) -> Result<Vec<LedgerEntry>, String> {
    let earn = ledger
        .iter()
        .find(|e| e.id == earn_id && e.kind == EntryKind::Earn)
        .ok_or_else(|| format!("reverse_transfer: unknown earn leg {earn_id}"))?;
    let rev = reversed_leg(earn, reversal_id)?;
    ledger_append(ledger, rev)
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
        // net = round(sub * MONEY_SCALE_MICRO / (MONEY_SCALE_MICRO + rate)); tax = sub - net
        // V3 1.4 (ROUND-2 GAP-AUDIT): a negative effective rate makes
        // `MONEY_SCALE_MICRO + rate_micro <= 0`, turning the half-up division
        // below into a div-by-zero panic. Refuse non-positive denominators.
        let denom = MONEY_SCALE_MICRO + rate_micro;
        if denom <= 0 {
            return Err("apply_tax: negative effective tax rate (denominator <= 0)".into());
        }
        let net = (sub * MONEY_SCALE_MICRO + denom / 2) / denom; // half-up
        sub - net
    } else {
        // tax = round(sub * rate / MONEY_SCALE_MICRO)
        // V3 1.6 (ROUND-2 GAP-AUDIT): `sub * rate_micro` can overflow i128 when
        // `tax_rate` is pathologically large (rate_micro saturates toward i128::MAX).
        // Use checked arithmetic so it returns Err instead of panicking.
        let prod = sub
            .checked_mul(rate_micro)
            .ok_or("apply_tax: subtotal * rate overflows i128")?;
        (prod + MONEY_SCALE_MICRO / 2) / MONEY_SCALE_MICRO // half-up
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
    /// Tax on the subtotal in minor units, or `None` when it can't be computed (a
    /// pathological `subtotal × rate` overflows i64 in `apply_tax`). Fail-closed like
    /// `delivery_fee`: the caller must degrade, never show a fabricated zero-tax total.
    pub tax_total: Option<i64>,
    /// Authoritative-by-construction total, or `None` when the fee OR the tax is unknown.
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
    // Fail-closed: a tax-computation failure (overflow) is `None`, NOT a silent zero.
    // `.ok()` mirrors the fee-unknown degrade — the estimate cannot back a number it
    // couldn't compute, so both `tax_total` and `total` degrade to `None`.
    let tax_total = apply_tax(subtotal, cfg.tax_rate, cfg.price_includes_tax).ok();
    let min_not_met = match cfg.min_order_value {
        Some(min) => subtotal < min,
        None => false,
    };
    let fee_known = delivery_fee.is_some();
    let total = match (delivery_fee, tax_total) {
        // V3 1.5 (ROUND-2 GAP-AUDIT): `subtotal + fee + tax` is unchecked i64 add —
        // a near-i64::MAX subtotal overflows (panic in debug / wrap in release).
        // Use checked_add so overflow degrades to `None` (fail-closed), consistent
        // with the tax-computation failure path above.
        (Some(fee), Some(tax)) => subtotal.checked_add(fee).and_then(|s| s.checked_add(tax)),
        _ => None,
    };
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

    // ── M2 (ROUND-2 GAP-AUDIT V3 1.5): estimate_order_total must not overflow
    //    the `subtotal + fee + tax` sum. A near-i64::MAX subtotal degrades the
    //    total to `None` (fail-closed), never panics/wraps. ──
    #[test]
    fn red_estimate_order_total_overflow_degrades_to_none() {
        let cfg = OrderTotalConfig {
            tax_rate: 0.20,
            price_includes_tax: false,
            fee: FeeConfig {
                is_pickup: false,
                delivery_fee_flat: Some(100),
                free_delivery_threshold: None,
                has_distance_tiers: false,
            },
            min_order_value: None,
        };
        // subtotal one short of i64::MAX; +100 fee + ~20% tax overflows i64.
        let est = estimate_order_total(i64::MAX - 1, &cfg);
        assert_eq!(
            est.total, None,
            "overflow must degrade total to None (fail-closed), not panic/wrap"
        );
        // A sane subtotal still computes a concrete total.
        let ok = estimate_order_total(1000, &cfg);
        assert_eq!(ok.total, Some(1000 + 100 + 200));
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

    // ── M1 (ROUND-2 GAP-AUDIT V3 1.4 / 1.6): apply_tax must not panic. ──
    #[test]
    fn red_tax_negative_rate_is_err_not_divzero() {
        // V3 1.4: rate_micro <= -MONEY_SCALE_MICRO makes the inclusive denominator
        // <= 0 → pre-fix this was a div-by-zero panic. Now refused as Err.
        let r = apply_tax(1000, -2.0, true);
        assert!(
            r.is_err(),
            "negative effective rate must be Err, got {:?}",
            r
        );
    }

    #[test]
    fn red_tax_i128_overflow_is_err_not_panic() {
        // V3 1.6: a pathologically large rate saturates rate_micro toward i128::MAX;
        // pre-fix `sub * rate_micro` overflowed i128 (panicked in release). Now Err.
        let r = apply_tax(1_000_000_000_000, 1e15, false);
        assert!(r.is_err(), "i128 overflow must be Err, got {:?}", r);
    }

    // ── A3: money-law SHADOW organ exact-integer parity pin (BLUEPRINT-P-A §3.3) ──
    // The generated organs (crate::eqc_gen::apply_tax_{exclusive,inclusive}_int) are a
    // verbatim transcription of the equations of truth; this test pins them against the
    // hand-written `apply_tax` (the still-authoritative law — the authority flip is
    // R-4-gated, NOT done here). Exact-integer equality, no tolerance. Any mismatch is RED.
    #[test]
    fn apply_tax_generated_parity_exact_integers() {
        // Every existing apply_tax corpus case from money.rs:454-495, reused as fixtures.
        const MONEY_TAX_FIXTURES: &[(i64, f64, bool)] = &[
            (1000, 0.20, false),    // green_tax_added_exclusive → 200
            (1200, 0.20, true),     // green_tax_inclusive_net → 200
            (0, 0.20, false),       // green_zero_subtotal_or_rate → 0
            (1000, 0.0, false),     // green_zero_subtotal_or_rate → 0
            (i64::MAX, 2.0, false), // red_tax_overflow_is_err → Err
            // FEYNMAN-10: the negative-rate edge the parity suite used to skip.
            // The law (apply_tax) refuses denom ≤ 0; the generated organs must
            // refuse the same — both directions, so a future authority flip to
            // the generated organ cannot silently change red-line behavior.
            (1000, -2.0, false), // red negative effective rate ⇒ Err
            (1000, -2.0, true),  // red negative effective rate (inclusive) ⇒ Err
        ];
        for &(sub, rate, incl) in MONEY_TAX_FIXTURES {
            // Same boundary conversion as apply_tax (money.rs:275).
            let rate_micro = (rate * 1_000_000.0).round() as i64;
            let want = apply_tax(sub, rate, incl);
            let got = if incl {
                crate::eqc_gen::apply_tax_inclusive_int(sub, rate_micro)
            } else {
                crate::eqc_gen::apply_tax_exclusive_int(sub, rate_micro)
            };
            match want {
                Ok(v) => assert_eq!(
                    got.unwrap(),
                    v,
                    "parity mismatch at (sub={sub}, rate={rate}, incl={incl})"
                ),
                Err(_) => assert!(
                    got.is_err(),
                    "both must refuse at (sub={sub}, rate={rate}, incl={incl}); got {got:?}"
                ),
            }
        }

        // Adversarial overflow sweep: the EXCLUSIVE organ at sub=i64::MAX,
        // rate_micro=2_000_000 MUST return Err (the half-up product overflows i64) —
        // never wrap. The INCLUSIVE organ can never overflow the final i64 narrowing
        // (tax = sub - net ≤ sub ≤ i64::MAX for non-negative rate, and sub*s fits i128),
        // so it returns Ok there and must equal the law exactly. Both paths are
        // fail-closed: whatever they return is the true, in-range value (no silent wrap).
        let got_excl = crate::eqc_gen::apply_tax_exclusive_int(i64::MAX, 2_000_000);
        let got_incl = crate::eqc_gen::apply_tax_inclusive_int(i64::MAX, 2_000_000);
        assert!(
            got_excl.is_err(),
            "exclusive organ must refuse overflow, got {got_excl:?}"
        );
        let want_incl = apply_tax(i64::MAX, 2.0, true);
        assert_eq!(
            got_incl,
            want_incl.map_err(|_| "tax overflow: subtotal * rate exceeds i64"),
            "inclusive organ must match the law at the i64::MAX boundary (no wrap)"
        );

        // Property grid: divergence-hunting sweep over the integer basis. Any single
        // mismatch is RED — this is the test *designed to break* the transcription.
        let subs = [0i64, 1, 999, 1_000_000, i64::MAX / 2];
        let rates = [0i64, 1, 200_000, 999_999, -2_000_000];
        for &sub in subs.iter() {
            for &rate_micro in rates.iter() {
                // f64 rate round-trips the micro basis for the hand-written law.
                let rate_f = rate_micro as f64 / 1_000_000.0;
                let want_excl = apply_tax(sub, rate_f, false);
                let want_incl = apply_tax(sub, rate_f, true);
                let got_excl = crate::eqc_gen::apply_tax_exclusive_int(sub, rate_micro);
                let got_incl = crate::eqc_gen::apply_tax_inclusive_int(sub, rate_micro);
                match want_excl {
                    Ok(v) => assert_eq!(
                        got_excl.unwrap(),
                        v,
                        "excl grid mismatch at sub={sub} rate_micro={rate_micro}"
                    ),
                    Err(_) => assert!(
                        got_excl.is_err(),
                        "excl grid both-refuse at sub={sub} rate_micro={rate_micro}; got {got_excl:?}"
                    ),
                }
                match want_incl {
                    Ok(v) => assert_eq!(
                        got_incl.unwrap(),
                        v,
                        "incl grid mismatch at sub={sub} rate_micro={rate_micro}"
                    ),
                    Err(_) => assert!(
                        got_incl.is_err(),
                        "incl grid both-refuse at sub={sub} rate_micro={rate_micro}; got {got_incl:?}"
                    ),
                }
            }
        }
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
        assert_eq!(r.tax_total, Some(200));
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
        assert_eq!(r.tax_total, Some(200));
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
        assert_eq!(r.tax_total, Some(200));
        assert_eq!(r.total, Some(1400)); // money.ts always adds tax_total to subtotal+fee
    }

    // ── RED→GREEN (Phase 7 §6, BLUEPRINT-P07): tax overflow must FAIL CLOSED ──
    // `apply_tax` overflows i64 here (subtotal 1000 × rate 1e17 far exceeds i64::MAX).
    // Pre-fix `.unwrap_or(0)`: tax_total=0, total=Some(1200) — a fabricated zero-tax total
    // the estimator cannot back. Post-fix: the estimate degrades exactly as it does for an
    // unknown (distance-tiered) fee — tax_total=None, total=None. Never a wrong number.
    #[test]
    fn red_tax_overflow_degrades_estimate_to_none() {
        let r = estimate_order_total(1000, &cfg(false, None, Some(200), false, 1e17, false, None));
        assert!(
            r.fee_known,
            "flat 200 fee is computable — the fee side is known"
        );
        assert_eq!(r.delivery_fee, Some(200));
        assert_eq!(
            r.tax_total, None,
            "tax overflow marks the tax field unknown, never a false zero"
        );
        assert_eq!(
            r.total, None,
            "on tax overflow the total must degrade to None, never a fabricated number"
        );
    }

    // ── P07 RED→GREEN: reversal primitive — m + neg(m) == 0, fail-closed ──
    #[test]
    fn green_checked_neg_nets_to_zero() {
        let m = Money::new(5000, Currency::All);
        let neg = m.checked_neg().unwrap();
        assert_eq!(m.checked_add(neg).unwrap(), Money::new(0, Currency::All));
    }

    #[test]
    fn red_checked_neg_min_overflow_is_err() {
        assert!(Money::new(i64::MIN, Currency::All).checked_neg().is_err());
    }

    #[test]
    fn green_checked_sub_same_currency() {
        let a = Money::new(1000, Currency::Eur);
        let b = Money::new(300, Currency::Eur);
        assert_eq!(a.checked_sub(b).unwrap(), Money::new(700, Currency::Eur));
    }

    #[test]
    fn red_checked_sub_cross_currency_is_err() {
        let a = Money::new(1000, Currency::Usd);
        let b = Money::new(1000, Currency::All);
        assert!(a.checked_sub(b).is_err());
    }

    // ── P07 RED→GREEN: ledger double-entry + reversal conservation ──
    #[test]
    fn green_ledger_earn_then_reversal_sums_to_zero() {
        let earn = LedgerEntry {
            id: 1,
            kind: EntryKind::Earn,
            amount: Money::new(1300, Currency::All),
            reverses: None,
        };
        let ledger = ledger_append(Vec::new(), earn).unwrap();
        assert_eq!(ledger_sum(&ledger), 1300);
        let ledger = reverse_transfer(ledger, 1, 2).unwrap();
        assert_eq!(
            ledger_sum(&ledger),
            0,
            "earn + reversal nets to exactly zero"
        );
    }

    #[test]
    fn red_ledger_reversal_unknown_earn_is_err() {
        assert!(reverse_transfer(Vec::new(), 42, 43).is_err());
    }

    #[test]
    fn red_ledger_duplicate_entry_id_is_err() {
        let earn = LedgerEntry {
            id: 1,
            kind: EntryKind::Earn,
            amount: Money::new(100, Currency::All),
            reverses: None,
        };
        let mut ledger = ledger_append(Vec::new(), earn).unwrap();
        let dup = LedgerEntry {
            id: 1, // same id
            kind: EntryKind::Earn,
            amount: Money::new(200, Currency::All),
            reverses: None,
        };
        assert!(ledger_append(ledger.clone(), dup).is_err());
    }
}
