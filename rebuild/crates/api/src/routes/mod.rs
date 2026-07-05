pub mod fallback_config;
pub mod health;
pub mod manifest;
pub mod media_proxy;
pub mod menu;
pub mod rates;
pub mod seo;
pub mod storefront;
pub mod theme;
pub mod vapid;
pub mod voice_config;

// ── S2 auth surface (openapi-s2-auth.yaml) ──
pub mod auth_claim;
pub mod auth_courier;
pub mod auth_customer;
#[cfg(feature = "dev-routes")]
pub mod auth_dev;
pub mod auth_owner;

// ── S3 catalog/admin CRUD surface (owner-route census rows 1-89) ──
pub mod owner;

// ── S5 orders/money surface (docs/design/rebuild-orders-s5-council/) — the crown-jewel red-line ──
pub mod orders;

// ── S4 media surface (docs/design/rebuild-media-s4-council/) — the UNAUTHENTICATED half.
// The owner-authenticated half (product-media, product-image, theme logo) lives inside
// `routes::owner` (product_media.rs, product_image.rs, themes.rs) — see that module's doc.
pub mod media_public;

// ── S7 courier/dispatch surface (docs/design/rebuild-courier-s7-council/) — the courier
// operational plane (shifts/assignments/me/settlements). The owner-side courier-management half
// (roster, invites) lives inside `routes::owner` (couriers.rs, courier_invites.rs) — see
// `routes::courier`'s module doc.
pub mod courier;

use tower_http::request_id::RequestId;

/// Shared by every S1 handler that builds an `ApiError`: the server-authoritative correlation id
/// (ADR-0010 — an inbound header is NEVER trusted; `SetRequestIdLayer`/`MakeRequestUuid` in
/// `main.rs` mint one per request). Falls back to `"unknown"` only if the header value is
/// somehow not UTF-8 (defensive; `MakeRequestUuid` always produces valid UTF-8).
pub(crate) fn correlation_id_string(request_id: &RequestId) -> String {
    request_id
        .header_value()
        .to_str()
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn correlation_id_falls_back_when_header_is_not_utf8() {
        let request_id = RequestId::new(HeaderValue::from_bytes(&[0xff, 0xfe]).unwrap());
        assert_eq!(correlation_id_string(&request_id), "unknown");
    }

    #[test]
    fn correlation_id_passes_through_valid_utf8() {
        let request_id = RequestId::new(HeaderValue::from_static("abc-123"));
        assert_eq!(correlation_id_string(&request_id), "abc-123");
    }
}
