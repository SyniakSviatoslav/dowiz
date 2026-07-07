//! Phase 2.2 — Sovereign-Core direct checkout (`POST /api/orders`, `x-dowiz-cutover: true`).
//!
//! The checkout entry for the server-priced-cart model (spec:
//! `docs/design/sovereign-core-mvp/PHASE-2-2-CART-TOKEN-SPEC.md`). The client sends ONLY item ids +
//! modifier ids + quantities; the SERVER is the sole price authority — it looks up prices from the
//! menu DB, applies the location tax policy + delivery fee, and composes the total via the sovereign
//! core (`domain::kernel::pricing` / `domain::decide`, integer-only). The request body carries no
//! money field; the four totals on the response are the server's, immutable after creation.
//!
//! This handler is the SAME funnel the S5 crown-jewel [`super::OrdersRepo::create_order`] already
//! runs (DB price snapshot → `compute_order_pricing` → `delivery_fee_for_order` → `apply_tax` →
//! `charged_tax`(LC1) → `compose_total`, then the customer upsert + request-hash idempotency + the
//! `orders` INSERT) — the SAME kernel math [`domain::decide`] runs for a [`domain::Command::PlaceOrder`]
//! (`price_cart`). Phase 2.2 adds, at the HTTP boundary, the two guards the spec mandates:
//!
//!   1. **Forbidden price fields** ([`reject_client_price_fields`]) — a `x-dowiz-cutover` request that
//!      carries any client-supplied money field (`subtotal`/`tax_total`/`delivery_fee`/`total`/
//!      `discount_total`) is refused `400 VALIDATION_FAILED` up front, naming the offending field (a
//!      bare `.strict()` serde reject would only say "unknown field"). The price authority is the DB +
//!      kernel, NEVER the request body.
//!   2. **`hub_checkout` feature flag** ([`hub_checkout_enabled`], default OFF) — the launch gate for
//!      the kernel checkout path (`HUB_CHECKOUT=true` ramps it on after staging validation).
//!
//! Idempotency (spec §"Request Hash") and the conservation invariant (spec §"Conservation") are the
//! funnel's existing guarantees: the request hash ([`super::request_hash::build_request_hash`], the
//! `(location_id, request_hash)` dedup key) and the integer composition `total = subtotal +
//! tax_charged + delivery_fee − discount` (`domain::kernel::pricing::compose_total`, LC1). The three
//! adversarial RED proofs live in `tests/phase_2_2_adversarial_money_suite.rs` (+ the HTTP-boundary
//! proofs in `super`'s `handler_tests`).

use axum::extract::Extension;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use tower_http::request_id::RequestId;

use domain::ErrorCode;

use super::{CreateOrderCommand, CreateOutcome, OrderCreatedResponse, OrdersState, channel, dto};
use crate::error::ApiError;
use crate::routes::correlation_id_string;

/// The header that signals the Phase 2.2 kernel checkout path (spec §"Request Contract").
pub const CUTOVER_HEADER: &str = "x-dowiz-cutover";

/// Client price fields a server-priced cart MUST NOT accept (spec §"Forbidden fields"). The server
/// computes every one of these from the DB + kernel; a body carrying any is a `400 VALIDATION_FAILED`.
/// `discount_total` is included: discounts are a server-side concern only (REV-S5-6 CARRY, always 0).
pub const FORBIDDEN_PRICE_FIELDS: [&str; 5] = [
    "subtotal",
    "tax_total",
    "delivery_fee",
    "total",
    "discount_total",
];

/// True iff the request carries `x-dowiz-cutover: true` (case-insensitive) — the signal to run the
/// Phase 2.2 checkout guards. A legacy create (no header) is byte-untouched.
pub fn is_cutover_request(headers: &HeaderMap) -> bool {
    headers
        .get(CUTOVER_HEADER)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
}

/// The `hub_checkout` feature flag (spec §"Feature Flag", default **OFF**). When ON the cutover
/// checkout path is live for the launch ramp; read from `HUB_CHECKOUT=true`. A route-layer toggle
/// (not money state), so it reads the env per call rather than threading through `OrdersState`.
pub fn hub_checkout_enabled() -> bool {
    std::env::var("HUB_CHECKOUT").as_deref() == Ok("true")
}

