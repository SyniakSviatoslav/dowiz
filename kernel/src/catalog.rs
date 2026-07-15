//! Trusted price catalog — M1/M2 money-integrity authority.
//!
//! RED LINE (money): the kernel must be the SINGLE authority on what a line item
//! COSTS. Before this module, `domain::place_order` trusted the caller-supplied
//! `OrderItem::unit_price` verbatim (M1: client sets its own price; M2: no server
//! re-derivation). An attacker could POST an order with `unit_price = 1` for a
//! product that costs 5000.
//!
//! This module provides a trusted [`PriceCatalog`] keyed by `product_id` (+ optional
//! modifier surcharges). When a catalog is supplied to `place_order`, every line's
//! `unit_price` is RE-DERIVED from the catalog and the caller value is IGNORED
//! (overwritten). When no catalog is supplied (legacy call path), the caller value
//! is kept but the order is marked `price_trusted = false` so downstream can refuse
//! to charge it. `std`-only, integer minor units, no float on money.

use std::collections::BTreeMap;

/// A trusted price entry for one product: base unit price + per-modifier surcharges.
/// All amounts are integer minor units (matching `money.rs`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PriceEntry {
    /// Base unit price (minor units) — the authoritative price for one unit.
    pub base: i64,
    /// Surcharge per modifier id (minor units). Missing modifier id → surcharge 0.
    pub modifiers: BTreeMap<String, i64>,
}

/// Trusted catalog: product_id → [`PriceEntry`]. The ONLY authority on line prices.
#[derive(Debug, Clone, Default)]
pub struct PriceCatalog {
    entries: BTreeMap<String, PriceEntry>,
}

impl PriceCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register/replace a product's trusted price entry.
    pub fn insert(&mut self, product_id: impl Into<String>, entry: PriceEntry) {
        self.entries.insert(product_id.into(), entry);
    }

    /// Convenience: register a flat base price with no modifier surcharges.
    pub fn insert_flat(&mut self, product_id: impl Into<String>, base: i64) {
        self.insert(
            product_id,
            PriceEntry {
                base,
                modifiers: BTreeMap::new(),
            },
        );
    }

    /// Authoritative unit price for a product with the given modifiers.
    ///
    /// Returns `Err` when the product is unknown to the catalog (fail-closed — a
    /// caller must never fall back to a client price for a product the trusted
    /// source doesn't recognize). Overflow-safe: modifier surcharges are summed
    /// with `checked_add`.
    pub fn unit_price(&self, product_id: &str, modifier_ids: &[String]) -> Result<i64, String> {
        let entry = self
            .entries
            .get(product_id)
            .ok_or_else(|| format!("unknown product in trusted catalog: {product_id}"))?;
        let mut price = entry.base;
        for m in modifier_ids {
            let sur = entry.modifiers.get(m).copied().unwrap_or(0);
            price = price
                .checked_add(sur)
                .ok_or("catalog unit price overflow (modifier surcharge)")?;
        }
        Ok(price)
    }

    /// True when the product is known to the trusted catalog.
    pub fn knows(&self, product_id: &str) -> bool {
        self.entries.contains_key(product_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> PriceCatalog {
        let mut c = PriceCatalog::new();
        c.insert(
            "p1",
            PriceEntry {
                base: 5000,
                modifiers: BTreeMap::from([("m1".to_string(), 250)]),
            },
        );
        c.insert_flat("p2", 300);
        c
    }

    // GREEN: trusted price = base + modifier surcharges.
    #[test]
    fn green_unit_price_with_modifier() {
        let c = catalog();
        assert_eq!(c.unit_price("p1", &["m1".into()]).unwrap(), 5250);
        assert_eq!(c.unit_price("p1", &[]).unwrap(), 5000);
        assert_eq!(c.unit_price("p2", &[]).unwrap(), 300);
    }

    // GREEN: unknown modifier id contributes 0 surcharge (not an error).
    #[test]
    fn green_unknown_modifier_is_zero_surcharge() {
        let c = catalog();
        assert_eq!(c.unit_price("p1", &["nope".into()]).unwrap(), 5000);
    }

    // RED→fail-closed: unknown product is rejected, never priced.
    #[test]
    fn red_unknown_product_is_err() {
        let c = catalog();
        assert!(c.unit_price("ghost", &[]).is_err());
        assert!(!c.knows("ghost"));
    }

    // RED: overflow on modifier surcharge is Err, never wraps.
    #[test]
    fn red_surcharge_overflow_is_err() {
        let mut c = PriceCatalog::new();
        c.insert(
            "big",
            PriceEntry {
                base: i64::MAX,
                modifiers: BTreeMap::from([("m".to_string(), 1)]),
            },
        );
        assert!(c.unit_price("big", &["m".into()]).is_err());
    }
}
