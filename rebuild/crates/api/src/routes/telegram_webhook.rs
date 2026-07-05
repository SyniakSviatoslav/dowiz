//! `POST /webhook/telegram/{secret}` — REV-S8-2 (🔴 breaker HIGH, counsel #4): FIX-IN-PORT of a
//! LIVE fail-open gap.
//!
//! ## The bug this fixes (source-verified)
//! `telegram-webhook.ts:87-100`: when `TELEGRAM_BOT_SECRET` is configured but the
//! `x-telegram-bot-api-secret-token` header is ABSENT, the old handler logged a warning and
//! **processed the request anyway** ("backward compat") — it 401'd only when the header was
//! PRESENT but wrong. The existing E2E spec (`e2e/tests/telegram-webhook.spec.ts:53-59`,
//! "missing secret returns 401") already asserts the CORRECT behavior against the buggy code —
//! i.e. the guardrail was red→green-ready before this port even started; the handler was the bug.
//! A forged webhook with no header could flip owner order state (`order.confirm`/`order.reject`)
//! on a LIVE production system. (A hotfix (`d6b3473e`) already landed this exact fix on the
//! Node side, dispatched in parallel with the council RESOLVE — this port carries the
//! ALREADY-FIXED behavior forward, not a still-open gap.)
//!
//! ## The fix — unconditionally fail-closed, constant-time compare
//! If `TELEGRAM_BOT_SECRET` is configured: header absent, empty, wrong-length, OR wrong-value →
//! **401**, every branch, no exceptions. [`secret_token_matches`] is `subtle`-backed
//! constant-time comparison (Q4: "constant-time compare"), gated on a length check FIRST (the
//! same two-step shape Node's `crypto.timingSafeEqual` uses — it also requires equal-length
//! buffers) — a length mismatch short-circuits to "no match" before the constant-time compare
//! ever runs, which is standard practice (buffer LENGTH is not itself the secret).
//!
//! ## Fail-closed ALSO when no secret is configured — in prod (guardian gate)
//! An UNSET `TELEGRAM_BOT_SECRET` must not silently accept everyone. [`webhook_decision`] keys the
//! no-secret case on [`TelegramWebhookState::require_secret`] (`true` in production): prod + unset
//! secret → **401** (fail-closed, mirroring the S2/plisio "no secret configured → reject in prod"
//! doctrine — an accept-anyone webhook is a forge hole the moment dispatch is wired); dev/test +
//! unset secret → 200 (a local-convenience accept only). This closes the second fail-open the
//! original REV-S8-2 fix left: it hardened the wrong/missing-HEADER path but still accepted all
//! callers whenever the secret itself was absent.
//!
//! ## The URL-path secret is a router token, not the auth (Q-TG-URL-SECRET)
//! The `{secret}` path segment is how Telegram ADDRESSES this endpoint (referer/log-leakable —
//! never trust it as an auth boundary); the HEADER `secret_token` is the real gate. This handler
//! accepts the path segment (so the route matches) but never compares it to anything — carrying
//! Telegram's own router-addressing model while keying the actual security decision on the header.
//!
//! ## Business logic is explicitly OUT of S8 scope
//! Per the council packet §2 ("NOT S8"), the order state machine is S5's and the dispatch
//! handshake is S7's — this handler's job ends at the auth gate. Once verified, it 200s
//! immediately (Q-TG-200-ALWAYS: "webhook always returns 200 to Telegram even on internal
//! failure... a 500 makes Telegram retry-storm") without processing the update body; wiring the
//! actual `/start`/`order.confirm`/`order.reject` command dispatch into S5/S7 state transitions is
//! a follow-up integration this pass does not claim to close.

use axum::extract::{Extension, Path};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use subtle::ConstantTimeEq;

use crate::config::Secret;

#[derive(Clone)]
pub struct TelegramWebhookState {
    pub bot_secret: Option<std::sync::Arc<Secret>>,
    /// `true` in production (the guardian fix): an UNSET `TELEGRAM_BOT_SECRET` in prod must
    /// fail-CLOSED (reject everyone) rather than accept any caller. `false` in dev/test, where an
    /// unset secret is a convenience-accept. Set in `main.rs` from `AuthConfig.node_env`.
    pub require_secret: bool,
}

const SECRET_HEADER: &str = "x-telegram-bot-api-secret-token";

/// Constant-time-on-CONTENT compare (length is checked first, non-constant-time — see module
/// doc for why that's the accepted, standard shape). Returns `false` on any length mismatch
/// without ever touching `subtle`, exactly mirroring `crypto.timingSafeEqual`'s own precondition.
fn secret_token_matches(configured: &str, received: &str) -> bool {
    let configured = configured.as_bytes();
    let received = received.as_bytes();
    if configured.len() != received.len() {
        return false;
    }
    configured.ct_eq(received).into()
}

