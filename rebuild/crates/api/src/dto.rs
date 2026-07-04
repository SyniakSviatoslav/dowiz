//! Wire DTOs for the S1 storefront-read surface — mirrors
//! `docs/design/rebuild-plan/openapi-contracts/openapi-s1-storefront-read.yaml`
//! `components.schemas.*` byte-for-byte (field names, nullability, mixed casing quirks).
//!
//! These are deliberately NOT `domain` types: `domain::Lek` is non-negative-only (money
//! authority for the order-create red-line, see `rebuild/README.md` "Open questions" item 4),
//! but `PublicModifier.price_delta` is a *signed* display value (a modifier can be a discount) —
//! representing it as `Lek` would either reject legitimate negative deltas or require a new
//! signed money type the council explicitly deferred. These DTOs carry plain `i64` minor-unit
//! integers: read-only display data, never a computation/authority surface (the order-create
//! transaction remains the sole amount authority, unchanged red-line).
//!
//! Field-name rule (CONVENTIONS.md "Naming"): wire names are verbatim, including the mixed
//! snake_case/camelCase the live API actually emits — reproduced here with per-field
//! `#[serde(rename = ...)]`, never a blanket `rename_all` that would re-case the quirky fields.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ── PublicMenu (getPublicMenu) ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublicMenuCurrency {
    pub code: String,
    pub minor_unit: i32,
}

/// `read_public_menu` jsonb (migration 1790000000072) + route enrichment (menu.ts:138-194).
/// x-quirk: `locationId` is a legacy camelCase ALIAS of `location_id` (menu.ts:146) — both
/// present, always mirroring each other.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublicMenu {
    pub menu_version: i64,
    pub location_id: Option<Uuid>,
    #[serde(rename = "locationId")]
    pub location_id_alias: Option<Uuid>,
    pub location_name: String,
    pub default_locale: String,
    pub supported_locales: Vec<String>,
    pub currency: PublicMenuCurrency,
    /// Present+true ONLY for shadow-tenant previews (menu.ts:43) — the SOLE signal flipping the
    /// storefront into NON-ORDERABLE preview mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_preview: Option<bool>,
    pub categories: Vec<PublicMenuCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublicMenuCategory {
    pub id: Uuid,
    /// Locale-resolved (translation -> default-locale translation -> base).
    pub name: String,
    pub sort_order: i32,
    pub products: Vec<PublicMenuProduct>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublicMenuProduct {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    /// Integer minor units (ADR-0005). Interpret with `menu.currency.minor_unit`. Deliberately
    /// NOT `domain::Lek` — see module doc.
    pub price: i64,
    /// x-quirk: always true on live payloads (unavailable products filtered out server-side,
    /// migration 072:130).
    pub available: bool,
    pub image_key: Option<String>,
    pub primary_media_id: Option<Uuid>,
    #[serde(rename = "imageUrl", skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    /// Free-form JSONB characteristics layer (ADR-0014) — kept as raw JSON since the known-keys
    /// set (kcal/protein/allergens/bom/...) is FE-consumed, not server-authoritative here.
    pub attributes: Option<serde_json::Value>,
    pub prep_time_minutes: Option<i32>,
    pub modifier_groups: Vec<PublicModifierGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublicModifierGroup {
    pub id: Uuid,
    pub name: String,
    pub min_select: i32,
    pub max_select: i32,
    pub required: bool,
    /// Null -> client infers radio (max_select==1) vs checkbox.
    pub display_type: Option<String>,
    pub sort_order: i32,
    pub modifiers: Vec<PublicModifier>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublicModifier {
    pub id: Uuid,
    pub name: String,
    /// Signed integer minor units (a modifier CAN be a discount) — not `domain::Lek`, see
    /// module doc.
    pub price_delta: i64,
    /// Always true on the wire (`WHERE m.available=true`, migration 072:83).
    pub available: bool,
    pub sort_order: i32,
}

// ── PublicLocationInfo (getPublicLocationInfo) ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum VenueStatus {
    Open,
    Closed,
    Busy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WeeklyHoursEntry {
    pub day: Weekday,
    #[serde(rename = "isOpen")]
    pub is_open: bool,
    pub open: Option<String>,
    pub close: Option<String>,
}

/// `menu.ts:383-409` verbatim. x-quirk: mixed snake_case (id/name/slug/currency_*/default_locale)
/// and camelCase (everything else) in ONE payload — preserved via per-field renames.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublicLocationInfo {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub currency_code: String,
    pub currency_minor_unit: i32,
    pub default_locale: String,
    #[serde(rename = "deliveryFeeFlat")]
    pub delivery_fee_flat: Option<i64>,
    #[serde(rename = "freeDeliveryThreshold")]
    pub free_delivery_threshold: Option<i64>,
    #[serde(rename = "minOrderValue")]
    pub min_order_value: Option<i64>,
    /// Config fraction (e.g. 0.2) — NOT money. Defaults 0 when null (menu.ts:392).
    #[serde(rename = "taxRate")]
    pub tax_rate: f64,
    #[serde(rename = "priceIncludesTax")]
    pub price_includes_tax: bool,
    #[serde(rename = "hasDistanceTiers")]
    pub has_distance_tiers: bool,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub address: Option<String>,
    pub phone: Option<String>,
    #[serde(rename = "isOpen")]
    pub is_open: bool,
    pub status: VenueStatus,
    #[serde(rename = "closesAt")]
    pub closes_at: Option<String>,
    #[serde(rename = "weeklyHours")]
    pub weekly_hours: Option<Vec<WeeklyHoursEntry>>,
    #[serde(rename = "googleRating")]
    pub google_rating: Option<f64>,
    #[serde(rename = "googleReviewCount")]
    pub google_review_count: Option<i32>,
    #[serde(rename = "googleMapsUrl")]
    pub google_maps_url: Option<String>,
    #[serde(rename = "googlePlaceId")]
    pub google_place_id: Option<String>,
    #[serde(rename = "socialInstagram")]
    pub social_instagram: Option<String>,
    #[serde(rename = "socialFacebook")]
    pub social_facebook: Option<String>,
}

// ── PublicTheme (getPublicTheme) ────────────────────────────────────────────────────────────

/// `spa-proxy.ts:513-527` verbatim.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublicTheme {
    #[serde(rename = "primaryColor")]
    pub primary_color: Option<String>,
    #[serde(rename = "bgColor")]
    pub bg_color: Option<String>,
    #[serde(rename = "textColor")]
    pub text_color: Option<String>,
    #[serde(rename = "logoUrl")]
    pub logo_url: Option<String>,
    #[serde(rename = "locationName")]
    pub location_name: String,
    #[serde(rename = "headingFont")]
    pub heading_font: Option<String>,
    #[serde(rename = "bodyFont")]
    pub body_font: Option<String>,
    /// x-quirk (spa-proxy.ts:524): null (not `[]`) when the DB column is not an array.
    #[serde(rename = "supportedLocales")]
    pub supported_locales: Option<Vec<String>>,
}

// ── ProductMedia (getProductMedia) ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ProductMediaKind {
    Image,
    Video,
    Spin,
    Model,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProductMediaMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_count: Option<i32>,
    #[serde(rename = "frameUrls", skip_serializing_if = "Option::is_none")]
    pub frame_urls: Option<Vec<String>>,
}

