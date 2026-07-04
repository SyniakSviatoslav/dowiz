//! OpenAPI 3.1 SSOT (REBUILD-MAP §1: "OpenAPI 3.1 as SSOT ... replaces shared-types/Zod as the
//! cross-boundary type authority"). Every route gains a `#[utoipa::path(...)]` annotation and a
//! `paths(...)` entry here — `openapi-diff` (REBUILD-MAP §Decision register, CI/CD row) is the
//! gate that keeps this from silently drifting from
//! `docs/design/rebuild-plan/openapi-contracts/openapi-s1-storefront-read.yaml` (the authored
//! contract, authority) once there's a generated FE client to diff against.

use utoipa::OpenApi;

use crate::dto::{
    PreflightReason, ProductMedia, ProductMediaMeta, ProductMediaResponse, PublicLocationInfo,
    PublicMenu, PublicMenuCategory, PublicMenuCurrency, PublicMenuProduct, PublicModifier,
    PublicModifierGroup, PublicTheme, VenueStatus, Weekday, WeeklyHoursEntry,
};
use crate::routes::fallback_config::FallbackConfigResponse;
use crate::routes::health::HealthStatus;
use crate::routes::manifest::{ManifestIcon, WebManifest};
use crate::routes::rates::ExchangeRateResponse;
use crate::routes::vapid::VapidPublicKeyResponse;
use crate::routes::voice_config::VoiceConfigResponse;

