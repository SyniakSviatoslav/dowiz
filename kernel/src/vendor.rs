//! P62 / M1 — the intra-hub vendor partition identity.
//!
//! A hub (`location_id` is the cross-hub red-line RLS boundary) hosts `1..N`
//! vendors (§16.15). `VendorId` is the coarser fan-out axis than `product_id`
//! (a leaf): `charge_legs`/`kitchen_tickets`/`validate_tree` all `group_by`
//! it. `N=1` (the common single-vendor hub) is the SAME type at `N>1` — there
//! is no reserved sentinel, so `VendorId(0)` and `VendorId(u64::MAX)` are both
//! ordinary keys (§1.4: a sentinel would be a hidden special-case).
//!
//! `Copy + Ord + Hash` so a `BTreeMap<VendorId, _>` iterates in ascending-id
//! order deterministically — the determinism `charge_legs`/`kitchen_tickets`
//! rely on. `std`-only, no float, no I/O.

/// Stable identity of a vendor WITHIN one hub (one `location_id`).
///
/// `u64` (maps to SQL `BIGINT`) — a cheap group-by key for KDS fan-out +
/// settlement legs; NOT a `String` (product_id is String — a leaf; a vendor is
/// the coarser axis).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VendorId(pub u64);
