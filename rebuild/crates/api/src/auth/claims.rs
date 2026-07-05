//! `Claims` — the JWT claims strict discriminated union, ported from
//! `packages/shared-types/src/legacy.ts:161-175`'s `AuthToken = z.discriminatedUnion('role', [...])`
//! byte-for-byte: one variant per role (owner/courier/customer), each `.strict()` (deny unknown
//! fields), sharing a base `{sub, iat, exp, kid}`.
//!
//! ## Why NOT `#[serde(tag = "role")]` on the enum
//! Serde's automatic internally-tagged-enum derive re-injects the tag key into the buffered map
//! before handing it to the variant's `Deserialize` impl — so if the variant struct does not
//! itself declare a `role` field, `deny_unknown_fields` unconditionally rejects the tag it was
//! just given (a well-known serde gotcha). The alternative — dropping `deny_unknown_fields` on the
//! variant structs — would silently defeat the S2 council's central strictness requirement
//! (proposal §3, threat-model T-3: "a lenient Rust deserializer... is why AR-2's activate token
//! is unverifiable today — the strictness is a live control, port it"). So each variant struct
//! below declares its own `role: Role` field (mirroring the TS source, which ALSO stores
//! `role: z.literal('owner')` as a real object property, not a wrapper discriminant) and `Claims`
//! hand-rolls `Serialize`/`Deserialize` by peeking that field — giving the exact same one-role-
//! literal-per-object wire shape as `legacy.ts`, with full `deny_unknown_fields` on every variant
//! and no double-injected `role` key on serialize.

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Owner,
    Courier,
    Customer,
}

impl Role {
    pub const fn as_str(self) -> &'static str {
        match self {
            Role::Owner => "owner",
            Role::Courier => "courier",
            Role::Customer => "customer",
        }
    }
}

/// `AuthBase` (`legacy.ts:162`): `{ sub: uuid, iat: int, exp: int, kid: string }`, present on
/// every variant. Not a separate Rust type (each variant struct below inlines these four fields)
/// — a shared base type would need `#[serde(flatten)]`, which is incompatible with
/// `deny_unknown_fields` on the flattened side (another serde limitation), so the fields are
/// simply repeated three times, exactly as the Zod source's `...AuthBase` spread does at the
/// value level (the TS source doesn't have this problem because Zod's `.strict()` operates after
/// the spread resolves to one flat shape).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct OwnerClaims {
    pub role: Role,
    #[serde(rename = "userId")]
    pub user_id: Uuid,
    #[serde(
        rename = "activeLocationId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub active_location_id: Option<Uuid>,
    pub sub: Uuid,
    pub iat: i64,
    pub exp: i64,
    pub kid: String,
}

