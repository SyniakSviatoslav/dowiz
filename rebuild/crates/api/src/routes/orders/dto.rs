//! S5 `CreateOrderInput` — REV-S5-3 (breaker H2: the strict-parse drift is broader than
//! messenger_kind). Ports `packages/shared-types/src/legacy.ts::CreateOrderInput` with the FULL
//! FE→schema field audit the RESOLVE mandates, so the schema admits EXACTLY what the live checkout
//! sends (`apps/web/src/pages/client/CheckoutPage.tsx:320-360`). Two live checkout breaks the port
//! closes (each gated on the DB CHECK/column confirmed to exist):
//!
//!   1. **messenger_kind 3→6** — the Zod enum was the stale `['telegram','whatsapp','viber']`; the
//!      checkout selector (ADR-0016) offers 6 (`phone/whatsapp/viber/telegram/signal/simplex`), and
//!      the DB CHECK (`mig 1790000000074:12`) ADMITS all 6. A customer picking phone/signal/simplex
//!      hit a `.strict()` 422 TODAY; this port unifies to the canonical 6-kind set → 201.
//!   2. **top-level `receiver{}`** — "deliver to someone else" (ADR-0016). The FE sends it top-level
//!      (`CheckoutPage.tsx:328-330`), the DB has the columns (`mig …074:32-36`,
//!      `receiver_{name,messenger_kind,handle}`, same 6-kind CHECK), but the Zod schema had no
//!      `receiver` key → a `.strict()` 400 TODAY. Added here, gated on the confirmed columns.
//!
//! `.strict()` (reject unknown keys) is ported as `#[serde(deny_unknown_fields)]` at every object
//! level. Constraint validation (lengths/ranges/`type`↔`delivery` consistency) that serde cannot
//! express is in [`CreateOrderInput::validate`], matching the Zod `.superRefine` + field refinements
//! — a failure is the route's `400 VALIDATION_FAILED`.

// REV-S5-3: the DTO's PURPOSE is to ADMIT the full live checkout payload (so a `signal` order or a
// `receiver{}` block no longer 400/422s). Several admitted fields (`receiver`, `messenger_*`,
// `otp_code`, `delivery_photo_key`, `prefs`, `acknowledged_codes`, the `payment.method` literal) are
// deliberately parse-and-accept-only in this focused dark port — they gate no money/tenancy/idempotency
// behavior yet, so the funnel does not read them. Admitting them IS the fix; consuming them is later
// FE-lockstep work. `dead_code` is expected here and does not mean the field is pointless.
#![allow(
    dead_code,
    reason = "S5 REV-3: schema admits the full checkout payload; not every admitted field is consumed by the focused create funnel yet — see module note"
)]

use serde::Deserialize;
use uuid::Uuid;

/// The canonical 6-kind messenger set (ADR-0016 / `apps/web/src/lib/messenger.ts:6`), matching the
/// DB CHECK (`mig 1790000000074`). Lowercase serde values = the wire tokens. An unknown kind is a
/// serde error → `400 VALIDATION_FAILED` (the `.strict()` enum behavior).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessengerKind {
    Phone,
    Whatsapp,
    Viber,
    Telegram,
    Signal,
    Simplex,
}

/// `OrderItemInput` (`legacy.ts:30-34`, `.strict()`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderItemInput {
    pub product_id: Uuid,
    pub quantity: i64,
    #[serde(default)]
    pub modifier_ids: Vec<Uuid>,
}

/// The customer contact block (`legacy.ts:44-49`, `.strict()`). messenger_kind is the 6-kind enum
/// (REV-S5-3 fix). All fields optional (a phone-less telegram/simplex order omits `phone`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomerInput {
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub messenger_kind: Option<MessengerKind>,
    #[serde(default)]
    pub messenger_handle: Option<String>,
}

/// REV-S5-3: the top-level `receiver{}` — "deliver to someone else" (ADR-0016). Its OWN 6-kind
/// channel (`CheckoutPage.tsx:329`: `{ name, messenger_kind, handle }`). Note the field is `handle`
/// (NOT `messenger_handle` — the receiver block uses a distinct key, matched verbatim).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiverInput {
    pub name: String,
    pub messenger_kind: MessengerKind,
    pub handle: String,
}

/// The delivery pin (`legacy.ts:51-54`, `.strict()`). Ranges validated in [`CreateOrderInput::validate`].
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryPin {
    pub lat: f64,
    pub lng: f64,
}

/// The delivery block (`legacy.ts:50-56`, `.strict()`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryInput {
    pub pin: DeliveryPin,
    #[serde(default)]
    pub address_text: Option<String>,
}

