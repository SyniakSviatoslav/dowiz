//! S5 money composition — the server-authoritative order total, integer minor units end-to-end
//! (`domain::Lek(i64)`). Ports `apps/api/src/lib/money.ts` (`applyTax`, `computeLineTotal`,
//! `assertNonNegative`) and `apps/api/src/lib/order-pricing.ts` (`computeOrderPricing`,
//! `resolveDeliveryFee`) VERBATIM, plus the section-9 composition of `apps/api/src/routes/orders.ts`
//! (subtotal → tax → chargedTax(LC1) → deliveryFee → discountTotal(CARRY) → total).
//!
//! ## REV-S5-4 — i64, NOT i128 (council RESOLVE overturned the packet)
//! The packet demanded an `i128` intermediate for `applyTax`; the breaker (M1) proved it is
//! UNNECESSARY and the RESOLVE dropped it: `subtotal` is `int4` (≤ 2.147e9), `rateMicro` ≤ ~1e6, so
//! `sub·rateMicro` ≤ ~2.15e15 « `i64::MAX` (9.2e18, ~4000× headroom). The whole `applyTax` runs in
//! `i64` with `checked_*` ops (a defensive belt — a real overflow here is unreachable, but a silent
//! wrap that REDUCED a charge would be a money bug, so we surface it as an error rather than wrap).
//! There is NO "overflow-vector" test (RESOLVE: "unfalsifiable theater → DROP it"). The money proof
//! is BYTE-PARITY vs the zero-import hand-derived vectors + the LC1 inclusive property.
//!
//! ## REV-S5-4 — LC1 (the double-charge invariant, ADR-audit-fix-money D1)
//! `chargedTax = price_includes_tax ? 0 : taxTotal`; `total` uses `chargedTax`, NEVER `taxTotal`.
//! On an inclusive venue the tax is already inside the subtotal (`applyTax` EXTRACTS it), so adding
//! it to `total` would double-charge by `r/(1+r)` of the cart. `taxTotal` stays computed (persisted
//! to `orders.tax_total` as an informational receipt line) but is structurally not an addend.
//!
//! ## REV-S5-6 — `discountTotal = 0` CARRY (Potemkin promo, the accepted-risk row)
//! See [`compose_total`]'s `discount_total` parameter. CARRIED as `Lek::ZERO`; the `− discountTotal`
//! seam is kept so a future redemption runtime is a one-line change, not a re-architecture.

use domain::{Lek, MoneyError};

/// The `applyTax` micro-unit scale — 6 dp of rate precision (`money.ts:10`, `SCALE = 1_000_000n`).
const SCALE: i64 = 1_000_000;

/// Ports `applyTax` (`apps/api/src/lib/money.ts:1-22`) — BigInt half-up, in `i64` (REV-S5-4).
///
/// `subtotal` is money (integer minor units); `tax_rate` is a CONFIG rate (a fraction, e.g. `0.2`
/// for 20% — a rate, NOT money, so the single `f64→i64` `rateMicro` conversion is the ONE place a
/// float touches this path, exactly as Node does `Math.round(taxRate * 1_000_000)`). Returns the
/// tax figure in minor units: on an inclusive venue this is the EXTRACTED embedded tax, on an
/// exclusive venue the tax to ADD. Short-circuits `subtotal == 0 || tax_rate == 0 ⇒ 0`
/// (`money.ts:5`) BEFORE any arithmetic.
///
/// `Err(MoneyError::Overflow)` is unreachable for real inputs (see module doc) — it exists so a
/// hypothetical wrap surfaces as an error instead of a silently reduced charge, never as a panic.
pub fn apply_tax(
    subtotal: i64,
    tax_rate: f64,
    price_includes_tax: bool,
) -> Result<i64, MoneyError> {
    // money.ts:5 — short-circuit. Also guards the (unreachable-for-real-config) NaN/negative rate:
    // a rate ≤ 0 collapses to the zero branch exactly like Node's `taxRate === 0`, and a negative
    // subtotal is impossible upstream (Lek rejects it) but the guard is cheap.
    if subtotal == 0 || tax_rate <= 0.0 || !tax_rate.is_finite() {
        return Ok(0);
    }
    // rateMicro = round(taxRate * 1e6). Node `Math.round` is half-UP (toward +∞); Rust `f64::round`
    // is half-AWAY-from-zero — identical for the non-negative rates this path ever sees. The one
    // f64→i64 cast is confined to `super::round_f64_to_i64` (workspace `clippy::as_conversions`).
    let rate_micro = super::round_f64_to_i64(tax_rate * 1_000_000.0);

    let overflow = || MoneyError::Overflow {
        op: "apply_tax",
        lhs: subtotal,
        rhs: rate_micro,
    };

    if price_includes_tax {
        // net = round(sub·SCALE / (SCALE + rateMicro)); tax = sub − net   (money.ts:16-18)
        let denom = SCALE.checked_add(rate_micro).ok_or_else(overflow)?;
        let numer = subtotal
            .checked_mul(SCALE)
            .and_then(|v| v.checked_add(denom / 2)) // half-up
            .ok_or_else(overflow)?;
        let net = numer / denom;
        subtotal.checked_sub(net).ok_or_else(overflow)
    } else {
        // tax = round(sub·rateMicro / SCALE)   (money.ts:21)
        subtotal
            .checked_mul(rate_micro)
            .and_then(|v| v.checked_add(SCALE / 2)) // half-up
            .map(|v| v / SCALE)
            .ok_or_else(overflow)
    }
}

