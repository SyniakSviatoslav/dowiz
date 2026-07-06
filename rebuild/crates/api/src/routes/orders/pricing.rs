//! S5 money composition — the **thin shell adapter** (shim) over the sovereign core
//! `domain::kernel::pricing` (GRAND-PLAN 0b-1, `docs/design/sovereign-core-money-boundary-0b1/`).
//!
//! The pure money arithmetic now lives in the core (integer-only, wasm-replayable). This module is
//! the **single float chokepoint** (ADR Option 1A/3A): it owns the two f64 boundaries the core
//! cannot name — the tax-rate `f64 → rate_micro: i64` conversion and the Haversine
//! `distance_km → distance_m: i64` conversion — plus the km→m tier-bound conversion. It exposes the
//! SAME f64 signatures `pg.rs` used before the extraction, so the crown-jewel INSERT is untouched,
//! and delegates every computation to `domain::kernel::pricing::*`.
//!
//! All three conversions use the workspace's one f64→i64 convention, [`super::round_f64_to_i64`]
//! (half-away-from-zero = Node `Math.round`). The pure integer functions (`compute_order_pricing`,
//! `charged_tax`, `compose_total`) and the snapshot/error types are RE-EXPORTED verbatim from the
//! core, so callers keep importing them from `super::pricing::{…}`.

use domain::{Lek, MoneyError};

/// The core module — every adapter fn below delegates here after integerizing its float inputs.
use domain::kernel::pricing as core;

// Pure integer core items re-exported unchanged (pg.rs imports these from `super::pricing::{…}`).
// NOTE: this is an EXPLICIT named re-export, never a glob — a `pub use core::*` would drag the core's
// integer `DeliveryTier`/`FeeLocation`/`apply_tax`/`resolve_delivery_fee`/`delivery_fee_for_order`
// into this module and collide with the f64 shim shapes/fns below (the 1000×-scale hazard L2 warns of).
pub use domain::kernel::pricing::{
    charged_tax, compose_total, compute_order_pricing, GroupInfo, ModifierInfo, PricingError,
    PricingItem, PricingSnapshot, ProductInfo,
};

/// Ports `distanceKm` (`apps/api/src/lib/geo.ts:1-15`) — Haversine, km, rounded to 3 dp. Pure f64
/// trig on COORDINATES (not money) — STAYS in the shell (the native↔wasm float-determinism hazard is
/// quarantined out of the replayable core; the fee it ultimately selects is `i64`). `shifts.rs`'s
/// courier-geofence path also imports THIS function (`crate::routes::orders::pricing::distance_km`).
pub fn distance_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371.0_f64; // Earth radius, km
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let a = (d_lat / 2.0).sin() * (d_lat / 2.0).sin()
        + lat1.to_radians().cos()
            * lat2.to_radians().cos()
            * (d_lon / 2.0).sin()
            * (d_lon / 2.0).sin();
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    let distance = r * c;
    (distance * 1000.0).round() / 1000.0
}

/// Shell adapter for `applyTax` — keeps the f64 signature `pg.rs` calls, integerizes, delegates.
///
/// SHELL GUARD (float-domain, BEFORE conversion — resolution.md H1): the OLD short-circuit's
/// `!tax_rate.is_finite()` arm MUST live here, because the core i64 guard is structurally blind to
/// it — `+Infinity` maps to `round_f64_to_i64(INF·1e6) = i64::MAX`, a POSITIVE `rate_micro` the core
/// `rate_micro <= 0` guard passes straight into a `checked_mul` OVERFLOW → 5xx. Catching non-finite
/// here reproduces the OLD `Ok(0)`. The sign/zero arm (`tax_rate <= 0.0`) is intentionally NOT
/// re-checked here: it is subsumed by the core guard (a negative or zero rate rounds to a
/// `rate_micro <= 0`), which additionally protects every future caller, not just this adapter.
pub fn apply_tax(
    subtotal: i64,
    tax_rate: f64,
    price_includes_tax: bool,
) -> Result<i64, MoneyError> {
    if !tax_rate.is_finite() {
        return Ok(0);
    }
    core::apply_tax(
        subtotal,
        super::round_f64_to_i64(tax_rate * 1_000_000.0),
        price_includes_tax,
    )
}

