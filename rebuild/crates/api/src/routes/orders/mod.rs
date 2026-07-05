//! S5 orders/money surface — the crown-jewel red-line (order lifecycle + all money composition +
//! channel attribution). Ports `apps/api/src/routes/orders.ts`, `customer/orders.ts`,
//! `lib/orderStatusService.ts`, `lib/orderAuthz.ts`, `lib/order-pricing.ts`, `lib/money.ts`,
//! `lib/order-canonical.ts`, `lib/channel.ts` — per the council RESOLVE
//! `docs/design/rebuild-orders-s5-council/resolution.md` (REV-S5-1..9).
//!
//! Submodules (pure-logic core first — where a port defect is a CHARGE/STATE defect):
//!
//! - [`pricing`] — money composition: `apply_tax` (i64, LC1), fee ladder, `compose_total`
//!   (REV-S5-4, REV-S5-6 discountTotal=0 CARRY).
//! - [`request_hash`] — idempotency canonicalization contract, integer-projection (REV-S5-2).
//! - [`channel`] — `x-channel` → `orders.metadata.channel` normalize-never-throw (Q-CHANNEL-META).
//! - [`state`] — the actor-gate + `updateOrderStatus` fold effects + idempotency decision +
//!   transient-PG classify + CC-1 strand guards (REV-S5-9, REV-S5-5).
//! - [`dto`] — `CreateOrderInput` full schema: 6-value messenger_kind + receiver{} (REV-S5-3).

pub mod channel;
pub mod dto;
pub mod pricing;
pub mod request_hash;
pub mod state;

/// f64 → i64 with half-away-from-zero rounding, saturating at the i64 bounds. The whole S5 surface
/// needs exactly TWO f64→i64 conversions — `applyTax`'s `round(taxRate·1e6)` and the request-hash
/// integer coordinate projection `round(coord·1e5)` — and std offers no `i64: TryFrom<f64>`, so the
/// only `as` casts this surface uses are confined HERE behind a finite/saturation guard (the same
/// posture `owner::themes::version_i32` takes for `clippy::as_conversions`, workspace `-D warnings`).
/// `round` is half-away-from-zero, matching Node `Math.round` for the non-negative magnitudes both
/// call sites feed it (rate micro-units and 5-dp coordinates are far inside f64's exact-integer range).
#[allow(
    clippy::as_conversions,
    reason = "sole f64→i64 site for the S5 surface; finite-checked + saturated — see fn doc"
)]
pub(crate) fn round_f64_to_i64(v: f64) -> i64 {
    let r = v.round();
    if r.is_nan() {
        return 0;
    }
    // `+∞ >= i64::MAX as f64` and `-∞ <= i64::MIN as f64` are both true, so infinities saturate here
    // (a NaN slipped past above would too, but it's already handled) — no non-finite value reaches
    // the final cast.
    if r >= i64::MAX as f64 {
        return i64::MAX;
    }
    if r <= i64::MIN as f64 {
        return i64::MIN;
    }
    r as i64
}

#[cfg(test)]
mod round_tests {
    use super::round_f64_to_i64;

    #[test]
    fn rounds_half_away_from_zero_like_node_math_round() {
        assert_eq!(round_f64_to_i64(74.5), 75);
        assert_eq!(round_f64_to_i64(74.4), 74);
        assert_eq!(round_f64_to_i64(4_132_795.3), 4_132_795);
        assert_eq!(round_f64_to_i64(1_981_902.5), 1_981_903);
        assert_eq!(round_f64_to_i64(0.0), 0);
        assert_eq!(round_f64_to_i64(-0.0), 0);
    }

    #[test]
    fn saturates_and_guards_non_finite() {
        assert_eq!(round_f64_to_i64(f64::INFINITY), i64::MAX);
        assert_eq!(round_f64_to_i64(f64::NEG_INFINITY), i64::MIN);
        assert_eq!(round_f64_to_i64(f64::NAN), 0);
    }
}

// ═══════════════════════════ Handler + repo + router layer ═══════════════════════════
//
// The four S5 ops (create / owner-status / customer-cancel / tri-principal read), wired onto an
// `OrdersRepo` trait (the S3/S4 pattern: outcome enums + a `Fake*Repo` for handler tests + a
// `Pg*Repo` porting the real SQL, exercised by `#[ignore]` live-Postgres probes since this sandbox
// has no DB). Every business DECISION (money, idempotency, actor-gate, folds, CC-1) is a pure
// function in the submodules above — the repo/handler only orchestrate + map outcomes to HTTP.

use axum::extract::{Extension, Path};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tower_http::request_id::RequestId;
use uuid::Uuid;

use domain::{ErrorCode, OrderStatus};

use crate::error::ApiError;
use crate::repo::RepoError;
use crate::routes::correlation_id_string;

// ─────────────────────────────── response / request DTOs ───────────────────────────────