// ── S2 auth schemas ──
use crate::auth::claims::{CourierClaims, CustomerClaims, OwnerClaims, Role};
use crate::auth::dto::{
    ClaimAcceptResponse, ClaimDeclineResponse, ClaimRequestBody, ClaimRequestResponse,
    ClaimTokenRequest, CourierBrief, CourierInviteResponse, CourierLocationBrief,
    CourierLoginRequest, CourierLoginResponse, CourierLogoutRequest, CourierLogoutResponse,
    CourierRedeemRequest, CourierRedeemResponse, ExchangeRequest, OwnerLoginRequest,
    OwnerLoginResponse, OwnerRefreshRequest, TelegramAuthenticated, TelegramStartResponse,
    TokenPairResponse, TrackExchangeRequest, TrackExchangeResponse,
};
use crate::auth::error::{
    ClaimBareError, ConcurrentRefreshBody, CourierManualZod400, DevGate404, GlobalBearerGate401,
    SyntheticCourierMissing, TrackLinkExpiredBody,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::health::healthz,
        crate::routes::health::livez,
        // S1 storefront-read — openapi-s1-storefront-read.yaml (20 operations).
        crate::routes::menu::get_public_menu,
        crate::routes::menu::get_public_location_info,
        crate::routes::menu::get_product_media,
        crate::routes::theme::get_public_theme,
        crate::routes::theme::get_theme_css,
        crate::routes::storefront::get_storefront_page,
        crate::routes::storefront::get_storefront_cart_page,
        crate::routes::storefront::get_storefront_checkout_page,
        crate::routes::storefront::get_storefront_order_page,
        crate::routes::storefront::get_storefront_order_page_legacy,
        crate::routes::manifest::get_web_manifest,
        crate::routes::fallback_config::get_fallback_config,
        crate::routes::media_proxy::get_image,
        crate::routes::media_proxy::get_media_object,
        crate::routes::voice_config::get_voice_config,
        crate::routes::vapid::get_vapid_public_key,
        crate::routes::rates::get_exchange_rate,
        crate::routes::seo::get_robots_txt,
        crate::routes::seo::get_sitemap_index,
        crate::routes::seo::get_sitemap_shard,
        // ── S2 auth — openapi-s2-auth.yaml (20 operations; 19 built + 1 RETIRED). ──
        // AUTH-GAP-2 courierActivateDead is RETIRED (unregistered → 404), so it is deliberately
        // ABSENT from this list (proof-of-deadness; council Q2 UNANIMOUS RETIRE).
        crate::routes::auth_owner::owner_local_login,
        crate::routes::auth_owner::google_oauth_start,
        crate::routes::auth_owner::google_oauth_callback,
        crate::routes::auth_owner::exchange_oauth_code,
        crate::routes::auth_owner::telegram_login_start,
        crate::routes::auth_owner::telegram_login_poll,
        crate::routes::auth_owner::owner_refresh,
        crate::routes::auth_owner::owner_logout,
        crate::routes::auth_courier::get_courier_invite,
        crate::routes::auth_courier::courier_redeem_invite,
        crate::routes::auth_courier::courier_login,
        crate::routes::auth_courier::courier_refresh,
        crate::routes::auth_courier::courier_logout,
        crate::routes::auth_claim::claim_accept,
        crate::routes::auth_claim::claim_request,
        crate::routes::auth_claim::claim_decline,
        crate::routes::auth_customer::customer_track_exchange,
    ),
    components(schemas(
        HealthStatus,
        domain::ErrorEnvelope,
        PublicMenu,
        PublicMenuCategory,
        PublicMenuProduct,
        PublicMenuCurrency,
        PublicModifierGroup,
        PublicModifier,
        PublicLocationInfo,
        VenueStatus,
        Weekday,
        WeeklyHoursEntry,
        PublicTheme,
        ProductMedia,
        ProductMediaMeta,
        ProductMediaResponse,
        FallbackConfigResponse,
        WebManifest,
        ManifestIcon,
        VoiceConfigResponse,
        VapidPublicKeyResponse,
        ExchangeRateResponse,
        // Reserved shared components (referenced by no S1 route; S5 imports them) — kept in the
        // generated document so `components.schemas` matches the authored YAML (openapi-diff).
        PreflightReason,
        // ── S2 auth schemas (openapi-s2-auth.yaml components) ──
        OwnerClaims,
        CourierClaims,
        CustomerClaims,
        Role,
        OwnerLoginRequest,
        OwnerLoginResponse,
        ExchangeRequest,
        TokenPairResponse,
        TelegramStartResponse,
        TelegramAuthenticated,
        OwnerRefreshRequest,
        CourierInviteResponse,
        CourierRedeemRequest,
        CourierRedeemResponse,
        CourierBrief,
        CourierLocationBrief,
        CourierLoginRequest,
        CourierLoginResponse,
        CourierLogoutRequest,
        CourierLogoutResponse,
        ClaimTokenRequest,
        ClaimRequestBody,
        ClaimAcceptResponse,
        ClaimRequestResponse,
        ClaimDeclineResponse,
        TrackExchangeRequest,
        TrackExchangeResponse,
        // The 4 divergent non-envelope shapes (Q4 carry) + the pre-route/dev gates.
        ClaimBareError,
        CourierManualZod400,
        ConcurrentRefreshBody,
        TrackLinkExpiredBody,
        GlobalBearerGate401,
        DevGate404,
        SyntheticCourierMissing,
    )),
    tags(
        (name = "health", description = "Liveness/health probes"),
        (name = "menu", description = "Public storefront menu"),
        (name = "theme", description = "Tenant branding (JSON + CSS)"),
        (name = "storefront", description = "SSR/SPA-shell HTML entry points"),
        (name = "pwa", description = "PWA manifest"),
        (name = "fallback", description = "Offline/error phone-fallback config"),
        (name = "media", description = "Product image/media proxies"),
        (name = "voice", description = "Voice-control runtime kill-switch"),
        (name = "push", description = "Web-push VAPID key"),
        (name = "rates", description = "ALL->EUR display exchange rate"),
        (name = "seo", description = "robots.txt / sitemap"),
        (name = "auth", description = "S2 owner/courier/claim/customer auth flows"),
        (name = "dev", description = "S2 dev-auth mint (dev-routes builds only)"),
    )
)]
pub struct ApiDoc;