/// SHELL f64 delivery tier — km bound, the row shape `pg.rs` reads from `delivery_tiers`
/// (`max_distance_km::double precision`).
///
/// ⚠ This is the f64 SHELL type — NOT the core's integer-meter
/// `domain::kernel::pricing::DeliveryTier` (`{ max_distance_m: i64 }`). Do not glob-import the core
/// type where THIS km shape is meant (or vice-versa): a 1000×-scale (km-vs-m) money bug. The adapter
/// converts km→m via `round_f64_to_i64(max_distance_km · 1000)` before calling the core.
#[derive(Debug, Clone, Copy)]
pub struct DeliveryTier {
    pub max_distance_km: f64,
    pub fee: i64,
}

/// SHELL f64 location — carries `lat`/`lng` (which feed `distance_km`), unlike the core's
/// `domain::kernel::pricing::FeeLocation` (integer-only, NO coordinates).
///
/// ⚠ Same 1000×-scale / coordinate-vs-none hazard as [`DeliveryTier`]: this is the f64 SHELL shape;
/// the adapter strips `lat`/`lng` (used only to compute `distance_m`) before constructing the core
/// `FeeLocation`.
#[derive(Debug, Clone, Copy)]
pub struct FeeLocation {
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub delivery_fee_flat: Option<i64>,
    pub free_delivery_threshold: Option<i64>,
    pub min_order_value: Option<i64>,
}

/// f64 shim `FeeLocation` → integer core `FeeLocation` (drops the coordinate pair).
fn to_core_location(location: FeeLocation) -> core::FeeLocation {
    core::FeeLocation {
        delivery_fee_flat: location.delivery_fee_flat,
        free_delivery_threshold: location.free_delivery_threshold,
        min_order_value: location.min_order_value,
    }
}

/// f64 shim tiers (km) → integer core tiers (meters), via the one f64→i64 convention.
fn to_core_tiers(tiers: &[DeliveryTier]) -> Vec<core::DeliveryTier> {
    tiers
        .iter()
        .map(|t| core::DeliveryTier {
            max_distance_m: super::round_f64_to_i64(t.max_distance_km * 1000.0),
            fee: t.fee,
        })
        .collect()
}

/// Compute the integer-meter delivery distance from a location's coordinates + the order pin, when
/// BOTH are present (else `None` — the core then skips the tier ladder to the flat fallback, exactly
/// as the old `if let (Some, Some, Some)` guard did). `distance_km`·1000 rounded to whole meters.
fn distance_m_from(location: FeeLocation, pin: Option<(f64, f64)>) -> Option<i64> {
    match (location.lat, location.lng, pin) {
        (Some(loc_lat), Some(loc_lng), Some((pin_lat, pin_lng))) => Some(super::round_f64_to_i64(
            distance_km(pin_lat, pin_lng, loc_lat, loc_lng) * 1000.0,
        )),
        _ => None,
    }
}

/// Shell adapter for `resolveDeliveryFee` — keeps the f64 `(location, pin, tiers)` signature,
/// integerizes distance + tiers, delegates to [`core::resolve_delivery_fee`]. Production reaches
/// tier resolution through [`delivery_fee_for_order`] (which owns the MIN_ORDER/pickup/free-threshold
/// ordering); this direct adapter is exercised by the f64 byte-parity `resolve_fee_*` tests, so it is
/// dead in the non-test build only.
#[cfg_attr(not(test), allow(dead_code))]
pub fn resolve_delivery_fee(
    location: FeeLocation,
    pin: Option<(f64, f64)>,
    tiers: &[DeliveryTier],
) -> Result<Lek, PricingError> {
    let distance_m = distance_m_from(location, pin);
    let core_tiers = to_core_tiers(tiers);
    core::resolve_delivery_fee(to_core_location(location), distance_m, &core_tiers)
}

