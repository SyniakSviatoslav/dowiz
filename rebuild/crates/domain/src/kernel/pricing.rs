//! Sovereign-core money composition — the server-authoritative order total, integer minor units
//! end-to-end ([`crate::Lek`]`(i64)`). Relocated from the `api` shell
//! (`routes/orders/pricing.rs`) into the sovereign core (GRAND-PLAN 0b-1,
//! `docs/design/sovereign-core-money-boundary-0b1/`). Ports `apps/api/src/lib/money.ts`
//! (`applyTax`, `computeLineTotal`, `assertNonNegative`) and `apps/api/src/lib/order-pricing.ts`
//! (`computeOrderPricing`, `resolveDeliveryFee`) VERBATIM, plus the section-9 composition of
//! `apps/api/src/routes/orders.ts` (subtotal → tax → chargedTax(LC1) → deliveryFee →
//! discountTotal(CARRY) → total).
//!
//! ## The f64→i64 boundary (0b-1) — this module is INTEGER-ONLY, by construction
//! The core cannot name `f64` (`crates/domain/clippy.toml` `disallowed-types`, enforced by
//! `sovereign-gate.sh` Gate 2), so every float boundary is marshalled to an integer in the SHELL
//! adapter (`api::routes::orders::pricing`) before reaching here:
//!   - the tax rate arrives pre-scaled to micro-units (`rate_micro: i64`, 6 dp), never as `tax_rate: f64`;
//!   - the delivery distance and each tier bound arrive as whole meters (`distance_m: i64`,
//!     `DeliveryTier.max_distance_m: i64`), never as km-`f64` — `distance_km` (Haversine trig, a
//!     native↔wasm float hazard) STAYS in the shell and only ever feeds this core an `i64` meter count.
//!
//! ## REV-S5-4 — i64, NOT i128 (council RESOLVE overturned the packet)
//! The packet demanded an `i128` intermediate for `apply_tax`; the breaker (M1) proved it is
//! UNNECESSARY and the RESOLVE dropped it: `subtotal` is `int4` (≤ 2.147e9), `rate_micro` ≤ ~1e6, so
//! `sub·rate_micro` ≤ ~2.15e15 « `i64::MAX` (9.2e18, ~4000× headroom). The whole `apply_tax` runs in
//! `i64` with `checked_*` ops (a defensive belt — a real overflow here is unreachable, but a silent
//! wrap that REDUCED a charge would be a money bug, so we surface it as an error rather than wrap).
//!
//! ## REV-S5-4 — LC1 (the double-charge invariant, ADR-audit-fix-money D1)
//! `charged_tax = price_includes_tax ? 0 : tax_total`; `total` uses `charged_tax`, NEVER `tax_total`.
//! On an inclusive venue the tax is already inside the subtotal (`apply_tax` EXTRACTS it), so adding
//! it to `total` would double-charge by `r/(1+r)` of the cart. `tax_total` stays computed (persisted
//! to `orders.tax_total` as an informational receipt line) but is structurally not an addend.
//!
//! ## REV-S5-6 — `discount_total = 0` CARRY (Potemkin promo, the accepted-risk row)
//! See [`compose_total`]'s `discount_total` parameter. CARRIED as `Lek::ZERO`; the `− discount_total`
//! seam is kept so a future redemption runtime is a one-line change, not a re-architecture.

use crate::{ErrorCode, Lek, MoneyError};

/// The `apply_tax` micro-unit scale — 6 dp of rate precision (`money.ts:10`, `SCALE = 1_000_000n`).
const SCALE: i64 = 1_000_000;