/// The fail-closed decision (REV-S8-2 + the guardian's prod-fail-open fix). Pure so every branch —
/// including "no secret configured in prod" — is a deterministic unit test:
/// - `configured = Some(s)`: constant-time compare `received` against `s` — absent / empty /
///   wrong-length / wrong-value all -> 401; exact match -> 200 (the original REV-S8-2 fix).
/// - `configured = None` + `require_secret` (PROD): **401** — an unconfigured webhook secret in
///   prod fails CLOSED, mirroring the S2/plisio "no secret configured -> reject in prod" doctrine.
///   Inert today (dispatch not wired), but once wired an accept-anyone path would be a forge hole.
/// - `configured = None` + `!require_secret` (dev/test): 200 — dev-only convenience accept.
fn webhook_decision(
    configured: Option<&str>,
    require_secret: bool,
    received: Option<&str>,
) -> StatusCode {
    match configured {
        Some(secret) => {
            if secret_token_matches(secret, received.unwrap_or("")) {
                StatusCode::OK
            } else {
                StatusCode::UNAUTHORIZED
            }
        }
        None if require_secret => StatusCode::UNAUTHORIZED, // prod fail-closed
        None => StatusCode::OK,                             // dev-only accept
    }
}

/// `POST /webhook/telegram/{secret}` — see module doc for the full REV-S8-2 rationale.
#[utoipa::path(
    post,
    path = "/webhook/telegram/{secret}",
    params(
        ("secret" = String, Path, description = "URL-path router token (Telegram addressing only — NOT the auth; the header x-telegram-bot-api-secret-token is the real gate, Q-TG-URL-SECRET)")
    ),
    responses(
        (status = 200, description = "Accepted (verified, or dev with no secret configured)"),
        (status = 401, description = "Rejected: header missing/empty/wrong, or (in prod) no secret configured"),
    ),
    tag = "telegram"
)]
pub async fn telegram_webhook(
    Path(_url_secret): Path<String>,
    Extension(state): Extension<TelegramWebhookState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let received = headers.get(SECRET_HEADER).and_then(|v| v.to_str().ok());
    webhook_decision(
        state.bot_secret.as_deref().map(Secret::expose),
        state.require_secret,
        received,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::post;
    use tower::ServiceExt;

    fn router(bot_secret: Option<&str>, require_secret: bool) -> axum::Router {
        let state = TelegramWebhookState {
            bot_secret: bot_secret.map(|s| std::sync::Arc::new(Secret::new(s))),
            require_secret,
        };
        axum::Router::new()
            .route("/webhook/telegram/{secret}", post(telegram_webhook))
            .layer(Extension(state))
    }

    async fn send(router: axum::Router, header_value: Option<&str>) -> StatusCode {
        let mut req = Request::builder()
            .method("POST")
            .uri("/webhook/telegram/url-path-secret")
            .body(Body::empty())
            .unwrap();
        if let Some(v) = header_value {
            req.headers_mut().insert(SECRET_HEADER, v.parse().unwrap());
        }
        router.oneshot(req).await.unwrap().status()
    }

    #[test]
    fn matches_requires_equal_length() {
        assert!(!secret_token_matches("abc", "ab"));
        assert!(!secret_token_matches("abc", "abcd"));
    }

    #[test]
    fn matches_compares_content() {
        assert!(secret_token_matches("same-secret", "same-secret"));
        assert!(!secret_token_matches("same-secret", "different!"));
    }

    // ── pure decision coverage (every branch, incl. the guardian's prod-fail-open fix) ──

    #[test]
    fn webhook_decision_covers_every_branch() {
        // Configured secret — the compare governs regardless of require_secret.
        assert_eq!(webhook_decision(Some("s"), true, Some("s")), StatusCode::OK);
        assert_eq!(
            webhook_decision(Some("s"), true, Some("wrong")),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            webhook_decision(Some("s"), false, None),
            StatusCode::UNAUTHORIZED
        );
        // No secret configured: PROD (require_secret) fails closed; dev accepts.
        assert_eq!(
            webhook_decision(None, true, None),
            StatusCode::UNAUTHORIZED,
            "no secret configured in prod must fail CLOSED (the guardian fix)"
        );
        assert_eq!(webhook_decision(None, false, None), StatusCode::OK);
    }

    // ── REV-S8-2 named DoD test: fail-closed, every branch (through the real axum handler) ──

    #[tokio::test]
    async fn missing_header_returns_401_when_secret_is_configured() {
        // The exact bug this fixes: the OLD Node handler processed this case anyway.
        let status = send(router(Some("configured-secret"), false), None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn empty_header_returns_401_when_secret_is_configured() {
        let status = send(router(Some("configured-secret"), false), Some("")).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn wrong_header_returns_401_when_secret_is_configured() {
        let status = send(
            router(Some("configured-secret"), false),
            Some("wrong-value"),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn wrong_length_header_returns_401_when_secret_is_configured() {
        let status = send(router(Some("configured-secret"), false), Some("short")).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn correct_header_returns_200() {
        let status = send(
            router(Some("configured-secret"), false),
            Some("configured-secret"),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn no_secret_configured_fails_closed_in_prod() {
        // The guardian's fix: prod + unset secret must reject, not accept-anyone.
        let status = send(router(None, true), None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn no_secret_configured_accepts_in_dev() {
        let status = send(router(None, false), None).await;
        assert_eq!(status, StatusCode::OK);
    }
}