impl OwnerClaims {
    /// `sub` always defaults to `userId` on the Rust port (proposal §5 note: "Net result is
    /// identical (`sub === userId` on every owner token). The Rust signer must reproduce 'sub
    /// defaults to userId'"). `iat`/`exp`/`kid` are placeholders (`0`/`0`/`""`) — `jwt::mint`
    /// overwrites them right before signing (see that module's doc for why minting, not
    /// construction, is the single place that stamps timestamps + kid).
    pub fn new(user_id: Uuid, active_location_id: Option<Uuid>) -> Self {
        OwnerClaims {
            role: Role::Owner,
            user_id,
            active_location_id,
            sub: user_id,
            iat: 0,
            exp: 0,
            kid: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CourierClaims {
    pub role: Role,
    /// REQUIRED for couriers (openapi `CourierClaims.activeLocationId`, not optional like the
    /// owner variant) — a courier token with no location is unrepresentable.
    #[serde(rename = "activeLocationId")]
    pub active_location_id: Uuid,
    /// `courier_sessions.id` — the live session-bind key (REV-1, `plugins/auth.ts:63-83`).
    /// Optional: absent only on a dev-mock-auth-minted token (no real session row behind it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jti: Option<Uuid>,
    pub sub: Uuid,
    pub iat: i64,
    pub exp: i64,
    pub kid: String,
}

impl CourierClaims {
    pub fn new(courier_id: Uuid, active_location_id: Uuid, jti: Option<Uuid>) -> Self {
        CourierClaims {
            role: Role::Courier,
            active_location_id,
            jti,
            sub: courier_id,
            iat: 0,
            exp: 0,
            kid: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CustomerClaims {
    pub role: Role,
    #[serde(rename = "orderId")]
    pub order_id: Uuid,
    #[serde(rename = "locationId")]
    pub location_id: Uuid,
    // P0-PII (`jwt.ts:122-125`): NEVER add a phone/contact field here. `deny_unknown_fields`
    // plus the total absence of such a field is the structural guarantee — a convenience-adding
    // edit that tried to smuggle one in would be a compile error at every construction site
    // below, not a runtime surprise (threat-model T-8).
    pub sub: Uuid,
    pub iat: i64,
    pub exp: i64,
    pub kid: String,
}

impl CustomerClaims {
    pub fn new(customer_id: Uuid, order_id: Uuid, location_id: Uuid) -> Self {
        CustomerClaims {
            role: Role::Customer,
            order_id,
            location_id,
            sub: customer_id,
            iat: 0,
            exp: 0,
            kid: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Claims {
    Owner(OwnerClaims),
    Courier(CourierClaims),
    Customer(CustomerClaims),
}

impl Claims {
    pub fn role(&self) -> Role {
        match self {
            Claims::Owner(_) => Role::Owner,
            Claims::Courier(_) => Role::Courier,
            Claims::Customer(_) => Role::Customer,
        }
    }

    pub fn sub(&self) -> Uuid {
        match self {
            Claims::Owner(c) => c.sub,
            Claims::Courier(c) => c.sub,
            Claims::Customer(c) => c.sub,
        }
    }

    pub fn kid(&self) -> &str {
        match self {
            Claims::Owner(c) => &c.kid,
            Claims::Courier(c) => &c.kid,
            Claims::Customer(c) => &c.kid,
        }
    }

    pub fn exp(&self) -> i64 {
        match self {
            Claims::Owner(c) => c.exp,
            Claims::Courier(c) => c.exp,
            Claims::Customer(c) => c.exp,
        }
    }

    /// Stamps `iat`/`exp`/`kid` in place — called ONLY from `jwt::mint` right before signing, so
    /// body-`kid` and header-`kid` can never diverge (REV-2/C1: `kid` is a required body claim,
    /// written to the body AND used for header key-select; both come from this one call).
    pub(crate) fn finalize(&mut self, iat: i64, exp: i64, kid: String) {
        match self {
            Claims::Owner(c) => {
                c.iat = iat;
                c.exp = exp;
                c.kid = kid;
            }
            Claims::Courier(c) => {
                c.iat = iat;
                c.exp = exp;
                c.kid = kid;
            }
            Claims::Customer(c) => {
                c.iat = iat;
                c.exp = exp;
                c.kid = kid;
            }
        }
    }

    pub fn as_owner(&self) -> Option<&OwnerClaims> {
        match self {
            Claims::Owner(c) => Some(c),
            _ => None,
        }
    }

    pub fn as_courier(&self) -> Option<&CourierClaims> {
        match self {
            Claims::Courier(c) => Some(c),
            _ => None,
        }
    }

    pub fn as_customer(&self) -> Option<&CustomerClaims> {
        match self {
            Claims::Customer(c) => Some(c),
            _ => None,
        }
    }
}

impl From<OwnerClaims> for Claims {
    fn from(c: OwnerClaims) -> Self {
        Claims::Owner(c)
    }
}
impl From<CourierClaims> for Claims {
    fn from(c: CourierClaims) -> Self {
        Claims::Courier(c)
    }
}
impl From<CustomerClaims> for Claims {
    fn from(c: CustomerClaims) -> Self {
        Claims::Customer(c)
    }
}

impl Serialize for Claims {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Claims::Owner(c) => c.serialize(serializer),
            Claims::Courier(c) => c.serialize(serializer),
            Claims::Customer(c) => c.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Claims {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Peek `role` via a buffered Value, then re-parse the WHOLE value (role included) into
        // the matching variant struct — each of which is `deny_unknown_fields`, so an unknown or
        // missing claim on that variant still fails closed exactly as `AuthToken.parse` does.
        let value = serde_json::Value::deserialize(deserializer)?;
        let role = value
            .get("role")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| DeError::missing_field("role"))?
            .to_string();
        match role.as_str() {
            "owner" => serde_json::from_value(value)
                .map(Claims::Owner)
                .map_err(DeError::custom),
            "courier" => serde_json::from_value(value)
                .map(Claims::Courier)
                .map_err(DeError::custom),
            "customer" => serde_json::from_value(value)
                .map(Claims::Customer)
                .map_err(DeError::custom),
            other => Err(DeError::custom(format!(
                "unknown or unrepresentable role: {other}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owner() -> OwnerClaims {
        let mut c = OwnerClaims::new(Uuid::new_v4(), None);
        c.iat = 1;
        c.exp = 2;
        c.kid = "kid-1".to_string();
        c
    }

    #[test]
    fn owner_claims_round_trip_uses_camelcase_userid() {
        let c = owner();
        let json = serde_json::to_value(Claims::Owner(c.clone())).unwrap();
        assert_eq!(json["role"], "owner");
        assert!(json.get("userId").is_some());
        assert!(json.get("user_id").is_none());
        assert!(
            json.get("activeLocationId").is_none(),
            "absent when None, not null"
        );

        let decoded: Claims = serde_json::from_value(json).unwrap();
        assert_eq!(decoded, Claims::Owner(c));
    }

    #[test]
    fn owner_claims_reject_unknown_field() {
        let json = serde_json::json!({
            "role": "owner", "userId": Uuid::new_v4(), "sub": Uuid::new_v4(),
            "iat": 1, "exp": 2, "kid": "k", "extra": "nope",
        });
        let err = serde_json::from_value::<Claims>(json).unwrap_err();
        assert!(err.to_string().contains("extra") || err.to_string().contains("unknown"));
    }

    #[test]
    fn owner_claims_reject_missing_required_field() {
        // Missing `userId` — required on the owner variant.
        let json = serde_json::json!({
            "role": "owner", "sub": Uuid::new_v4(), "iat": 1, "exp": 2, "kid": "k",
        });
        assert!(serde_json::from_value::<Claims>(json).is_err());
    }

    #[test]
    fn courier_claims_require_active_location_id() {
        let json = serde_json::json!({
            "role": "courier", "sub": Uuid::new_v4(), "iat": 1, "exp": 2, "kid": "k",
        });
        assert!(
            serde_json::from_value::<Claims>(json).is_err(),
            "activeLocationId is REQUIRED for couriers (not optional like owner)"
        );
    }

    #[test]
    fn courier_claims_jti_is_optional() {
        let json = serde_json::json!({
            "role": "courier", "activeLocationId": Uuid::new_v4(), "sub": Uuid::new_v4(),
            "iat": 1, "exp": 2, "kid": "k",
        });
        let decoded: Claims = serde_json::from_value(json).unwrap();
        assert_eq!(decoded.as_courier().unwrap().jti, None);
    }

    #[test]
    fn customer_claims_have_no_phone_field_even_if_smuggled_in() {
        // P0-PII (T-8): a phone claim on the wire must be REJECTED (deny_unknown_fields), not
        // silently dropped — proving the claim shape structurally cannot carry PII.
        let json = serde_json::json!({
            "role": "customer", "orderId": Uuid::new_v4(), "locationId": Uuid::new_v4(),
            "sub": Uuid::new_v4(), "iat": 1, "exp": 2, "kid": "k", "phone": "+355000000",
        });
        let err = serde_json::from_value::<Claims>(json).unwrap_err();
        assert!(
            err.to_string().to_lowercase().contains("phone") || err.to_string().contains("unknown")
        );
    }

    #[test]
    fn customer_claims_constructed_in_rust_never_serialize_a_phone_key() {
        let c = CustomerClaims::new(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let json = serde_json::to_value(Claims::Customer(c)).unwrap();
        assert!(json.get("phone").is_none());
        assert_eq!(
            json.as_object().unwrap().len(),
            7,
            "exactly role+orderId+locationId+sub+iat+exp+kid (7) — no phone/PII field"
        );
    }

    #[test]
    fn unknown_role_is_rejected() {
        let json = serde_json::json!({ "role": "superadmin", "sub": Uuid::new_v4() });
        assert!(serde_json::from_value::<Claims>(json).is_err());
    }

    #[test]
    fn finalize_stamps_kid_iat_exp_uniformly_across_variants() {
        let mut c: Claims = OwnerClaims::new(Uuid::new_v4(), None).into();
        c.finalize(10, 20, "prod-kid".to_string());
        assert_eq!(c.kid(), "prod-kid");
        assert_eq!(c.as_owner().unwrap().iat, 10);
        assert_eq!(c.as_owner().unwrap().exp, 20);
    }
}
