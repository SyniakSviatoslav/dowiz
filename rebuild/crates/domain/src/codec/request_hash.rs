//! S5 idempotency request-hash — REV-S5-2 (breaker H1: cross-stack canonicalization drift).
//!
//! ## The drift the RESOLVE corrects
//! Node's `buildRequestHash` (`apps/api/src/lib/order-canonical.ts:37-50`) embeds
//! `Math.round(pin.lat * 100000) / 100000` — a float RE-DIVIDED back to a float — into
//! `JSON.stringify`. That re-divided float is where V8 and Rust diverge: V8 serializes `42` where
//! Rust's ryu would serialize `42.0`, `-0` vs `-0.0`, etc. A single golden vector cannot cover the
//! float domain, and a one-byte hash difference across stacks turns a legitimate retry that lands on
//! the OTHER stack into a false `IDEMPOTENCY_KEY_REUSED` (422).
//!
//! ## The contract (REV-S5-2): hash over a NORMALIZED INTEGER PROJECTION
//! This hash is taken over `pin.lat`/`pin.lng` as the **integer** `round(coord · 1e5)` (an `i64`),
//! NOT the re-divided float. An integer serializes byte-identically under V8 `JSON.stringify` and
//! serde_json (`42`, never `42.0`; no `-0`; no ryu shortest-float ambiguity) — so the canonicalization
//! is provably byte-identical across both stacks BY CONSTRUCTION.
//!
//! ## Sovereignty split (Phase-Zero Step 3, Option B — integers-only core)
//! The projection ITSELF — the one `f64 → i64` `round(coord · 1e5)` — is the float boundary, so it
//! stays in the `api` shell (`routes/orders/request_hash.rs::project_coord`, proven pure over the
//! float domain there). This core module receives the pin ALREADY as integers and only canonicalises
//! and hashes it — no float, no clock, no entropy, no IO — so the wasm sovereignty gate proves it
//! belongs in `dowiz-core`.
//!
//! ## De-escalation (RESOLVE): a hash mismatch is a false-422, NOT a duplicate paid order
//! The real duplicate-PREVENTION is the DB `idempotency_keys (key, location_id)` UNIQUE — the second
//! create conflicts at the DB (409). The request-hash only DETECTS a mismatched retry; a drift causes
//! a false-422 (annoying, retryable), never a duplicate order and never a duplicate charge.

use sha2::{Digest, Sha256};

/// One canonical line item (`order-canonical.ts:32-36`): `product_id`, `quantity`, and the
/// `modifier_ids` SORTED (so `[a,b]` and `[b,a]` hash identically — the dedup must be
/// order-insensitive on modifiers). Field order here IS the JSON key order V8 emits.
#[derive(Debug, Clone, serde::Serialize)]
struct CanonicalItem {
    product_id: String,
    quantity: i64,
    modifier_ids: Vec<String>,
}

/// The integer-projected pin (REV-S5-2): `lat`/`lng` are `round(coord · 1e5)` as `i64` — the whole
/// point of the correction. Serializes as bare integers, byte-identical across stacks.
#[derive(Debug, Clone, serde::Serialize)]
struct CanonicalPin {
    lat: i64,
    lng: i64,
}

/// The canonical body, fields in the EXACT order V8 `JSON.stringify` emits them
/// (`order-canonical.ts:40-50`) — serde preserves struct field declaration order, and
/// `serde_json::to_string` produces compact (no-space) output matching `JSON.stringify`.
#[derive(Debug, Clone, serde::Serialize)]
struct CanonicalBody {
    #[serde(rename = "locationId")]
    location_id: String,
    #[serde(rename = "type")]
    order_type: String,
    items: Vec<CanonicalItem>,
    pin: Option<CanonicalPin>,
    address_text: Option<String>,
    cash_pay_with: Option<i64>,
    currency_code: String,
    menu_version: String,
    customer_id: String,
}

/// The typed input to [`build_request_hash`] — mirrors `CanonicalRequestInput`
/// (`order-canonical.ts:15-27`). `customer_id` is `request.user.sub` for a customer token, else
/// `"anonymous"` (the #8 security-hardening fix — port the `.sub` read, `order-canonical.ts:308`).
#[derive(Debug, Clone)]
pub struct CanonicalRequestInput {
    pub location_id: String,
    pub order_type: String,
    pub items: Vec<CanonicalItemInput>,
    /// Delivery pin (`None` for pickup) — ALREADY PROJECTED to integers by the shell
    /// (`round(coord · 1e5)`); the sovereign core never sees a raw coordinate float.
    pub pin: Option<(i64, i64)>,
    pub address_text: Option<String>,
    pub cash_pay_with: Option<i64>,
    pub currency_code: String,
    pub menu_version: String,
    pub customer_id: String,
}

#[derive(Debug, Clone)]
pub struct CanonicalItemInput {
    pub product_id: String,
    pub quantity: i64,
    pub modifier_ids: Vec<String>,
}

