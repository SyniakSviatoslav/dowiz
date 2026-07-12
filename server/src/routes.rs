//! HTTP handlers + router construction.
//!
//! Routes:
//!   * `POST /api/orders`                 — kernel `place_order`, persist, return 201
//!   * `POST /api/orders/:id/event`       — kernel `apply_event`; 409 on illegal
//!   * `POST /api/courier/push/subscribe` — persist a web-push subscription
//!   * `POST /api/courier/push/resubscribe` — re-persist a push subscription (SW resubscribe)
//!   * `GET  /api/orders/channel`         — channel-attribution counts
//!   * `GET  /api/healthz`                — reliability ratchet status
//!   * `GET  /` (and SPA fallback)        — serve `web/dist`
//!
//! No courier scoring/rating occurs anywhere in this module.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use dowiz_kernel::{apply_event, place_order, OrderStatus};
use serde_json::json;
use tower_http::services::{ServeDir, ServeFile};

use crate::models::{
    ChannelCount, ChannelResponse, ClaimVenueRequest, CreateOrderRequest, EventRequest,
    OrderResponse, PushSubResponse, SubscribeRequest, VenueResponse,
};
use crate::notify::NotifyHub;
use crate::store::Store;

/// Shared app state.
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Store>,
    /// Courier out-of-app notify hub (Tier-2 N1/N2). Optional so a test/app can
    /// run without a sink (signals are best-effort and never fail the request).
    pub notify: Option<Arc<NotifyHub>>,
}

/// Monotonic id helper (avoids an extra uuid dependency while guaranteeing
/// uniqueness within a process run).
static ORDER_SEQ: AtomicU64 = AtomicU64::new(0);
static SUB_SEQ: AtomicU64 = AtomicU64::new(0);

fn new_order_id() -> String {
    let n = ORDER_SEQ.fetch_add(1, Ordering::SeqCst);
    format!("ord_{}", n)
}

fn new_sub_id() -> String {
    let n = SUB_SEQ.fetch_add(1, Ordering::SeqCst);
    format!("sub_{}", n)
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// `POST /api/orders`
async fn create_order(
    State(state): State<AppState>,
    Json(req): Json<CreateOrderRequest>,
) -> impl IntoResponse {
    let id = new_order_id();
    let created_at_ms = now_ms();
    let items = crate::models::kernel_items(&req.items);

    let order = match place_order(
        id,
        Some(req.location_id.clone()),
        items,
        created_at_ms,
        req.channel.clone(),
        req.cash_pay_with.clone(),
    ) {
        Ok(o) => o,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e.message(), "code": e.code() })),
            )
                .into_response();
        }
    };

    if let Err(e) = state.store.insert_order(&order) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string(), "code": "DbError" })),
        )
            .into_response();
    }

    (StatusCode::CREATED, Json(OrderResponse::from(&order))).into_response()
}

/// `POST /api/orders/:id/event`
async fn order_event(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<EventRequest>,
) -> impl IntoResponse {
    let next = match OrderStatus::from_str(&req.next_status) {
        Some(s) => s,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Unknown status: {}", req.next_status), "code": "UnknownStatus" })),
            )
                .into_response();
        }
    };

    let existing = match state.store.get_order(&id) {
        Ok(Some(o)) => o,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Order not found", "code": "NotFound" })),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string(), "code": "DbError" })),
            )
                .into_response();
        }
    };

    // The kernel is the single source of truth for legality.
    let updated = match apply_event(&existing, next) {
        Ok(o) => o,
        // ── RED: illegal transition -> 409 Conflict ──
        Err(e) => {
            return (
                StatusCode::CONFLICT,
                Json(json!({ "error": e.message(), "code": e.code() })),
            )
                .into_response();
        }
    };

    if let Err(e) = state.store.update_status(&id, updated.status.as_str()) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string(), "code": "DbError" })),
        )
            .into_response();
    }

    // Tier-2 N1/N2: signal couriers out-of-app on the status change (best-effort;
    // never fails the transition). No scoring/ranking — all known couriers.
    if let Some(hub) = &state.notify {
        hub.signal(&id, updated.status.as_str());
    }

    (StatusCode::OK, Json(OrderResponse::from(&updated))).into_response()
}

/// `POST /api/courier/push/subscribe`
async fn push_subscribe(
    State(state): State<AppState>,
    Json(req): Json<SubscribeRequest>,
) -> impl IntoResponse {
    let id = new_sub_id();
    let created_at_ms = now_ms();
    if let Err(e) = state.store.insert_push_sub(&id, created_at_ms, &req) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string(), "code": "DbError" })),
        )
            .into_response();
    }
    (
        StatusCode::CREATED,
        Json(PushSubResponse {
            id,
            courier_id: req.courier_id,
            endpoint: req.endpoint,
            created_at_ms,
        }),
    )
        .into_response()
}

