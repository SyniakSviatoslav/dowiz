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


use alloc::string::{String, ToString};
use alloc::vec::Vec;
/// Reject non-integer amounts (money is integer minor units).
///
/// NOTE (BP-17): the old code guarded `amount != amount` (a NaN check) — dead on `i64`,
// Single scale authority (promoted from private `SCALE`, BLUEPRINT-P-A §2/A3).
pub const MONEY_SCALE_MICRO: i128 = 1_000_000;

/// THE RATE, and the tax law it feeds. The implementation lives in
/// [`crate::tax`] so this file stays under the 300-line cap; it is re-exported
/// here because `money::tax_of` / `money::summarise` is where the blueprint
/// (§3.1, §3.2) and every caller look for it, and because `kernel/src/money.rs`
/// is `pub use dowiz_core::money::*;` — one re-export carries the whole law
/// into the kernel's namespace unchanged.
pub use crate::tax::{summarise, tax_of, RatePpm, TaxGroup, TaxInput, TaxLine, TaxSummary};

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
    /// Every currency the product renders, in one place: the rates endpoint and
    /// `tools/gen-vocab` walk this, so a fourth currency is added here or nowhere.
    pub const EVERY: [Currency; 3] = [Currency::All, Currency::Eur, Currency::Usd];

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
    /// Minor units (decimal places) for this currency.
    ///
    /// Returns how many decimal places the minor unit represents.
    /// ALL (Albanian lek) has 0 decimal places; EUR and USD have 2.
    pub fn minor_units(self) -> u32 {
        match self {
            Currency::All => 0,
            Currency::Eur => 2,
            Currency::Usd => 2,
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

// `apply_tax(subtotal, f64, incl)` — the f64 adapter over the tax law — is
// DELETED (blueprint §6 item 8). The law is [`crate::tax::tax_of`], taking a
// [`crate::tax::RatePpm`]; a decimal rate off the wire becomes ppm only at a
// parse edge (`json_bridge::field_rate_ppm`), which refuses non-finite,
// negative and above-100 % rates.

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

/// ALL→EUR display conversion (shared-types utils.ts `formatMoney`). Integer only.
/// `rate_ppm` is the EUR-per-ALL rate in parts per million (0.01 = `10_000`);
/// returns EUR cents. `rate_ppm <= 0` is refused.
///
/// The rounding is `eqc_gen::apply_tax_exclusive_int`'s DivHalfUp, copied:
/// `(amount · rate_ppm · 100 + 10⁶/2) / 10⁶` in `i128`, every product checked,
/// the `i128 → i64` narrowing range-checked (BP-17). It used to take an `f64`
/// rate scaled to 10⁹; the integer basis is the one `services/ordering/rates.rs`
/// already ships.
pub fn convert_all_to_eur_cents(amount_all: i64, rate_ppm: i64) -> Result<i64, String> {
    if rate_ppm <= 0 {
        return Err("rate must be > 0".into());
    }
    let b = 1_000_000i128;
    let prod = (amount_all as i128)
        .checked_mul(rate_ppm as i128)
        .and_then(|v| v.checked_mul(100))
        .ok_or("EUR conversion overflow")?;
    i64::try_from((prod + b / 2) / b).map_err(|_| "EUR conversion overflow".into())
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
    pub tax_rate: crate::tax::RatePpm,
    pub price_includes_tax: bool,
    pub min_order_value: Option<i64>,
}

#[derive(Clone, Copy)]
pub struct OrderTotalEstimate {
    /// True only when the delivery fee is computable client-side (flat/free/pickup).
    pub fee_known: bool,
    pub delivery_fee: Option<i64>,
    /// Tax on the subtotal in minor units, or `None` when it can't be computed (a
    /// pathological `subtotal × rate` overflows i64 in `tax_of`). Fail-closed like
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
    let tax_total = crate::tax::tax_of(subtotal, cfg.tax_rate, cfg.price_includes_tax).ok();
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
    use crate::tax::{tax_of, RatePpm};

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
    // Expected values recorded from the deleted f64 `apply_tax` before it went.
    #[test]
    fn green_tax_added_exclusive() {
        // 1000 minor units, 20% tax → 200
        assert_eq!(tax_of(1000, RatePpm(200_000), false).unwrap(), 200);
    }
    #[test]
    fn green_tax_inclusive_net() {
        // 1200 inclusive at 20% → tax = 1200 - net(1000) = 200
        assert_eq!(tax_of(1200, RatePpm(200_000), true).unwrap(), 200);
    }
    #[test]
    fn green_zero_subtotal_or_rate() {
        assert_eq!(tax_of(0, RatePpm(200_000), false).unwrap(), 0);
        assert_eq!(tax_of(1000, RatePpm(0), false).unwrap(), 0);
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
            tax_rate: RatePpm(200_000),
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
        // pathological: huge subtotal × 200 % → tax ≈ i64::MAX*2 exceeds i64.
        // 2_000_000 ppm is above `RatePpm::MAX` — unreachable from the parse
        // edge, reachable through the pub field, and the law still refuses it.
        let r = tax_of(i64::MAX, RatePpm(2_000_000), false);
        assert!(r.is_err(), "tax overflow must be Err, got {:?}", r);
    }
    #[test]
    fn green_tax_at_the_same_rate_on_a_sane_subtotal_is_ok() {
        assert_eq!(tax_of(1000, RatePpm(2_000_000), false).unwrap(), 2000);
    }

    // A NEGATIVE rate is no longer expressible (`RatePpm` is unsigned, and the
    // parse edge refuses it); the organ's own denominator guard is still pinned.
    #[test]
    fn red_organ_negative_rate_is_err_not_divzero() {
        // V3 1.4: rate_micro <= -MONEY_SCALE_MICRO makes the inclusive denominator
        // <= 0 → pre-fix this was a div-by-zero panic. Now refused as Err.
        let r = crate::eqc_gen::apply_tax_inclusive_int(1000, -2_000_000);
        assert!(r.is_err(), "negative effective rate must be Err, got {:?}", r);
    }

    // ── The values the deleted f64 `apply_tax` produced, recorded before the
    //    deletion (lane F64, 2026-09-23) and pinned against `tax_of` / the organ.
    //    The f64 rate is shown beside each ppm it became. ──
    #[test]
    fn tax_of_reproduces_every_recorded_f64_era_value() {
        const RECORDED: &[(i64, u32, bool, Option<i64>)] = &[
            (1000, 200_000, false, Some(200)),  // 0.20
            (1200, 200_000, true, Some(200)),   // 0.20 inclusive
            (0, 200_000, false, Some(0)),       // 0.20
            (1000, 0, false, Some(0)),          // 0.0
            (1000, 100_000, false, Some(100)),  // 0.10
            (1300, 200_000, false, Some(260)),  // 0.20
            (2000, 100_000, false, Some(200)),  // 0.10
            (1500, 200_000, false, Some(300)),  // 0.20
            (400, 200_000, false, Some(80)),    // 0.20
            (i64::MAX - 1, 200_000, false, Some(1_844_674_407_370_955_161)),
            (i64::MAX, 2_000_000, false, None), // 2.0 → Err (overflow)
            (i64::MAX, 2_000_000, true, Some(6_148_914_691_236_517_205)), // 2.0 inclusive
        ];
        for &(sub, ppm, incl, want) in RECORDED {
            let got = tax_of(sub, RatePpm(ppm), incl);
            assert_eq!(got.ok(), want, "sub={sub} ppm={ppm} incl={incl}");
        }
        // Negative rates: recorded through the organ, which still accepts a raw
        // micro rate. -2.0 exclusive was Ok(-1999) — a NEGATIVE tax the f64
        // adapter used to hand back; the parse edge now refuses that rate.
        assert_eq!(crate::eqc_gen::apply_tax_exclusive_int(1000, -2_000_000), Ok(-1999));
        assert!(crate::eqc_gen::apply_tax_inclusive_int(1000, -2_000_000).is_err());
    }

    #[test]
    fn red_eur_conversion_overflow_is_err() {
        let r = convert_all_to_eur_cents(i64::MAX, 1_000_000);
        assert!(r.is_err(), "EUR overflow must be Err, got {:?}", r);
    }

    // ── GREEN: ALL→EUR conversion integer math ──
    #[test]
    fn green_all_to_eur() {
        // 1000 ALL at rate 0.01 (100 ALL = 1 EUR) → 10 EUR => 1000 cents
        assert_eq!(convert_all_to_eur_cents(1000, 10_000).unwrap(), 1000);
        // Recorded from the f64 path (rate 0.0075): half-up, truncating toward
        // zero on the negative side exactly as before.
        assert_eq!(convert_all_to_eur_cents(100_000, 7_500).unwrap(), 75_000);
        assert_eq!(convert_all_to_eur_cents(-100_000, 7_500).unwrap(), -74_999);
    }

    #[test]
    fn red_eur_conversion_zero_or_negative_rate_is_refused() {
        assert_eq!(
            convert_all_to_eur_cents(100_000, 0).unwrap_err(),
            "rate must be > 0"
        );
        assert!(convert_all_to_eur_cents(100_000, -1).is_err());
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
        tax_rate: RatePpm,
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
        let r = estimate_order_total(1000, &cfg(false, None, Some(200), false, RatePpm(200_000), false, None));
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
            &cfg(false, Some(2000), Some(200), false, RatePpm(100_000), false, None),
        );
        assert_eq!(r.delivery_fee, Some(0));
        assert_eq!(r.tax_total, Some(200));
        assert_eq!(r.total, Some(2200));
    }

    // Pickup → fee 0, tax still applies
    #[test]
    fn green_parity_pickup() {
        let r = estimate_order_total(1500, &cfg(true, None, Some(200), false, RatePpm(200_000), false, None));
        assert_eq!(r.delivery_fee, Some(0));
        assert_eq!(r.total, Some(1500 + 300));
    }

    // Distance-tiered → fee unknown → total None (caller must degrade)
    #[test]
    fn green_parity_distance_unknown() {
        let r = estimate_order_total(1000, &cfg(false, None, Some(200), true, RatePpm(200_000), false, None));
        assert!(!r.fee_known);
        assert_eq!(r.delivery_fee, None);
        assert_eq!(r.total, None);
    }

    // Min-order gate (min 500, subtotal 400 → min_not_met)
    #[test]
    fn green_parity_min_not_met() {
        let r = estimate_order_total(
            400,
            &cfg(false, None, Some(200), false, RatePpm(200_000), false, Some(500)),
        );
        assert!(r.min_not_met);
        assert_eq!(r.total, Some(400 + 200 + 80));
    }

    // Inclusive tax: 1200 inclusive at 20% → tax 200, total = 1200 + fee(0)
    #[test]
    fn green_parity_inclusive_tax() {
        let r = estimate_order_total(
            1200,
            &cfg(false, Some(9999), Some(0), false, RatePpm(200_000), true, None),
        );
        assert_eq!(r.tax_total, Some(200));
        assert_eq!(r.total, Some(1400)); // money.ts always adds tax_total to subtotal+fee
    }

    // ── RED→GREEN (Phase 7 §6, BLUEPRINT-P07): tax overflow must FAIL CLOSED ──
    // The f64 era used subtotal 1000 × rate 1e17; a rate that size is refused at
    // the parse edge now (`json_bridge`), so the overflow is reached with the
    // largest `RatePpm` on a large subtotal. Recorded expectation unchanged:
    // fee Some(200), tax None, total None.
    // Pre-fix `.unwrap_or(0)`: tax_total=0, total=Some(1200) — a fabricated zero-tax total
    // the estimator cannot back. Post-fix: the estimate degrades exactly as it does for an
    // unknown (distance-tiered) fee — tax_total=None, total=None. Never a wrong number.
    #[test]
    fn red_tax_overflow_degrades_estimate_to_none() {
        let r = estimate_order_total(
            i64::MAX / 2,
            &cfg(false, None, Some(200), false, RatePpm(u32::MAX), false, None),
        );
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
        let ledger = ledger_append(Vec::new(), earn).unwrap();
        let dup = LedgerEntry {
            id: 1, // same id
            kind: EntryKind::Earn,
            amount: Money::new(200, Currency::All),
            reverses: None,
        };
        assert!(ledger_append(ledger.clone(), dup).is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Property tests — Flow 1: Payment / Value Transfer invariants
    // ═══════════════════════════════════════════════════════════════════════════

    /// prop-1: Roundtrip — create → credit → debit → balance == expected.
    #[test]
    fn prop_roundtrip_credit_debit_balance_equals_expected() {
        let zero = Money::new(0, Currency::Eur);
        let credit = Money::new(5000, Currency::Eur);
        let debit = Money::new(-2000, Currency::Eur);
        let step1 = zero.checked_add(credit).unwrap();
        assert_eq!(step1.minor, 5000, "after credit, balance must be 5000");
        let step2 = step1.checked_add(debit).unwrap();
        assert_eq!(
            step2,
            Money::new(3000, Currency::Eur),
            "credit 5000 then debit 2000 => balance 3000"
        );
        let step3 = step2.checked_add(Money::new(-3000, Currency::Eur)).unwrap();
        assert_eq!(
            step3,
            Money::new(0, Currency::Eur),
            "full debit to zero — balance must be exactly 0"
        );
    }

    /// prop-2: Overflow guard — credit MAX then credit 1 ⇒ Err (NOT wrap).
    #[test]
    fn prop_overflow_guard_saturates_error_not_wrap() {
        let max = Money::new(i64::MAX, Currency::All);
        let one = Money::new(1, Currency::All);
        assert!(
            max.checked_add(one).is_err(),
            "i64::MAX + 1 must return Err — never wrap to i64::MIN"
        );
        let min = Money::new(i64::MIN, Currency::All);
        assert!(
            min.checked_sub(one).is_err(),
            "i64::MIN - 1 must return Err — never wrap"
        );
    }

    /// prop-3: Cross-currency rejection — credit USD to EUR wallet ⇒ rejected.
    #[test]
    fn prop_cross_currency_credit_usd_to_eur_rejected() {
        let eur = Money::new(1000, Currency::Eur);
        let usd = Money::new(500, Currency::Usd);
        assert!(
            eur.checked_add(usd).is_err(),
            "EUR + USD must be rejected — cross-currency never silently mixes"
        );
        assert!(
            usd.checked_add(eur).is_err(),
            "USD + EUR must be rejected symmetrically"
        );
    }

    /// prop-4: Negative rejection — `assert_non_negative` refuses totals < 0.
    #[test]
    fn prop_negative_amount_rejected_by_non_negative_guard() {
        assert!(
            assert_non_negative(-1).is_err(),
            "negative total -1 must be rejected"
        );
        assert!(
            assert_non_negative(-1_000_000).is_err(),
            "negative total -1000000 must be rejected"
        );
        assert!(assert_non_negative(0).is_ok(), "zero is non-negative");
        assert!(assert_non_negative(1).is_ok(), "positive accepted");
    }

    /// prop-5: No double-spend — second reversal of same earn leg ⇒ rejected.
    #[test]
    fn prop_double_spend_same_earn_leg_second_reversal_rejected() {
        let earn = LedgerEntry {
            id: 10,
            kind: EntryKind::Earn,
            amount: Money::new(700, Currency::All),
            reverses: None,
        };
        let ledger = ledger_append(Vec::new(), earn).unwrap();
        let ledger = reverse_transfer(ledger, 10, 20).unwrap();
        let result = reverse_transfer(ledger, 10, 30);
        assert!(
            result.is_err(),
            "second reversal of the same earn leg must be rejected — no double-spend"
        );
    }

    /// prop-6: Zero invariant — empty ledger sums to zero.
    #[test]
    fn prop_zero_invariant_empty_ledger_sums_to_zero() {
        let empty: Vec<LedgerEntry> = Vec::new();
        assert_eq!(ledger_sum(&empty), 0, "empty ledger must sum to 0");
    }

    /// prop-7: Idempotent credit — same entry id twice ⇒ rejected.
    #[test]
    fn prop_idempotent_credit_same_entry_id_twice_rejected() {
        let e1 = LedgerEntry {
            id: 1,
            kind: EntryKind::Earn,
            amount: Money::new(100, Currency::Eur),
            reverses: None,
        };
        let ledger = ledger_append(Vec::new(), e1).unwrap();
        let e2 = LedgerEntry {
            id: 1,
            kind: EntryKind::Earn,
            amount: Money::new(100, Currency::Eur),
            reverses: None,
        };
        assert!(
            ledger_append(ledger, e2).is_err(),
            "duplicate entry id must be rejected — idempotent credit"
        );
    }

    /// prop-8: Deterministic equality — same minor+currency ⇒ equal.
    #[test]
    fn prop_deterministic_money_equality() {
        let a = Money::new(42, Currency::Eur);
        let b = Money::new(42, Currency::Eur);
        assert_eq!(a, b, "same minor + same currency => equal");
        assert_ne!(a, Money::new(42, Currency::Usd), "cross-currency => not equal");
        assert_ne!(a, Money::new(43, Currency::Eur), "different amount => not equal");
        assert_eq!(a, a, "Copy must preserve equality");
    }
}
