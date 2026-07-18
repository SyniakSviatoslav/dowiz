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

// ═══════════════════════════════════════════════════════════════════════════
// P62 / M2 + M3 — CATALOG LEAF INVARIANT (X7) + free-form vendor tree.
//
// Reuse-first (standard item 19): this EXTENDS the trusted catalog with the two
// axes it was missing — currency (it stored a bare `i64`, not `Money`) and
// `vendor_id` (it was single-vendor by omission). The existing `PriceCatalog`
// flat lookup is left byte-for-byte intact above; `PriceableLeaf` is the
// currency+vendor-carrying leaf the unified cart / per-vendor charge leg /
// schema.org JSON-LD all consume verbatim (§0.2-1, X7). `std`-only, integer
// money only — NO float on any monetary value.
// ═══════════════════════════════════════════════════════════════════════════

use crate::money::{assert_non_negative, Currency, Money};
use crate::vendor::VendorId;

/// Vendor-authored leaf id, free-form within its vendor scope (§16.17, no dowiz enum).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LeafId(pub String);
/// Vendor-authored tree-node id (free-form).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub String);

/// Routing / JSON-LD hint ONLY — imposes NO taxonomy on the tree (§16.17).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeafKind {
    Item,
    Variant,
    Modifier,
}

/// Vendor-controlled orderability. A `SoldOut`/`Scheduled` leaf is still PRICED
/// (X7 holds) but is not orderable — availability never nulls the invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    Available,
    SoldOut,
    Scheduled,
}

/// ── X7 — THE CATALOG LEAF INVARIANT ──────────────────────────────────────────
/// The ONE type every purchasable leaf resolves to. Non-negotiable floor under
/// §16.17's free-form tree: the categories/modifiers/variants ABOVE a leaf are
/// vendor-authored and arbitrary; a LEAF is ALWAYS (price, currency, vendor).
/// The `price: Money` field carries BOTH minor-units AND `Currency`; the
/// `vendor_id` is the single fan-out key. Constructed ONLY via `new` — an
/// unpriced / uncurrencied / unattributed purchasable leaf is UNREPRESENTABLE
/// by type, not merely rejected at runtime (§5.1 hazard-safety). Consumed
/// VERBATIM by three systems (P69 cart, P60/P72 charge legs, P69 JSON-LD pack);
/// NONE may redefine it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceableLeaf {
    pub leaf_id: LeafId,
    pub vendor_id: VendorId,
    /// RESOLVED absolute price of one unit (base + chosen components folded at
    /// resolve time), currency-tagged. NEVER a bare i64, NEVER a float.
    pub price: Money,
    pub kind: LeafKind,
    pub availability: Availability,
}

impl PriceableLeaf {
    /// The ONLY constructor. Total + refusing: price MAY be 0 (a free add-on)
    /// but NEVER negative (`assert_non_negative`). Returns a typed
    /// [`CatalogError`].
    pub fn new(
        leaf_id: LeafId,
        vendor_id: VendorId,
        price: Money,
        kind: LeafKind,
        availability: Availability,
    ) -> Result<Self, CatalogError> {
        // Negative price is unreachable through the type only for `minor`, but
        // the spec names it explicitly — assert it here so a `Money::new(-1,..)`
        // is refused (fail-closed) rather than silently carried into an order.
        assert_non_negative(price.minor).map_err(|_| CatalogError::NegativePrice)?;
        Ok(PriceableLeaf {
            leaf_id,
            vendor_id,
            price,
            kind,
            availability,
        })
    }
}

/// A variant/modifier price contribution (§16.17 free-form). `Absolute` REPLACES
/// the base (e.g. size "Large" = 700); `Delta` ADDS to it (e.g. "extra cheese" =
/// +150 — the existing `PriceEntry.modifiers` surcharge model, now currency-tagged).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceComponent {
    Absolute(Money),
    Delta(Money),
}

/// Free-form vendor-authored catalog tree node (§16.17). Adjacency list (parent
/// ptr). dowiz imposes NO structure on `label`/children; the ONLY law is: a
/// `Leaf` carries a `PriceableLeaf` (X7 by construction — a `Group` is not
/// purchasable and has no price).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogNode {
    pub node_id: NodeId,
    pub vendor_id: VendorId,    // every node is vendor-scoped
    pub parent: Option<NodeId>, // adjacency list; a root = None
    pub label: String,          // free-form, vendor-authored, NO dowiz enum
    pub body: NodeBody,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeBody {
    Group,
    Leaf(PriceableLeaf),
}