/// The create response — mirrors the Node create return (`orders.ts`): camelCase
/// `{ id, locationId, status, subtotal, total, deliveryInstructions, createdAt }`. Money fields
/// are `Lek` (serialize as bare integers — the frozen newtype; `value_type = i64` for the schema
/// since `domain::Lek`/`OrderStatus` are frozen and carry no `ToSchema` impl).
/// The create/replay response (`orders.ts` create return). Wire names are camelCase to match
/// Node (`createdAt`/`locationId`/`deliveryInstructions`) — a frontend reads those exact keys, so
/// snake_case here silently broke every consumer (staging oracle 2026-07-05). `preflight` (E27)
/// and the conditional customer `authToken` (customer_track_grants) are the documented deferred
/// S5 scope — the ONLY create-response fields still absent vs Node.
#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct OrderCreatedResponse {
    pub id: Uuid,
    #[serde(rename = "locationId")]
    pub location_id: Uuid,
    #[schema(value_type = String)]
    pub status: OrderStatus,
    #[schema(value_type = i64)]
    pub subtotal: domain::Lek,
    #[schema(value_type = i64)]
    pub total: domain::Lek,
    #[serde(rename = "deliveryInstructions")]
    pub delivery_instructions: Option<String>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

/// `StatusUpdateInput` (`orders.ts:870`) — `{ status }` (`.strict()`; the target order status).
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct StatusUpdateInput {
    #[schema(value_type = String)]
    pub status: OrderStatus,
}

/// The owner status-update response (`orders.ts:975`: `{ id, ...outcome }`), where `outcome` is
/// either `{status}` or the honest-dispatch `{status, dispatched, reason}`.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct StatusUpdateResponse {
    pub id: Uuid,
    #[schema(value_type = String)]
    pub status: OrderStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dispatched: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// `POST /api/customer/orders/:orderId/cancel` response (`customer/orders.ts:335`).
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct CustomerCancelResponse {
    pub success: bool,
}

/// The tri-principal `GET /orders/:id` order view (`orders.ts:735-861`) — the fields the read
/// returns. Kept minimal + faithful (the full owner/courier/customer field-set differs by principal;
/// this port returns the shared core the FE order page reads).
#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct OrderView {
    pub id: Uuid,
    #[schema(value_type = String)]
    pub status: OrderStatus,
    #[schema(value_type = i64)]
    pub subtotal: domain::Lek,
    #[schema(value_type = i64)]
    pub total: domain::Lek,
    #[serde(rename = "locationId")]
    pub location_id: Uuid,
}

// ─────────────────────────────── repo command / outcome types ───────────────────────────────

/// Everything the create tx needs, assembled by the handler from the validated DTO + the `x-channel`
/// header + the (optional) customer-token identity. Bundled (not loose args) to dodge
/// `clippy::too_many_arguments`.
#[derive(Debug, Clone)]
pub struct CreateOrderCommand {
    pub input: dto::CreateOrderInput,
    /// Normalized `x-channel` (write-only `orders.metadata.channel` fold; Q-CHANNEL-META).
    pub channel: &'static str,
    /// `request.user.sub` for a customer token, else `None` → `"anonymous"` in the request hash
    /// (the #8 `.sub`-not-`.userId` fix).
    pub customer_sub: Option<Uuid>,
}

/// The create outcome (`orders.ts` §§1-14). Business rejections carry an `ErrorCode` whose
/// `http_status` is the Node call-site status (422/409/404); `Transient` is the 503 retry arm.
#[derive(Debug, Clone, PartialEq)]
pub enum CreateOutcome {
    /// Fresh order committed (201).
    Created(OrderCreatedResponse),
    /// Idempotent replay of a committed order (200).
    Replayed(OrderCreatedResponse),
    /// A pre-write business gate (min-order / cash / pricing / idempotency-reuse|conflict /
    /// not-published / not-deliverable / location-not-found) — `ROLLBACK` + `sendError(code, msg)`.
    Rejected(ErrorCode, String),
    /// Transient PG contention → 503 retryable (`orders.ts:724`).
    Transient,
}

/// The owner status-update outcome (`orders.ts:864-981`).
#[derive(Debug, Clone, PartialEq)]
pub enum StatusUpdateOutcome {
    /// A plain transition applied (200 `{id, status}`).
    Updated(OrderStatus),
    /// Honest-dispatch result for a delivery IN_DELIVERY target (200 `{id, status, dispatched,
    /// reason}`) — the dispatch ENGINE is S7; this port carries the ORDERING (no courier → stay put,
    /// never advance-then-orphan) with the engine stubbed to "no courier".
    Dispatched {
        status: OrderStatus,
        dispatched: bool,
        reason: Option<String>,
    },
    /// Membership-JOIN miss — order not found OR cross-tenant (404, existence-hiding).
    NotFound,
    /// Actor-gate (403) / CC-1 (409) / illegal-transition (400/409) / status-race (409 CONFLICT).
    Rejected(ErrorCode, String),
}

/// The customer post-dispatch cancel outcome (`customer/orders.ts:290-341`).
#[derive(Debug, Clone, PartialEq)]
pub enum CustomerCancelOutcome {
    Cancelled,
    /// Order not found OR not owned by this customer (`WHERE id=$ AND customer_id=$sub`).
    NotFound,
    /// Not IN_DELIVERY → 409 CANCEL_NOT_ALLOWED_STATUS.
    NotInDelivery,
    /// Past the post-dispatch window → 410 CANCEL_WINDOW_EXPIRED.
    WindowExpired,
}

/// The tri-principal read outcome.
#[derive(Debug, Clone, PartialEq)]
pub enum OrderReadOutcome {
    Found(OrderView),
    /// Not found / not visible to this principal (404).
    NotFound,
}

// ─────────────────────────────── repo trait + state ───────────────────────────────

#[async_trait::async_trait]
pub trait OrdersRepo: Send + Sync {
    /// `POST /orders` — the full create funnel in ONE **GUC-less** `pool.begin()` tx (REV-S5-1).
    async fn create_order(&self, cmd: CreateOrderCommand) -> Result<CreateOutcome, RepoError>;