/// `payment: { method: literal 'cash' }` (`legacy.ts:57-59`, `.strict()`). `Cash` is the only
/// admitted method (crypto stays dark; the crypto fork is a create-side flag, not a payment method
/// on this input).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaymentMethod {
    Cash,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentInput {
    pub method: PaymentMethod,
}

/// `type: z.enum(['delivery','pickup'])` (`legacy.ts:42`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderType {
    Delivery,
    Pickup,
}

/// `CreateOrderInput` (`legacy.ts:40-77`, `.strict()`), with the REV-S5-3 additions. `prefs` is
/// carried as an opaque object (the dropoff/substitution sub-schema is not load-bearing for the
/// money/tenancy/idempotency port and the FE sends a nested `.strict()` object; a permissive
/// `serde_json::Value` keeps the field admitted without re-porting the whole `OrderPreferences`
/// tree, which no S5 handler reads).
// NOTE the mixed casing (verified against `CheckoutPage.tsx` + `legacy.ts`): `locationId` is the
// ONLY camelCase key the FE sends — every other field is snake_case (`cash_pay_with`,
// `idempotency_key`, `acknowledged_codes`, …). So NOT a blanket `rename_all`; `location_id` is the
// one explicit rename, `type` is the Rust-keyword rename, the rest match their Rust field names.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateOrderInput {
    #[serde(rename = "locationId")]
    pub location_id: Uuid,
    #[serde(rename = "type")]
    pub order_type: OrderType,
    pub items: Vec<OrderItemInput>,
    #[serde(default)]
    pub customer: Option<CustomerInput>,
    /// REV-S5-3: top-level receiver (ADR-0016 deliver-to-other).
    #[serde(default)]
    pub receiver: Option<ReceiverInput>,
    #[serde(default)]
    pub delivery: Option<DeliveryInput>,
    pub payment: PaymentInput,
    #[serde(default)]
    pub cash_pay_with: Option<i64>,
    #[serde(default)]
    pub delivery_instructions: Option<String>,
    #[serde(default)]
    pub delivery_photo_key: Option<String>,
    #[serde(default)]
    pub tip_amount: Option<i64>,
    #[serde(default)]
    pub prefs: Option<serde_json::Value>,
    pub idempotency_key: Uuid,
    #[serde(default)]
    pub acknowledged_codes: Vec<String>,
    #[serde(default)]
    pub otp_code: Option<String>,
}