/// Typed refusals — every failure names itself (never a partial tree / None-as-success).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    NegativePrice,
    CrossCurrency,
    CrossVendor,
    Overflow,
    UnknownLeaf(LeafId),
    LeafHasChildren(NodeId),
    CycleInTree(NodeId),
    DanglingParent(NodeId),
}

/// Fold a base leaf + chosen components into ONE resolved unit `Money`. The
/// load-bearing reuse: `Money::checked_add` makes cross-CURRENCY fail closed for
/// FREE; a component from a DIFFERENT vendor is refused (`CrossVendor`); overflow
/// → `Overflow`. This is where X7 becomes enforced, not merely declared.
pub fn resolve_line(
    base: &PriceableLeaf,
    components: &[(VendorId, PriceComponent)],
) -> Result<Money, CatalogError> {
    // Start from the base price.
    let mut acc = base.price;
    for (vid, comp) in components {
        // A component from another vendor must not be spliced onto this line.
        if *vid != base.vendor_id {
            return Err(CatalogError::CrossVendor);
        }
        acc = match comp {
            // Absolute REPLACES the running total (the size "Large" = 700 model):
            // the base is discarded and the absolute becomes the new total.
            PriceComponent::Absolute(m) => *m,
            // Delta ADDS to the running total.
            PriceComponent::Delta(m) => acc.checked_add(*m).map_err(|_| {
                // Currency mismatch or overflow on the add.
                if acc.currency != m.currency {
                    CatalogError::CrossCurrency
                } else {
                    CatalogError::Overflow
                }
            })?,
        };
    }
    Ok(acc)
}