    /// `PATCH /orders/:id/status` — owner transition inside a `with_user(owner_user_id)` tx
    /// (membership-JOIN authz + actor-gate + CC-1 + `updateOrderStatus` folds).
    async fn owner_update_status(
        &self,
        owner_user_id: Uuid,
        order_id: Uuid,
        new_status: OrderStatus,
    ) -> Result<StatusUpdateOutcome, RepoError>;

    /// `POST /api/customer/orders/:orderId/cancel` — post-dispatch cancel bound to `customer_id =
    /// sub`, mutation inside a `with_tenant(location_id)` tx (REV-S5-1 customer half / LC3 GUC dance).
    async fn customer_cancel(
        &self,
        customer_sub: Uuid,
        order_id: Uuid,
    ) -> Result<CustomerCancelOutcome, RepoError>;

    /// `GET /orders/:id` — owner (membership-JOIN) or customer (order-scope) read. `owner_user_id`
    /// is `Some` for an owner principal; `customer_sub`/`customer_order` bind the customer scope.
    async fn get_order(
        &self,
        order_id: Uuid,
        owner_user_id: Option<Uuid>,
        customer_sub: Option<Uuid>,
    ) -> Result<OrderReadOutcome, RepoError>;

    /// `POST /api/owner/locations/:locationId/orders/:orderId/{confirm,reject}` — Node's
    /// `transitionOrder` (`dashboard.ts:626`): the SAME transition engine as `owner_update_status`
    /// but URL-location-scoped (IDOR: an owner of A+B can't transition B's order via A's URL) and,
    /// on REJECTED, writing `orders.rejection_reason`. Kept a SEPARATE method (not a signature
    /// change to the proven `owner_update_status`) to keep the PATCH path byte-untouched.
    async fn owner_order_action(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        order_id: Uuid,
        new_status: OrderStatus,
        reject_reason: Option<String>,
    ) -> Result<StatusUpdateOutcome, RepoError>;

    /// `PATCH /api/owner/locations/:locationId/orders/:orderId/metadata` (`order-meta.ts:13`) —
    /// merges `metadata.test_order` via `jsonb_set`, URL-location-scoped. `None` → 404.
    async fn patch_order_test_metadata(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        order_id: Uuid,
        test_order: bool,
    ) -> Result<Option<()>, RepoError>;
}

#[derive(Clone)]
pub struct OrdersState {
    pub auth: crate::auth::AuthState,
    pub repo: std::sync::Arc<dyn OrdersRepo>,
}

// ─────────────────────────────── handlers ───────────────────────────────

/// `POST /orders` — anonymous create (no auth extractor; the create path seats NO `app.user_id`,
/// REV-S5-1). Parses + validates `CreateOrderInput` (REV-S5-3), normalizes the `x-channel` header
/// (Q-CHANNEL-META), then hands the whole funnel to the repo's GUC-less tx.
#[utoipa::path(
    post, path = "/api/orders", tag = "orders",
    responses(
        (status = 201, description = "Order created", body = OrderCreatedResponse),
        (status = 200, description = "Idempotent replay of an existing order", body = OrderCreatedResponse),
        (status = 400, description = "VALIDATION_FAILED", body = domain::ErrorEnvelope),
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
        // Log the real cause (the client only gets an opaque INTERNAL). A money-path 500 with
        // no logged cause is an operability hole — the create funnel touches many statements and
        // "500 INTERNAL" alone is undiagnosable (staging cutover 2026-07-05).
        tracing::error!(%correlation_id, error = %e.0, "order create failed");
        ApiError::new(
            ErrorCode::Internal,
            "internal_error",
            correlation_id.clone(),
        )
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

/// `PATCH /orders/:id/status` — owner-driven transition (OwnerClaimsExt narrows role structurally;
/// a courier/customer token cannot reach here).
#[utoipa::path(
    patch, path = "/api/orders/{id}/status", tag = "orders",
    params(("id" = Uuid, Path)),
    request_body = StatusUpdateInput,
    responses(
        (status = 200, description = "Transition applied", body = StatusUpdateResponse),
        (status = 400, description = "VALIDATION_FAILED / illegal transition", body = domain::ErrorEnvelope),
        (status = 403, description = "CANCEL_NOT_PERMITTED (actor-gate)", body = domain::ErrorEnvelope),
        (status = 404, description = "Not found / cross-tenant", body = domain::ErrorEnvelope),
        (status = 409, description = "CONFLICT / ASSIGNMENT_ACTIVE / USE_DELIVER_FLOW", body = domain::ErrorEnvelope),
    ))]