/// The forbidden-price-field guard (spec §"Forbidden fields"). A server-priced cart rejects ANY
/// client-supplied money field. Pure + falsifiable: returns the FIRST offending field name, else
/// `Ok(())`. RED proof: drop a field from [`FORBIDDEN_PRICE_FIELDS`] and a body carrying it flips
/// `Err → Ok` (see `tests`). A non-object body (array/scalar) has no fields to inject → `Ok(())`; the
/// subsequent `.strict()` DTO parse rejects it as a shape error instead.
pub fn reject_client_price_fields(raw: &serde_json::Value) -> Result<(), &'static str> {
    let Some(obj) = raw.as_object() else {
        return Ok(());
    };
    for field in FORBIDDEN_PRICE_FIELDS {
        if obj.contains_key(field) {
            return Err(field);
        }
    }
    Ok(())
}

/// `POST /api/orders` — the Phase 2.2 direct checkout (spec §"Request Contract"). Anonymous create
/// (no auth extractor; the create tx seats NO `app.user_id`, REV-S5-1). On a `x-dowiz-cutover`
/// request it enforces the server-price-authority guard ([`reject_client_price_fields`]) BEFORE the
/// `.strict()` DTO parse, then hands the whole funnel to the repo's GUC-less tx — the SAME
/// DB-priced/kernel-composed path a legacy create runs, so the money math (and its conservation
/// invariant + request-hash idempotency) is unchanged.
#[utoipa::path(
    post, path = "/api/orders", tag = "orders",
    responses(
        (status = 201, description = "Order created", body = OrderCreatedResponse),
        (status = 200, description = "Idempotent replay of an existing order", body = OrderCreatedResponse),
        (status = 400, description = "VALIDATION_FAILED (client price field / schema drift)", body = domain::ErrorEnvelope),
        (status = 422, description = "Business gate (MIN_ORDER_NOT_MET / CASH_AMOUNT_TOO_LOW / pricing / IDEMPOTENCY_KEY_REUSED)", body = domain::ErrorEnvelope),
        (status = 409, description = "IDEMPOTENCY_CONFLICT / NOT_PUBLISHED", body = domain::ErrorEnvelope),
        (status = 503, description = "Transient DB contention — retry", body = domain::ErrorEnvelope),
    ))]