/// Ports `computeLineTotal` (`money.ts:24-30`): `(product_price + Σ modifier_price_delta) ·
/// quantity`, all integer minor units. Uses `Lek` checked arithmetic — a line total that
/// overflowed would be a money bug, not a wrap.
pub fn compute_line_total(
    product_price: Lek,
    modifier_deltas: &[Lek],
    quantity: i64,
) -> Result<Lek, MoneyError> {
    let mut unit = product_price;
    for delta in modifier_deltas {
        unit = unit.checked_add(*delta)?;
    }
    unit.checked_mul_qty(quantity)
}

/// The section-9 composition (`orders.ts:530-535`): `total = subtotal + deliveryFee + chargedTax −
/// discountTotal`, in `Lek` checked arithmetic (a negative total is unrepresentable — `assertNonNegative`
/// ports as `Lek::checked_sub` returning `Err(Negative)` rather than a silent negative charge).
///
/// - `charged_tax` is the LC1 term the CALLER computed (`price_includes_tax ? 0 : tax_total`) — see
///   [`charged_tax`]. This function takes `charged_tax` already-resolved so the LC1 decision lives in
///   exactly one place and this composition can never accidentally add `tax_total`.
/// - `discount_total` — **REV-S5-6 CARRY**: callers pass `Lek::ZERO`. The `− discount_total` term is
///   kept as the seam for a future redemption runtime. CARRY-accepted-risk: the owner Promotions CRM
///   (`PromotionsPage.tsx`, `/owner/promotions`) is fully built + routed but has NO redemption
///   runtime (`current_uses` never increments; `discountTotal` is always 0) — an owner can create
///   `SUMMER20 −20%` and be misled by their own tool. Building redemption is its OWN council (schema
///   + ledger + abuse model); this port does NOT wire a customer discount input into the money tx.
pub fn compose_total(
    subtotal: Lek,
    delivery_fee: Lek,
    charged_tax: Lek,
    discount_total: Lek,
) -> Result<Lek, MoneyError> {
    subtotal
        .checked_add(delivery_fee)?
        .checked_add(charged_tax)?
        .checked_sub(discount_total)
}

/// LC1 (`orders.ts:532`): the tax actually ADDED to `total`. `price_includes_tax ⇒ 0` (already
/// inside the subtotal), else the extracted/added `tax_total`. This is the ONE place the inclusive
/// decision is made — [`compose_total`] takes the result, so the composition can never re-add tax.
pub fn charged_tax(tax_total: Lek, price_includes_tax: bool) -> Lek {
    if price_includes_tax {
        Lek::ZERO
    } else {
        tax_total
    }
}