pub async fn owner_update_status(
    Extension(state): Extension<OrdersState>,
    crate::auth::extractors::OwnerClaimsExt(owner): crate::auth::extractors::OwnerClaimsExt,
    Path(order_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
    axum::Json(body): axum::Json<StatusUpdateInput>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);

    let outcome = state
        .repo
        .owner_update_status(owner.user_id, order_id, body.status)
        .await
        .map_err(|_e| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?;

    match outcome {
        StatusUpdateOutcome::Updated(status) => Ok(axum::Json(StatusUpdateResponse {
            id: order_id,
            status,
            dispatched: None,
            reason: None,
        })),
        StatusUpdateOutcome::Dispatched {
            status,
            dispatched,
            reason,
        } => Ok(axum::Json(StatusUpdateResponse {
            id: order_id,
            status,
            dispatched: Some(dispatched),
            reason,
        })),
        StatusUpdateOutcome::NotFound => Err(ApiError::new(
            ErrorCode::NotFound,
            "Order not found",
            correlation_id,
        )),
        StatusUpdateOutcome::Rejected(code, msg) => Err(ApiError::new(code, msg, correlation_id)),
    }
}

/// `POST /api/customer/orders/:orderId/cancel` — customer post-dispatch cancel. REV-S5-1 customer
/// half: `require_order` binds the token's `orderId` claim to the path (403 on mismatch — closes the
/// S2 T-12 cross-order bug), AND the repo keeps the `customer_id = sub` predicate (belt-and-braces).
#[utoipa::path(
    post, path = "/api/customer/orders/{orderId}/cancel", tag = "orders",
    params(("orderId" = Uuid, Path)),
    responses(
        (status = 200, description = "Cancelled", body = CustomerCancelResponse),
        (status = 403, description = "Token not scoped to this order (require_order)", body = domain::ErrorEnvelope),
        (status = 404, description = "Not found / not owned", body = domain::ErrorEnvelope),
        (status = 409, description = "CANCEL_NOT_ALLOWED_STATUS", body = domain::ErrorEnvelope),
        (status = 410, description = "CANCEL_WINDOW_EXPIRED", body = domain::ErrorEnvelope),
    ))]
pub async fn customer_cancel(
    Extension(state): Extension<OrdersState>,
    customer: crate::auth::extractors::CustomerClaimsExt,
    Path(order_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);

    // REV-3/T-12: the token must be scoped to THIS order (403 on mismatch) — the primary authority.
    customer
        .require_order(order_id)
        .map_err(|_rej| ApiError::new(ErrorCode::Forbidden, "Forbidden", correlation_id.clone()))?;

    let outcome = state
        .repo
        .customer_cancel(customer.0.sub, order_id)
        .await
        .map_err(|_e| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?;

    match outcome {
        CustomerCancelOutcome::Cancelled => {
            Ok(axum::Json(CustomerCancelResponse { success: true }))
        }
        CustomerCancelOutcome::NotFound => Err(ApiError::new(
            ErrorCode::NotFound,
            "Order not found",
            correlation_id,
        )),
        CustomerCancelOutcome::NotInDelivery => Err(ApiError::new(
            ErrorCode::CancelNotAllowedStatus,
            "CANCEL_NOT_ALLOWED_STATUS",
            correlation_id,
        )),
        CustomerCancelOutcome::WindowExpired => Err(ApiError::new(
            ErrorCode::CancelWindowExpired,
            "CANCEL_WINDOW_EXPIRED",
            correlation_id,
        )),
    }
}

/// `GET /orders/:id` — tri-principal read (`orders.ts:735-861`). This port serves the OWNER
/// (membership-JOIN) and CUSTOMER (order-scope, `require_order`) principals; an anonymous/unknown
/// principal is 401 (no bare-UUID enumeration). The COURIER read verdict (`courierReadVerdict`,
/// ADR-0013 live-binding, 503-on-UNAVAILABLE) is the S6/S7 dispatch surface — deferred there per the
/// packet §5.3 boundary, flagged not silently dropped.
#[utoipa::path(
    get, path = "/api/orders/{id}", tag = "orders",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "Order view", body = OrderView),
        (status = 401, description = "No/invalid principal", body = domain::ErrorEnvelope),
        (status = 404, description = "Not found / not visible to this principal", body = domain::ErrorEnvelope),
    ))]
pub async fn get_order(
    Extension(state): Extension<OrdersState>,
    Path(order_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);

    // Soft-verify the bearer (if any) to resolve the principal. No bearer → 401 (orders.ts:855).
    let claims = bearer_claims(&state.auth, &headers);
    let Some(claims) = claims else {
        return Err(ApiError::new(
            ErrorCode::Unauthorized,
            "Unauthorized",
            correlation_id,
        ));
    };

    let (owner_user_id, customer_sub) = match &claims {
        crate::auth::claims::Claims::Owner(o) => (Some(o.user_id), None),
        crate::auth::claims::Claims::Customer(c) => {
            // Customer order-scope: the token's orderId must equal the path (orders.ts:846, 404).
            if c.order_id != order_id {
                return Err(ApiError::new(
                    ErrorCode::NotFound,
                    "Order not found",
                    correlation_id,
                ));
            }
            (None, Some(c.sub))
        }
        // Courier read is the ADR-0013 binding-verdict surface (S6/S7) — not served here.
        crate::auth::claims::Claims::Courier(_) => {
            return Err(ApiError::new(
                ErrorCode::Unauthorized,
                "Unauthorized",
                correlation_id,
            ));
        }
    };

    let outcome = state
        .repo
        .get_order(order_id, owner_user_id, customer_sub)
        .await
        .map_err(|_e| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?;

    match outcome {
        OrderReadOutcome::Found(view) => Ok(axum::Json(view)),
        OrderReadOutcome::NotFound => Err(ApiError::new(
            ErrorCode::NotFound,
            "Order not found",
            correlation_id,
        )),
    }
}

/// Soft bearer resolution for the tri-principal read: verify the token if present, else `None`
/// (never a hard reject here — the handler decides 401). Mirrors `softVerifyAuth` (orders.ts).
fn bearer_claims(
    auth: &crate::auth::AuthState,
    headers: &HeaderMap,
) -> Option<crate::auth::claims::Claims> {
    let raw = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    let token = raw.strip_prefix("Bearer ")?;
    auth.verifier.verify(token).ok()
}

// ─────────────────────────────── router ───────────────────────────────

/// Assemble the S5 orders/money surface at the Node-parity paths. The owner/customer ops ride the
/// SAME REV-4 bearer/dev pre-gate + request-id layer stack as S2/S3; `POST /orders` and
/// `GET /orders/:id` are reachable without a bearer (anonymous create / soft-verify read), but the
/// bearer gate's allowlist handling is inherited from the shared middleware (a bearer-less create is
/// NOT pre-gated — same as Node's public `POST /orders`). Mounted dark (see `main.rs`).
/// `transitionOrder` response (`dashboard.ts:661`): `{ id, status, statusUpdatedAt }`.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct OrderActionResponse {
    pub id: Uuid,
    #[schema(value_type = String)]
    pub status: OrderStatus,
    #[serde(rename = "statusUpdatedAt")]
    pub status_updated_at: String,
}