pub async fn openapi_json() -> axum::Json<utoipa::openapi::OpenApi> {
    axum::Json(ApiDoc::openapi())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_document_lists_health_routes() {
        let doc = ApiDoc::openapi();
        let paths: Vec<&String> = doc.paths.paths.keys().collect();
        assert!(paths.iter().any(|p| p.as_str() == "/healthz"));
        assert!(paths.iter().any(|p| p.as_str() == "/livez"));
    }

    /// Every path this crate carries opinions about SHOULD equal the authored YAML's path list —
    /// this test pins the 20 S1 paths this build actually annotated, so a future edit that
    /// silently drops a `#[utoipa::path]` from the `paths(...)` list fails loudly here rather
    /// than only in a CI `openapi-diff` run this sandbox cannot execute (no network/registry —
    /// see `rebuild/README.md` "Validation status"). The literal path strings are transcribed
    /// from `openapi-s1-storefront-read.yaml` verbatim.
    #[test]
    fn openapi_document_lists_all_20_s1_operations() {
        let doc = ApiDoc::openapi();
        let paths: std::collections::HashSet<&str> =
            doc.paths.paths.keys().map(String::as_str).collect();
        let expected = [
            "/public/locations/{locationIdOrSlug}/menu",
            "/public/locations/{slug}/info",
            "/public/locations/{slug}/products/{productId}/media",
            "/api/public/theme/{slug}",
            "/public/locations/{locationId}/theme.css",
            "/s/{slug}",
            "/s/{slug}/cart",
            "/s/{slug}/checkout",
            "/s/{slug}/order/{id}",
            "/s/{slug}/orders/{orderId}",
            "/s/{slug}/manifest.webmanifest",
            "/api/public/locations/{slug}/fallback-config",
            "/images/{key}",
            "/media/{key}",
            "/api/public/voice-config",
            "/api/push/vapid-public-key",
            "/v1/rates",
            "/robots.txt",
            "/sitemap.xml",
            "/sitemap-locations-{shard}.xml",
        ];
        for path in expected {
            assert!(
                paths.contains(path),
                "missing S1 path in generated OpenAPI: {path}"
            );
        }
        assert_eq!(
            expected.len(),
            20,
            "the S1 contract has exactly 20 operations"
        );
    }

    #[test]
    fn openapi_document_includes_error_envelope_and_reserved_components() {
        let doc = ApiDoc::openapi();
        let schemas = &doc.components.as_ref().unwrap().schemas;
        assert!(schemas.contains_key("ErrorEnvelope"));
        assert!(schemas.contains_key("PreflightReason"));
        assert!(schemas.contains_key("PublicMenu"));
    }

    /// The S2 auth paths this build annotated — the 17 non-dev live operations (owner 8, courier 5,
    /// claim 3, customer 1). The 2 dev mock-auth ops are `#[cfg(feature="dev-routes")]` (present
    /// only in a dev build); `courierActivateDead` is RETIRED (unregistered → must be ABSENT).
    #[test]
    fn openapi_document_lists_the_s2_auth_operations_and_omits_retired_activate() {
        let doc = ApiDoc::openapi();
        let paths: std::collections::HashSet<&str> =
            doc.paths.paths.keys().map(String::as_str).collect();
        let expected = [
            "/api/auth/local/login",
            "/api/auth/google",
            "/api/auth/google/callback",
            "/api/auth/exchange",
            "/api/auth/telegram/start",
            "/api/auth/telegram/poll",
            "/api/auth/refresh",
            "/api/auth/logout",
            "/api/courier/auth/invites/{inviteId}",
            "/api/courier/auth/invites/{inviteId}/redeem",
            "/api/courier/auth/login",
            "/api/courier/auth/refresh",
            "/api/courier/auth/logout",
            "/api/claim/accept",
            "/api/claim/request",
            "/api/claim/decline",
            "/api/customer/track/exchange",
        ];
        for path in expected {
            assert!(paths.contains(path), "missing S2 auth path: {path}");
        }
        assert_eq!(expected.len(), 17, "17 non-dev live S2 auth operations");
        // RETIRE proof (council Q2): the dead courier-activate flow must NOT appear.
        assert!(
            !paths.contains("/api/auth/courier/activate"),
            "RETIRED courier-activate must be absent from the generated doc"
        );
    }

    /// The S2 claims + divergent-shape schemas are registered (Q4 carry).
    #[test]
    fn openapi_document_includes_s2_claims_and_divergent_shapes() {
        let doc = ApiDoc::openapi();
        let schemas = &doc.components.as_ref().unwrap().schemas;
        for name in [
            "OwnerClaims",
            "CourierClaims",
            "CustomerClaims",
            "ClaimBareError",
            "CourierManualZod400",
            "ConcurrentRefreshBody",
            "TrackLinkExpiredBody",
        ] {
            assert!(schemas.contains_key(name), "missing S2 schema: {name}");
        }
    }
}
