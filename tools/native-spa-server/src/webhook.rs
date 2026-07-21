//! P48-INTAKE Phase 1 — `/webhook/{channel}/{hub_id}` route handlers.
//!
//! The webhook receive surface lives here (in `native-spa-server`), NOT under
//! the `/api/*` capability-cert gate — the trust boundary is external signature
//! verification, not dowiz's own capability certs (§5.3 in the blueprint).
//!
//! Each handler: verify external signature → normalize to `InboundMessage` →
//! return 200 (at-least-once ack; errors are non-retry-inducing 200/401 per
//! each platform's documented retry semantics).

use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};

use intake_adapters::{
    telegram::{TgUpdate, TelegramAdapter},
    IntakeError, IntakeWebhookHeaders,
};

/// Shared webhook state — holds per-hub adapters. In a real deployment this
/// would be a `HashMap<hub_id, Arc<TelegramAdapter>>` loaded from hub config;
/// for Phase 1 we support a single hub for the test suite.
pub struct WebhookState {
    pub telegram: Arc<TelegramAdapter>,
}

/// Build the `/webhook/*` route family. This is merged into the main router
/// in `lib.rs` OUTSIDE the `/api/*` capability-cert gate.
pub fn build_webhook_router(state: Arc<WebhookState>) -> Router {
    Router::new()
        .route("/webhook/telegram/{hub_id}", post(telegram_webhook))
        .route("/webhook/telegram/{hub_id}", get(telegram_webhook_get))
        .with_state(state)
}

/// POST `/webhook/telegram/{hub_id}` — Telegram Bot API push webhook.
///
/// Verification: `X-Telegram-Bot-Api-Secret-Token` header must match the
/// per-hub secret (constant-time compare via `intake_adapters::constant_time_eq`).
///
/// On success, returns 200 with an empty body (Telegram expects a fast 200 ack;
/// retries on non-2xx/timeout). The `InboundMessage` is constructed but NOT yet
/// wired to order placement — that's the intake service's job (§5.4/§5.5).
async fn telegram_webhook(
    Path(hub_id): Path<String>,
    State(state): State<Arc<WebhookState>>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> Response {
    // Extract the Telegram secret token header.
    let secret = headers
        .get("X-Telegram-Bot-Api-Secret-Token")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let intake_headers = IntakeWebhookHeaders {
        telegram_secret: secret,
        ..Default::default()
    };

    match state
        .telegram
        .verify_and_normalize(&body, &intake_headers, &hub_id)
    {
        Ok(messages) if messages.is_empty() => {
            // Non-message update or no text — acknowledge silently.
            StatusCode::OK.into_response()
        }
        Ok(messages) => {
            // Phase 1: log the normalized message. In Phase 2 the intake
            // service will receive these and call `place_order`.
            for msg in &messages {
                eprintln!(
                    "[webhook] telegram inbound: venue={} channel={} sender={} text={:?}",
                    msg.venue_id, msg.channel, msg.sender, msg.text
                );
            }
            // Return the normalized messages as JSON for testing / debugging.
            // In production this would be silently consumed by the intake service.
            (StatusCode::OK, Json(serde_json::json!({"ok": true, "messages": messages.len()})))
                .into_response()
        }
        Err(IntakeError::SignatureMismatch) => {
            (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"})))
                .into_response()
        }
        Err(IntakeError::Duplicate(id)) => {
            // Ack the duplicate — Telegram should not retry, but we 200 anyway
            // to prevent retry storms.
            eprintln!("[webhook] telegram duplicate update_id={id}, acking silently");
            StatusCode::OK.into_response()
        }
        Err(IntakeError::MalformedPayload(e)) => {
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e})))
                .into_response()
        }
        Err(IntakeError::StaleTimestamp) => {
            (StatusCode::OK, Json(serde_json::json!({"error": "stale"})))
                .into_response()
        }
    }
}

/// GET `/webhook/telegram/{hub_id}` — not a real Telegram endpoint, but
/// returns 405 Method Not Allowed to distinguish from 404 (the route exists,
/// but Telegram only POSTs to it).
async fn telegram_webhook_get() -> Response {
    StatusCode::METHOD_NOT_ALLOWED.into_response()
}