/// Ports `applyTax` (`apps/api/src/lib/money.ts:1-22`) — BigInt half-up, in `i64` (REV-S5-4).
///
/// `subtotal` is money (integer minor units); `rate_micro` is the tax rate PRE-SCALED to 6-dp
/// micro-units (e.g. `0.2` → `200_000`). The f64→micro conversion (`round(rate·1e6)`, Node's
/// `Math.round(taxRate · 1_000_000)`) happens ONCE, in the shell adapter — no float ever reaches
/// this core function. Returns the tax figure in minor units: on an inclusive venue the EXTRACTED
/// embedded tax, on an exclusive venue the tax to ADD.
///
/// GUARD (core, i64-domain — the sign/zero arm of the OLD f64 short-circuit
/// `subtotal == 0 || tax_rate <= 0.0`, resolution.md H1): a non-positive `rate_micro` (a negative or
/// zero rate — including a misconfigured negative `tax_rate` read `unwrap_or(0.0)` off the nullable
/// `numeric` column) short-circuits to `Ok(0)` BEFORE any `checked_mul`, so it can never produce a
/// negative `Lek` (→ `Err` → 5xx) or reach an overflow. This i64 guard protects EVERY caller, not
/// just today's adapter. The non-finite (`±Inf`/`NaN`) arm is NOT here — it has no faithful i64
/// image (`+Inf·1e6` rounds to a POSITIVE `i64::MAX`, which this guard would pass through), so it is
/// caught in the shell adapter's float domain instead.
///
/// `Err(MoneyError::Overflow)` is unreachable for real inputs (see module doc) — it exists so a
/// hypothetical wrap surfaces as an error instead of a silently reduced charge, never as a panic.
pub fn apply_tax(
    subtotal: i64,
    rate_micro: i64,
    price_includes_tax: bool,
) -> Result<i64, MoneyError> {
    // money.ts:5 short-circuit, split across the f64→i64 boundary (resolution.md H1). Sign/zero arm:
    // a rate_micro ≤ 0 collapses to the zero branch exactly as Node's `taxRate === 0` does for a
    // zero rate, and defensively covers a negative rate (which Node would undercharge on).
    if subtotal == 0 || rate_micro <= 0 {
        return Ok(0);
    }

    let overflow = || MoneyError::Overflow {
        op: "apply_tax",
        lhs: subtotal,
        rhs: rate_micro,
    };

    if price_includes_tax {
        // net = round(sub·SCALE / (SCALE + rate_micro)); tax = sub − net   (money.ts:16-18)
        let denom = SCALE.checked_add(rate_micro).ok_or_else(overflow)?;
        let numer = subtotal
            .checked_mul(SCALE)
            .and_then(|v| v.checked_add(denom / 2)) // half-up
            .ok_or_else(overflow)?;
        let net = numer / denom;
        subtotal.checked_sub(net).ok_or_else(overflow)
    } else {
        // tax = round(sub·rate_micro / SCALE)   (money.ts:21)
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
/// (`order-pricing.ts:32-35`). `code` is the domain [`ErrorCode`] whose `#[serde(rename_all =
/// "SCREAMING_SNAKE_CASE")]` serializes to the exact wire string the Node route sends (e.g.
/// `ErrorCode::ProductNotFound` → `"PRODUCT_NOT_FOUND"`) — the shell no longer needs a `&str →
/// ErrorCode` mirror (0b-1 Option 4A: the core speaks the domain's own error vocabulary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PricingError {
    pub code: ErrorCode,
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
/// `ComputeOrderPricingInput`). The caller (the shell repo) builds these `HashMap`s from the in-tx
/// product/modifier/group reads — the core only READS them by key (never constructs a map, so no
/// `RandomState`/entropy is seeded in the core; see `compute_order_pricing`'s BTree* internals).
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
///
/// **Entropy-free (resolution.md M1):** the per-line `group_counts`/`seen` collections are
/// `BTreeMap`/`BTreeSet` (ordered, `no RandomState`) — `HashMap`/`HashSet::new()` would seed the
/// core's FIRST OS-entropy read, violating core Law 2. Identical API surface, `O(log n)` on a
/// handful of groups (immaterial), and a deterministic iteration order should any future edit iterate.
pub fn compute_order_pricing(
    items: &[PricingItem],
    snapshot: &PricingSnapshot,
) -> Result<(Lek, Vec<PricedOrderItemRow>), PricingError> {
    use std::collections::{BTreeMap, BTreeSet};

    let mut subtotal = Lek::ZERO;
    let mut order_item_rows = Vec::with_capacity(items.len());

    for item in items {
        let product = snapshot
            .product_map
            .get(&item.product_id)
            .ok_or(PricingError {
                code: ErrorCode::ProductNotFound,
                message: "Product not found".to_string(),
            })?;
        let group_rows = snapshot
            .groups_by_product
            .get(&item.product_id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);

        // ordered by &str key; zero RandomState (resolution.md M1)
        let mut group_counts: BTreeMap<&str, i64> = BTreeMap::new();
        let mut modifier_deltas: Vec<Lek> = Vec::new();
        let mut item_modifier_rows: Vec<PricedModifierRow> = Vec::new();

        // Reject duplicate modifier ids on ONE line item before counting (order-pricing.ts:88-92).
        {
            let mut seen: BTreeSet<&String> = BTreeSet::new(); // ordered; entropy-free dedup
            for mid in &item.modifier_ids {
                if !seen.insert(mid) {
                    return Err(PricingError {
                        code: ErrorCode::DuplicateModifier,
                        message: "Duplicate modifier".to_string(),
                    });
                }
            }
        }

        for mid in &item.modifier_ids {
            let key = format!("{}_{}", item.product_id, mid);
            let mod_info = snapshot.mod_map.get(&key).ok_or_else(|| PricingError {
                code: ErrorCode::ModifierUnavailable,
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
                    code: ErrorCode::ModifierMinNotMet,
                    message: format!("Modifier group {} min select not met", g.id),
                });
            }
            if count > g.max_select {
                return Err(PricingError {
                    code: ErrorCode::ModifierMaxExceeded,
                    message: format!("Modifier group {} max select exceeded", g.id),
                });
            }
        }

        let line_total = compute_line_total(product.price, &modifier_deltas, item.quantity)
            .map_err(|_e| PricingError {
                code: ErrorCode::ProductUnavailable,
                message: "Line total overflow".to_string(),
            })?;
        subtotal = subtotal
            .checked_add(line_total)
            .map_err(|_e| PricingError {
                code: ErrorCode::ProductUnavailable,
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

// ─────────────────────────── delivery-fee ladder (integer meters) ───────────────────────────

/// One delivery tier row (`delivery_tiers`), bound in **integer METERS**.
///
/// ⚠ Do NOT confuse with the SHELL's f64 `api::routes::orders::pricing::DeliveryTier`
/// (`{ max_distance_km: f64, fee: i64 }`) — grabbing the wrong one where the other is meant is a
/// 1000×-scale (km-vs-m) money bug. This core type's bound is METERS; the shell adapter converts
/// km→m (`round_half_up(km·1000)`) before constructing it. Tiers are ordered by `max_distance_m ASC`.
#[derive(Debug, Clone, Copy)]
pub struct DeliveryTier {
    pub max_distance_m: i64,
    pub fee: i64,
}

/// A location's fee-relevant config (the columns the fee ladder reads), integer minor units.
///
/// ⚠ Do NOT confuse with the SHELL's f64 `api::routes::orders::pricing::FeeLocation`, which ALSO
/// carries `lat: Option<f64>` / `lng: Option<f64>`. This core type has **NO lat/lng** — they only
/// ever fed `distance_km` (Haversine), which STAYS in the shell; the core receives the already-
/// computed `distance_m: Option<i64>` as a separate argument, never coordinates.
#[derive(Debug, Clone, Copy)]
pub struct FeeLocation {
    pub delivery_fee_flat: Option<i64>,
    pub free_delivery_threshold: Option<i64>,
    pub min_order_value: Option<i64>,
}

/// Ports `resolveDeliveryFee` (`order-pricing.ts:168-184`) VERBATIM, integer meters — distance-tier
/// first (first tier covering the distance, `distance_m <= tier.max_distance_m`), then flat fallback,
/// else `DELIVERY_NOT_CONFIGURED`; a delivery beyond the last tier is `NOT_DELIVERABLE`. `distance_m`
/// is `Some` for a delivery order whose location has coordinates (the shell computes it via
/// `distance_km`·1000 rounded to whole meters). Returns the fee in minor units (`Lek`). The compare
/// is `<=`, so a `distance_m` exactly equal to a tier's `max_distance_m` selects that tier.
pub fn resolve_delivery_fee(
    location: FeeLocation,
    distance_m: Option<i64>,
    tiers: &[DeliveryTier],
) -> Result<Lek, PricingError> {
    if !tiers.is_empty() {
        if let Some(dist) = distance_m {
            for tier in tiers {
                if dist <= tier.max_distance_m {
                    return Lek::new(tier.fee).map_err(|_e| PricingError {
                        code: ErrorCode::DeliveryNotConfigured,
                        message: "Delivery not configured".to_string(),
                    });
                }
            }
            return Err(PricingError {
                code: ErrorCode::NotDeliverable,
                message: "Location out of delivery range".to_string(),
            });
        }
    }
    match location.delivery_fee_flat {
        Some(flat) => Lek::new(flat).map_err(|_e| PricingError {
            code: ErrorCode::DeliveryNotConfigured,
            message: "Delivery not configured".to_string(),
        }),
        None => Err(PricingError {
            code: ErrorCode::DeliveryNotConfigured,
            message: "Delivery not configured".to_string(),
        }),
    }
}

/// The full section-8 fee ladder (`orders.ts:508-524`): pickup → 0; `subtotal ≥
/// free_delivery_threshold` → 0; else [`resolve_delivery_fee`]. The `MIN_ORDER_NOT_MET` gate
/// (`orders.ts:498`, applies to pickup AND delivery) is checked FIRST and returned as the same
/// `PricingError`. `is_pickup` short-circuits to a zero fee (pickup carries no distance/address/fee).
pub fn delivery_fee_for_order(
    subtotal: Lek,
    is_pickup: bool,
    location: FeeLocation,
    distance_m: Option<i64>,
    tiers: &[DeliveryTier],
) -> Result<Lek, PricingError> {
    // MIN_ORDER_NOT_MET — pickup AND delivery (orders.ts:498).
    if let Some(min) = location.min_order_value {
        if subtotal.minor_units() < min {
            return Err(PricingError {
                code: ErrorCode::MinOrderNotMet,
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
    resolve_delivery_fee(location, distance_m, tiers)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lek(v: i64) -> Lek {
        Lek::new(v).unwrap()
    }

    // ───────────────────── applyTax byte-parity — integer rate_micro vectors ─────────────────────

    /// The integer-`rate_micro` twin of the shell's f64 `apply_tax_matches_node_money_tax_vectors`.
    /// The rate_micro literals are the decorrelated oracle §B conversions
    /// (`0.075→75000, 0.1→100000, 0.2→200000, 0.0744→74400, 0.0745→74500, 0.0825→82500`); the outputs
    /// are byte-identical to the Node `applyTax` figures — the f64→micro boundary is exercised
    /// end-to-end by the SHELL's f64 vectors, this pins the pure integer arithmetic in the core.
    #[test]
    fn apply_tax_matches_rate_micro_vectors() {
        // exclusive: round-number rates
        assert_eq!(apply_tax(1000, 75_000, false).unwrap(), 75); // 0.075
        assert_eq!(apply_tax(1000, 100_000, false).unwrap(), 100); // 0.1
        assert_eq!(apply_tax(1200, 200_000, false).unwrap(), 240); // 0.2
        // exclusive: half-up rounding at the boundary
        assert_eq!(apply_tax(1000, 74_400, false).unwrap(), 74); // 0.0744 → 74.4 → 74
        assert_eq!(apply_tax(1000, 74_500, false).unwrap(), 75); // 0.0745 → 74.5 → 75 (half-up)
        assert_eq!(apply_tax(999, 200_000, false).unwrap(), 200); // 199.8 → 200
        // inclusive: extracts embedded tax
        assert_eq!(apply_tax(1075, 75_000, true).unwrap(), 75);
        assert_eq!(apply_tax(1200, 200_000, true).unwrap(), 200);
        // zero rate / zero subtotal → 0
        assert_eq!(apply_tax(1000, 0, false).unwrap(), 0);
        assert_eq!(apply_tax(0, 200_000, false).unwrap(), 0);
        // large values stay exact (no drift): 123456789 * 0.0825 = 10185185.0925 → 10185185
        assert_eq!(apply_tax(123_456_789, 82_500, false).unwrap(), 10_185_185);
    }

    /// The core i64 guard (resolution.md H1, the Breaker-HIGH gap) — a non-positive `rate_micro` or a
    /// zero subtotal short-circuits to `Ok(0)` BEFORE any `checked_mul`, never a negative `Lek`/5xx.
    /// Each line asserts byte-parity with the OLD f64 short-circuit's `Ok(0)`.
    #[test]
    fn apply_tax_core_guard_returns_zero_on_nonpositive_rate_and_zero_subtotal() {
        assert_eq!(apply_tax(1000, -200_000, false), Ok(0)); // negative rate_micro (was: guard tax_rate<=0.0)
        assert_eq!(apply_tax(1000, -1, false), Ok(0)); // smallest negative
        assert_eq!(apply_tax(1000, 0, false), Ok(0)); // zero rate_micro
        assert_eq!(apply_tax(0, 75_000, false), Ok(0)); // zero subtotal
        assert_eq!(apply_tax(1000, -200_000, true), Ok(0)); // inclusive branch, same guard
    }

    /// A large-cart × 100% rate (`rate_micro = 1_000_000`) does NOT wrap in `i64` (REV-S5-4).
    #[test]
    fn apply_tax_large_cart_full_rate_stays_exact_in_i64() {
        let sub = 2_000_000_000; // ~int4 max, a 2e9 minor-unit cart
        assert_eq!(apply_tax(sub, 1_000_000, false).unwrap(), 2_000_000_000); // exclusive 100%
        assert_eq!(apply_tax(sub, 1_000_000, true).unwrap(), 1_000_000_000); // inclusive 100%
    }

    // ─────────────── LC1 / exclusive composition properties (integer rate_micro) ───────────────

    /// LC1 property (integer twin of the shell's f64 `lc1_inclusive_never_adds_tax…`) — an inclusive
    /// venue NEVER adds tax to the charge, for EVERY rate_micro. `chargedTax == 0`, `total ==
    /// subtotal + fee`, definitionally.
    #[test]
    fn lc1_inclusive_never_adds_tax_total_is_subtotal_plus_fee() {
        let rates_micro = [0i64, 75_000, 100_000, 200_000];
        let subtotals = [500i64, 1000, 1075, 1999, 5000, 123_456];
        let fee = 300i64;
        for rm in rates_micro {
            for sub in subtotals {
                let tax_total = apply_tax(sub, rm, true).unwrap();
                let ct = charged_tax(lek(tax_total), true);
                assert_eq!(ct, Lek::ZERO, "inclusive chargedTax must be 0 @ rm={rm} sub={sub}");
                let total = compose_total(lek(sub), lek(fee), ct, Lek::ZERO).unwrap();
                assert_eq!(
                    total.minor_units(),
                    sub + fee,
                    "inclusive total must equal subtotal+fee @ rm={rm} sub={sub}"
                );
            }
        }
    }

    /// Exclusive property (integer twin) — an exclusive venue adds exactly the extracted tax:
    /// `chargedTax == taxTotal`, `total == subtotal + fee + taxTotal`.
    #[test]
    fn exclusive_adds_exactly_the_extracted_tax() {
        let rm = 200_000i64; // 0.2
        let fee = 300i64;
        for sub in [500i64, 1000, 2000, 5000] {
            let tax_total = apply_tax(sub, rm, false).unwrap();
            let ct = charged_tax(lek(tax_total), false);
            assert_eq!(ct.minor_units(), tax_total, "exclusive chargedTax === taxTotal @ sub={sub}");
            let total = compose_total(lek(sub), lek(fee), ct, Lek::ZERO).unwrap();
            assert_eq!(total.minor_units(), sub + fee + tax_total, "exclusive total @ sub={sub}");
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

    /// REV-S5-6 seam: the `− discountTotal` term is real — a nonzero discount subtracts.
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

    // ─────────────────── delivery-fee ladder — integer-meter oracle vectors ───────────────────

    fn loc(flat: Option<i64>, free: Option<i64>, min: Option<i64>) -> FeeLocation {
        FeeLocation {
            delivery_fee_flat: flat,
            free_delivery_threshold: free,
            min_order_value: min,
        }
    }

    /// The decorrelated oracle §C's 9 integer-meter delivery-fee scenarios (8 rows, #8 split
    /// pickup/delivery) — hand-derived from the Node reference with `max_distance_m`/`distance_m`
    /// whole-meter literals. This is the core's NEW money BYTE-PARITY proof for the integerized ladder.
    #[test]
    fn delivery_fee_for_order_integer_meter_oracle_vectors() {
        let two_tier = [
            DeliveryTier { max_distance_m: 1000, fee: 300 },
            DeliveryTier { max_distance_m: 5000, fee: 500 },
        ];
        let one_tier = [DeliveryTier { max_distance_m: 1000, fee: 300 }];

        // 1 — within tier 1: 800 <= 1000 → 300
        assert_eq!(
            delivery_fee_for_order(lek(2000), false, loc(None, None, None), Some(800), &two_tier)
                .unwrap()
                .minor_units(),
            300
        );
        // 2 — between t1 & t2: 1390 > 1000, 1390 <= 5000 → 500
        assert_eq!(
            delivery_fee_for_order(lek(2000), false, loc(None, None, None), Some(1390), &two_tier)
                .unwrap()
                .minor_units(),
            500
        );
        // 3 — beyond last tier: 7000 covered by none → NOT_DELIVERABLE
        assert_eq!(
            delivery_fee_for_order(lek(2000), false, loc(None, None, None), Some(7000), &two_tier)
                .unwrap_err()
                .code,
            ErrorCode::NotDeliverable
        );
        // 4 — no tiers + flat: empty ladder, flat=250 → 250
        assert_eq!(
            delivery_fee_for_order(lek(2000), false, loc(Some(250), None, None), None, &[])
                .unwrap()
                .minor_units(),
            250
        );
        // 5 — no tiers + no flat → DELIVERY_NOT_CONFIGURED
        assert_eq!(
            delivery_fee_for_order(lek(2000), false, loc(None, None, None), None, &[])
                .unwrap_err()
                .code,
            ErrorCode::DeliveryNotConfigured
        );
        // 6 — free threshold met: 5000 >= 3000 → 0 (tier ladder + 9999 out-of-range never evaluated)
        assert_eq!(
            delivery_fee_for_order(lek(5000), false, loc(None, Some(3000), None), Some(9999), &one_tier)
                .unwrap(),
            Lek::ZERO
        );
        // 7 — pickup short-circuits: tiers/distance ignored → 0
        assert_eq!(
            delivery_fee_for_order(lek(2000), true, loc(None, None, None), Some(9999), &one_tier)
                .unwrap(),
            Lek::ZERO
        );
        // 8a — min not met (delivery): 500 < 1000 → error before any fee resolution
        assert_eq!(
            delivery_fee_for_order(lek(500), false, loc(None, None, Some(1000)), Some(500), &one_tier)
                .unwrap_err()
                .code,
            ErrorCode::MinOrderNotMet
        );
        // 8b — min not met (pickup): min check precedes the pickup fee=0 → error fires for pickup too
        assert_eq!(
            delivery_fee_for_order(lek(500), true, loc(None, None, Some(1000)), None, &[])
                .unwrap_err()
                .code,
            ErrorCode::MinOrderNotMet
        );
    }

    /// The `<=` equality edge (oracle §C boundary note): `distance_m` exactly equal to a tier's
    /// `max_distance_m` selects that tier.
    #[test]
    fn resolve_delivery_fee_equality_boundary_selects_that_tier() {
        let tiers = [DeliveryTier { max_distance_m: 1000, fee: 300 }];
        assert_eq!(
            resolve_delivery_fee(loc(Some(999), None, None), Some(1000), &tiers)
                .unwrap()
                .minor_units(),
            300
        );
    }

    #[test]
    fn resolve_delivery_fee_beyond_last_tier_is_not_deliverable() {
        let tiers = [DeliveryTier { max_distance_m: 100, fee: 200 }];
        let err = resolve_delivery_fee(loc(Some(300), None, None), Some(50_000), &tiers).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotDeliverable);
    }

    #[test]
    fn resolve_delivery_fee_unconfigured_is_delivery_not_configured() {
        let err = resolve_delivery_fee(loc(None, None, None), Some(1000), &[]).unwrap_err();
        assert_eq!(err.code, ErrorCode::DeliveryNotConfigured);
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
        assert_eq!(err.code, ErrorCode::DuplicateModifier);
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
        assert_eq!(err.code, ErrorCode::ModifierUnavailable);
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
        assert_eq!(err.code, ErrorCode::ModifierMinNotMet);
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
        assert_eq!(err.code, ErrorCode::ModifierMaxExceeded);
    }
}