/// A pricing/fee validation failure — the `{code, message}` a `422` ROLLBACK returns
/// (`order-pricing.ts:32-35`). The `code` strings are the exact wire codes the Node route sends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PricingError {
    pub code: &'static str,
    pub message: String,
}

// ─────────────────────────── computeOrderPricing (order-pricing.ts:72-142) ───────────────────────────

/// A cart line item (the validated `OrderItemInput`, ids as strings for map lookup).
#[derive(Debug, Clone)]
pub struct PricingItem {
    pub product_id: String,
    pub quantity: i64,
    pub modifier_ids: Vec<String>,
}

/// Product snapshot (price authority — the in-tx MVCC read, `orders.ts:414-432`).
#[derive(Debug, Clone)]
pub struct ProductInfo {
    pub name: String,
    pub price: Lek,
}

/// Modifier snapshot, keyed by `"{product_id}_{modifier_id}"` (only AVAILABLE modifiers present).
#[derive(Debug, Clone)]
pub struct ModifierInfo {
    pub name: String,
    pub price_delta: Lek,
    pub group_id: String,
}

/// A product's modifier group (for min/max-select validation).
#[derive(Debug, Clone)]
pub struct GroupInfo {
    pub id: String,
    pub min_select: i64,
    pub max_select: i64,
    pub required: bool,
}

/// One priced line-item modifier row (persisted to `order_item_modifiers`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PricedModifierRow {
    pub modifier_id: String,
    pub name_snapshot: String,
    pub price_delta_snapshot: Lek,
}

/// One priced order-item row (persisted to `order_items` + its modifiers).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PricedOrderItemRow {
    pub product_id: String,
    pub name_snapshot: String,
    pub price_snapshot: Lek,
    pub quantity: i64,
    pub modifiers: Vec<PricedModifierRow>,
}

/// The already-fetched snapshot maps `compute_order_pricing` consumes (mirrors
/// `ComputeOrderPricingInput`). Uses `std::collections::HashMap` — the caller (repo) builds these
/// from the in-tx product/modifier/group reads.
pub struct PricingSnapshot<'a> {
    pub product_map: &'a std::collections::HashMap<String, ProductInfo>,
    /// key = `"{product_id}_{modifier_id}"`.
    pub mod_map: &'a std::collections::HashMap<String, ModifierInfo>,
    pub groups_by_product: &'a std::collections::HashMap<String, Vec<GroupInfo>>,
}