/// `GET /api/orders/channel`
async fn orders_channel(State(state): State<AppState>) -> impl IntoResponse {
    match state.store.list_by_channel() {
        Ok(rows) => {
            let orders_by_channel = rows
                .into_iter()
                .map(|(channel, count)| ChannelCount { channel, count })
                .collect::<Vec<_>>();
            (StatusCode::OK, Json(ChannelResponse { orders_by_channel })).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string(), "code": "DbError" })),
        )
            .into_response(),
    }
}

/// `POST /api/courier/push/resubscribe`
///
/// Mirror of `push_subscribe`; the Service Worker calls this on
/// `pushsubscriptionchange` to re-register a rotated push endpoint. RED item
/// (sw.js resubscribe loop was 404'ing — route now exists).
async fn push_resubscribe(
    State(state): State<AppState>,
    Json(req): Json<SubscribeRequest>,
) -> impl IntoResponse {
    let id = new_sub_id();
    let created_at_ms = now_ms();
    if let Err(e) = state.store.insert_push_sub(&id, created_at_ms, &req) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string(), "code": "DbError" })),
        )
            .into_response();
    }
    (
        StatusCode::CREATED,
        Json(PushSubResponse {
            id,
            courier_id: req.courier_id,
            endpoint: req.endpoint,
            created_at_ms,
        }),
    )
        .into_response()
}

/// `GET /api/healthz`
///
/// Exposes the reliability ratchet status (Tier-0 B). The ratchet is a
/// process-global handle (`RELIABILITY`), so flags reset on a fresh process —
/// the RED "flags reset on restart" guarantee.
async fn healthz() -> impl IntoResponse {
    let s = crate::reliability::RELIABILITY.status();
    (StatusCode::OK, Json(serde_json::to_value(s).unwrap())).into_response()
}

/// `GET /api/venues/:id`
///
/// Returns the venue's claim status (Tier-3 plumbing: a venue must be claimed
/// before G11 — a real order from a non-operator customer on a claimed venue).
async fn get_venue(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    match state.store.get_venue(&id) {
        Ok(Some((name, claimed, owner_id))) => (
            StatusCode::OK,
            Json(VenueResponse {
                id,
                name,
                claimed,
                owner_id,
            }),
        )
            .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Venue not found", "code": "NotFound" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string(), "code": "DbError" })),
        )
            .into_response(),
    }
}

/// `POST /api/venues/:id/claim`
///
/// Claims a venue (idempotent upsert). After this, the venue is "claimed" and
/// eligible for a real G11 order attributed via `?ch=<venue_id>`.
async fn claim_venue(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ClaimVenueRequest>,
) -> impl IntoResponse {
    let created_at_ms = now_ms();
    if let Err(e) = state.store.claim_venue(
        &id,
        &req.owner_id,
        req.name.as_deref().unwrap_or(&id),
        created_at_ms,
    ) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string(), "code": "DbError" })),
        )
            .into_response();
    }
    // Reflect the newly-claimed state.
    match state.store.get_venue(&id) {
        Ok(Some((name, claimed, owner_id))) => (
            StatusCode::OK,
            Json(VenueResponse {
                id,
                name,
                claimed,
                owner_id,
            }),
        )
            .into_response(),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "claim write lost", "code": "DbError" })),
        )
            .into_response(),
    }
}

/// Build the application router. `dist_dir` is the path to `web/dist` (served as
/// a SPA fallback). When `dist_dir` does not exist, the SPA fallback is omitted
/// (the API routes still work) so the server is usable before the frontend build.
///
/// `notify` is the optional courier out-of-app hub (Tier-2 N1/N2). Pass `None`
/// to disable signalling (best-effort — the lifecycle never depends on it).
pub fn build_router(state: AppState, dist_dir: PathBuf) -> Router {
    let api = Router::new()
        .route("/api/orders", post(create_order))
        .route("/api/orders/:id/event", post(order_event))
        .route("/api/courier/push/subscribe", post(push_subscribe))
        .route("/api/courier/push/resubscribe", post(push_resubscribe))
        .route("/api/orders/channel", get(orders_channel))
        .route("/api/healthz", get(healthz))
        .route("/api/venues/:id", get(get_venue))
        .route("/api/venues/:id/claim", post(claim_venue));

    let api = api.with_state(state);

    // SPA: serve web/dist and fall back to index.html for client-side routes.
    if dist_dir.exists() {
        let spa = ServeDir::new(&dist_dir).fallback(ServeFile::new(dist_dir.join("index.html")));
        Router::new().fallback_service(spa).merge(api)
    } else {
        api
    }
}
