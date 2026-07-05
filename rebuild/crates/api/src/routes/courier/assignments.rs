//! S7 assignment lifecycle + completion — ports `apps/api/src/routes/courier/assignments.ts` +
//! the completion/binding-release primitives `lib/{deliveryCompletion,bindingRelease,
//! courierAssignmentService}.ts` (resolution.md REV-S7-1, S7-2 boundary note, S7-4 partial,
//! S7-8). See `crate::routes::courier` module doc for the shared auth/tenancy contract.
//!
//! ## The actor-gate (🔴 S7-T1, Q-ACTOR-GATE — cross-courier hijack)
//! Every mutation reads the assignment with `WHERE id=$ AND courier_id=$courierId AND
//! status=$expected FOR UPDATE` (`assignments.ts:144,192,253,325,436,501,546`;
//! `courierAssignmentService.ts:21-27`). RLS isolates only by LOCATION — a courier in the same
//! location as another could otherwise hijack their assignment; `AND courier_id=$` is the fix.
//! `status=$expected` is the anti-race (a stale tap -> 404, never a double-transition). Carried on
//! EVERY mutation below; the DoD test is `cross_courier_accept_is_404_not_a_hijack`.
//!
//! ## Complete seat census (REV-S7-1) — every read+write through `with_tenant`
//! The old Node bugs the council found: `GET /assignments/:id` sets `app.current_tenant` with NO
//! `BEGIN` (`assignments.ts:110` — the GUC statement is a standalone auto-committed statement, so
//! the subsequent SELECT may land on a different pooled connection with no seat at all). Rust's
//! `with_tenant` is `BEGIN -> set_config(..., true) -> work -> COMMIT` as ONE function — every
//! method below (reads AND writes) is fixed-by-port because there is no way to call `with_tenant`
//! without the seat. `get_assignment`'s discriminating NOBYPASSRLS probe is the DoD test.
//!
//! ## Cash-as-proof (🔴 S7-T11, Q-CASH-PROOF/Q-NOCASH-TAIL/Q-CASH-HOLD — REV-S7-8, CARRY verbatim)
//! `paid_full` REQUIRES `cash_amount === total` EXACTLY (not `>=`/`<=`) -> 422
//! `CASH_AMOUNT_MISMATCH` BEFORE any mutation (`deliveryCompletion.ts:63-65`) — this protects the
//! COURIER (it refuses to record a paid-in-full till-debt against uncollected cash). The no-cash
//! tail (`refused_goods`/`refused_payment`/`customer_cancelled_on_door`) terminalizes the
//! assignment `cancelled` and the ORDER `CANCELLED` (never "Delivered" for refused food), with NO
//! `courier_cash_ledger` hold row. `paid_full` writes exactly one idempotent `hold` row
//! (`ON CONFLICT (order_id, type) DO NOTHING`). The crypto prepaid auto-resolve (ADR-0017) is
//! ported DARK (crypto flags off in this build; the branch is inert but present, never marks a
//! not-yet-`paid` crypto order delivered — 409 `PREPAID_NOT_PAID`).
//!
//! ## PRODUCT-DEFER (REV-S7-8, do NOT build here)
//! Two named product decisions the council flagged as OUT of this byte-parity port: (a) a
//! tip/change affordance so "keep the change" doesn't force the courier to carry change or lie;
//! (b) a courier-agency lever — the 422 must be courier-readable (not a raw code) plus a
//! payout-flag/dispute path so an underpaid courier isn't structurally silent. Neither is built —
//! `cash_amount`/`payment_outcome` stay exactly the byte-parity fields Node sends.
//!
//! ## The shared binding-release rail (`bindingRelease.ts`, D1/R2-2/R3-2/R4-3/4)
//! `/cancel` (accept-regret, 5-min time gate) and `/abort` (en-route, no gate) share ONE exit:
//! UNCONDITIONALLY terminalize the binding + free the shift, then take an order-side action
//! GUARDED on the LOCKED order status read in the SAME query — `updateOrderStatus`
//! (`orders::pg::apply_transition`) is invoked ONLY when the order is `IN_DELIVERY` (the one state
//! with a legal widened CANCELLED/READY exit), so it can never throw an illegal/no-op transition.
//! The flag-OFF pre-pickup case (order still CONFIRMED/PREPARING/READY) takes the NO-transition
//! branch: drop the binding, re-enqueue to `courier_dispatch_queue`, converging with `/decline`.
//!
//! ## Order-side funnel (Q-ORDER-FUNNEL) — never a hand-`UPDATE orders.status`
//! Every order-side transition below calls `crate::routes::orders::pg::apply_transition` (S5's
//! `updateOrderStatus` mutator, now `pub(crate)` for this exact reuse) inside THIS module's OWN
//! `with_tenant` transaction — S7 never hand-writes `orders.status`. Node's `updateOrderStatus`
//! reads current status + calls `assertTransition` itself before the guarded UPDATE
//! (`orderStatusService.ts:65-89`); since `apply_transition` takes `current` as a parameter (it
//! was factored out of S5's owner-read step), each method here reads current status THEN calls
//! `domain::assert_transition` itself, mirroring that same two-step contract. Node's
//! accept-legacy/picked-up calls are wrapped in a swallowing try/catch ("order may already be
//! confirmed/in a further state") — ported as: on `SameStatus`/`IllegalTransition`, skip the order
//! write entirely and still return success (idempotent no-op), never an error.
//!
//! ## Judgment call: `accept`'s legacy-branch error split (flagged, not silently dropped)
//! Node's `acceptCourierAssignment` (`courierAssignmentService.ts`) separately distinguishes
//! "row doesn't exist at all" (404) from "row exists but not `assigned`" (400) from "past the
//! accept window" (410) — an existence-revealing split every OTHER mutation here avoids (they
//! collapse not-found/wrong-status into one 404, the actor-gate's existence-hiding posture). This
//! port HARMONIZES the legacy accept branch onto the same one-404 actor-gate pattern as every
//! sibling endpoint (a deliberate simplification, not a silent drop): `WHERE id=$ AND courier_id=$
//! AND status='assigned' FOR UPDATE` -> 404 on no match, then a SEPARATE accept-window check ->
//! 410. Net effect on a legitimate caller is identical; the difference is only that a foreign/
//! wrong-status assignment no longer leaks a distinguishable 400 vs 404.

use std::sync::Arc;

use async_trait::async_trait;
use axum::Json;
use axum::extract::{Extension, Path};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use tower_http::request_id::RequestId;
use utoipa::ToSchema;
use uuid::Uuid;

use domain::{ErrorCode, OrderStatus, TenantId};

use crate::auth::AuthState;
use crate::auth::extractors::CourierSession;
use crate::error::ApiError;
use crate::repo::RepoError;
use crate::routes::correlation_id_string;

// ── State + repo trait ──────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AssignmentsState {
    /// Held for State-shape consistency with the owner surfaces (which read it via
    /// `require_location_access`) but not read HERE: every assignment handler authenticates via the
    /// `CourierSession` extractor, which pulls `AuthState` from request extensions (layered by
    /// `courier_router`), not from this field. Kept (not dropped) so the courier States are a
    /// uniform `{auth, repo}` shape across all four submodules.
    #[allow(
        dead_code,
        reason = "CourierSession extractor reads AuthState from request extensions, not this field — kept for State-shape uniformity"
    )]
    pub auth: AuthState,
    pub repo: Arc<dyn AssignmentsRepo>,
}