/// Ports `computeOrderPricing` (`order-pricing.ts:72-142`) VERBATIM — the section-7 pricing loop as
/// a pure function: per-line duplicate-modifier reject → modifier lookup (MODIFIER_UNAVAILABLE) →
/// per-group min/max validation (MODIFIER_MIN_NOT_MET / MODIFIER_MAX_EXCEEDED) → line total → sum.
/// Returns `(subtotal, priced rows)` or the FIRST validation failure (exact code/message parity).
/// Caller guarantees every item's product is present in `product_map` (the §6 existence gate ran
/// first — a missing product is `PRODUCT_NOT_FOUND` there, not here).
pub fn compute_order_pricing(
    items: &[PricingItem],
    snapshot: &PricingSnapshot,
) -> Result<(Lek, Vec<PricedOrderItemRow>), PricingError> {
    use std::collections::HashMap;

    let mut subtotal = Lek::ZERO;
    let mut order_item_rows = Vec::with_capacity(items.len());

    for item in items {
        let product = snapshot
            .product_map
            .get(&item.product_id)
            .ok_or(PricingError {
                code: "PRODUCT_NOT_FOUND",
                message: "Product not found".to_string(),
            })?;
        let group_rows = snapshot
            .groups_by_product
            .get(&item.product_id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);

        let mut group_counts: HashMap<&str, i64> = HashMap::new();
        let mut modifier_deltas: Vec<Lek> = Vec::new();
        let mut item_modifier_rows: Vec<PricedModifierRow> = Vec::new();

        // Reject duplicate modifier ids on ONE line item before counting (order-pricing.ts:88-92).
        {
            let mut seen = std::collections::HashSet::new();
            for mid in &item.modifier_ids {
                if !seen.insert(mid) {
                    return Err(PricingError {
                        code: "DUPLICATE_MODIFIER",
                        message: "Duplicate modifier".to_string(),
                    });
                }
            }
        }

        for mid in &item.modifier_ids {
            let key = format!("{}_{}", item.product_id, mid);
            let mod_info = snapshot.mod_map.get(&key).ok_or_else(|| PricingError {
                code: "MODIFIER_UNAVAILABLE",
                message: format!("Modifier {mid} unavailable or invalid for product"),
            })?;
            *group_counts.entry(mod_info.group_id.as_str()).or_insert(0) += 1;
            modifier_deltas.push(mod_info.price_delta);
            item_modifier_rows.push(PricedModifierRow {
                modifier_id: mid.clone(),
                name_snapshot: mod_info.name.clone(),
                price_delta_snapshot: mod_info.price_delta,
            });
        }

        // Per-group min/max validation (order-pricing.ts:113-127).
        for g in group_rows {
            let count = group_counts.get(g.id.as_str()).copied().unwrap_or(0);
            if g.required && count < g.min_select {
                return Err(PricingError {
                    code: "MODIFIER_MIN_NOT_MET",
                    message: format!("Modifier group {} min select not met", g.id),
                });
            }
            if count > g.max_select {
                return Err(PricingError {
                    code: "MODIFIER_MAX_EXCEEDED",
                    message: format!("Modifier group {} max select exceeded", g.id),
                });
            }
        }

        let line_total = compute_line_total(product.price, &modifier_deltas, item.quantity)
            .map_err(|_e| PricingError {
                code: "PRODUCT_UNAVAILABLE",
                message: "Line total overflow".to_string(),
            })?;
        subtotal = subtotal
            .checked_add(line_total)
            .map_err(|_e| PricingError {
                code: "PRODUCT_UNAVAILABLE",
                message: "Subtotal overflow".to_string(),
            })?;

        order_item_rows.push(PricedOrderItemRow {
            product_id: item.product_id.clone(),
            name_snapshot: product.name.clone(),
            price_snapshot: product.price,
            quantity: item.quantity,
            modifiers: item_modifier_rows,
        });
    }

    Ok((subtotal, order_item_rows))
}

/// Ports `distanceKm` (`apps/api/src/lib/geo.ts:1-15`) — Haversine, km, rounded to 3 dp. Pure f64
/// trig on COORDINATES (not money) — the money RED-LINE is untouched (the fee it selects is `i64`).
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

/// One delivery tier row (`delivery_tiers`, ordered by `max_distance_km ASC`).
#[derive(Debug, Clone, Copy)]
pub struct DeliveryTier {
    pub max_distance_km: f64,
    pub fee: i64,
}

/// A location's fee-relevant config (the columns the fee ladder reads).
#[derive(Debug, Clone, Copy)]
pub struct FeeLocation {
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub delivery_fee_flat: Option<i64>,
    pub free_delivery_threshold: Option<i64>,
    pub min_order_value: Option<i64>,
}

/// Ports `resolveDeliveryFee` (`order-pricing.ts:168-184`) VERBATIM — distance-tier first (first
/// tier covering the distance), then flat fallback, else `DELIVERY_NOT_CONFIGURED`; a delivery
/// beyond the last tier is `NOT_DELIVERABLE`. `pin` is `Some` for a delivery order (caller
/// guarantees). Returns the fee in minor units (`Lek`).
pub fn resolve_delivery_fee(
    location: FeeLocation,
    pin: Option<(f64, f64)>,
    tiers: &[DeliveryTier],
) -> Result<Lek, PricingError> {
    if !tiers.is_empty() {
        if let (Some(loc_lat), Some(loc_lng), Some((pin_lat, pin_lng))) =
            (location.lat, location.lng, pin)
        {
            let dist = distance_km(pin_lat, pin_lng, loc_lat, loc_lng);
            for tier in tiers {
                if dist <= tier.max_distance_km {
                    return Lek::new(tier.fee).map_err(|_e| PricingError {
                        code: "DELIVERY_NOT_CONFIGURED",
                        message: "Delivery not configured".to_string(),
                    });
                }
            }
            return Err(PricingError {
                code: "NOT_DELIVERABLE",
                message: "Location out of delivery range".to_string(),
            });
        }
    }
    match location.delivery_fee_flat {
        Some(flat) => Lek::new(flat).map_err(|_e| PricingError {
            code: "DELIVERY_NOT_CONFIGURED",
            message: "Delivery not configured".to_string(),
        }),
        None => Err(PricingError {
            code: "DELIVERY_NOT_CONFIGURED",
            message: "Delivery not configured".to_string(),
        }),
    }
}