/// Shell adapter for the section-8 fee ladder — keeps the f64 `(subtotal, is_pickup, location, pin,
/// tiers)` signature `pg.rs` calls, integerizes distance + tiers, delegates to
/// [`core::delivery_fee_for_order`] (which owns the MIN_ORDER / pickup / free-threshold ordering).
pub fn delivery_fee_for_order(
    subtotal: Lek,
    is_pickup: bool,
    location: FeeLocation,
    pin: Option<(f64, f64)>,
    tiers: &[DeliveryTier],
) -> Result<Lek, PricingError> {
    let distance_m = distance_m_from(location, pin);
    let core_tiers = to_core_tiers(tiers);
    core::delivery_fee_for_order(
        subtotal,
        is_pickup,
        to_core_location(location),
        distance_m,
        &core_tiers,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::ErrorCode;

    fn lek(v: i64) -> Lek {
        Lek::new(v).unwrap()
    }

    // ───────────── applyTax f64 end-to-end byte-parity (proves the f64→rate_micro boundary) ─────────────

    /// Ports `apps/api/tests/money-tax.test.ts` VERBATIM — the exact assertions, one-to-one, now
    /// exercising the ADAPTER (f64 rate) → core (`rate_micro`), so the whole f64→i64 boundary is
    /// byte-identical to Node's `applyTax` on every case the Node suite pins.
    #[test]
    fn apply_tax_matches_node_money_tax_vectors() {
        // exclusive: round-number rates
        assert_eq!(apply_tax(1000, 0.075, false).unwrap(), 75);
        assert_eq!(apply_tax(1000, 0.1, false).unwrap(), 100);
        assert_eq!(apply_tax(1200, 0.2, false).unwrap(), 240);
        // exclusive: half-up rounding at the boundary
        assert_eq!(apply_tax(1000, 0.0744, false).unwrap(), 74); // 74.4 → 74
        assert_eq!(apply_tax(1000, 0.0745, false).unwrap(), 75); // 74.5 → 75 (half-up)
        assert_eq!(apply_tax(999, 0.2, false).unwrap(), 200); // 199.8 → 200
        // inclusive: extracts embedded tax
        assert_eq!(apply_tax(1075, 0.075, true).unwrap(), 75);
        assert_eq!(apply_tax(1200, 0.2, true).unwrap(), 200);
        // zero rate / zero subtotal → 0
        assert_eq!(apply_tax(1000, 0.0, false).unwrap(), 0);
        assert_eq!(apply_tax(0, 0.2, false).unwrap(), 0);
        // large values stay exact (no float drift): 123456789 * 0.0825 = 10185185.0925 → 10185185
        assert_eq!(apply_tax(123_456_789, 0.0825, false).unwrap(), 10_185_185);
    }

    /// A large-cart × 100% rate does NOT wrap in `i64` (REV-S5-4) — through the real adapter.
    #[test]
    fn apply_tax_large_cart_full_rate_stays_exact_in_i64() {
        let sub = 2_000_000_000; // ~int4 max, a 2e9 minor-unit cart
        assert_eq!(apply_tax(sub, 1.0, false).unwrap(), 2_000_000_000);
        assert_eq!(apply_tax(sub, 1.0, true).unwrap(), 1_000_000_000);
    }

    /// NEW (resolution.md H1, the float-domain arm of the guard split) — the shell `!is_finite`
    /// guard + the core `rate_micro <= 0` guard together reproduce the OLD `Ok(0)` for all four
    /// exotic inputs. Without the shell guard, `±Infinity` would map to a positive `i64::MAX`
    /// `rate_micro` and 5xx-overflow; a negative f64 rate rounds to a negative `rate_micro` the core
    /// guard catches. This is the red→green proof for the exact behavior integerization risked losing.
    #[test]
    fn adapter_apply_tax_nonfinite_and_negative_rate_match_old_ok_zero() {
        assert_eq!(apply_tax(1000, f64::INFINITY, false), Ok(0)); // shell !is_finite → Ok(0)
        assert_eq!(apply_tax(1000, f64::NEG_INFINITY, false), Ok(0));
        assert_eq!(apply_tax(1000, f64::NAN, false), Ok(0));
        assert_eq!(apply_tax(1000, -0.2, false), Ok(0)); // f64 -0.2 → rate_micro -200000 → core guard
        assert_eq!(apply_tax(1000, f64::INFINITY, true), Ok(0)); // inclusive branch
    }

    // ─────────────── order-total composition byte-parity (order-total-vectors.ts) ───────────────

    struct Vector {
        name: &'static str,
        subtotal: i64,
        tax_rate: f64,
        price_includes_tax: bool,
        delivery_fee_flat: i64,
        expected_tax: i64,
        expected_charged_tax: i64,
        expected_total: i64,
    }

    /// The zero-import hand-derived vectors from `apps/api/tests/vectors/order-total-vectors.ts`,
    /// transcribed as literals (NOT computed from the implementation). The money BYTE-PARITY proof,
    /// now driving the adapter (f64 rate) + the re-exported core `charged_tax`/`compose_total`.
    const ORDER_TOTAL_VECTORS: &[Vector] = &[
        Vector {
            name: "exclusive round rate — tax added",
            subtotal: 1000,
            tax_rate: 0.2,
            price_includes_tax: false,
            delivery_fee_flat: 200,
            expected_tax: 200,
            expected_charged_tax: 200,
            expected_total: 1400,
        },
        Vector {
            name: "inclusive round rate — tax NOT re-added (the LC1 case)",
            subtotal: 1200,
            tax_rate: 0.2,
            price_includes_tax: true,
            delivery_fee_flat: 250,
            expected_tax: 200,
            expected_charged_tax: 0,
            expected_total: 1450,
        },
        Vector {
            name: "inclusive 7.5% pickup — embedded tax, no fee",
            subtotal: 1075,
            tax_rate: 0.075,
            price_includes_tax: true,
            delivery_fee_flat: 0,
            expected_tax: 75,
            expected_charged_tax: 0,
            expected_total: 1075,
        },
        Vector {
            name: "exclusive half-up boundary rate",
            subtotal: 1000,
            tax_rate: 0.0745,
            price_includes_tax: false,
            delivery_fee_flat: 200,
            expected_tax: 75,
            expected_charged_tax: 75,
            expected_total: 1275,
        },
        Vector {
            name: "zero rate inclusive — nothing added",
            subtotal: 1000,
            tax_rate: 0.0,
            price_includes_tax: true,
            delivery_fee_flat: 250,
            expected_tax: 0,
            expected_charged_tax: 0,
            expected_total: 1250,
        },
        Vector {
            name: "zero rate exclusive — nothing added",
            subtotal: 1000,
            tax_rate: 0.0,
            price_includes_tax: false,
            delivery_fee_flat: 250,
            expected_tax: 0,
            expected_charged_tax: 0,
            expected_total: 1250,
        },
    ];

    #[test]
    fn order_total_composition_byte_parity_vs_hand_derived_vectors() {
        for v in ORDER_TOTAL_VECTORS {
            let tax_total = apply_tax(v.subtotal, v.tax_rate, v.price_includes_tax).unwrap();
            assert_eq!(tax_total, v.expected_tax, "{}: taxTotal", v.name);

            let ct = charged_tax(lek(tax_total), v.price_includes_tax);
            assert_eq!(
                ct.minor_units(),
                v.expected_charged_tax,
                "{}: chargedTax",
                v.name
            );

            // discountTotal = 0 CARRY (REV-S5-6).
            let total =
                compose_total(lek(v.subtotal), lek(v.delivery_fee_flat), ct, Lek::ZERO).unwrap();
            assert_eq!(total.minor_units(), v.expected_total, "{}: total", v.name);
        }
    }

    /// LC1 property (`order-total-composition.test.ts:38-52`) over the real f64 adapter — an
    /// inclusive venue NEVER adds tax to the charge, for EVERY rate.
    #[test]
    fn lc1_inclusive_never_adds_tax_total_is_subtotal_plus_fee() {
        let rates = [0.0, 0.075, 0.1, 0.2];
        let subtotals = [500i64, 1000, 1075, 1999, 5000, 123_456];
        let fee = 300i64;
        for rate in rates {
            for sub in subtotals {
                let tax_total = apply_tax(sub, rate, true).unwrap();
                let ct = charged_tax(lek(tax_total), true);
                assert_eq!(ct, Lek::ZERO, "inclusive chargedTax must be 0 @ rate={rate} sub={sub}");
                let total = compose_total(lek(sub), lek(fee), ct, Lek::ZERO).unwrap();
                assert_eq!(
                    total.minor_units(),
                    sub + fee,
                    "inclusive total must equal subtotal+fee @ rate={rate} sub={sub}"
                );
            }
        }
    }

    /// Exclusive property (`order-total-composition.test.ts:54-65`) over the f64 adapter.
    #[test]
    fn exclusive_adds_exactly_the_extracted_tax() {
        let rate = 0.2;
        let fee = 300i64;
        for sub in [500i64, 1000, 2000, 5000] {
            let tax_total = apply_tax(sub, rate, false).unwrap();
            let ct = charged_tax(lek(tax_total), false);
            assert_eq!(ct.minor_units(), tax_total, "exclusive chargedTax === taxTotal @ sub={sub}");
            let total = compose_total(lek(sub), lek(fee), ct, Lek::ZERO).unwrap();
            assert_eq!(total.minor_units(), sub + fee + tax_total, "exclusive total @ sub={sub}");
        }
    }

    // ───────────────────────── delivery-fee ladder parity (f64 pin + tiers) ─────────────────────────

    fn loc(flat: Option<i64>, free: Option<i64>, min: Option<i64>) -> FeeLocation {
        FeeLocation {
            lat: Some(41.33),
            lng: Some(19.82),
            delivery_fee_flat: flat,
            free_delivery_threshold: free,
            min_order_value: min,
        }
    }

    /// `distanceKm` parity: Tirana center to itself is 0; a known short hop is > 0 and rounded to
    /// 3 dp (the geo.ts contract). Not a money value — this only feeds tier selection.
    #[test]
    fn distance_km_zero_and_rounding() {
        assert_eq!(distance_km(41.33, 19.82, 41.33, 19.82), 0.0);
        let d = distance_km(41.33, 19.82, 41.34, 19.83);
        assert!(d > 0.0 && d < 2.0, "adjacent points ~1km, got {d}");
        // rounded to 3 dp
        assert_eq!(d, (d * 1000.0).round() / 1000.0);
    }

    #[test]
    fn fee_ladder_pickup_is_zero_even_below_free_threshold() {
        let fee =
            delivery_fee_for_order(lek(500), true, loc(Some(300), Some(2000), None), None, &[])
                .unwrap();
        assert_eq!(fee, Lek::ZERO);
    }

    #[test]
    fn fee_ladder_free_over_threshold() {
        let fee = delivery_fee_for_order(
            lek(2500),
            false,
            loc(Some(300), Some(2000), None),
            Some((41.34, 19.83)),
            &[],
        )
        .unwrap();
        assert_eq!(fee, Lek::ZERO);
    }

    #[test]
    fn fee_ladder_flat_when_under_threshold_no_tiers() {
        let fee = delivery_fee_for_order(
            lek(1000),
            false,
            loc(Some(300), Some(2000), None),
            Some((41.34, 19.83)),
            &[],
        )
        .unwrap();
        assert_eq!(fee.minor_units(), 300);
    }

    #[test]
    fn fee_ladder_min_order_not_met_pickup_and_delivery() {
        let l = loc(Some(300), None, Some(1500));
        let pickup = delivery_fee_for_order(lek(1000), true, l, None, &[]).unwrap_err();
        assert_eq!(pickup.code, ErrorCode::MinOrderNotMet);
        let delivery =
            delivery_fee_for_order(lek(1000), false, l, Some((41.34, 19.83)), &[]).unwrap_err();
        assert_eq!(delivery.code, ErrorCode::MinOrderNotMet);
    }

    #[test]
    fn resolve_fee_tier_selects_first_covering_distance() {
        let tiers = [
            DeliveryTier {
                max_distance_km: 1.0,
                fee: 200,
            },
            DeliveryTier {
                max_distance_km: 5.0,
                fee: 400,
            },
        ];
        // ~1.4km hop → beyond tier 1 (1km) → tier 2 (5km) = 400.
        let fee =
            resolve_delivery_fee(loc(Some(300), None, None), Some((41.34, 19.83)), &tiers).unwrap();
        assert_eq!(fee.minor_units(), 400);
    }

    #[test]
    fn resolve_fee_beyond_last_tier_is_not_deliverable() {
        let tiers = [DeliveryTier {
            max_distance_km: 0.1,
            fee: 200,
        }];
        let err = resolve_delivery_fee(loc(Some(300), None, None), Some((42.0, 20.5)), &tiers)
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::NotDeliverable);
    }

    #[test]
    fn resolve_fee_unconfigured_is_delivery_not_configured() {
        let err =
            resolve_delivery_fee(loc(None, None, None), Some((41.34, 19.83)), &[]).unwrap_err();
        assert_eq!(err.code, ErrorCode::DeliveryNotConfigured);
    }
}