/// Resolved media view (menu.ts:446-467).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProductMedia {
    pub id: Uuid,
    pub kind: ProductMediaKind,
    /// `''` when `storage_key` unresolvable (menu.ts:457 fallback).
    pub url: String,
    #[serde(rename = "posterUrl")]
    pub poster_url: Option<String>,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    #[serde(rename = "durationMs")]
    pub duration_ms: Option<i32>,
    pub alt: Option<String>,
    #[serde(rename = "sortOrder")]
    pub sort_order: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ProductMediaMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProductMediaResponse {
    pub media: Vec<ProductMedia>,
}

// ── Reserved shared components (referenced by no S1 route; S5 imports them) ────────────────

/// RESERVED — `apps/api/src/lib/preflight.ts:71-138`. Business-outcome namespace, lowercase,
/// distinct from `ErrorEnvelope.code` (never merge). No S1 operation references this; kept here
/// so the generated OpenAPI document's `components.schemas` matches the authored YAML
/// (openapi-diff gate) and so S5 (order-create) imports rather than re-invents it.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PreflightReason {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// RESERVED — order-attribution channel (hub taxonomy, REBUILD-MAP §6). Referenced by no S1/S2
/// operation; S5 adds it as an OPTIONAL order-create field (absent = web-direct). `x-open-enum`
/// (additive-only) is not representable as a Rust closed enum without a client-tolerant unknown
/// variant, which no S1 caller needs yet — kept as an opaque `String` here rather than inventing
/// unused enum-exhaustiveness machinery for a component no route touches. Genuinely unused until
/// S5 (no S1 handler references it, and a plain `type` alias can't itself carry a `ToSchema`
/// impl the way `PreflightReason`'s struct does, so it can't even be listed in
/// `openapi.rs`'s `components(schemas(...))` the way that reserved component is) — allowed
/// rather than deleted so S5 has a documented landing spot instead of re-deriving this decision.
#[allow(dead_code, reason = "S5-reserved placeholder, see doc comment above")]
pub type Channel = String;

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip against a literal transcription of the YAML's PublicMenu example shape,
    /// pinning the mixed-casing quirk (`location_id` + `locationId` alias) and the money
    /// integer-minor-units field.
    #[test]
    fn public_menu_round_trips_with_camel_snake_mix() {
        let menu = PublicMenu {
            menu_version: 7,
            location_id: Some(Uuid::nil()),
            location_id_alias: Some(Uuid::nil()),
            location_name: "Eljo's Pizza".to_string(),
            default_locale: "sq".to_string(),
            supported_locales: vec!["sq".to_string(), "en".to_string()],
            currency: PublicMenuCurrency {
                code: "ALL".to_string(),
                minor_unit: 0,
            },
            is_preview: None,
            categories: vec![PublicMenuCategory {
                id: Uuid::nil(),
                name: "Pizza".to_string(),
                sort_order: 0,
                products: vec![PublicMenuProduct {
                    id: Uuid::nil(),
                    name: "Margherita".to_string(),
                    description: None,
                    price: 1200,
                    available: true,
                    image_key: None,
                    primary_media_id: None,
                    image_url: None,
                    attributes: None,
                    prep_time_minutes: Some(15),
                    modifier_groups: vec![],
                }],
            }],
        };

        let json = serde_json::to_value(&menu).unwrap();
        assert_eq!(json["location_id"], json["locationId"], "alias must mirror");
        assert_eq!(json["categories"][0]["products"][0]["price"], 1200);
        assert!(
            json.get("is_preview").is_none(),
            "is_preview must be absent (not false) when not a shadow preview"
        );

        let decoded: PublicMenu = serde_json::from_value(json).unwrap();
        assert_eq!(decoded.location_name, "Eljo's Pizza");
    }

    #[test]
    fn public_menu_is_preview_true_serializes_for_shadow() {
        let mut menu_json = serde_json::json!({
            "menu_version": 0,
            "location_id": null,
            "locationId": null,
            "location_name": "Shadow Cafe",
            "default_locale": "sq",
            "supported_locales": ["sq"],
            "currency": {"code": "ALL", "minor_unit": 0},
            "is_preview": true,
            "categories": [],
        });
        let menu: PublicMenu = serde_json::from_value(menu_json.take()).unwrap();
        assert_eq!(menu.is_preview, Some(true));
        assert!(menu.location_id.is_none());
    }

    /// A modifier price_delta CAN be negative (a discount) — proves this DTO does not reject it
    /// the way `domain::Lek` would (see module doc for why this is intentional, not an oversight).
    #[test]
    fn modifier_price_delta_accepts_negative_values() {
        let modifier = PublicModifier {
            id: Uuid::nil(),
            name: "No cheese (-50)".to_string(),
            price_delta: -50,
            available: true,
            sort_order: 0,
        };
        let json = serde_json::to_value(&modifier).unwrap();
        assert_eq!(json["price_delta"], -50);
    }

    #[test]
    fn public_location_info_uses_mixed_casing_verbatim() {
        let info = PublicLocationInfo {
            id: Uuid::nil(),
            name: "Eljo's Pizza".to_string(),
            slug: "eljos-pizza".to_string(),
            currency_code: "ALL".to_string(),
            currency_minor_unit: 0,
            default_locale: "sq".to_string(),
            delivery_fee_flat: Some(150),
            free_delivery_threshold: Some(2000),
            min_order_value: None,
            tax_rate: 0.2,
            price_includes_tax: true,
            has_distance_tiers: false,
            lat: Some(41.33),
            lng: Some(19.82),
            address: Some("Rruga e Kavajes".to_string()),
            phone: None,
            is_open: true,
            status: VenueStatus::Open,
            closes_at: Some("22:00".to_string()),
            weekly_hours: None,
            google_rating: None,
            google_review_count: None,
            google_maps_url: None,
            google_place_id: None,
            social_instagram: None,
            social_facebook: None,
        };
        let json = serde_json::to_value(&info).unwrap();
        // snake_case fields verbatim
        assert!(json.get("currency_code").is_some());
        assert!(json.get("default_locale").is_some());
        // camelCase fields verbatim, in the SAME payload
        assert!(json.get("deliveryFeeFlat").is_some());
        assert!(json.get("isOpen").is_some());
        assert!(json.get("hasDistanceTiers").is_some());
        assert_eq!(json["status"], "open");
    }

    #[test]
    fn public_theme_supported_locales_absent_serializes_null_not_empty_array() {
        let theme = PublicTheme {
            primary_color: None,
            bg_color: None,
            text_color: None,
            logo_url: None,
            location_name: "Eljo's Pizza".to_string(),
            heading_font: None,
            body_font: None,
            supported_locales: None,
        };
        let json = serde_json::to_value(&theme).unwrap();
        assert!(json["supportedLocales"].is_null(), "x-quirk: null, not []");
    }

    #[test]
    fn product_media_response_empty_media_serializes_as_empty_array() {
        let resp = ProductMediaResponse { media: vec![] };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["media"], serde_json::json!([]));
    }
}