impl CreateOrderInput {
    /// Ports the Zod field refinements + `.superRefine` (`legacy.ts:60-77`) that serde cannot
    /// express structurally. Returns the FIRST failure message (the route joins issue messages into
    /// `400 VALIDATION_FAILED`). Scoped to the constraints that guard correctness/DB CHECKs — the
    /// exhaustive per-field length bounds are a FE mirror (server re-validates on write), so only the
    /// load-bearing ones are re-checked here (items non-empty + quantity range + the delivery↔pin
    /// consistency superRefine + the pin ranges the DB `double precision` columns accept).
    pub fn validate(&self) -> Result<(), &'static str> {
        // items: z.array(...).min(1)
        if self.items.is_empty() {
            return Err("items must contain at least one item");
        }
        // OrderItemInput.quantity: z.number().int().positive().max(99)
        for item in &self.items {
            if item.quantity < 1 || item.quantity > 99 {
                return Err("item quantity must be between 1 and 99");
            }
        }
        // tip_amount: z.number().int().min(0).max(10_000_000)
        if let Some(tip) = self.tip_amount {
            if !(0..=10_000_000).contains(&tip) {
                return Err("tip_amount out of range");
            }
        }
        // cash_pay_with: z.number().int().positive()
        if let Some(cash) = self.cash_pay_with {
            if cash <= 0 {
                return Err("cash_pay_with must be positive");
            }
        }
        // superRefine (legacy.ts:73-76): a delivery order MUST carry a delivery block …
        if self.order_type == OrderType::Delivery && self.delivery.is_none() {
            return Err("delivery is required for delivery orders");
        }
        // … and the pin ranges (legacy.ts:52-53: lat -90..90, lng -180..180).
        if let Some(delivery) = &self.delivery {
            let pin = &delivery.pin;
            if !(-90.0..=90.0).contains(&pin.lat) || !(-180.0..=180.0).contains(&pin.lng) {
                return Err("delivery pin out of range");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_json() -> serde_json::Value {
        serde_json::json!({
            "locationId": "11111111-1111-1111-1111-111111111111",
            "type": "delivery",
            "items": [{ "product_id": "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa", "quantity": 2 }],
            "payment": { "method": "cash" },
            "delivery": { "pin": { "lat": 41.32, "lng": 19.82 } },
            "idempotency_key": "22222222-2222-2222-2222-222222222222"
        })
    }

    fn parse(v: serde_json::Value) -> Result<CreateOrderInput, serde_json::Error> {
        serde_json::from_value(v)
    }

    #[test]
    fn base_delivery_order_parses_and_validates() {
        let input = parse(base_json()).unwrap();
        assert!(input.validate().is_ok());
        assert_eq!(input.order_type, OrderType::Delivery);
        assert_eq!(input.items.len(), 1);
    }

    /// REV-S5-3 core #1: a `signal` order — a live 422 TODAY — now PARSES (was rejected by the stale
    /// 3-value enum). Ditto `phone`/`simplex`.
    #[test]
    fn signal_phone_simplex_messenger_kinds_now_parse() {
        for kind in ["signal", "phone", "simplex"] {
            let mut v = base_json();
            v["customer"] = serde_json::json!({ "messenger_kind": kind, "messenger_handle": "x" });
            let input = parse(v).unwrap_or_else(|e| panic!("{kind} must parse: {e}"));
            assert!(input.customer.unwrap().messenger_kind.is_some());
        }
    }

    /// The pre-existing 3 kinds still parse (no regression).
    #[test]
    fn legacy_three_kinds_still_parse() {
        for kind in ["telegram", "whatsapp", "viber"] {
            let mut v = base_json();
            v["customer"] = serde_json::json!({ "messenger_kind": kind });
            assert!(parse(v).is_ok(), "{kind} must still parse");
        }
    }

    /// An unknown messenger_kind is rejected (the 6-kind enum is closed — DB CHECK parity).
    #[test]
    fn unknown_messenger_kind_is_rejected() {
        let mut v = base_json();
        v["customer"] = serde_json::json!({ "messenger_kind": "myspace" });
        assert!(parse(v).is_err());
    }

    /// REV-S5-3 core #2: top-level `receiver{}` (deliver-to-other) — a live 400 TODAY — now PARSES.
    #[test]
    fn top_level_receiver_now_parses() {
        let mut v = base_json();
        v["receiver"] = serde_json::json!({
            "name": "Ana", "messenger_kind": "signal", "handle": "@ana"
        });
        let input = parse(v).unwrap();
        let r = input.receiver.unwrap();
        assert_eq!(r.name, "Ana");
        assert_eq!(r.messenger_kind, MessengerKind::Signal);
        assert_eq!(r.handle, "@ana");
    }

    /// `.strict()` — an unknown TOP-LEVEL key is rejected (deny_unknown_fields).
    #[test]
    fn unknown_top_level_key_is_rejected() {
        let mut v = base_json();
        v["surprise"] = serde_json::json!("boom");
        assert!(parse(v).is_err());
    }

    /// `.strict()` — an unknown key inside the receiver block is rejected too.
    #[test]
    fn unknown_receiver_key_is_rejected() {
        let mut v = base_json();
        v["receiver"] = serde_json::json!({
            "name": "Ana", "messenger_kind": "signal", "handle": "@ana", "extra": 1
        });
        assert!(parse(v).is_err());
    }

    /// superRefine: a delivery order WITHOUT a delivery block parses (serde) but FAILS validate().
    #[test]
    fn delivery_without_delivery_block_fails_validate() {
        let mut v = base_json();
        v.as_object_mut().unwrap().remove("delivery");
        let input = parse(v).unwrap();
        assert_eq!(
            input.validate(),
            Err("delivery is required for delivery orders")
        );
    }

    /// A pickup order needs no delivery block (superRefine only fires for delivery).
    #[test]
    fn pickup_without_delivery_block_validates() {
        let mut v = base_json();
        v["type"] = serde_json::json!("pickup");
        v.as_object_mut().unwrap().remove("delivery");
        let input = parse(v).unwrap();
        assert!(input.validate().is_ok());
    }

    #[test]
    fn out_of_range_pin_fails_validate() {
        let mut v = base_json();
        v["delivery"]["pin"]["lat"] = serde_json::json!(120.0);
        let input = parse(v).unwrap();
        assert_eq!(input.validate(), Err("delivery pin out of range"));
    }

    #[test]
    fn empty_items_fails_validate() {
        let mut v = base_json();
        v["items"] = serde_json::json!([]);
        let input = parse(v).unwrap();
        assert_eq!(
            input.validate(),
            Err("items must contain at least one item")
        );
    }

    #[test]
    fn out_of_range_quantity_fails_validate() {
        let mut v = base_json();
        v["items"][0]["quantity"] = serde_json::json!(200);
        let input = parse(v).unwrap();
        assert_eq!(
            input.validate(),
            Err("item quantity must be between 1 and 99")
        );
    }

    #[test]
    fn non_cash_payment_method_is_rejected() {
        let mut v = base_json();
        v["payment"]["method"] = serde_json::json!("crypto");
        assert!(parse(v).is_err(), "only cash is admitted on this input");
    }
}