/// The full section-8 fee ladder (`orders.ts:508-524`): pickup → 0; `subtotal ≥
/// free_delivery_threshold` → 0; else [`resolve_delivery_fee`]. The `MIN_ORDER_NOT_MET` gate
/// (`orders.ts:498`, applies to pickup AND delivery) is checked FIRST and returned as the same
/// `PricingError`. `is_pickup` short-circuits to a zero fee (pickup carries no pin/address/fee).
pub fn delivery_fee_for_order(
    subtotal: Lek,
    is_pickup: bool,
    location: FeeLocation,
    pin: Option<(f64, f64)>,
    tiers: &[DeliveryTier],
) -> Result<Lek, PricingError> {
    // MIN_ORDER_NOT_MET — pickup AND delivery (orders.ts:498).
    if let Some(min) = location.min_order_value {
        if subtotal.minor_units() < min {
            return Err(PricingError {
                code: "MIN_ORDER_NOT_MET",
                message: "Minimum order value not met".to_string(),
            });
        }
    }
    if is_pickup {
        return Ok(Lek::ZERO);
    }
    if let Some(threshold) = location.free_delivery_threshold {
        if subtotal.minor_units() >= threshold {
            return Ok(Lek::ZERO);
        }
    }
    resolve_delivery_fee(location, pin, tiers)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lek(v: i64) -> Lek {
        Lek::new(v).unwrap()
    }

    // ───────────────────────── applyTax byte-parity (money-tax.test.ts) ─────────────────────────

    /// Ports `apps/api/tests/money-tax.test.ts` VERBATIM — the exact assertions, one-to-one, so the
    /// Rust `apply_tax` is byte-identical to the Node `applyTax` on every case the Node suite pins.
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

    /// A large-cart × 100% rate does NOT wrap in `i64` (REV-S5-4: no i128 needed, but no wrap
    /// either). `int4`-max subtotal × 100% is well inside `i64`. This is NOT the dropped
    /// "overflow-vector theater" — it asserts the REAL boundary (max realistic cart) computes the
    /// correct tax, not that an unrepresentable input errors.
    #[test]
    fn apply_tax_large_cart_full_rate_stays_exact_in_i64() {
        let sub = 2_000_000_000; // ~int4 max, a 2e9 minor-unit cart
        // exclusive 100%: tax = round(2e9 * 1.0) = 2e9
        assert_eq!(apply_tax(sub, 1.0, false).unwrap(), 2_000_000_000);
        // inclusive 100%: net = round(2e9*1e6 / 2e6) = 1e9; tax = 2e9 − 1e9 = 1e9
        assert_eq!(apply_tax(sub, 1.0, true).unwrap(), 1_000_000_000);
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
    /// transcribed as literals (NOT computed from the implementation — the oracle-independence the
    /// Node file's header rule requires). This is the money BYTE-PARITY proof (REV-S5-4 DoD).
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

    /// LC1 property (`order-total-composition.test.ts:38-52`) — oracle-independent: an inclusive
    /// venue NEVER adds tax to the charge, for EVERY rate. `chargedTax == 0` and
    /// `total == subtotal + fee`, definitionally. References no implementation output.
    #[test]
    fn lc1_inclusive_never_adds_tax_total_is_subtotal_plus_fee() {
        let rates = [0.0, 0.075, 0.1, 0.2];
        let subtotals = [500i64, 1000, 1075, 1999, 5000, 123_456];
        let fee = 300i64;
        for rate in rates {
            for sub in subtotals {
                let tax_total = apply_tax(sub, rate, true).unwrap();
                let ct = charged_tax(lek(tax_total), true);
                assert_eq!(
                    ct,
                    Lek::ZERO,
                    "inclusive chargedTax must be 0 @ rate={rate} sub={sub}"
                );
                let total = compose_total(lek(sub), lek(fee), ct, Lek::ZERO).unwrap();
                assert_eq!(
                    total.minor_units(),
                    sub + fee,
                    "inclusive total must equal subtotal+fee @ rate={rate} sub={sub}"
                );
            }
        }
    }

    /// Exclusive property (`order-total-composition.test.ts:54-65`): an exclusive venue adds exactly
    /// the extracted tax — `chargedTax == taxTotal` and `total == subtotal + fee + taxTotal`.
    #[test]
    fn exclusive_adds_exactly_the_extracted_tax() {
        let rate = 0.2;
        let fee = 300i64;
        for sub in [500i64, 1000, 2000, 5000] {
            let tax_total = apply_tax(sub, rate, false).unwrap();
            let ct = charged_tax(lek(tax_total), false);
            assert_eq!(
                ct.minor_units(),
                tax_total,
                "exclusive chargedTax === taxTotal @ sub={sub}"
            );
            let total = compose_total(lek(sub), lek(fee), ct, Lek::ZERO).unwrap();
            assert_eq!(
                total.minor_units(),
                sub + fee + tax_total,
                "exclusive total @ sub={sub}"
            );
        }
    }

    // ───────────────────────── computeLineTotal parity ─────────────────────────

    /// Ports `apps/api/tests/money-tax.test.ts`'s `computeLineTotal` cases.
    #[test]
    fn compute_line_total_matches_node() {
        assert_eq!(
            compute_line_total(lek(500), &[lek(100), lek(50)], 3)
                .unwrap()
                .minor_units(),
            1950
        );
        assert_eq!(compute_line_total(lek(0), &[], 5).unwrap().minor_units(), 0);
    }

    /// REV-S5-6 seam: the `− discountTotal` term is real — a nonzero discount subtracts. (Proves the
    /// seam is wired, even though every S5 caller passes `Lek::ZERO`; a future redemption runtime
    /// flips this without touching `compose_total`.)
    #[test]
    fn compose_total_discount_seam_subtracts_when_nonzero() {
        let total = compose_total(lek(1000), lek(200), lek(100), lek(150)).unwrap();
        assert_eq!(total.minor_units(), 1150); // 1000 + 200 + 100 − 150
    }

    /// A composition that would go negative (discount > subtotal+fee+tax) is an `Err`, never a
    /// silent negative charge — ports `assertNonNegative` throwing.
    #[test]
    fn compose_total_rejects_a_negative_result() {
        let out = compose_total(lek(100), lek(0), lek(0), lek(500));
        assert!(matches!(out, Err(MoneyError::Negative(_))));
    }

    // ───────────────────────── delivery-fee ladder parity ─────────────────────────

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
        assert_eq!(pickup.code, "MIN_ORDER_NOT_MET");
        let delivery =
            delivery_fee_for_order(lek(1000), false, l, Some((41.34, 19.83)), &[]).unwrap_err();
        assert_eq!(delivery.code, "MIN_ORDER_NOT_MET");
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
        assert_eq!(err.code, "NOT_DELIVERABLE");
    }

    #[test]
    fn resolve_fee_unconfigured_is_delivery_not_configured() {
        let err =
            resolve_delivery_fee(loc(None, None, None), Some((41.34, 19.83)), &[]).unwrap_err();
        assert_eq!(err.code, "DELIVERY_NOT_CONFIGURED");
    }

    // ───────────────────────── computeOrderPricing parity ─────────────────────────

    use std::collections::HashMap;

    fn product(name: &str, price: i64) -> ProductInfo {
        ProductInfo {
            name: name.to_string(),
            price: lek(price),
        }
    }

    fn snapshot<'a>(
        products: &'a HashMap<String, ProductInfo>,
        mods: &'a HashMap<String, ModifierInfo>,
        groups: &'a HashMap<String, Vec<GroupInfo>>,
    ) -> PricingSnapshot<'a> {
        PricingSnapshot {
            product_map: products,
            mod_map: mods,
            groups_by_product: groups,
        }
    }

    #[test]
    fn pricing_sums_products_and_modifiers() {
        let mut products = HashMap::new();
        products.insert("p1".to_string(), product("Pizza", 1000));
        let mut mods = HashMap::new();
        mods.insert(
            "p1_m1".to_string(),
            ModifierInfo {
                name: "Extra cheese".to_string(),
                price_delta: lek(200),
                group_id: "g1".to_string(),
            },
        );
        let groups = HashMap::new();
        let items = vec![PricingItem {
            product_id: "p1".to_string(),
            quantity: 2,
            modifier_ids: vec!["m1".to_string()],
        }];
        let (subtotal, rows) =
            compute_order_pricing(&items, &snapshot(&products, &mods, &groups)).unwrap();
        // (1000 + 200) * 2 = 2400
        assert_eq!(subtotal.minor_units(), 2400);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].modifiers.len(), 1);
    }

    #[test]
    fn pricing_rejects_duplicate_modifier() {
        let mut products = HashMap::new();
        products.insert("p1".to_string(), product("Pizza", 1000));
        let mut mods = HashMap::new();
        mods.insert(
            "p1_m1".to_string(),
            ModifierInfo {
                name: "x".to_string(),
                price_delta: lek(0),
                group_id: "g1".to_string(),
            },
        );
        let groups = HashMap::new();
        let items = vec![PricingItem {
            product_id: "p1".to_string(),
            quantity: 1,
            modifier_ids: vec!["m1".to_string(), "m1".to_string()],
        }];
        let err = compute_order_pricing(&items, &snapshot(&products, &mods, &groups)).unwrap_err();
        assert_eq!(err.code, "DUPLICATE_MODIFIER");
    }

    #[test]
    fn pricing_rejects_unavailable_modifier() {
        let mut products = HashMap::new();
        products.insert("p1".to_string(), product("Pizza", 1000));
        let mods = HashMap::new(); // m1 not present → unavailable
        let groups = HashMap::new();
        let items = vec![PricingItem {
            product_id: "p1".to_string(),
            quantity: 1,
            modifier_ids: vec!["m1".to_string()],
        }];
        let err = compute_order_pricing(&items, &snapshot(&products, &mods, &groups)).unwrap_err();
        assert_eq!(err.code, "MODIFIER_UNAVAILABLE");
    }

    #[test]
    fn pricing_enforces_required_min_select() {
        let mut products = HashMap::new();
        products.insert("p1".to_string(), product("Pizza", 1000));
        let mods = HashMap::new();
        let mut groups = HashMap::new();
        groups.insert(
            "p1".to_string(),
            vec![GroupInfo {
                id: "g1".to_string(),
                min_select: 1,
                max_select: 2,
                required: true,
            }],
        );
        let items = vec![PricingItem {
            product_id: "p1".to_string(),
            quantity: 1,
            modifier_ids: vec![], // none selected, but group requires ≥1
        }];
        let err = compute_order_pricing(&items, &snapshot(&products, &mods, &groups)).unwrap_err();
        assert_eq!(err.code, "MODIFIER_MIN_NOT_MET");
    }

    #[test]
    fn pricing_enforces_max_select() {
        let mut products = HashMap::new();
        products.insert("p1".to_string(), product("Pizza", 1000));
        let mut mods = HashMap::new();
        for m in ["m1", "m2"] {
            mods.insert(
                format!("p1_{m}"),
                ModifierInfo {
                    name: m.to_string(),
                    price_delta: lek(100),
                    group_id: "g1".to_string(),
                },
            );
        }
        let mut groups = HashMap::new();
        groups.insert(
            "p1".to_string(),
            vec![GroupInfo {
                id: "g1".to_string(),
                min_select: 0,
                max_select: 1,
                required: false,
            }],
        );
        let items = vec![PricingItem {
            product_id: "p1".to_string(),
            quantity: 1,
            modifier_ids: vec!["m1".to_string(), "m2".to_string()], // 2 > max 1
        }];
        let err = compute_order_pricing(&items, &snapshot(&products, &mods, &groups)).unwrap_err();
        assert_eq!(err.code, "MODIFIER_MAX_EXCEEDED");
    }
}