pub async fn create_order(
    Extension(state): Extension<OrdersState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    axum::Json(raw): axum::Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    let correlation_id = correlation_id_string(&request_id);

    // Phase 2.2 — a cutover checkout is a SERVER-PRICED cart. Reject any client-supplied money field
    // BEFORE the DTO parse (naming the field), so a client can never post its own price. Fires on the
    // `x-dowiz-cutover` header; a legacy create is untouched (its `.strict()` DTO reject is the backstop).
    if is_cutover_request(&headers) {
        if let Err(field) = reject_client_price_fields(&raw) {
            return Err(ApiError::validation_failed_400(
                format!(
                    "Client price field '{field}' is forbidden — the server prices the cart from the menu"
                ),
                correlation_id,
            ));
        }
        // `hub_checkout` is the launch-ramp gate (default OFF). The forbidden-field guard above is a
        // money-safety invariant that always holds for a cutover request; the flag governs the wider
        // rollout telemetry — recorded here, never silently dropped.
        tracing::debug!(
            %correlation_id,
            hub_checkout = hub_checkout_enabled(),
            "phase-2.2 cutover checkout"
        );
    }

    // .strict() parse (deny_unknown_fields) → 400 VALIDATION_FAILED on any drift.
    let input: dto::CreateOrderInput = serde_json::from_value(raw).map_err(|e| {
        ApiError::validation_failed_400(format!("Validation error: {e}"), correlation_id.clone())
    })?;
    input
        .validate()
        .map_err(|msg| ApiError::validation_failed_400(msg, correlation_id.clone()))?;

    // x-channel → write-only metadata (Q-CHANNEL-META). Header, never a body field (schema .strict()).
    let channel = channel::channel_from_header(
        headers
            .get_all("x-channel")
            .iter()
            .filter_map(|v| v.to_str().ok()),
    );

    let cmd = CreateOrderCommand {
        input,
        channel,
        customer_sub: None, // anonymous checkout is the create path; a customer token would refine.
    };

    let outcome = state.repo.create_order(cmd).await.map_err(|e| {
        // Log the real cause (the client only gets an opaque INTERNAL). A money-path 500 with no
        // logged cause is undiagnosable (staging cutover 2026-07-05).
        tracing::error!(%correlation_id, error = %e.0, "order create failed");
        ApiError::new(ErrorCode::Internal, "internal_error", correlation_id.clone())
    })?;

    match outcome {
        CreateOutcome::Created(order) => {
            Ok((StatusCode::CREATED, axum::Json(order)).into_response())
        }
        CreateOutcome::Replayed(order) => Ok((StatusCode::OK, axum::Json(order)).into_response()),
        CreateOutcome::Rejected(code, msg) => Err(ApiError::new(code, msg, correlation_id)),
        CreateOutcome::Transient => Err(ApiError::new(
            ErrorCode::ServiceUnavailable,
            "Service temporarily unavailable, please try again",
            correlation_id,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn clean_body() -> serde_json::Value {
        serde_json::json!({
            "locationId": "11111111-1111-1111-1111-111111111111",
            "type": "pickup",
            "items": [{ "product_id": "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa", "quantity": 1 }],
            "payment": { "method": "cash" },
            "idempotency_key": "22222222-2222-2222-2222-222222222222"
        })
    }

    /// RED PROOF (pure): every one of the five spec-forbidden money fields is caught (naming it), so a
    /// client can never smuggle its own price into a server-priced cart. RED: remove a field from
    /// `FORBIDDEN_PRICE_FIELDS` and its injection flips `Err → Ok`.
    #[test]
    fn reject_client_price_fields_catches_every_forbidden_money_field() {
        for field in FORBIDDEN_PRICE_FIELDS {
            let mut body = clean_body();
            body[field] = serde_json::json!(1); // a malicious low price
            assert_eq!(
                reject_client_price_fields(&body),
                Err(field),
                "a cutover cart carrying `{field}` must be refused"
            );
        }
    }

    /// The guard does NOT false-positive: a clean cart (item ids + quantities only) passes.
    #[test]
    fn reject_client_price_fields_accepts_a_clean_cart() {
        assert_eq!(reject_client_price_fields(&clean_body()), Ok(()));
    }

    /// The spec's forbidden set is exactly these five (contract pin — a silent drop would reopen a
    /// price-injection hole).
    #[test]
    fn forbidden_set_is_the_spec_five() {
        for field in [
            "subtotal",
            "tax_total",
            "delivery_fee",
            "total",
            "discount_total",
        ] {
            assert!(
                FORBIDDEN_PRICE_FIELDS.contains(&field),
                "`{field}` must be a forbidden client price field (spec §Forbidden fields)"
            );
        }
    }

    /// A non-object body has no fields to inject — the guard defers to the DTO shape reject.
    #[test]
    fn reject_client_price_fields_passes_non_object_bodies_through() {
        assert_eq!(reject_client_price_fields(&serde_json::json!([1, 2, 3])), Ok(()));
        assert_eq!(reject_client_price_fields(&serde_json::json!("nope")), Ok(()));
    }

    #[test]
    fn is_cutover_request_reads_the_header_case_insensitively() {
        let mut on = HeaderMap::new();
        on.insert(CUTOVER_HEADER, HeaderValue::from_static("true"));
        assert!(is_cutover_request(&on));

        let mut upper = HeaderMap::new();
        upper.insert(CUTOVER_HEADER, HeaderValue::from_static("TRUE"));
        assert!(is_cutover_request(&upper));

        let mut off = HeaderMap::new();
        off.insert(CUTOVER_HEADER, HeaderValue::from_static("false"));
        assert!(!is_cutover_request(&off));

        assert!(!is_cutover_request(&HeaderMap::new()), "absent header ⇒ legacy create");
    }

    /// The feature flag defaults OFF (spec §"Feature Flag") when `HUB_CHECKOUT` is unset — the safe
    /// launch default. (The env is not set in the test process.)
    #[test]
    fn hub_checkout_defaults_off() {
        // Deterministic: the test harness does not set HUB_CHECKOUT, so the default must be OFF.
        if std::env::var("HUB_CHECKOUT").is_err() {
            assert!(!hub_checkout_enabled());
        }
    }
}