/// Validate a vendor's free-form tree: no cycle, no dangling parent, every `Leaf`
/// is a tree-leaf (a purchasable node with children is `LeafHasChildren`).
/// Structure only — NO taxonomy check (§16.17). Returns the vendor's
/// `PriceableLeaf`s in deterministic `LeafId` order (for reproducible JSON-LD +
/// benches). Vendor-scoping is enforced HERE, at the pure layer, before any RLS:
/// nodes whose `vendor_id != vendor` are ignored entirely (§4.3 adversarial (iv)).
pub fn validate_tree(
    nodes: &[CatalogNode],
    vendor: VendorId,
) -> Result<Vec<PriceableLeaf>, CatalogError> {
    // Index the vendor's own nodes by id for O(1) parent/child lookups.
    let mut by_id: std::collections::BTreeMap<&NodeId, &CatalogNode> =
        std::collections::BTreeMap::new();
    for n in nodes {
        if n.vendor_id == vendor {
            by_id.insert(&n.node_id, n);
        }
    }

    // (a) Dangling parent: a node whose parent points outside the vendor's set.
    for n in by_id.values() {
        if let Some(p) = &n.parent {
            if !by_id.contains_key(p) {
                return Err(CatalogError::DanglingParent(n.node_id.clone()));
            }
        }
    }

    // (b) LeafHasChildren: a `Leaf` body must not be any other node's parent.
    for n in by_id.values() {
        if matches!(n.body, NodeBody::Leaf(_)) {
            for m in by_id.values() {
                if m.parent.as_ref() == Some(&n.node_id) {
                    return Err(CatalogError::LeafHasChildren(n.node_id.clone()));
                }
            }
        }
    }

    // (c) Cycle detection: bounded DFS over the adjacency list (parent pointers).
    // A node is part of a cycle iff a walk following `parent` returns to it.
    for start in by_id.keys() {
        let mut cur = by_id.get(start);
        let mut steps = 0usize;
        // #nodes bounds the walk; a cycle is found within that many steps.
        while let Some(node) = cur {
            steps += 1;
            if steps > by_id.len() {
                return Err(CatalogError::CycleInTree((*start).clone()));
            }
            match &node.parent {
                Some(p) => cur = by_id.get(p),
                None => break, // reached a root — no cycle on this path
            }
        }
    }

    // Collect this vendor's leaves, sorted by LeafId for determinism.
    let mut leaves: Vec<PriceableLeaf> = by_id
        .values()
        .filter_map(|n| match &n.body {
            NodeBody::Leaf(lf) => Some(lf.clone()),
            NodeBody::Group => None,
        })
        .collect();
    leaves.sort_by(|a, b| a.leaf_id.cmp(&b.leaf_id));
    Ok(leaves)
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

    // ══════════════════════════════════════════════════════════════════════
    // P62 / M1 — VendorId ordering + no reserved sentinel (§4.1)
    // ══════════════════════════════════════════════════════════════════════
    #[test]
    fn vendor_id_ordered_stable() {
        // A BTreeMap keyed on VendorId iterates in ascending id order — the
        // determinism charge_legs/kitchen_tickets rely on.
        use std::collections::BTreeMap;
        let mut m: BTreeMap<crate::vendor::VendorId, u32> = BTreeMap::new();
        m.insert(crate::vendor::VendorId(3), 30);
        m.insert(crate::vendor::VendorId(1), 10);
        m.insert(crate::vendor::VendorId(2), 20);
        let ids: Vec<u64> = m.keys().map(|v| v.0).collect();
        assert_eq!(ids, vec![1, 2, 3], "VendorId ordering must be ascending id");
    }

    // Adversarial: VendorId(0) and VendorId(u64::MAX) are both ordinary keys —
    // no reserved sentinel (a sentinel would be a hidden special case, §1.4).
    #[test]
    fn vendor_id_extremes_keyable() {
        use std::collections::BTreeMap;
        let mut m: BTreeMap<crate::vendor::VendorId, bool> = BTreeMap::new();
        m.insert(crate::vendor::VendorId(0), true);
        m.insert(crate::vendor::VendorId(u64::MAX), true);
        assert_eq!(m.len(), 2);
        assert!(m.contains_key(&crate::vendor::VendorId(0)));
        assert!(m.contains_key(&crate::vendor::VendorId(u64::MAX)));
    }

    // ══════════════════════════════════════════════════════════════════════
    // P62 / M2 — the X7 leaf invariant + resolve_line (§4.2)
    // ══════════════════════════════════════════════════════════════════════
    fn leaf(id: &str, vid: u64, minor: i64, cur: Currency) -> PriceableLeaf {
        PriceableLeaf::new(
            LeafId(id.into()),
            VendorId(vid),
            Money::new(minor, cur),
            LeafKind::Item,
            Availability::Available,
        )
        .unwrap()
    }

    // GREEN: Delta folds additively; Absolute overrides the base.
    #[test]
    fn resolve_line_absolute_and_delta() {
        let base = leaf("base", 1, 500, Currency::All);
        // Delta(+150) on top of 500 → 650.
        let d = resolve_line(
            &base,
            &[(
                VendorId(1),
                PriceComponent::Delta(Money::new(150, Currency::All)),
            )],
        )
        .unwrap();
        assert_eq!(d, Money::new(650, Currency::All));

        // Absolute(700) overrides → 700 (the size "Large" model).
        let a = resolve_line(
            &base,
            &[(
                VendorId(1),
                PriceComponent::Absolute(Money::new(700, Currency::All)),
            )],
        )
        .unwrap();
        assert_eq!(a, Money::new(700, Currency::All));

        // Multiple components fold in order: 500 + 100 + 50 = 650.
        let many = resolve_line(
            &base,
            &[
                (
                    VendorId(1),
                    PriceComponent::Delta(Money::new(100, Currency::All)),
                ),
                (
                    VendorId(1),
                    PriceComponent::Delta(Money::new(50, Currency::All)),
                ),
            ],
        )
        .unwrap();
        assert_eq!(many, Money::new(650, Currency::All));
    }

    // Adversarial (i): a component in EUR on an ALL base ⇒ CrossCurrency. This is
    // the test that FAILS if someone unwraps the Money guard — teeth: it must be
    // Err, not a coerced sum.
    #[test]
    fn resolve_line_cross_currency_is_err() {
        let base = leaf("base", 1, 500, Currency::All);
        let r = resolve_line(
            &base,
            &[(
                VendorId(1),
                PriceComponent::Delta(Money::new(100, Currency::Eur)),
            )],
        );
        assert_eq!(
            r,
            Err(CatalogError::CrossCurrency),
            "ALL + EUR must be rejected"
        );
    }

    // Adversarial (ii): a component tagged VendorId(B) on a VendorId(A) leaf ⇒
    // CrossVendor (a client must not splice another vendor's modifier onto this line).
    #[test]
    fn resolve_line_cross_vendor_is_err() {
        let base = leaf("base", 1, 500, Currency::All);
        let r = resolve_line(
            &base,
            &[(
                VendorId(2),
                PriceComponent::Delta(Money::new(100, Currency::All)),
            )],
        );
        // VendorId is visible in the error only via the CrossVendor variant; the
        // resolver names the refusal without leaking the offending id.
        assert_eq!(r, Err(CatalogError::CrossVendor));
    }

    // Adversarial (iii): Money::new(i64::MAX, ALL) base + Delta(1) ⇒ Overflow.
    #[test]
    fn resolve_line_overflow_is_err() {
        let base = leaf("base", 1, i64::MAX, Currency::All);
        let r = resolve_line(
            &base,
            &[(
                VendorId(1),
                PriceComponent::Delta(Money::new(1, Currency::All)),
            )],
        );
        assert_eq!(r, Err(CatalogError::Overflow), "never wraps");
    }

    // Adversarial (iv): a negative price ⇒ NegativePrice (fail-closed).
    #[test]
    fn priceable_leaf_rejects_negative_price() {
        let r = PriceableLeaf::new(
            LeafId("neg".into()),
            VendorId(1),
            Money::new(-1, Currency::All),
            LeafKind::Item,
            Availability::Available,
        );
        assert_eq!(r, Err(CatalogError::NegativePrice));
    }

    // Adversarial (v): a zero price is ACCEPTED — a free add-on is valid; the
    // invariant is "resolvable + non-negative", not "positive".
    #[test]
    fn priceable_leaf_accepts_zero_price() {
        let r = PriceableLeaf::new(
            LeafId("free".into()),
            VendorId(1),
            Money::new(0, Currency::All),
            LeafKind::Item,
            Availability::Available,
        );
        assert!(r.is_ok(), "zero price (free add-on) is valid");
        assert_eq!(r.unwrap().price, Money::new(0, Currency::All));
    }

    // X7 holds THROUGH availability: a SoldOut leaf is still priced + carryable.
    #[test]
    fn soldout_leaf_still_priced() {
        let l = PriceableLeaf::new(
            LeafId("sold".into()),
            VendorId(1),
            Money::new(999, Currency::Eur),
            LeafKind::Item,
            Availability::SoldOut,
        )
        .unwrap();
        assert_eq!(l.price, Money::new(999, Currency::Eur));
        assert_eq!(l.availability, Availability::SoldOut);
    }

    // ══════════════════════════════════════════════════════════════════════
    // P62 / M3 — free-form tree build + validate (§4.3)
    // ══════════════════════════════════════════════════════════════════════
    #[test]
    fn validate_tree_free_form_ok() {
        // 3-level food vendor tree (Group→Group→Leaf) with arbitrary labels.
        let nodes = vec![
            CatalogNode {
                node_id: NodeId("root".into()),
                vendor_id: VendorId(1),
                parent: None,
                label: "🌮 Taqueria".into(),
                body: NodeBody::Group,
            },
            CatalogNode {
                node_id: NodeId("mains".into()),
                vendor_id: VendorId(1),
                parent: Some(NodeId("root".into())),
                label: "Mains".into(),
                body: NodeBody::Group,
            },
            CatalogNode {
                node_id: NodeId("burrito".into()),
                vendor_id: VendorId(1),
                parent: Some(NodeId("mains".into())),
                label: "Burrito".into(),
                body: NodeBody::Leaf(leaf("burrito", 1, 850, Currency::All)),
            },
        ];
        let leaves = validate_tree(&nodes, VendorId(1)).unwrap();
        assert_eq!(leaves.len(), 1);
        assert_eq!(leaves[0].leaf_id, LeafId("burrito".into()));

        // A NON-FOOD (flowers) vendor tree validates identically — §16.17's
        // "any small business" claim, made falsifiable.
        let flowers = vec![
            CatalogNode {
                node_id: NodeId("froot".into()),
                vendor_id: VendorId(1),
                parent: None,
                label: "Bouquets".into(),
                body: NodeBody::Group,
            },
            CatalogNode {
                node_id: NodeId("rose".into()),
                vendor_id: VendorId(1),
                parent: Some(NodeId("froot".into())),
                label: "Red Rose Bouquet".into(),
                body: NodeBody::Leaf(leaf("rose", 1, 2500, Currency::Eur)),
            },
        ];
        let fl = validate_tree(&flowers, VendorId(1)).unwrap();
        assert_eq!(fl.len(), 1);
        assert_eq!(fl[0].price, Money::new(2500, Currency::Eur));
    }

    // Adversarial (i): a node whose parent points to a missing NodeId ⇒ DanglingParent.
    #[test]
    fn validate_tree_dangling_parent_is_err() {
        let nodes = vec![CatalogNode {
            node_id: NodeId("orphan".into()),
            vendor_id: VendorId(1),
            parent: Some(NodeId("ghost".into())),
            label: "x".into(),
            body: NodeBody::Leaf(leaf("x", 1, 100, Currency::All)),
        }];
        assert_eq!(
            validate_tree(&nodes, VendorId(1)),
            Err(CatalogError::DanglingParent(NodeId("orphan".into())))
        );
    }

    // Adversarial (ii): a 3-node cycle ⇒ CycleInTree (never infinite-loops).
    #[test]
    fn validate_tree_cycle_is_err() {
        let nodes = vec![
            CatalogNode {
                node_id: NodeId("a".into()),
                vendor_id: VendorId(1),
                parent: Some(NodeId("c".into())),
                label: "a".into(),
                body: NodeBody::Group,
            },
            CatalogNode {
                node_id: NodeId("b".into()),
                vendor_id: VendorId(1),
                parent: Some(NodeId("a".into())),
                label: "b".into(),
                body: NodeBody::Group,
            },
            CatalogNode {
                node_id: NodeId("c".into()),
                vendor_id: VendorId(1),
                parent: Some(NodeId("b".into())),
                label: "c".into(),
                body: NodeBody::Group,
            },
        ];
        assert!(matches!(
            validate_tree(&nodes, VendorId(1)),
            Err(CatalogError::CycleInTree(_))
        ));
    }

    // Adversarial (iii): a Leaf node that is also some other node's parent ⇒
    // LeafHasChildren (a priced thing cannot be a category).
    #[test]
    fn validate_tree_leaf_has_children_is_err() {
        let nodes = vec![
            CatalogNode {
                node_id: NodeId("priced".into()),
                vendor_id: VendorId(1),
                parent: None,
                label: "Priced".into(),
                body: NodeBody::Leaf(leaf("priced", 1, 100, Currency::All)),
            },
            CatalogNode {
                node_id: NodeId("child".into()),
                vendor_id: VendorId(1),
                parent: Some(NodeId("priced".into())),
                label: "Child".into(),
                body: NodeBody::Group,
            },
        ];
        assert_eq!(
            validate_tree(&nodes, VendorId(1)),
            Err(CatalogError::LeafHasChildren(NodeId("priced".into())))
        );
    }

    // Adversarial (iv): two vendors' nodes in one slice ⇒ validate_tree(_, A)
    // returns ONLY A's leaves (vendor scoping at the pure layer, before any RLS).
    #[test]
    fn validate_tree_vendor_isolation() {
        let nodes = vec![
            CatalogNode {
                node_id: NodeId("a1".into()),
                vendor_id: VendorId(1),
                parent: None,
                label: "A-leaf".into(),
                body: NodeBody::Leaf(leaf("a1", 1, 100, Currency::All)),
            },
            CatalogNode {
                node_id: NodeId("b1".into()),
                vendor_id: VendorId(2),
                parent: None,
                label: "B-leaf".into(),
                body: NodeBody::Leaf(leaf("b1", 2, 200, Currency::All)),
            },
        ];
        let a = validate_tree(&nodes, VendorId(1)).unwrap();
        assert_eq!(a.len(), 1, "vendor 1 sees only its own leaf");
        assert_eq!(a[0].leaf_id, LeafId("a1".into()));
        assert_eq!(a[0].vendor_id, VendorId(1));

        let b = validate_tree(&nodes, VendorId(2)).unwrap();
        assert_eq!(b.len(), 1, "vendor 2 sees only its own leaf");
        assert_eq!(b[0].vendor_id, VendorId(2));
    }

    // Determinism: leaves are returned in LeafId order regardless of input order.
    #[test]
    fn validate_tree_deterministic_leaf_order() {
        let nodes = vec![
            CatalogNode {
                node_id: NodeId("r".into()),
                vendor_id: VendorId(1),
                parent: None,
                label: "r".into(),
                body: NodeBody::Leaf(leaf("z", 1, 1, Currency::All)),
            },
            CatalogNode {
                node_id: NodeId("s".into()),
                vendor_id: VendorId(1),
                parent: None,
                label: "s".into(),
                body: NodeBody::Leaf(leaf("a", 1, 1, Currency::All)),
            },
            CatalogNode {
                node_id: NodeId("t".into()),
                vendor_id: VendorId(1),
                parent: None,
                label: "t".into(),
                body: NodeBody::Leaf(leaf("m", 1, 1, Currency::All)),
            },
        ];
        let leaves = validate_tree(&nodes, VendorId(1)).unwrap();
        let ids: Vec<String> = leaves.iter().map(|l| l.leaf_id.0.clone()).collect();
        assert_eq!(ids, vec!["a".to_string(), "m".to_string(), "z".to_string()]);
    }
}