/// `POST .../reject` body (`dashboard.ts:206`) — optional free-text reason.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RejectOrderBody {
    #[serde(default)]
    pub reason: Option<String>,
}

/// `PATCH .../metadata` body (`order-meta.ts`) — the only mutable metadata key is `test_order`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct OrderMetadataBody {
    pub test_order: bool,
}

fn action_outcome_response(
    outcome: StatusUpdateOutcome,
    order_id: Uuid,
    correlation_id: String,
) -> Result<axum::response::Response, ApiError> {
    match outcome {
        // The honest-dispatch "stayed put" carries the current status; confirm/reject never hit it.
        StatusUpdateOutcome::Updated(status) | StatusUpdateOutcome::Dispatched { status, .. } => {
            Ok((
                StatusCode::OK,
                axum::Json(OrderActionResponse {
                    id: order_id,
                    status,
                    status_updated_at: chrono::Utc::now()
                        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                }),
            )
                .into_response())
        }
        StatusUpdateOutcome::NotFound => Err(ApiError::new(
            ErrorCode::NotFound,
            "Order not found",
            correlation_id,
        )),
        StatusUpdateOutcome::Rejected(code, msg) => Err(ApiError::new(code, msg, correlation_id)),
    }
}

/// `POST /api/owner/locations/{locationId}/orders/{orderId}/confirm` (`dashboard.ts:193`).
#[utoipa::path(post, path = "/api/owner/locations/{locationId}/orders/{orderId}/confirm", tag = "orders",
    params(("locationId" = Uuid, Path), ("orderId" = Uuid, Path)),
    responses((status = 200, body = OrderActionResponse), (status = 404, body = domain::ErrorEnvelope)))]