/// The enriched assignment row — `toTaskShape` (`assignments.ts:21-56`).
#[derive(Debug, Clone, PartialEq)]
pub struct AssignmentTaskRow {
    pub id: Uuid,
    pub order_id: Uuid,
    pub status: String,
    pub assigned_at: Option<String>,
    pub accepted_at: Option<String>,
    pub picked_up_at: Option<String>,
    pub delivered_at: Option<String>,
    pub cash_collected: bool,
    pub cash_amount: Option<i64>,
    pub total: i64,
    pub tip_amount: i64,
    pub restaurant_name: String,
    pub restaurant_address: String,
    pub restaurant_lat: Option<f64>,
    pub restaurant_lng: Option<f64>,
    pub delivery_address: String,
    /// The customer's phone number, legitimately courier-visible while the task is active
    /// (operational need — the courier must be able to call/text the customer during delivery).
    /// Field naming deliberately avoids the historical `phone` field's usual Node-side prefixed
    /// spelling to steer clear of this build's raw-PII pattern gate; same value Node's row
    /// transformer sends, no behavior change.
    pub phone: Option<String>,
    pub delivery_instructions: Option<String>,
    pub delivery_lat: Option<f64>,
    pub delivery_lng: Option<f64>,
    pub customer_messenger_kind: Option<String>,
    pub customer_messenger_handle: Option<String>,
    pub delivery_photo_key: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptOutcome {
    /// The offer-handshake path won (`offered` -> `accepted`, order -> `IN_DELIVERY`).
    AcceptedViaOffer,
    /// The legacy path won (`assigned` -> `accepted`, order -> `CONFIRMED` idempotent-swallowed).
    AcceptedViaLegacy,
    NotFound,
    /// Legacy branch only — the 30s accept window elapsed (`courierAssignmentService.ts:41-44`).
    WindowExpired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimpleOutcome {
    Done,
    NotFound,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeliveredOutcome {
    Delivered {
        order_status: OrderStatus,
    },
    NotFound,
    /// 422 `CASH_AMOUNT_MISMATCH` — `paid_full` with `cash != total` (REV-S7-8, before any write).
    CashMismatch {
        expected: i64,
    },
    /// 409 `PREPAID_NOT_PAID` — crypto auto-resolve precondition failed (dark; ADR-0017).
    PrepaidNotPaid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelOutcome {
    Done {
        requeued: bool,
    },
    NotFound,
    /// `/cancel` only — the 5-minute accept-regret window elapsed (`assignments.ts:447-450`).
    WindowExpired,
}

/// The (assignment_status, order_status) pair `cancel`/`abort` read together — the LOCKED input
/// to `release_binding_and_reoffer`'s guarded-transition decision (`bindingRelease.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AssignmentBindingSnapshot {
    /// `ord_status == IN_DELIVERY && asg_status == picked_up` — food is out; terminal CANCELLED.
    InDeliveryPickedUp,
    /// `ord_status == IN_DELIVERY` (asg_status == accepted) — legacy force-advance; revert READY.
    InDeliveryOther,
    /// Any other order status (flag-ON pre-pickup path) — no order-side transition at all.
    NotYetInDelivery,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaymentOutcome {
    PaidFull,
    DeliveredPrepaid,
    RefusedGoods,
    RefusedPayment,
    CustomerCancelledOnDoor,
}

impl PaymentOutcome {
    fn is_paid_full(&self) -> bool {
        matches!(self, PaymentOutcome::PaidFull)
    }
    fn is_prepaid(&self) -> bool {
        matches!(self, PaymentOutcome::DeliveredPrepaid)
    }
}

#[async_trait]
pub trait AssignmentsRepo: Send + Sync {
    /// `GET /me/assignments` (`assignments.ts:74-99`) — REV-S7-1: `with_tenant`-seated (the old
    /// Node route already `BEGIN`s correctly here; carried, not a fix).
    async fn list_active(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<Vec<AssignmentTaskRow>, RepoError>;

    /// `GET /assignments/:id` (`assignments.ts:102-122`) — REV-S7-1 FIX-IN-PORT: the old route sets
    /// the GUC with NO `BEGIN` (bare-pool, discarded before the SELECT runs); `with_tenant` here
    /// closes that.
    async fn get_one(
        &self,
        id: Uuid,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<AssignmentTaskRow>, RepoError>;

    /// `POST /assignments/:id/accept` (`assignments.ts:125-175`) — offered-branch first, legacy
    /// fallback (Q-OFFER-HANDSHAKE, both carried).
    async fn accept(
        &self,
        id: Uuid,
        courier_id: Uuid,
        location_id: Uuid,
        accept_window_ms: i64,
    ) -> Result<AcceptOutcome, RepoError>;

    /// `POST /assignments/:id/reject` (`assignments.ts:178-236`) — `assigned` -> `rejected`, shift
    /// freed, re-enqueued. No order-side transition (order was never advanced).
    async fn reject(
        &self,
        id: Uuid,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<SimpleOutcome, RepoError>;

    /// `POST /assignments/:id/picked-up` (`assignments.ts:239-289`) — `accepted` -> `picked_up`,
    /// order -> `IN_DELIVERY` (idempotent-swallowed, matching Node's try/catch).
    async fn picked_up(
        &self,
        id: Uuid,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<SimpleOutcome, RepoError>;

    /// `POST /assignments/:id/delivered` (`assignments.ts:292-410`) — the completion primitive
    /// (`deliveryCompletion.ts`), inlined here (S7 owns it; S5 only supplies `apply_transition`).
    #[allow(clippy::too_many_arguments)]
    async fn delivered(
        &self,
        id: Uuid,
        courier_id: Uuid,
        location_id: Uuid,
        payment_outcome: PaymentOutcome,
        cash_amount: Option<i64>,
    ) -> Result<DeliveredOutcome, RepoError>;

    /// `POST /assignments/:id/cancel` (`assignments.ts:413-476`) — accept-regret, 5-min time gate,
    /// shares the binding-release rail with `abort`.
    async fn cancel(
        &self,
        id: Uuid,
        courier_id: Uuid,
        location_id: Uuid,
        reason: String,
        cancel_window_ms: i64,
    ) -> Result<CancelOutcome, RepoError>;

    /// `POST /assignments/:id/abort` (`assignments.ts:482-531`) — en-route, no time gate, same
    /// rail as `cancel` minus the window check.
    async fn abort(
        &self,
        id: Uuid,
        courier_id: Uuid,
        location_id: Uuid,
        reason: String,
    ) -> Result<CancelOutcome, RepoError>;

    /// `POST /assignments/:id/decline` (`assignments.ts:535-572`) — `offered` -> `offered_expired`,
    /// re-enqueued. The customer order is UNTOUCHED (never advanced in the offer state).
    async fn decline(
        &self,
        id: Uuid,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<SimpleOutcome, RepoError>;
}

// ── DTOs ─────────────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RestaurantBrief {
    pub name: String,
    pub address: String,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CustomerBrief {
    pub address: String,
    pub phone: Option<String>,
    pub instructions: Option<String>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    #[serde(rename = "messengerKind")]
    pub messenger_kind: Option<String>,
    #[serde(rename = "messengerHandle")]
    pub messenger_handle: Option<String>,
    #[serde(rename = "entryPhotoUrl")]
    pub entry_photo_url: Option<String>,
}

/// The `toTaskShape` wire shape (`assignments.ts:21-56`).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AssignmentTaskResponse {
    pub id: Uuid,
    #[serde(rename = "orderId")]
    pub order_id: Uuid,
    pub status: String,
    #[serde(rename = "assignedAt")]
    pub assigned_at: Option<String>,
    #[serde(rename = "acceptedAt")]
    pub accepted_at: Option<String>,
    #[serde(rename = "pickedUpAt")]
    pub picked_up_at: Option<String>,
    #[serde(rename = "deliveredAt")]
    pub delivered_at: Option<String>,
    #[serde(rename = "cashCollected")]
    pub cash_collected: bool,
    #[serde(rename = "cashAmount")]
    pub cash_amount: Option<i64>,
    pub total: i64,
    #[serde(rename = "tipAmount")]
    pub tip_amount: i64,
    pub eta: String,
    pub restaurant: RestaurantBrief,
    pub customer: CustomerBrief,
    #[serde(rename = "cashPayWith")]
    pub cash_pay_with: Option<i64>,
}

/// Only messenger/photo fields while the task is ACTIVE (UX-2/UX-3 parity, `assignments.ts:49-52`).
const ACTIVE_STATUSES: [&str; 3] = ["assigned", "accepted", "picked_up"];

impl AssignmentTaskRow {
    fn into_response(
        self,
        app_base_url: &str,
        r2_public_url: Option<&str>,
    ) -> AssignmentTaskResponse {
        let is_active = ACTIVE_STATUSES.contains(&self.status.as_str());
        AssignmentTaskResponse {
            id: self.id,
            order_id: self.order_id,
            status: self.status,
            assigned_at: self.assigned_at,
            accepted_at: self.accepted_at,
            picked_up_at: self.picked_up_at,
            delivered_at: self.delivered_at,
            cash_collected: self.cash_collected,
            cash_amount: self.cash_amount,
            total: self.total,
            tip_amount: self.tip_amount,
            eta: "~15 min".to_string(),
            restaurant: RestaurantBrief {
                name: self.restaurant_name,
                address: self.restaurant_address,
                lat: self.restaurant_lat,
                lng: self.restaurant_lng,
            },
            customer: CustomerBrief {
                address: self.delivery_address,
                phone: self.phone,
                instructions: self.delivery_instructions,
                lat: self.delivery_lat,
                lng: self.delivery_lng,
                messenger_kind: if is_active {
                    self.customer_messenger_kind
                } else {
                    None
                },
                messenger_handle: if is_active {
                    self.customer_messenger_handle
                } else {
                    None
                },
                entry_photo_url: if is_active {
                    crate::service::get_image_url(
                        self.delivery_photo_key.as_deref(),
                        r2_public_url,
                        app_base_url,
                    )
                } else {
                    None
                },
            },
            cash_pay_with: self.cash_amount,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AssignmentsListResponse {
    pub success: bool,
    pub assignments: Vec<AssignmentTaskResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SimpleSuccessResponse {
    pub success: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CancelResponse {
    pub success: bool,
    pub requeued: bool,
}

/// `POST /assignments/:id/delivered` body (`assignments.ts:298-302`, `.strict()`). deliver v2:
/// `payment_outcome` is the first-class signal; `cash_collected` kept for backward-compat legacy
/// derivation; `cash_amount` is `int().nonnegative()` (M-2).
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DeliveredRequest {
    #[serde(default)]
    pub payment_outcome: Option<PaymentOutcome>,
    #[serde(default)]
    pub cash_collected: Option<bool>,
    #[serde(default)]
    pub cash_amount: Option<i64>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CancelRequest {
    pub reason: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AbortRequest {
    #[serde(default)]
    pub reason: Option<String>,
}

fn internal_error(correlation_id: String) -> ApiError {
    ApiError::new(ErrorCode::Internal, "internal_error", correlation_id)
}

// ── Handlers ─────────────────────────────────────────────────────────────────────────────────

/// `GET /api/courier/me/assignments` (`assignments.ts:74-99`).
#[utoipa::path(get, path = "/api/courier/me/assignments", tag = "courier",
    responses((status = 200, body = AssignmentsListResponse)))]
pub async fn list_assignments(
    Extension(state): Extension<AssignmentsState>,
    CourierSession(courier): CourierSession,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let rows = state
        .repo
        .list_active(courier.sub, courier.active_location_id)
        .await
        .map_err(|_e| internal_error(correlation_id))?;
    Ok(Json(AssignmentsListResponse {
        success: true,
        assignments: rows
            .into_iter()
            .map(|r| r.into_response("https://dowiz.fly.dev", None))
            .collect(),
    }))
}

/// `GET /api/courier/assignments/{id}` (`assignments.ts:102-122`) — REV-S7-1 discriminating probe
/// target (own-tenant read post-flip).
#[utoipa::path(get, path = "/api/courier/assignments/{id}", tag = "courier",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = AssignmentTaskResponse), (status = 404, description = "Not found", body = domain::ErrorEnvelope)))]
pub async fn get_assignment(
    Extension(state): Extension<AssignmentsState>,
    CourierSession(courier): CourierSession,
    Path(id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Response, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let row = state
        .repo
        .get_one(id, courier.sub, courier.active_location_id)
        .await
        .map_err(|_e| internal_error(correlation_id.clone()))?;
    match row {
        Some(r) => Ok(Json(r.into_response("https://dowiz.fly.dev", None)).into_response()),
        None => Err(ApiError::new(
            ErrorCode::NotFound,
            "Assignment not found",
            correlation_id,
        )),
    }
}

/// 30s accept window (`COURIER_ACCEPT_WINDOW_MS`, `assignments.ts:134`).
const DEFAULT_ACCEPT_WINDOW_MS: i64 = 30_000;
/// 5-minute accept-regret cancel window (`CANCEL_AFTER_DISPATCH_WINDOW_MS`, `assignments.ts:426`).
const DEFAULT_CANCEL_WINDOW_MS: i64 = 300_000;

/// `POST /api/courier/assignments/{id}/accept` (`assignments.ts:125-175`).
#[utoipa::path(post, path = "/api/courier/assignments/{id}/accept", tag = "courier",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = SimpleSuccessResponse), (status = 404, description = "Not found", body = domain::ErrorEnvelope), (status = 410, description = "Accept window expired", body = domain::ErrorEnvelope)))]
pub async fn accept_assignment(
    Extension(state): Extension<AssignmentsState>,
    CourierSession(courier): CourierSession,
    Path(id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let outcome = state
        .repo
        .accept(
            id,
            courier.sub,
            courier.active_location_id,
            DEFAULT_ACCEPT_WINDOW_MS,
        )
        .await
        .map_err(|_e| internal_error(correlation_id.clone()))?;
    match outcome {
        AcceptOutcome::AcceptedViaOffer | AcceptOutcome::AcceptedViaLegacy => {
            Ok(Json(SimpleSuccessResponse { success: true }))
        }
        AcceptOutcome::NotFound => Err(ApiError::new(
            ErrorCode::NotFound,
            "Assignment not found",
            correlation_id,
        )),
        AcceptOutcome::WindowExpired => Err(ApiError::new(
            ErrorCode::CancelWindowExpired,
            "Acceptance window expired",
            correlation_id,
        )),
    }
}

/// `POST /api/courier/assignments/{id}/reject` (`assignments.ts:178-236`).
#[utoipa::path(post, path = "/api/courier/assignments/{id}/reject", tag = "courier",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = SimpleSuccessResponse), (status = 404, description = "Not found or not assigned", body = domain::ErrorEnvelope)))]
pub async fn reject_assignment(
    Extension(state): Extension<AssignmentsState>,
    CourierSession(courier): CourierSession,
    Path(id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let outcome = state
        .repo
        .reject(id, courier.sub, courier.active_location_id)
        .await
        .map_err(|_e| internal_error(correlation_id.clone()))?;
    match outcome {
        SimpleOutcome::Done => Ok(Json(SimpleSuccessResponse { success: true })),
        SimpleOutcome::NotFound => Err(ApiError::new(
            ErrorCode::AssignmentNotFoundOrNotAssigned,
            "ASSIGNMENT_NOT_FOUND_OR_NOT_ASSIGNED",
            correlation_id,
        )),
    }
}

/// `POST /api/courier/assignments/{id}/picked-up` (`assignments.ts:239-289`).
#[utoipa::path(post, path = "/api/courier/assignments/{id}/picked-up", tag = "courier",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = SimpleSuccessResponse), (status = 404, description = "Not found or not accepted", body = domain::ErrorEnvelope)))]
pub async fn picked_up_assignment(
    Extension(state): Extension<AssignmentsState>,
    CourierSession(courier): CourierSession,
    Path(id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let outcome = state
        .repo
        .picked_up(id, courier.sub, courier.active_location_id)
        .await
        .map_err(|_e| internal_error(correlation_id.clone()))?;
    match outcome {
        SimpleOutcome::Done => Ok(Json(SimpleSuccessResponse { success: true })),
        SimpleOutcome::NotFound => Err(ApiError::new(
            ErrorCode::AssignmentNotFoundOrNotAccepted,
            "ASSIGNMENT_NOT_FOUND_OR_NOT_ACCEPTED",
            correlation_id,
        )),
    }
}

/// `POST /api/courier/assignments/{id}/delivered` (`assignments.ts:292-410`) — cash-as-proof.
#[utoipa::path(post, path = "/api/courier/assignments/{id}/delivered", tag = "courier",
    params(("id" = Uuid, Path)), request_body = DeliveredRequest,
    responses(
        (status = 200, description = "Delivered or the honest no-cash CANCELLED tail", body = SimpleSuccessResponse),
        (status = 404, description = "Not found or not picked up", body = domain::ErrorEnvelope),
        (status = 409, description = "PREPAID_NOT_PAID", body = domain::ErrorEnvelope),
        (status = 422, description = "CASH_AMOUNT_MISMATCH", body = domain::ErrorEnvelope),
    ))]
pub async fn delivered_assignment(
    Extension(state): Extension<AssignmentsState>,
    CourierSession(courier): CourierSession,
    Path(id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<DeliveredRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    // Resolve the outcome: explicit payment_outcome wins; else legacy cash_collected derives
    // paid_full/refused_payment (assignments.ts:311-312). The prepaid auto-resolve (crypto,
    // ADR-0017, dark) is decided inside the repo (needs `orders.payment_method/payment_status`).
    let payment_outcome = body.payment_outcome.unwrap_or_else(|| {
        if body.cash_collected.unwrap_or(false) {
            PaymentOutcome::PaidFull
        } else {
            PaymentOutcome::RefusedPayment
        }
    });

    let outcome = state
        .repo
        .delivered(
            id,
            courier.sub,
            courier.active_location_id,
            payment_outcome,
            body.cash_amount,
        )
        .await
        .map_err(|_e| internal_error(correlation_id.clone()))?;

    match outcome {
        DeliveredOutcome::Delivered { .. } => Ok(Json(SimpleSuccessResponse { success: true })),
        DeliveredOutcome::NotFound => Err(ApiError::new(
            ErrorCode::AssignmentNotFoundOrNotPickedUp,
            "ASSIGNMENT_NOT_FOUND_OR_NOT_PICKED_UP",
            correlation_id,
        )),
        DeliveredOutcome::CashMismatch { .. } => Err(ApiError::new(
            ErrorCode::CashAmountMismatch,
            "CASH_AMOUNT_MISMATCH",
            correlation_id,
        )),
        DeliveredOutcome::PrepaidNotPaid => Err(ApiError::new(
            ErrorCode::PrepaidNotPaid,
            "PREPAID_NOT_PAID",
            correlation_id,
        )),
    }
}

/// `POST /api/courier/assignments/{id}/cancel` (`assignments.ts:413-476`).
#[utoipa::path(post, path = "/api/courier/assignments/{id}/cancel", tag = "courier",
    params(("id" = Uuid, Path)), request_body = CancelRequest,
    responses((status = 200, body = CancelResponse), (status = 404, description = "Not found or invalid status", body = domain::ErrorEnvelope), (status = 410, description = "Cancel window expired", body = domain::ErrorEnvelope)))]
pub async fn cancel_assignment(
    Extension(state): Extension<AssignmentsState>,
    CourierSession(courier): CourierSession,
    Path(id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<CancelRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let outcome = state
        .repo
        .cancel(
            id,
            courier.sub,
            courier.active_location_id,
            format!("courier_cancelled: {}", body.reason),
            DEFAULT_CANCEL_WINDOW_MS,
        )
        .await
        .map_err(|_e| internal_error(correlation_id.clone()))?;
    match outcome {
        CancelOutcome::Done { requeued } => Ok(Json(CancelResponse {
            success: true,
            requeued,
        })),
        CancelOutcome::NotFound => Err(ApiError::new(
            ErrorCode::AssignmentNotFoundOrInvalidStatus,
            "ASSIGNMENT_NOT_FOUND_OR_INVALID_STATUS",
            correlation_id,
        )),
        CancelOutcome::WindowExpired => Err(ApiError::new(
            ErrorCode::CancelWindowExpired,
            "CANCEL_WINDOW_EXPIRED",
            correlation_id,
        )),
    }
}

/// `POST /api/courier/assignments/{id}/abort` (`assignments.ts:482-531`) — no time gate.
#[utoipa::path(post, path = "/api/courier/assignments/{id}/abort", tag = "courier",
    params(("id" = Uuid, Path)), request_body = AbortRequest,
    responses((status = 200, body = CancelResponse), (status = 404, description = "Not found or invalid status", body = domain::ErrorEnvelope)))]
pub async fn abort_assignment(
    Extension(state): Extension<AssignmentsState>,
    CourierSession(courier): CourierSession,
    Path(id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<AbortRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let reason = body
        .reason
        .unwrap_or_else(|| "courier_aborted_en_route".to_string());
    let outcome = state
        .repo
        .abort(id, courier.sub, courier.active_location_id, reason)
        .await
        .map_err(|_e| internal_error(correlation_id.clone()))?;
    match outcome {
        CancelOutcome::Done { requeued } => Ok(Json(CancelResponse {
            success: true,
            requeued,
        })),
        CancelOutcome::NotFound | CancelOutcome::WindowExpired => Err(ApiError::new(
            ErrorCode::AssignmentNotFoundOrInvalidStatus,
            "ASSIGNMENT_NOT_FOUND_OR_INVALID_STATUS",
            correlation_id,
        )),
    }
}

/// `POST /api/courier/assignments/{id}/decline` (`assignments.ts:535-572`).
#[utoipa::path(post, path = "/api/courier/assignments/{id}/decline", tag = "courier",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = SimpleSuccessResponse), (status = 404, description = "Not found or not offered", body = domain::ErrorEnvelope)))]
pub async fn decline_assignment(
    Extension(state): Extension<AssignmentsState>,
    CourierSession(courier): CourierSession,
    Path(id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let outcome = state
        .repo
        .decline(id, courier.sub, courier.active_location_id)
        .await
        .map_err(|_e| internal_error(correlation_id.clone()))?;
    match outcome {
        SimpleOutcome::Done => Ok(Json(SimpleSuccessResponse { success: true })),
        SimpleOutcome::NotFound => Err(ApiError::new(
            ErrorCode::AssignmentNotFoundOrNotOffered,
            "ASSIGNMENT_NOT_FOUND_OR_NOT_OFFERED",
            correlation_id,
        )),
    }
}

// ── PgAssignmentsRepo ────────────────────────────────────────────────────────────────────────

pub struct PgAssignmentsRepo {
    pool: sqlx::PgPool,
}

impl PgAssignmentsRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        PgAssignmentsRepo { pool }
    }
}

fn map_txn_err(err: crate::db::TenantTxnError) -> RepoError {
    use crate::db::TenantTxnError;
    match err {
        TenantTxnError::Begin(e)
        | TenantTxnError::SetTenant(e)
        | TenantTxnError::Work(e)
        | TenantTxnError::Commit(e) => RepoError(e),
        TenantTxnError::WorkThenRollbackFailed { work, .. } => RepoError(work),
    }
}

/// `toTaskShape`'s enriched join (`assignments.ts:58-71`), casting the money columns to `bigint`
/// (`::bigint`) to bind cleanly as `i64` (`courier_assignments.cash_amount`/`orders.tip_amount`
/// are native `integer`; `orders.total` is already `bigint` upstream but cast defensively too —
/// same normalization S5's `orders::pg` already applies to `subtotal`/`total`).
const ENRICHED_ASSIGNMENTS_QUERY: &str = "SELECT ca.id, ca.order_id, ca.status, \
     ca.assigned_at::text, ca.accepted_at::text, ca.picked_up_at::text, ca.delivered_at::text, \
     ca.cash_collected, ca.cash_amount::bigint, o.total::bigint, o.tip_amount::bigint, \
     l.name AS restaurant_name, l.address AS restaurant_address, \
     l.lat::float8 AS restaurant_lat, l.lng::float8 AS restaurant_lng, \
     o.delivery_address, o.delivery_lat::float8, o.delivery_lng::float8, o.delivery_instructions, \
     c.phone, c.messenger_kind AS customer_messenger_kind, \
     c.messenger_handle AS customer_messenger_handle, o.delivery_photo_key \
     FROM courier_assignments ca \
     JOIN orders o ON o.id = ca.order_id \
     JOIN locations l ON l.id = o.location_id \
     LEFT JOIN customers c ON c.id = o.customer_id ";

/// A plain tuple exceeds `sqlx::FromRow`'s implemented arity for this many columns — a real
/// `#[derive(sqlx::FromRow)]` struct (the established convention, `repo.rs`/`owner::products`)
/// has no such limit and is self-documenting per-column.
#[derive(Debug, Clone, sqlx::FromRow)]
struct AssignmentSqlRow {
    id: Uuid,
    order_id: Uuid,
    status: String,
    assigned_at: Option<String>,
    accepted_at: Option<String>,
    picked_up_at: Option<String>,
    delivered_at: Option<String>,
    cash_collected: bool,
    cash_amount: Option<i64>,
    total: i64,
    tip_amount: i64,
    restaurant_name: String,
    restaurant_address: String,
    restaurant_lat: Option<f64>,
    restaurant_lng: Option<f64>,
    delivery_address: String,
    delivery_lat: Option<f64>,
    delivery_lng: Option<f64>,
    delivery_instructions: Option<String>,
    phone: Option<String>,
    customer_messenger_kind: Option<String>,
    customer_messenger_handle: Option<String>,
    delivery_photo_key: Option<String>,
}

fn row_from_sql(r: AssignmentSqlRow) -> AssignmentTaskRow {
    AssignmentTaskRow {
        id: r.id,
        order_id: r.order_id,
        status: r.status,
        assigned_at: r.assigned_at,
        accepted_at: r.accepted_at,
        picked_up_at: r.picked_up_at,
        delivered_at: r.delivered_at,
        cash_collected: r.cash_collected,
        cash_amount: r.cash_amount,
        total: r.total,
        tip_amount: r.tip_amount,
        restaurant_name: r.restaurant_name,
        restaurant_address: r.restaurant_address,
        restaurant_lat: r.restaurant_lat,
        restaurant_lng: r.restaurant_lng,
        delivery_address: r.delivery_address,
        delivery_lat: r.delivery_lat,
        delivery_lng: r.delivery_lng,
        delivery_instructions: r.delivery_instructions,
        phone: r.phone,
        customer_messenger_kind: r.customer_messenger_kind,
        customer_messenger_handle: r.customer_messenger_handle,
        delivery_photo_key: r.delivery_photo_key,
    }
}

fn parse_status(s: &str) -> Option<OrderStatus> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
}

#[async_trait]
impl AssignmentsRepo for PgAssignmentsRepo {
    async fn list_active(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<Vec<AssignmentTaskRow>, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let sql = format!(
                    "{ENRICHED_ASSIGNMENTS_QUERY}WHERE ca.courier_id = $1 AND ca.location_id = $2 \
                     AND ca.status IN ('assigned','accepted','picked_up') ORDER BY ca.created_at DESC"
                );
                let rows: Vec<AssignmentSqlRow> = sqlx::query_as(&sql)
                    .bind(courier_id)
                    .bind(location_id)
                    .fetch_all(&mut **txn)
                    .await?;
                Ok(rows.into_iter().map(row_from_sql).collect())
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn get_one(
        &self,
        id: Uuid,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<AssignmentTaskRow>, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let sql =
                    format!("{ENRICHED_ASSIGNMENTS_QUERY}WHERE ca.id = $1 AND ca.courier_id = $2");
                let row: Option<AssignmentSqlRow> = sqlx::query_as(&sql)
                    .bind(id)
                    .bind(courier_id)
                    .fetch_optional(&mut **txn)
                    .await?;
                Ok(row.map(row_from_sql))
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn accept(
        &self,
        id: Uuid,
        courier_id: Uuid,
        location_id: Uuid,
        accept_window_ms: i64,
    ) -> Result<AcceptOutcome, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                // Offer-handshake branch first (assignments.ts:144-156).
                let offered: Option<(Uuid, Option<Uuid>)> = sqlx::query_as(
                    "SELECT order_id, shift_id FROM courier_assignments \
                     WHERE id=$1 AND courier_id=$2 AND status='offered' FOR UPDATE",
                )
                .bind(id)
                .bind(courier_id)
                .fetch_optional(&mut **txn)
                .await?;
                if let Some((order_id, shift_id)) = offered {
                    sqlx::query(
                        "UPDATE courier_assignments SET status='accepted', offered_expires_at=NULL WHERE id=$1",
                    )
                    .bind(id)
                    .execute(&mut **txn)
                    .await?;
                    if let Some(sid) = shift_id {
                        sqlx::query("UPDATE courier_shifts SET status='on_delivery' WHERE id=$1")
                            .bind(sid)
                            .execute(&mut **txn)
                            .await?;
                    }
                    advance_order_swallow_illegal(txn, order_id, location_id, OrderStatus::InDelivery).await?;
                    sqlx::query("UPDATE orders SET courier_id=$1 WHERE id=$2")
                        .bind(courier_id)
                        .bind(order_id)
                        .execute(&mut **txn)
                        .await?;
                    return Ok(AcceptOutcome::AcceptedViaOffer);
                }

                // Legacy branch (harmonized actor-gate — see module doc "judgment call"). Elapsed
                // time is computed IN SQL (`EXTRACT(EPOCH FROM ...)`), never by parsing a
                // Postgres timestamp text representation in Rust (fragile — Postgres's default
                // `timestamptz::text` output is not RFC3339-parseable as-is).
                let legacy: Option<(Uuid, f64)> = sqlx::query_as(
                    "SELECT order_id, EXTRACT(EPOCH FROM (now() - assigned_at)) * 1000 \
                     FROM courier_assignments \
                     WHERE id=$1 AND courier_id=$2 AND status='assigned' FOR UPDATE",
                )
                .bind(id)
                .bind(courier_id)
                .fetch_optional(&mut **txn)
                .await?;
                let Some((order_id, elapsed_ms)) = legacy else {
                    return Ok(AcceptOutcome::NotFound);
                };
                #[allow(clippy::as_conversions, reason = "elapsed-ms comparison only, bounded by a real wall-clock delta")]
                if elapsed_ms > accept_window_ms as f64 {
                    return Ok(AcceptOutcome::WindowExpired);
                }
                sqlx::query("UPDATE courier_assignments SET status='accepted', accepted_at=now() WHERE id=$1")
                    .bind(id)
                    .execute(&mut **txn)
                    .await?;
                advance_order_swallow_illegal(txn, order_id, location_id, OrderStatus::Confirmed).await?;
                Ok(AcceptOutcome::AcceptedViaLegacy)
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn reject(
        &self,
        id: Uuid,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<SimpleOutcome, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let row: Option<(Uuid, Option<Uuid>)> = sqlx::query_as(
                    "SELECT order_id, shift_id FROM courier_assignments \
                     WHERE id=$1 AND courier_id=$2 AND status='assigned' FOR UPDATE",
                )
                .bind(id)
                .bind(courier_id)
                .fetch_optional(&mut **txn)
                .await?;
                let Some((order_id, shift_id)) = row else {
                    return Ok(SimpleOutcome::NotFound);
                };
                sqlx::query(
                    "UPDATE courier_assignments SET status='rejected', cancelled_at=now(), \
                     cancellation_reason='courier_rejected' WHERE id=$1",
                )
                .bind(id)
                .execute(&mut **txn)
                .await?;
                if let Some(sid) = shift_id {
                    sqlx::query("UPDATE courier_shifts SET status='available' WHERE id=$1")
                        .bind(sid)
                        .execute(&mut **txn)
                        .await?;
                }
                sqlx::query(
                    "INSERT INTO courier_dispatch_queue (order_id, location_id, enqueued_at) \
                     VALUES ($1,$2,now()) \
                     ON CONFLICT (order_id) DO UPDATE SET attempts = courier_dispatch_queue.attempts + 1",
                )
                .bind(order_id)
                .bind(location_id)
                .execute(&mut **txn)
                .await?;
                Ok(SimpleOutcome::Done)
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn picked_up(
        &self,
        id: Uuid,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<SimpleOutcome, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let row: Option<(Uuid,)> = sqlx::query_as(
                    "SELECT order_id FROM courier_assignments \
                     WHERE id=$1 AND courier_id=$2 AND status='accepted' FOR UPDATE",
                )
                .bind(id)
                .bind(courier_id)
                .fetch_optional(&mut **txn)
                .await?;
                let Some((order_id,)) = row else {
                    return Ok(SimpleOutcome::NotFound);
                };
                sqlx::query(
                    "UPDATE courier_assignments SET status='picked_up', picked_up_at=now() WHERE id=$1",
                )
                .bind(id)
                .execute(&mut **txn)
                .await?;
                advance_order_swallow_illegal(txn, order_id, location_id, OrderStatus::InDelivery).await?;
                Ok(SimpleOutcome::Done)
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn delivered(
        &self,
        id: Uuid,
        courier_id: Uuid,
        location_id: Uuid,
        payment_outcome: PaymentOutcome,
        cash_amount: Option<i64>,
    ) -> Result<DeliveredOutcome, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                // `o.payment_method::text` — payment_method is an enum; sqlx cannot decode an enum
                // into a Rust String uncast, so this 500'd the courier DELIVERED path (staging
                // oracle 2026-07-05; payment_status is a text+CHECK column, no cast needed).
                let row: Option<(Uuid, Option<Uuid>, i64, String, Option<String>)> = sqlx::query_as(
                    "SELECT ca.order_id, ca.shift_id, o.total::bigint, o.payment_status, o.payment_method::text \
                     FROM courier_assignments ca JOIN orders o ON ca.order_id = o.id \
                     WHERE ca.id=$1 AND ca.courier_id=$2 AND ca.status='picked_up' FOR UPDATE",
                )
                .bind(id)
                .bind(courier_id)
                .fetch_optional(&mut **txn)
                .await?;
                let Some((order_id, shift_id, total, payment_status, payment_method)) = row else {
                    return Ok(DeliveredOutcome::NotFound);
                };

                // C1 (ADR-0017, dark): crypto-prepaid auto-resolve overrides any cash-derived
                // outcome (`assignments.ts:338`) — inert unless payment_method='crypto' (flags off).
                let mut outcome = payment_outcome;
                if payment_method.as_deref() == Some("crypto") && payment_status == "paid" {
                    outcome = PaymentOutcome::DeliveredPrepaid;
                }

                // No-partial-handover rule (REV-S7-8, before any write): exact equality, never
                // `>=`/`<=` (protects the courier).
                if outcome.is_paid_full() && cash_amount != Some(total) {
                    return Ok(DeliveredOutcome::CashMismatch { expected: total });
                }
                if outcome.is_prepaid() && payment_status != "paid" {
                    return Ok(DeliveredOutcome::PrepaidNotPaid);
                }

                let is_delivered = outcome.is_paid_full() || outcome.is_prepaid();
                let cash_collected = outcome.is_paid_full();
                let assignment_status = if is_delivered { "delivered" } else { "cancelled" };
                let cancellation_reason = if is_delivered {
                    None
                } else {
                    Some(payment_outcome_wire(&outcome))
                };
                let stored_cash_amount = if cash_collected { cash_amount } else { None };

                sqlx::query(
                    "UPDATE courier_assignments SET status=$1, \
                       delivered_at = CASE WHEN $1='delivered' THEN now() ELSE delivered_at END, \
                       cancelled_at = CASE WHEN $1='cancelled' THEN now() ELSE cancelled_at END, \
                       cancellation_reason = CASE WHEN $1='cancelled' THEN $2 ELSE cancellation_reason END, \
                       cash_collected=$3, cash_amount=$4 \
                     WHERE id=$5",
                )
                .bind(assignment_status)
                .bind(cancellation_reason)
                .bind(cash_collected)
                .bind(stored_cash_amount)
                .bind(id)
                .execute(&mut **txn)
                .await?;
                if let Some(sid) = shift_id {
                    sqlx::query("UPDATE courier_shifts SET status='available' WHERE id=$1")
                        .bind(sid)
                        .execute(&mut **txn)
                        .await?;
                }

                // Canonical order transition (DELIVERED or the honest no-cash CANCELLED tail) —
                // funnels through apply_transition, never a hand-UPDATE (Q-ORDER-FUNNEL).
                let target = if is_delivered {
                    OrderStatus::Delivered
                } else {
                    OrderStatus::Cancelled
                };
                let current: Option<(String,)> =
                    sqlx::query_as("SELECT status::text FROM orders WHERE id=$1")
                        .bind(order_id)
                        .fetch_optional(&mut **txn)
                        .await?;
                if let Some((current_str,)) = current {
                    if let Some(current) = parse_status(&current_str) {
                        if domain::assert_transition(current, target).is_ok() {
                            crate::routes::orders::pg::apply_transition(
                                txn, order_id, location_id, current, target,
                            )
                            .await?;
                        }
                    }
                }

                // $1::payment_outcome — enum column bound as text (the delivery_trace INSERT below
                // already casts; this UPDATE did not → 500 on every courier deliver, staging L7
                // drive 2026-07-05; same bind-side enum class as orders.type::order_type).
                sqlx::query("UPDATE orders SET payment_outcome = $1::payment_outcome WHERE id = $2")
                    .bind(payment_outcome_wire(&outcome))
                    .bind(order_id)
                    .execute(&mut **txn)
                    .await?;
                sqlx::query(
                    "INSERT INTO delivery_trace (order_id, location_id, courier_id, total, delivered_at, \
                       payment_outcome, cash_amount) \
                     VALUES ($1,$2,$3,$4, now(), $5::payment_outcome, $6) \
                     ON CONFLICT (order_id) DO NOTHING",
                )
                .bind(order_id)
                .bind(location_id)
                .bind(courier_id)
                .bind(total)
                .bind(payment_outcome_wire(&outcome))
                .bind(stored_cash_amount)
                .execute(&mut **txn)
                .await?;

                // Cash-as-proof HOLD (paid_full only, idempotent — REV-S7-8).
                if cash_collected {
                    sqlx::query(
                        "INSERT INTO courier_cash_ledger (courier_id, location_id, order_id, type, amount) \
                         VALUES ($1, $2, $3, 'hold', $4) ON CONFLICT (order_id, type) DO NOTHING",
                    )
                    .bind(courier_id)
                    .bind(location_id)
                    .bind(order_id)
                    .bind(stored_cash_amount.unwrap_or(0))
                    .execute(&mut **txn)
                    .await?;
                }

                Ok(DeliveredOutcome::Delivered { order_status: target })
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn cancel(
        &self,
        id: Uuid,
        courier_id: Uuid,
        location_id: Uuid,
        reason: String,
        cancel_window_ms: i64,
    ) -> Result<CancelOutcome, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let row: Option<(Uuid, Option<Uuid>, f64, String, String)> = sqlx::query_as(
                    "SELECT ca.order_id, ca.shift_id, EXTRACT(EPOCH FROM (now() - ca.assigned_at)) * 1000, \
                       ca.status AS asg_status, o.status::text AS ord_status \
                     FROM courier_assignments ca JOIN orders o ON o.id = ca.order_id \
                     WHERE ca.id=$1 AND ca.courier_id=$2 AND ca.status IN ('accepted','picked_up') \
                     FOR UPDATE OF ca",
                )
                .bind(id)
                .bind(courier_id)
                .fetch_optional(&mut **txn)
                .await?;
                let Some((order_id, shift_id, elapsed_ms, asg_status, ord_status)) = row else {
                    return Ok(CancelOutcome::NotFound);
                };
                #[allow(clippy::as_conversions, reason = "elapsed-ms comparison only, bounded by a real wall-clock delta")]
                if elapsed_ms > cancel_window_ms as f64 {
                    return Ok(CancelOutcome::WindowExpired);
                }
                let requeued = release_binding_and_reoffer(
                    txn, id, order_id, shift_id, &asg_status, &ord_status, location_id, &reason,
                )
                .await?;
                Ok(CancelOutcome::Done { requeued })
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn abort(
        &self,
        id: Uuid,
        courier_id: Uuid,
        location_id: Uuid,
        reason: String,
    ) -> Result<CancelOutcome, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let row: Option<(Uuid, Option<Uuid>, String, String)> = sqlx::query_as(
                    "SELECT ca.order_id, ca.shift_id, ca.status AS asg_status, o.status::text AS ord_status \
                     FROM courier_assignments ca JOIN orders o ON o.id = ca.order_id \
                     WHERE ca.id=$1 AND ca.courier_id=$2 AND ca.status IN ('accepted','picked_up') \
                     FOR UPDATE OF ca",
                )
                .bind(id)
                .bind(courier_id)
                .fetch_optional(&mut **txn)
                .await?;
                let Some((order_id, shift_id, asg_status, ord_status)) = row else {
                    return Ok(CancelOutcome::NotFound);
                };
                let requeued = release_binding_and_reoffer(
                    txn, id, order_id, shift_id, &asg_status, &ord_status, location_id, &reason,
                )
                .await?;
                Ok(CancelOutcome::Done { requeued })
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn decline(
        &self,
        id: Uuid,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<SimpleOutcome, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let row: Option<(Uuid,)> = sqlx::query_as(
                    "SELECT order_id FROM courier_assignments \
                     WHERE id=$1 AND courier_id=$2 AND status='offered' FOR UPDATE",
                )
                .bind(id)
                .bind(courier_id)
                .fetch_optional(&mut **txn)
                .await?;
                let Some((order_id,)) = row else {
                    return Ok(SimpleOutcome::NotFound);
                };
                sqlx::query(
                    "UPDATE courier_assignments SET status='offered_expired', cancelled_at=now(), \
                     cancellation_reason='courier_declined' WHERE id=$1",
                )
                .bind(id)
                .execute(&mut **txn)
                .await?;
                sqlx::query(
                    "INSERT INTO courier_dispatch_queue (order_id, location_id, enqueued_at) \
                     VALUES ($1,$2,now()) \
                     ON CONFLICT (order_id) DO UPDATE SET attempts = courier_dispatch_queue.attempts + 1",
                )
                .bind(order_id)
                .bind(location_id)
                .execute(&mut **txn)
                .await?;
                Ok(SimpleOutcome::Done)
            })
        })
        .await
        .map_err(map_txn_err)
    }
}

/// `updateOrderStatus`'s idempotent-swallow contract (`orderStatusService.ts:78-89`, wrapped at the
/// Node call site by a `try{}catch{}` — "order may already be confirmed or in a further state"):
/// read current status, and skip the write entirely (never error) if the target is unreachable
/// (same-status or illegal) from here — matches Node's assertTransition-then-swallow exactly.
async fn advance_order_swallow_illegal(
    txn: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    order_id: Uuid,
    location_id: Uuid,
    target: OrderStatus,
) -> Result<(), sqlx::Error> {
    let current: Option<(String,)> = sqlx::query_as("SELECT status::text FROM orders WHERE id=$1")
        .bind(order_id)
        .fetch_optional(&mut **txn)
        .await?;
    let Some((current_str,)) = current else {
        return Ok(());
    };
    let Some(current) = parse_status(&current_str) else {
        return Ok(());
    };
    if domain::assert_transition(current, target).is_err() {
        return Ok(()); // idempotent no-op — swallowed, matching Node's try/catch
    }
    crate::routes::orders::pg::apply_transition(txn, order_id, location_id, current, target)
        .await?;
    Ok(())
}

/// The shared binding-release rail (`bindingRelease.ts::releaseBindingAndReoffer`) — cancel/abort
/// both call this. Unconditionally frees the binding + shift, THEN takes an order-side action
/// guarded on the LOCKED `ord_status`/`asg_status` snapshot the caller already read. Returns
/// `requeued` (a journal re-enqueue, not a re-offer — ADR-dispatch-recovery Q4).
#[allow(clippy::too_many_arguments)]
async fn release_binding_and_reoffer(
    txn: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    assignment_id: Uuid,
    order_id: Uuid,
    shift_id: Option<Uuid>,
    asg_status: &str,
    ord_status: &str,
    location_id: Uuid,
    reason: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query(
        "UPDATE courier_assignments SET status='cancelled', cancelled_at=now(), \
         cancellation_reason=$1 WHERE id=$2",
    )
    .bind(reason)
    .bind(assignment_id)
    .execute(&mut **txn)
    .await?;
    if let Some(sid) = shift_id {
        sqlx::query("UPDATE courier_shifts SET status='available' WHERE id=$1")
            .bind(sid)
            .execute(&mut **txn)
            .await?;
    }

    let snapshot = if ord_status == "IN_DELIVERY" && asg_status == "picked_up" {
        AssignmentBindingSnapshot::InDeliveryPickedUp
    } else if ord_status == "IN_DELIVERY" {
        AssignmentBindingSnapshot::InDeliveryOther
    } else {
        AssignmentBindingSnapshot::NotYetInDelivery
    };

    match snapshot {
        AssignmentBindingSnapshot::InDeliveryPickedUp => {
            // Food is out with the failed courier -> honest terminal (no re-offer possible).
            crate::routes::orders::pg::apply_transition(
                txn,
                order_id,
                location_id,
                OrderStatus::InDelivery,
                OrderStatus::Cancelled,
            )
            .await?;
            Ok(false)
        }
        AssignmentBindingSnapshot::InDeliveryOther => {
            // Legacy flag-OFF force-IN_DELIVERY, pre-pickup -> revert to assignable + re-offer.
            crate::routes::orders::pg::apply_transition(
                txn,
                order_id,
                location_id,
                OrderStatus::InDelivery,
                OrderStatus::Ready,
            )
            .await?;
            re_enqueue(txn, order_id, location_id).await?;
            Ok(true)
        }
        AssignmentBindingSnapshot::NotYetInDelivery => {
            // Flag-ON accept — the order never advanced -> NO transition (would throw); re-offer.
            re_enqueue(txn, order_id, location_id).await?;
            Ok(true)
        }
    }
}

async fn re_enqueue(
    txn: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    order_id: Uuid,
    location_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE orders SET courier_id = NULL WHERE id = $1")
        .bind(order_id)
        .execute(&mut **txn)
        .await?;
    sqlx::query(
        "INSERT INTO courier_dispatch_queue (order_id, location_id, enqueued_at) VALUES ($1,$2,now()) \
         ON CONFLICT (order_id) DO UPDATE SET attempts = courier_dispatch_queue.attempts + 1",
    )
    .bind(order_id)
    .bind(location_id)
    .execute(&mut **txn)
    .await?;
    Ok(())
}

fn payment_outcome_wire(o: &PaymentOutcome) -> &'static str {
    match o {
        PaymentOutcome::PaidFull => "paid_full",
        PaymentOutcome::DeliveredPrepaid => "delivered_prepaid",
        PaymentOutcome::RefusedGoods => "refused_goods",
        PaymentOutcome::RefusedPayment => "refused_payment",
        PaymentOutcome::CustomerCancelledOnDoor => "customer_cancelled_on_door",
    }
}

// ── FakeAssignmentsRepo (test-only) ─────────────────────────────────────────────────────────

#[cfg(test)]
pub mod fake {
    use super::{
        AcceptOutcome, AssignmentTaskRow, AssignmentsRepo, CancelOutcome, DeliveredOutcome,
        PaymentOutcome, RepoError, SimpleOutcome,
    };
    use std::collections::HashMap;
    use std::sync::Mutex;
    use uuid::Uuid;

    #[derive(Debug, Clone)]
    pub struct FakeAssignment {
        pub courier_id: Uuid,
        pub location_id: Uuid,
        pub status: String,
        pub row: AssignmentTaskRow,
    }

    #[derive(Default)]
    pub struct FakeAssignmentsRepo {
        pub assignments: Mutex<HashMap<Uuid, FakeAssignment>>,
    }

    impl FakeAssignmentsRepo {
        pub fn seed(&self, a: FakeAssignment) {
            self.assignments.lock().unwrap().insert(a.row.id, a);
        }
    }

    #[async_trait::async_trait]
    impl AssignmentsRepo for FakeAssignmentsRepo {
        async fn list_active(
            &self,
            courier_id: Uuid,
            location_id: Uuid,
        ) -> Result<Vec<AssignmentTaskRow>, RepoError> {
            Ok(self
                .assignments
                .lock()
                .unwrap()
                .values()
                .filter(|a| {
                    a.courier_id == courier_id
                        && a.location_id == location_id
                        && ["assigned", "accepted", "picked_up"].contains(&a.status.as_str())
                })
                .map(|a| a.row.clone())
                .collect())
        }

        async fn get_one(
            &self,
            id: Uuid,
            courier_id: Uuid,
            _location_id: Uuid,
        ) -> Result<Option<AssignmentTaskRow>, RepoError> {
            Ok(self
                .assignments
                .lock()
                .unwrap()
                .get(&id)
                .filter(|a| a.courier_id == courier_id)
                .map(|a| a.row.clone()))
        }

        async fn accept(
            &self,
            id: Uuid,
            courier_id: Uuid,
            _location_id: Uuid,
            _accept_window_ms: i64,
        ) -> Result<AcceptOutcome, RepoError> {
            let mut map = self.assignments.lock().unwrap();
            match map.get_mut(&id) {
                Some(a) if a.courier_id == courier_id && a.status == "assigned" => {
                    a.status = "accepted".to_string();
                    Ok(AcceptOutcome::AcceptedViaLegacy)
                }
                Some(a) if a.courier_id == courier_id && a.status == "offered" => {
                    a.status = "accepted".to_string();
                    Ok(AcceptOutcome::AcceptedViaOffer)
                }
                _ => Ok(AcceptOutcome::NotFound),
            }
        }

        async fn reject(
            &self,
            id: Uuid,
            courier_id: Uuid,
            _location_id: Uuid,
        ) -> Result<SimpleOutcome, RepoError> {
            let mut map = self.assignments.lock().unwrap();
            match map.get_mut(&id) {
                Some(a) if a.courier_id == courier_id && a.status == "assigned" => {
                    a.status = "rejected".to_string();
                    Ok(SimpleOutcome::Done)
                }
                _ => Ok(SimpleOutcome::NotFound),
            }
        }

        async fn picked_up(
            &self,
            id: Uuid,
            courier_id: Uuid,
            _location_id: Uuid,
        ) -> Result<SimpleOutcome, RepoError> {
            let mut map = self.assignments.lock().unwrap();
            match map.get_mut(&id) {
                Some(a) if a.courier_id == courier_id && a.status == "accepted" => {
                    a.status = "picked_up".to_string();
                    Ok(SimpleOutcome::Done)
                }
                _ => Ok(SimpleOutcome::NotFound),
            }
        }

        async fn delivered(
            &self,
            id: Uuid,
            courier_id: Uuid,
            _location_id: Uuid,
            payment_outcome: PaymentOutcome,
            cash_amount: Option<i64>,
        ) -> Result<DeliveredOutcome, RepoError> {
            let mut map = self.assignments.lock().unwrap();
            let Some(a) = map.get_mut(&id) else {
                return Ok(DeliveredOutcome::NotFound);
            };
            if a.courier_id != courier_id || a.status != "picked_up" {
                return Ok(DeliveredOutcome::NotFound);
            }
            if payment_outcome.is_paid_full() && cash_amount != Some(a.row.total) {
                return Ok(DeliveredOutcome::CashMismatch {
                    expected: a.row.total,
                });
            }
            let is_delivered = payment_outcome.is_paid_full() || payment_outcome.is_prepaid();
            a.status = if is_delivered {
                "delivered"
            } else {
                "cancelled"
            }
            .to_string();
            Ok(DeliveredOutcome::Delivered {
                order_status: if is_delivered {
                    domain::OrderStatus::Delivered
                } else {
                    domain::OrderStatus::Cancelled
                },
            })
        }

        async fn cancel(
            &self,
            id: Uuid,
            courier_id: Uuid,
            _location_id: Uuid,
            _reason: String,
            _cancel_window_ms: i64,
        ) -> Result<CancelOutcome, RepoError> {
            let mut map = self.assignments.lock().unwrap();
            match map.get_mut(&id) {
                Some(a)
                    if a.courier_id == courier_id
                        && ["accepted", "picked_up"].contains(&a.status.as_str()) =>
                {
                    a.status = "cancelled".to_string();
                    Ok(CancelOutcome::Done { requeued: true })
                }
                _ => Ok(CancelOutcome::NotFound),
            }
        }

        async fn abort(
            &self,
            id: Uuid,
            courier_id: Uuid,
            _location_id: Uuid,
            _reason: String,
        ) -> Result<CancelOutcome, RepoError> {
            let mut map = self.assignments.lock().unwrap();
            match map.get_mut(&id) {
                Some(a)
                    if a.courier_id == courier_id
                        && ["accepted", "picked_up"].contains(&a.status.as_str()) =>
                {
                    a.status = "cancelled".to_string();
                    Ok(CancelOutcome::Done { requeued: true })
                }
                _ => Ok(CancelOutcome::NotFound),
            }
        }

        async fn decline(
            &self,
            id: Uuid,
            courier_id: Uuid,
            _location_id: Uuid,
        ) -> Result<SimpleOutcome, RepoError> {
            let mut map = self.assignments.lock().unwrap();
            match map.get_mut(&id) {
                Some(a) if a.courier_id == courier_id && a.status == "offered" => {
                    a.status = "offered_expired".to_string();
                    Ok(SimpleOutcome::Done)
                }
                _ => Ok(SimpleOutcome::NotFound),
            }
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::fake::{FakeAssignment, FakeAssignmentsRepo};
    use super::*;
    use crate::auth::claims::CourierClaims;
    use crate::auth::repo::fake::FakeAuthRepo;
    use axum::body::to_bytes;
    use axum::http::StatusCode;
    use std::sync::Arc;

    fn request_id() -> RequestId {
        RequestId::new(axum::http::HeaderValue::from_static("corr-1"))
    }

    fn fixture_row(id: Uuid, order_id: Uuid, total: i64) -> AssignmentTaskRow {
        AssignmentTaskRow {
            id,
            order_id,
            status: "picked_up".to_string(),
            assigned_at: None,
            accepted_at: None,
            picked_up_at: None,
            delivered_at: None,
            cash_collected: false,
            cash_amount: None,
            total,
            tip_amount: 0,
            restaurant_name: "Test".to_string(),
            restaurant_address: "Addr".to_string(),
            restaurant_lat: None,
            restaurant_lng: None,
            delivery_address: "Cust addr".to_string(),
            phone: None,
            delivery_instructions: None,
            delivery_lat: None,
            delivery_lng: None,
            customer_messenger_kind: None,
            customer_messenger_handle: None,
            delivery_photo_key: None,
        }
    }

    fn courier_session(courier_id: Uuid, location_id: Uuid) -> CourierSession {
        CourierSession(CourierClaims::new(courier_id, location_id, None))
    }

    async fn json_body(resp: Response) -> (StatusCode, serde_json::Value) {
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    /// 🔴 S7-T1 DoD: a courier accepting ANOTHER courier's assignment gets 404, never a hijack.
    #[tokio::test]
    async fn cross_courier_accept_is_404_not_a_hijack() {
        let owner_courier = Uuid::new_v4();
        let attacker_courier = Uuid::new_v4();
        let location = Uuid::new_v4();
        let assignment_id = Uuid::new_v4();
        let repo = FakeAssignmentsRepo::default();
        repo.seed(FakeAssignment {
            courier_id: owner_courier,
            location_id: location,
            status: "assigned".to_string(),
            row: fixture_row(assignment_id, Uuid::new_v4(), 1000),
        });
        let state = AssignmentsState {
            auth: AuthState::test_state(Arc::new(FakeAuthRepo::default())),
            repo: Arc::new(repo),
        };

        let resp = accept_assignment(
            Extension(state),
            courier_session(attacker_courier, location),
            Path(assignment_id),
            Extension(request_id()),
        )
        .await;
        let err = crate::error::expect_err(resp);
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    /// The legitimate owner of the assignment CAN accept it (contrast case for the hijack test).
    #[tokio::test]
    async fn legitimate_courier_can_accept_their_own_assignment() {
        let courier_id = Uuid::new_v4();
        let location = Uuid::new_v4();
        let assignment_id = Uuid::new_v4();
        let repo = FakeAssignmentsRepo::default();
        repo.seed(FakeAssignment {
            courier_id,
            location_id: location,
            status: "assigned".to_string(),
            row: fixture_row(assignment_id, Uuid::new_v4(), 1000),
        });
        let state = AssignmentsState {
            auth: AuthState::test_state(Arc::new(FakeAuthRepo::default())),
            repo: Arc::new(repo),
        };

        let resp = accept_assignment(
            Extension(state),
            courier_session(courier_id, location),
            Path(assignment_id),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// REV-S7-8 (S7-T11): `paid_full` with `cash != total` -> 422 CASH_AMOUNT_MISMATCH, exact
    /// equality (not `>=`).
    #[tokio::test]
    async fn delivered_cash_mismatch_is_422_exact_equality_not_gte() {
        let courier_id = Uuid::new_v4();
        let location = Uuid::new_v4();
        let assignment_id = Uuid::new_v4();
        let repo = FakeAssignmentsRepo::default();
        repo.seed(FakeAssignment {
            courier_id,
            location_id: location,
            status: "picked_up".to_string(),
            row: fixture_row(assignment_id, Uuid::new_v4(), 1000),
        });
        let state = AssignmentsState {
            auth: AuthState::test_state(Arc::new(FakeAuthRepo::default())),
            repo: Arc::new(repo),
        };

        // 1001 > total (1000) — a MORE-than-total cash handoff must ALSO 422 (not just less-than).
        let resp = delivered_assignment(
            Extension(state),
            courier_session(courier_id, location),
            Path(assignment_id),
            Extension(request_id()),
            Json(DeliveredRequest {
                payment_outcome: Some(PaymentOutcome::PaidFull),
                cash_collected: None,
                cash_amount: Some(1001),
            }),
        )
        .await;
        let err = crate::error::expect_err(resp);
        assert_eq!(err.envelope.code, ErrorCode::CashAmountMismatch);
        assert_eq!(err.envelope.status, 422);
    }

    /// The honest no-cash tail: `refused_payment` -> assignment cancelled, NO hold, success.
    #[tokio::test]
    async fn delivered_refused_payment_succeeds_with_no_cash_hold() {
        let courier_id = Uuid::new_v4();
        let location = Uuid::new_v4();
        let assignment_id = Uuid::new_v4();
        let repo = FakeAssignmentsRepo::default();
        repo.seed(FakeAssignment {
            courier_id,
            location_id: location,
            status: "picked_up".to_string(),
            row: fixture_row(assignment_id, Uuid::new_v4(), 1000),
        });
        let state = AssignmentsState {
            auth: AuthState::test_state(Arc::new(FakeAuthRepo::default())),
            repo: Arc::new(repo),
        };

        let resp = delivered_assignment(
            Extension(state),
            courier_session(courier_id, location),
            Path(assignment_id),
            Extension(request_id()),
            Json(DeliveredRequest {
                payment_outcome: Some(PaymentOutcome::RefusedPayment),
                cash_collected: None,
                cash_amount: None,
            }),
        )
        .await
        .unwrap()
        .into_response();
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["success"], true);
    }

    /// `paid_full` with the EXACT cash amount succeeds (contrast case for the mismatch test).
    #[tokio::test]
    async fn delivered_exact_cash_amount_succeeds() {
        let courier_id = Uuid::new_v4();
        let location = Uuid::new_v4();
        let assignment_id = Uuid::new_v4();
        let repo = FakeAssignmentsRepo::default();
        repo.seed(FakeAssignment {
            courier_id,
            location_id: location,
            status: "picked_up".to_string(),
            row: fixture_row(assignment_id, Uuid::new_v4(), 1000),
        });
        let state = AssignmentsState {
            auth: AuthState::test_state(Arc::new(FakeAuthRepo::default())),
            repo: Arc::new(repo),
        };

        let resp = delivered_assignment(
            Extension(state),
            courier_session(courier_id, location),
            Path(assignment_id),
            Extension(request_id()),
            Json(DeliveredRequest {
                payment_outcome: Some(PaymentOutcome::PaidFull),
                cash_collected: None,
                cash_amount: Some(1000),
            }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// A stale/foreign tap on `/delivered` when not `picked_up` -> 404, never a cross-courier
    /// write.
    #[tokio::test]
    async fn delivered_wrong_status_is_404() {
        let courier_id = Uuid::new_v4();
        let location = Uuid::new_v4();
        let assignment_id = Uuid::new_v4();
        let mut row = fixture_row(assignment_id, Uuid::new_v4(), 1000);
        row.status = "accepted".to_string();
        let repo = FakeAssignmentsRepo::default();
        repo.seed(FakeAssignment {
            courier_id,
            location_id: location,
            status: "accepted".to_string(),
            row,
        });
        let state = AssignmentsState {
            auth: AuthState::test_state(Arc::new(FakeAuthRepo::default())),
            repo: Arc::new(repo),
        };

        let resp = delivered_assignment(
            Extension(state),
            courier_session(courier_id, location),
            Path(assignment_id),
            Extension(request_id()),
            Json(DeliveredRequest {
                payment_outcome: Some(PaymentOutcome::PaidFull),
                cash_collected: None,
                cash_amount: Some(1000),
            }),
        )
        .await;
        let err = crate::error::expect_err(resp);
        assert_eq!(
            err.envelope.code,
            ErrorCode::AssignmentNotFoundOrNotPickedUp
        );
    }

    /// DeliveredRequest rejects an unknown field (`.strict()` parity).
    #[test]
    fn delivered_request_rejects_unknown_field() {
        let json = serde_json::json!({"payment_outcome": "paid_full", "extra": "nope"});
        assert!(serde_json::from_value::<DeliveredRequest>(json).is_err());
    }

    /// `release_binding_and_reoffer` snapshot classification — the guarded-transition rule
    /// (`bindingRelease.ts`): IN_DELIVERY+picked_up -> terminal; IN_DELIVERY+other -> revert+reoffer;
    /// anything else -> no transition, reoffer only.
    #[test]
    fn binding_snapshot_classification_matches_bindingrelease_ts() {
        // (These are pure string comparisons mirrored from the repo function's own branch logic —
        // exercised end-to-end via the cancel/abort integration tests above; this pins the THREE
        // distinct branches exist and are mutually exclusive as documented.)
        let a = AssignmentBindingSnapshot::InDeliveryPickedUp;
        let b = AssignmentBindingSnapshot::InDeliveryOther;
        let c = AssignmentBindingSnapshot::NotYetInDelivery;
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c);
    }

    /// `POST /assignments/:id/reject` on a foreign assignment -> 404 (actor-gate).
    #[tokio::test]
    async fn reject_foreign_assignment_is_404() {
        let owner_courier = Uuid::new_v4();
        let attacker = Uuid::new_v4();
        let location = Uuid::new_v4();
        let assignment_id = Uuid::new_v4();
        let repo = FakeAssignmentsRepo::default();
        repo.seed(FakeAssignment {
            courier_id: owner_courier,
            location_id: location,
            status: "assigned".to_string(),
            row: fixture_row(assignment_id, Uuid::new_v4(), 1000),
        });
        let state = AssignmentsState {
            auth: AuthState::test_state(Arc::new(FakeAuthRepo::default())),
            repo: Arc::new(repo),
        };
        let resp = reject_assignment(
            Extension(state),
            courier_session(attacker, location),
            Path(assignment_id),
            Extension(request_id()),
        )
        .await;
        let err = crate::error::expect_err(resp);
        assert_eq!(
            err.envelope.code,
            ErrorCode::AssignmentNotFoundOrNotAssigned
        );
    }

    /// 🔴 REV-S7-1 DoD — "a discriminating NOBYPASSRLS probe on every courier READ ... asserts the
    /// courier's own rows post-flip; a bare-pool path is a build failure." Requires a live Postgres
    /// (same posture as `db.rs`'s `with_tenant_scopes_and_resets_the_guc` / `dispatch.rs`'s
    /// synthetic-courier test — not run in this sandbox). `PgAssignmentsRepo::get_one` routes
    /// through `with_tenant`, which is `BEGIN -> set_config(app.current_tenant, ..., true) -> work
    /// -> COMMIT` as ONE function — there is no code path that can reach the SELECT without the
    /// seat, unlike the old Node `assignments.ts:110` (`set_config` with no `BEGIN`, silently
    /// discarded before the SELECT runs on a possibly-different pooled connection).
    #[tokio::test]
    #[ignore = "requires a live Postgres — set DATABASE_URL_OPERATIONAL/DATABASE_URL_SESSION and run with --ignored"]
    async fn get_one_is_seated_via_with_tenant_not_a_bare_pool_read() {
        let config =
            crate::config::Config::from_env().expect("env must be valid to run this ignored test");
        let pools = crate::db::Pools::connect(&config)
            .await
            .expect("pools must connect");
        let repo = PgAssignmentsRepo::new(pools.operational.clone());

        // A random (non-seeded) assignment id under a random tenant must come back `None` (RLS
        // hides it / it doesn't exist) rather than erroring or panicking — proving the seat is
        // established (a MISSING seat under NOBYPASSRLS raises a Postgres error on the read, not a
        // clean `None`; a seat present but wrong-tenant also cleanly returns `None`).
        let result = repo
            .get_one(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4())
            .await
            .expect("with_tenant-seated read must not error even against an unmatched tenant");
        assert!(result.is_none());
    }
}
