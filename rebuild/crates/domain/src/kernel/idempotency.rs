//! The idempotency-branch DECISION (`orders.ts:394-412`) — REV-S5-5 (the delete-and-recreate arm).
//!
//! A pure decision over an existing `idempotency_keys` row + the incoming request hash: proceed /
//! replay / 422-reuse / delete-and-recreate. NO IO — the repo performs the lookup and enacts the
//! decision; this only decides. Extracted into the sovereign core in Phase-Zero Step 3 (was
//! `crates/api/.../orders/state.rs`). Carries no float, clock, entropy, or IO.

/// An existing `idempotency_keys` row (tenant-scoped lookup, `orders.ts:400`).
#[derive(Debug, Clone)]
pub struct ExistingKey {
    pub request_hash: String,
    /// The referenced order still exists (`orders.ts:407` re-select found a row).
    pub order_present: bool,
}

/// The idempotency branch decision (`orders.ts:394-412`) — REV-S5-5 (the delete-and-recreate arm).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdempotencyDecision {
    /// No key row → fresh create, proceed to price + persist.
    Proceed,
    /// Key hit + matching hash + order present → replay the committed order (200).
    Replay,
    /// Key hit + DIFFERENT hash → **422 IDEMPOTENCY_KEY_REUSED** (a reused key with a mutated cart
    /// is refused, never silently re-priced).
    Reuse422,
    /// REV-S5-5: key hit + matching hash but the referenced order is GONE (`orders.ts:406-411`) →
    /// `DELETE FROM idempotency_keys WHERE key = $ AND location_id = $`, then fall through and
    /// re-price + re-persist as a fresh create.
    DeleteAndRecreate,
}

/// Ports the section-5 idempotency branch (`orders.ts:394-412`). `new_request_hash` is the incoming
/// request's hash (REV-S5-2). A hit is compared by hash FIRST (mismatch → 422), then by order
/// presence (present → replay; gone → delete-and-recreate).
pub fn idempotency_decision(
    existing: Option<&ExistingKey>,
    new_request_hash: &str,
) -> IdempotencyDecision {
    match existing {
        None => IdempotencyDecision::Proceed,
        Some(key) if key.request_hash != new_request_hash => IdempotencyDecision::Reuse422,
        Some(key) if key.order_present => IdempotencyDecision::Replay,
        Some(_) => IdempotencyDecision::DeleteAndRecreate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idempotency_no_key_proceeds() {
        assert_eq!(
            idempotency_decision(None, "hashA"),
            IdempotencyDecision::Proceed
        );
    }

    #[test]
    fn idempotency_matching_hash_with_order_replays() {
        let key = ExistingKey {
            request_hash: "hashA".to_string(),
            order_present: true,
        };
        assert_eq!(
            idempotency_decision(Some(&key), "hashA"),
            IdempotencyDecision::Replay
        );
    }

    #[test]
    fn idempotency_different_hash_is_reuse_422() {
        let key = ExistingKey {
            request_hash: "hashA".to_string(),
            order_present: true,
        };
        assert_eq!(
            idempotency_decision(Some(&key), "hashB"),
            IdempotencyDecision::Reuse422
        );
    }

    /// REV-S5-5: key hit, hash matches, but the order row is GONE → delete-and-recreate.
    #[test]
    fn idempotency_matching_hash_missing_order_deletes_and_recreates() {
        let key = ExistingKey {
            request_hash: "hashA".to_string(),
            order_present: false,
        };
        assert_eq!(
            idempotency_decision(Some(&key), "hashA"),
            IdempotencyDecision::DeleteAndRecreate
        );
    }

    /// A mismatched hash takes precedence over the missing-order case (hash is checked FIRST,
    /// orders.ts:402 before the re-select) — a reused key with a mutated cart is ALWAYS a 422.
    #[test]
    fn idempotency_hash_mismatch_wins_over_missing_order() {
        let key = ExistingKey {
            request_hash: "hashA".to_string(),
            order_present: false,
        };
        assert_eq!(
            idempotency_decision(Some(&key), "hashB"),
            IdempotencyDecision::Reuse422
        );
    }
}