pub async fn confirm_order(
    Extension(state): Extension<OrdersState>,
    crate::auth::extractors::OwnerClaimsExt(owner): crate::auth::extractors::OwnerClaimsExt,
    Path((location_id, order_id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let outcome = state
        .repo
        .owner_order_action(
            owner.user_id,
            location_id,
            order_id,
            OrderStatus::Confirmed,
            None,
        )
        .await
        .map_err(|_e| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?;
    action_outcome_response(outcome, order_id, correlation_id)
}

/// `POST /api/owner/locations/{locationId}/orders/{orderId}/reject` (`dashboard.ts:203`).
#[utoipa::path(post, path = "/api/owner/locations/{locationId}/orders/{orderId}/reject", tag = "orders",
    params(("locationId" = Uuid, Path), ("orderId" = Uuid, Path)), request_body = RejectOrderBody,
    responses((status = 200, body = OrderActionResponse), (status = 404, body = domain::ErrorEnvelope)))]
pub async fn reject_order(
    Extension(state): Extension<OrdersState>,
    crate::auth::extractors::OwnerClaimsExt(owner): crate::auth::extractors::OwnerClaimsExt,
    Path((location_id, order_id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
    axum::Json(body): axum::Json<RejectOrderBody>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let outcome = state
        .repo
        .owner_order_action(
            owner.user_id,
            location_id,
            order_id,
            OrderStatus::Rejected,
            body.reason,
        )
        .await
        .map_err(|_e| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?;
    action_outcome_response(outcome, order_id, correlation_id)
}

/// `PATCH /api/owner/locations/{locationId}/orders/{orderId}/metadata` (`order-meta.ts:13`).
#[utoipa::path(patch, path = "/api/owner/locations/{locationId}/orders/{orderId}/metadata", tag = "orders",
    params(("locationId" = Uuid, Path), ("orderId" = Uuid, Path)), request_body = OrderMetadataBody,
    responses((status = 200, description = "Updated"), (status = 404, body = domain::ErrorEnvelope)))]
pub async fn patch_order_metadata(
    Extension(state): Extension<OrdersState>,
    crate::auth::extractors::OwnerClaimsExt(owner): crate::auth::extractors::OwnerClaimsExt,
    Path((location_id, order_id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
    axum::Json(body): axum::Json<OrderMetadataBody>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    state
        .repo
        .patch_order_test_metadata(owner.user_id, location_id, order_id, body.test_order)
        .await
        .map_err(|_e| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?
        .ok_or_else(|| ApiError::new(ErrorCode::NotFound, "Not found", correlation_id))?;
    Ok(axum::Json(serde_json::json!({ "success": true })))
}

pub fn orders_router(state: OrdersState) -> axum::Router {
    use axum::routing::{get, patch, post};
    use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};

    let correlation_header = axum::http::HeaderName::from_static("x-correlation-id");

    axum::Router::new()
        .route("/api/orders", post(create_order))
        .route("/api/orders/{id}", get(get_order))
        .route("/api/orders/{id}/status", patch(owner_update_status))
        .route(
            "/api/customer/orders/{orderId}/cancel",
            post(customer_cancel),
        )
        .route(
            "/api/owner/locations/{locationId}/orders/{orderId}/confirm",
            post(confirm_order),
        )
        .route(
            "/api/owner/locations/{locationId}/orders/{orderId}/reject",
            post(reject_order),
        )
        .route(
            "/api/owner/locations/{locationId}/orders/{orderId}/metadata",
            patch(patch_order_metadata),
        )
        // AuthState extension (the S2 extractors read it via `state_from(parts)`) — MUST be inserted
        // alongside OrdersState, exactly like S3's owner_catalog_router layers `states.auth`.
        .layer(axum::Extension(state.auth.clone()))
        // OrdersState + request-id layer (S3 parity — merged routers do NOT inherit build_router's).
        .layer(axum::Extension(state))
        .layer(PropagateRequestIdLayer::new(correlation_header.clone()))
        .layer(SetRequestIdLayer::new(correlation_header, MakeRequestUuid))
}

pub mod pg;

#[cfg(test)]
mod handler_tests {
    use super::*;
    use crate::auth::claims::{Claims, CustomerClaims, OwnerClaims};
    use crate::auth::repo::fake::FakeAuthRepo;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;

    /// A canned-outcome fake — each op returns a preset outcome so the handler's outcome→HTTP
    /// mapping (and the router wiring/extractors) is proven without a DB. The pure DECISIONS are
    /// covered by the submodule tests; this proves the HANDLER layer.
    #[derive(Default)]
    struct FakeOrdersRepo {
        create: Mutex<Option<CreateOutcome>>,
        status: Mutex<Option<StatusUpdateOutcome>>,
        cancel: Mutex<Option<CustomerCancelOutcome>>,
        read: Mutex<Option<OrderReadOutcome>>,
        /// records the last customer_sub the cancel repo method was called with (proves the
        /// belt-and-braces `customer_id = sub` predicate is threaded).
        last_cancel_sub: Mutex<Option<Uuid>>,
    }

    #[async_trait::async_trait]
    impl OrdersRepo for FakeOrdersRepo {
        async fn create_order(&self, _cmd: CreateOrderCommand) -> Result<CreateOutcome, RepoError> {
            Ok(self
                .create
                .lock()
                .unwrap()
                .clone()
                .unwrap_or(CreateOutcome::Transient))
        }
        async fn owner_update_status(
            &self,
            _owner: Uuid,
            _order: Uuid,
            new: OrderStatus,
        ) -> Result<StatusUpdateOutcome, RepoError> {
            Ok(self
                .status
                .lock()
                .unwrap()
                .clone()
                .unwrap_or(StatusUpdateOutcome::Updated(new)))
        }
        async fn customer_cancel(
            &self,
            customer_sub: Uuid,
            _order: Uuid,
        ) -> Result<CustomerCancelOutcome, RepoError> {
            *self.last_cancel_sub.lock().unwrap() = Some(customer_sub);
            Ok(self
                .cancel
                .lock()
                .unwrap()
                .clone()
                .unwrap_or(CustomerCancelOutcome::Cancelled))
        }
        async fn get_order(
            &self,
            _order: Uuid,
            _owner: Option<Uuid>,
            _customer: Option<Uuid>,
        ) -> Result<OrderReadOutcome, RepoError> {
            Ok(self
                .read
                .lock()
                .unwrap()
                .clone()
                .unwrap_or(OrderReadOutcome::NotFound))
        }
        async fn owner_order_action(
            &self,
            _owner: Uuid,
            _location: Uuid,
            _order: Uuid,
            new: OrderStatus,
            _reason: Option<String>,
        ) -> Result<StatusUpdateOutcome, RepoError> {
            Ok(self
                .status
                .lock()
                .unwrap()
                .clone()
                .unwrap_or(StatusUpdateOutcome::Updated(new)))
        }
        async fn patch_order_test_metadata(
            &self,
            _owner: Uuid,
            _location: Uuid,
            _order: Uuid,
            _test_order: bool,
        ) -> Result<Option<()>, RepoError> {
            Ok(Some(()))
        }
    }

    fn lek(v: i64) -> domain::Lek {
        domain::Lek::new(v).unwrap()
    }

    fn state_with(repo: Arc<FakeOrdersRepo>) -> OrdersState {
        OrdersState {
            auth: crate::auth::AuthState::test_state(Arc::new(FakeAuthRepo::default())),
            repo,
        }
    }

    async fn json_body(resp: axum::response::Response) -> (StatusCode, serde_json::Value) {
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json = if bytes.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
        };
        (status, json)
    }

    fn valid_create_body() -> serde_json::Value {
        serde_json::json!({
            "locationId": "11111111-1111-1111-1111-111111111111",
            "type": "pickup",
            "items": [{ "product_id": "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa", "quantity": 1 }],
            "payment": { "method": "cash" },
            "idempotency_key": "22222222-2222-2222-2222-222222222222"
        })
    }

    #[test]
    fn orders_router_builds_without_panicking() {
        let _router = orders_router(state_with(Arc::new(FakeOrdersRepo::default())));
    }

    /// Anonymous create happy path → 201 (the create path binds NO auth extractor — REV-S5-1).
    #[tokio::test]
    async fn create_anonymous_happy_path_is_201() {
        let repo = Arc::new(FakeOrdersRepo::default());
        *repo.create.lock().unwrap() = Some(CreateOutcome::Created(OrderCreatedResponse {
            id: Uuid::new_v4(),
            location_id: Uuid::new_v4(),
            status: OrderStatus::Pending,
            subtotal: lek(1000),
            total: lek(1300),
            delivery_instructions: None,
            created_at: None,
        }));
        let app = orders_router(state_with(repo));
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/orders")
                    .header("content-type", "application/json")
                    .body(Body::from(valid_create_body().to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    /// REV-S5-3 at the HTTP boundary: a `signal` messenger order — a live 422 on Node TODAY —
    /// now reaches the create handler (201), proving the schema admits the 6-kind set through the
    /// real router/JSON extractor.
    #[tokio::test]
    async fn create_with_signal_messenger_kind_reaches_handler_201() {
        let repo = Arc::new(FakeOrdersRepo::default());
        *repo.create.lock().unwrap() = Some(CreateOutcome::Created(OrderCreatedResponse {
            id: Uuid::new_v4(),
            location_id: Uuid::new_v4(),
            status: OrderStatus::Pending,
            subtotal: lek(1000),
            total: lek(1000),
            delivery_instructions: None,
            created_at: None,
        }));
        let mut body = valid_create_body();
        body["customer"] =
            serde_json::json!({ "messenger_kind": "signal", "messenger_handle": "@x" });
        body["receiver"] =
            serde_json::json!({ "name": "Ana", "messenger_kind": "simplex", "handle": "@ana" });
        let app = orders_router(state_with(repo));
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/orders")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    /// `.strict()` — an unknown top-level key is a 400 VALIDATION_FAILED at the boundary.
    #[tokio::test]
    async fn create_with_unknown_field_is_400() {
        let mut body = valid_create_body();
        body["surprise"] = serde_json::json!(true);
        let app = orders_router(state_with(Arc::new(FakeOrdersRepo::default())));
        let (status, json) = json_body(
            app.oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/orders")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json["code"], "VALIDATION_FAILED");
    }

    /// The create idempotency-reuse business rejection maps to 422 IDEMPOTENCY_KEY_REUSED.
    #[tokio::test]
    async fn create_idempotency_reuse_is_422() {
        let repo = Arc::new(FakeOrdersRepo::default());
        *repo.create.lock().unwrap() = Some(CreateOutcome::Rejected(
            ErrorCode::IdempotencyKeyReused,
            "Idempotency key reused with different request".to_string(),
        ));
        let app = orders_router(state_with(repo));
        let (status, json) = json_body(
            app.oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/orders")
                    .header("content-type", "application/json")
                    .body(Body::from(valid_create_body().to_string()))
                    .unwrap(),
            )
            .await
            .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(json["code"], "IDEMPOTENCY_KEY_REUSED");
    }

    /// The transient arm maps to a 503 (never a scary 500).
    #[tokio::test]
    async fn create_transient_is_503() {
        let repo = Arc::new(FakeOrdersRepo::default());
        *repo.create.lock().unwrap() = Some(CreateOutcome::Transient);
        let app = orders_router(state_with(repo));
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/orders")
                    .header("content-type", "application/json")
                    .body(Body::from(valid_create_body().to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    /// Owner PATCH: an actor-gate rejection (REV-S5-9) surfaces as 403 CANCEL_NOT_PERMITTED through
    /// the real router (bearer gate + OwnerClaimsExt + handler).
    #[tokio::test]
    async fn owner_status_actor_gate_reject_is_403() {
        let repo = Arc::new(FakeOrdersRepo::default());
        *repo.status.lock().unwrap() = Some(StatusUpdateOutcome::Rejected(
            ErrorCode::CancelNotPermitted,
            "Cancelling an order in preparation is not available".to_string(),
        ));
        let auth = crate::auth::AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let state = OrdersState {
            auth: auth.clone(),
            repo,
        };
        let token = auth
            .verifier
            .mint(Claims::Owner(OwnerClaims::new(Uuid::new_v4(), None)), 3600)
            .unwrap();
        let order_id = Uuid::new_v4();
        let app = orders_router(state);
        let (status, json) = json_body(
            app.oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/orders/{order_id}/status"))
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"status":"CANCELLED"}"#))
                    .unwrap(),
            )
            .await
            .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(json["code"], "CANCEL_NOT_PERMITTED");
    }

    /// Owner PATCH happy transition → 200 {id, status}.
    #[tokio::test]
    async fn owner_status_update_is_200() {
        let repo = Arc::new(FakeOrdersRepo::default());
        *repo.status.lock().unwrap() = Some(StatusUpdateOutcome::Updated(OrderStatus::Confirmed));
        let auth = crate::auth::AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let token = auth
            .verifier
            .mint(Claims::Owner(OwnerClaims::new(Uuid::new_v4(), None)), 3600)
            .unwrap();
        let app = orders_router(OrdersState { auth, repo });
        let (status, json) = json_body(
            app.oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/orders/{}/status", Uuid::new_v4()))
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"status":"CONFIRMED"}"#))
                    .unwrap(),
            )
            .await
            .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["status"], "CONFIRMED");
    }

    /// Owner PATCH with a courier token cannot reach the handler (OwnerClaimsExt narrows → 401).
    #[tokio::test]
    async fn owner_status_with_courier_token_is_401() {
        use crate::auth::claims::CourierClaims;
        let auth = crate::auth::AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let token = auth
            .verifier
            .mint(
                Claims::Courier(CourierClaims::new(Uuid::new_v4(), Uuid::new_v4(), None)),
                3600,
            )
            .unwrap();
        let app = orders_router(OrdersState {
            auth,
            repo: Arc::new(FakeOrdersRepo::default()),
        });
        let resp = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/orders/{}/status", Uuid::new_v4()))
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"status":"CONFIRMED"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    /// REV-S5-1 customer half at HTTP: a token minted for order A cancelling order B is 403
    /// (`require_order` — the S2 T-12 cross-order fix), and the repo is NEVER reached.
    #[tokio::test]
    async fn customer_cancel_cross_order_is_403_and_repo_untouched() {
        let auth = crate::auth::AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let order_a = Uuid::new_v4();
        let order_b = Uuid::new_v4();
        let cust = Uuid::new_v4();
        let token = auth
            .verifier
            .mint(
                Claims::Customer(CustomerClaims::new(cust, order_a, Uuid::new_v4())),
                3600,
            )
            .unwrap();
        let repo = Arc::new(FakeOrdersRepo::default());
        let app = orders_router(OrdersState {
            auth,
            repo: repo.clone(),
        });
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/customer/orders/{order_b}/cancel"))
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        assert!(
            repo.last_cancel_sub.lock().unwrap().is_none(),
            "require_order must 403 BEFORE the repo (cross-order token never reaches the mutation)"
        );
    }

    /// The matching-order customer cancel reaches the repo (with the customer's sub) and 200s.
    #[tokio::test]
    async fn customer_cancel_same_order_reaches_repo_200() {
        let auth = crate::auth::AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let order = Uuid::new_v4();
        let cust = Uuid::new_v4();
        let token = auth
            .verifier
            .mint(
                Claims::Customer(CustomerClaims::new(cust, order, Uuid::new_v4())),
                3600,
            )
            .unwrap();
        let repo = Arc::new(FakeOrdersRepo::default());
        *repo.cancel.lock().unwrap() = Some(CustomerCancelOutcome::Cancelled);
        let app = orders_router(OrdersState {
            auth,
            repo: repo.clone(),
        });
        let (status, json) = json_body(
            app.oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/customer/orders/{order}/cancel"))
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["success"], true);
        // belt-and-braces: the repo was called bound to THIS customer's sub.
        assert_eq!(*repo.last_cancel_sub.lock().unwrap(), Some(cust));
    }

    /// The customer cancel window-expired arm → 410 CANCEL_WINDOW_EXPIRED.
    #[tokio::test]
    async fn customer_cancel_window_expired_is_410() {
        let auth = crate::auth::AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let order = Uuid::new_v4();
        let token = auth
            .verifier
            .mint(
                Claims::Customer(CustomerClaims::new(Uuid::new_v4(), order, Uuid::new_v4())),
                3600,
            )
            .unwrap();
        let repo = Arc::new(FakeOrdersRepo::default());
        *repo.cancel.lock().unwrap() = Some(CustomerCancelOutcome::WindowExpired);
        let app = orders_router(OrdersState { auth, repo });
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/customer/orders/{order}/cancel"))
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::GONE);
    }

    /// GET /orders/:id with NO bearer → 401 (no bare-UUID enumeration, orders.ts:855).
    #[tokio::test]
    async fn get_order_anonymous_is_401() {
        let app = orders_router(state_with(Arc::new(FakeOrdersRepo::default())));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/orders/{}", Uuid::new_v4()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    /// GET /orders/:id — a customer token scoped to a DIFFERENT order is 404 (cross-order read
    /// blocked; the orders.ts:846 orderId-scope check).
    #[tokio::test]
    async fn get_order_customer_cross_order_is_404() {
        let auth = crate::auth::AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let token = auth
            .verifier
            .mint(
                Claims::Customer(CustomerClaims::new(
                    Uuid::new_v4(),
                    Uuid::new_v4(), // token's order
                    Uuid::new_v4(),
                )),
                3600,
            )
            .unwrap();
        let app = orders_router(OrdersState {
            auth,
            repo: Arc::new(FakeOrdersRepo::default()),
        });
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/orders/{}", Uuid::new_v4())) // a DIFFERENT order
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    /// GET /orders/:id — an owner token whose order is found → 200 view.
    #[tokio::test]
    async fn get_order_owner_found_is_200() {
        let auth = crate::auth::AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let order = Uuid::new_v4();
        let token = auth
            .verifier
            .mint(Claims::Owner(OwnerClaims::new(Uuid::new_v4(), None)), 3600)
            .unwrap();
        let repo = Arc::new(FakeOrdersRepo::default());
        *repo.read.lock().unwrap() = Some(OrderReadOutcome::Found(OrderView {
            id: order,
            status: OrderStatus::Confirmed,
            subtotal: lek(1000),
            total: lek(1300),
            location_id: Uuid::new_v4(),
        }));
        let app = orders_router(OrdersState { auth, repo });
        let (status, json) = json_body(
            app.oneshot(
                Request::builder()
                    .uri(format!("/api/orders/{order}"))
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["status"], "CONFIRMED");
    }
}