/// Ports `buildRequestHash` (`order-canonical.ts:29-51`) with the REV-S5-2 integer-pin correction.
/// SHA-256 of the canonical JSON body, lowercase hex. The pin arrives already integer-projected.
pub fn build_request_hash(input: &CanonicalRequestInput) -> String {
    let items = input
        .items
        .iter()
        .map(|i| {
            let mut modifier_ids = i.modifier_ids.clone();
            modifier_ids.sort(); // [...(modifier_ids||[])].sort()
            CanonicalItem {
                product_id: i.product_id.clone(),
                quantity: i.quantity,
                modifier_ids,
            }
        })
        .collect();

    let pin = input.pin.map(|(lat, lng)| CanonicalPin { lat, lng });

    let body = CanonicalBody {
        location_id: input.location_id.clone(),
        order_type: input.order_type.clone(),
        items,
        pin,
        address_text: input.address_text.clone(),
        // Node: `cashPayWith || null` — 0 and undefined both collapse to null (JS falsy).
        cash_pay_with: input.cash_pay_with.filter(|&v| v != 0),
        currency_code: input.currency_code.clone(),
        menu_version: input.menu_version.clone(),
        customer_id: input.customer_id.clone(),
    };

    // serde_json::to_string is compact (no spaces) — matches V8 JSON.stringify; struct field order
    // is the emitted key order. A serialization failure is impossible for this all-owned struct.
    let canonical = serde_json::to_string(&body).unwrap_or_default();
    let digest = Sha256::digest(canonical.as_bytes());
    hex::encode(digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Pin arrives already integer-projected: 41.327953 → round(4132795.3) = 4132795 ;
    // 19.819025 → round(1981902.5) = 1981903 (the shell's project_coord does this half-up).
    fn base_input() -> CanonicalRequestInput {
        CanonicalRequestInput {
            location_id: "11111111-1111-1111-1111-111111111111".to_string(),
            order_type: "delivery".to_string(),
            items: vec![CanonicalItemInput {
                product_id: "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa".to_string(),
                quantity: 2,
                modifier_ids: vec![
                    "cccccccc-cccc-cccc-cccc-cccccccccccc".to_string(),
                    "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb".to_string(),
                ],
            }],
            pin: Some((4_132_795, 1_981_903)),
            address_text: Some("Rruga X".to_string()),
            cash_pay_with: Some(5000),
            currency_code: "ALL".to_string(),
            menu_version: "7".to_string(),
            customer_id: "anonymous".to_string(),
        }
    }

    /// The hash is deterministic for a fixed cart (the whole point — a retry re-hashes identically).
    #[test]
    fn hash_is_deterministic() {
        let a = build_request_hash(&base_input());
        let b = build_request_hash(&base_input());
        assert_eq!(a, b);
        assert_eq!(a.len(), 64, "sha256 hex is 64 chars");
    }

    /// Modifier order does NOT change the hash (sorted) — `[c,b]` == `[b,c]`.
    #[test]
    fn modifier_order_does_not_change_the_hash() {
        let mut swapped = base_input();
        swapped.items[0].modifier_ids.reverse();
        assert_eq!(build_request_hash(&base_input()), build_request_hash(&swapped));
    }

    /// REV-S5-2 core: the canonical body contains the pin as INTEGERS, never a float token. There is
    /// no `.` inside the `"pin":{...}` object — this is what makes it byte-identical across V8/Rust
    /// (a float `42.0`/`42` ambiguity cannot arise). Proven by inspecting the exact serialized bytes.
    #[test]
    fn pin_is_serialized_as_integers_no_float_token() {
        let input = base_input();
        let items = input
            .items
            .iter()
            .map(|i| {
                let mut m = i.modifier_ids.clone();
                m.sort();
                CanonicalItem {
                    product_id: i.product_id.clone(),
                    quantity: i.quantity,
                    modifier_ids: m,
                }
            })
            .collect();
        let pin = input.pin.map(|(lat, lng)| CanonicalPin { lat, lng });
        let body = CanonicalBody {
            location_id: input.location_id.clone(),
            order_type: input.order_type.clone(),
            items,
            pin,
            address_text: input.address_text.clone(),
            cash_pay_with: input.cash_pay_with.filter(|&v| v != 0),
            currency_code: input.currency_code.clone(),
            menu_version: input.menu_version.clone(),
            customer_id: input.customer_id.clone(),
        };
        let json = serde_json::to_string(&body).unwrap();
        assert!(
            json.contains(r#""pin":{"lat":4132795,"lng":1981903}"#),
            "pin must be integer-projected, got: {json}"
        );
        // Key order matches V8 JSON.stringify (order-canonical.ts).
        assert!(json.starts_with(
            r#"{"locationId":"11111111-1111-1111-1111-111111111111","type":"delivery","items":"#
        ));
    }

    /// `cash_pay_with` falsy-collapse parity: `0` and `None` both serialize as `null` (JS `x||null`).
    #[test]
    fn cash_pay_with_zero_and_none_both_null() {
        let mut zero = base_input();
        zero.cash_pay_with = Some(0);
        let mut none = base_input();
        none.cash_pay_with = None;
        assert_eq!(build_request_hash(&zero), build_request_hash(&none));
        // …and both differ from a real cash amount.
        assert_ne!(build_request_hash(&zero), build_request_hash(&base_input()));
    }

    /// Changing the customer identity (anonymous → a real sub) changes the hash — the #8 fix ensures
    /// the fingerprint actually incorporates identity.
    #[test]
    fn customer_identity_participates_in_the_hash() {
        let mut identified = base_input();
        identified.customer_id = "99999999-9999-9999-9999-999999999999".to_string();
        assert_ne!(build_request_hash(&base_input()), build_request_hash(&identified));
    }

    /// A pickup order (no pin) hashes `pin: null` and stays deterministic.
    #[test]
    fn pickup_order_hashes_null_pin() {
        let mut pickup = base_input();
        pickup.order_type = "pickup".to_string();
        pickup.pin = None;
        let h = build_request_hash(&pickup);
        assert_eq!(h.len(), 64);
        assert_ne!(h, build_request_hash(&base_input()));
    }
}
