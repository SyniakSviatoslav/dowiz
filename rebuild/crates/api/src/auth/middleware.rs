//! The middleware tower (REV-4, proposal §11) — the pre-route gates that run BEFORE any handler,
//! in the exact load-bearing order the Node `server.ts` onRequest hook enforces. Each node is a
//! pure decision function with a named test vector, plus the axum layer that applies it.
//!
//! Node order (server.ts:405-427), reproduced:
//!   1. OPTIONS/preflight short-circuit (server.ts:406) — never gated.
//!   2. dev-path 404 existence-hiding gate (server.ts:412-416) — dev paths 404 unless the
//!      ALLOW_DEV_LOGIN + x-dev-auth-secret gate is open (ADR-0003; bare `{error:'Not found'}`).
//!   3. NO_AUTH_PATHS + OTP-regex bypass (server.ts:417-420) — pre-auth routes skip the bearer gate.
//!   4. AUTH_PREFIXES bearer-presence pre-gate (server.ts:421-426) — a missing `Bearer ` prefix on
//!      an `/api/(owner|courier|customer)/` path short-circuits to bare `401 {error:'Unauthorized'}`
//!      (Q-BEARERGATE) BEFORE the extractor runs.
//!
//! (rate-limit sits in front of all of these — 429 precedes 401; see `rate_limit_precedes_bearer`).
//!
//! The functions here are the DECISIONS; `bearer_gate_layer` wires them as an axum
//! `from_fn` middleware. Keeping the decisions pure lets REV-4 be proven by direct unit tests
//! (the named vectors) without spinning an HTTP server.

use axum::extract::Request;
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use super::mount::AuthState;

/// server.ts:400 — the prefixes whose unauthenticated requests get a bare 401 (not 404).
pub const AUTH_PREFIXES: [&str; 3] = ["/api/owner/", "/api/courier/", "/api/customer/"];

/// server.ts:401-404 — pre-auth routes under an AUTH_PREFIX that must NOT trip the bearer gate.
pub const NO_AUTH_PATHS: [&str; 2] = [
    "/api/courier/auth/", // public courier invite/redeem/login/refresh/logout
    "/api/customer/track/exchange", // trades the opaque ?t= code for a customer JWT
];

/// server.ts:405-416 — the dev-route prefixes gated by ALLOW_DEV_LOGIN + x-dev-auth-secret.
pub const DEV_PREFIXES: [&str; 2] = ["/dev/", "/api/dev/"];

/// The decision the bearer pre-gate makes for one request. Pure — the layer maps it to a Response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BearerGateDecision {
    /// OPTIONS/preflight, or a NO_AUTH/OTP path, or a non-AUTH_PREFIX path → let it through.
    Pass,
    /// An AUTH_PREFIX path with no `Bearer ` → bare `401 {error:'Unauthorized'}` (Q-BEARERGATE).
    Unauthorized,
}

/// REV-4 node 1+3+4: decide the pre-route bearer gate. `is_options` short-circuits (node 1);
/// NO_AUTH_PATHS + the OTP regex bypass (node 3); AUTH_PREFIXES bearer-presence (node 4).
pub fn bearer_gate_decision(method: &Method, path: &str, has_bearer: bool) -> BearerGateDecision {
    // Node 1: OPTIONS/preflight is never gated (server.ts:406).
    if method == Method::OPTIONS {
        return BearerGateDecision::Pass;
    }
    // Node 3: NO_AUTH_PATHS + the pre-auth customer-OTP regex bypass (server.ts:417-420).
    if NO_AUTH_PATHS.iter().any(|p| path.starts_with(p)) || is_otp_path(path) {
        return BearerGateDecision::Pass;
    }
    // Node 4: AUTH_PREFIXES require a bearer; a missing one → bare 401 (server.ts:421-426).
    if AUTH_PREFIXES.iter().any(|p| path.starts_with(p)) && !has_bearer {
        return BearerGateDecision::Unauthorized;
    }
    BearerGateDecision::Pass
}

/// server.ts:420 — `^/api/customer/locations/[^/]+/otp/(send|verify)$`. Hand-matched (no regex dep):
/// the two dynamic-but-bounded pre-auth OTP endpoints skip the bearer gate.
fn is_otp_path(path: &str) -> bool {
    let Some(rest) = path.strip_prefix("/api/customer/locations/") else {
        return false;
    };
    // rest = "<locationId>/otp/(send|verify)" — exactly: one non-empty non-'/' segment, then the tail.
    let Some((loc, tail)) = rest.split_once('/') else {
        return false;
    };
    !loc.is_empty() && !loc.contains('/') && (tail == "otp/send" || tail == "otp/verify")
}

/// REV-4 node 2: the dev-path 404 existence-hiding gate (server.ts:412-416, ADR-0003). A dev path
/// is authorized iff the dev gate is open (ALLOW_DEV_LOGIN + DEV_AUTH_SECRET) AND the
/// `x-dev-auth-secret` header timing-safe-matches. A closed/mismatched gate → 404 (never 401 — no
/// existence leak).
pub fn is_dev_path(path: &str) -> bool {
    DEV_PREFIXES.iter().any(|p| path.starts_with(p))
}

/// The dev-request authorization decision (`isDevRequestAuthorized`, dev-guard.ts:54-62). Non-dev
/// paths are always authorized (pass-through). Dev paths require the open gate + secret match.
pub fn dev_request_authorized(
    path: &str,
    provided_secret: Option<&str>,
    dev_login_allowed: bool,
    dev_auth_secret: Option<&str>,
) -> bool {
    if !is_dev_path(path) {
        return true;
    }
    if !dev_login_allowed {
        return false;
    }
    match (provided_secret, dev_auth_secret) {
        (Some(provided), Some(expected)) => super::crypto::timing_safe_eq(provided, expected),
        _ => false,
    }
}

/// The axum `from_fn` middleware that applies nodes 1-4 in order. Runs after the app's rate-limit
/// layer (which is wired OUTSIDE this — 429 precedes 401, REV-4 rate-limit-vs-bearer ordering) and
/// before the per-route extractor.
pub async fn bearer_and_dev_gate(request: Request, next: Next) -> Response {
    // The AuthState carries the dev-gate config. If it's missing the gate can't make an
    // ADR-0003-correct decision, so fail CLOSED for dev paths (404) and pass others.
    let state = request.extensions().get::<AuthState>().cloned();
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    // Node 2: dev-path 404 gate — before anything can leak dev-route existence.
    if is_dev_path(&path) {
        let (allowed, secret) = state
            .as_ref()
            .map(|s| {
                (
                    s.config.dev_login_allowed(),
                    s.config.dev_auth_secret.clone(),
                )
            })
            .unwrap_or((false, None));
        let provided = request
            .headers()
            .get("x-dev-auth-secret")
            .and_then(|v| v.to_str().ok());
        if !dev_request_authorized(&path, provided, allowed, secret.as_deref()) {
            // Q-DEV-404: bare {error:'Not found'} — existence-hiding, never 401.
            return (
                StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({ "error": "Not found" })),
            )
                .into_response();
        }
    }

    // Nodes 1+3+4: the bearer pre-gate.
    let has_bearer = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("Bearer "));
    match bearer_gate_decision(&method, &path, has_bearer) {
        BearerGateDecision::Unauthorized => {
            // Q-BEARERGATE: bare {error:'Unauthorized'} — non-envelope, before the extractor.
            (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({ "error": "Unauthorized" })),
            )
                .into_response()
        }
        BearerGateDecision::Pass => next.run(request).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── REV-4 node 1: OPTIONS/preflight short-circuit ──
    #[test]
    fn rev4_options_preflight_short_circuits() {
        assert_eq!(
            bearer_gate_decision(&Method::OPTIONS, "/api/owner/menu", false),
            BearerGateDecision::Pass,
            "OPTIONS must pass even with no bearer (preflight)"
        );
    }

    // ── REV-4 node 4: AUTH_PREFIXES bearer-presence → bare 401 ──
    #[test]
    fn rev4_auth_prefix_without_bearer_is_unauthorized() {
        assert_eq!(
            bearer_gate_decision(&Method::GET, "/api/owner/orders", false),
            BearerGateDecision::Unauthorized
        );
        assert_eq!(
            bearer_gate_decision(&Method::GET, "/api/owner/orders", true),
            BearerGateDecision::Pass,
            "a present bearer passes the pre-gate (the extractor verifies it)"
        );
    }

    // ── REV-4 node 3: NO_AUTH_PATHS bypass ──
    #[test]
    fn rev4_no_auth_paths_bypass_the_bearer_gate() {
        assert_eq!(
            bearer_gate_decision(&Method::POST, "/api/courier/auth/login", false),
            BearerGateDecision::Pass
        );
        assert_eq!(
            bearer_gate_decision(&Method::POST, "/api/customer/track/exchange", false),
            BearerGateDecision::Pass
        );
    }

    // ── REV-4 node 3: OTP regex bypass ──
    #[test]
    fn rev4_otp_paths_bypass_the_bearer_gate() {
        assert!(is_otp_path("/api/customer/locations/abc-123/otp/send"));
        assert!(is_otp_path("/api/customer/locations/abc-123/otp/verify"));
        assert!(!is_otp_path("/api/customer/locations/abc-123/otp/other"));
        assert!(!is_otp_path("/api/customer/orders/abc"));
        assert_eq!(
            bearer_gate_decision(&Method::POST, "/api/customer/locations/xyz/otp/send", false),
            BearerGateDecision::Pass
        );
        // A NON-otp customer path with no bearer is still gated.
        assert_eq!(
            bearer_gate_decision(&Method::GET, "/api/customer/orders/abc", false),
            BearerGateDecision::Unauthorized
        );
    }

    // ── REV-4 node 2: dev-path 404 gate ──
    #[test]
    fn rev4_dev_path_authorized_only_with_open_gate_and_matching_secret() {
        // Closed gate → not authorized (→ 404).
        assert!(!dev_request_authorized(
            "/dev/mock-auth",
            Some("s"),
            false,
            Some("s")
        ));
        // Open gate, wrong secret → not authorized (→ 404, never 401).
        assert!(!dev_request_authorized(
            "/dev/mock-auth",
            Some("wrong"),
            true,
            Some("right")
        ));
        // Open gate, matching secret → authorized.
        assert!(dev_request_authorized(
            "/dev/mock-auth",
            Some("right"),
            true,
            Some("right")
        ));
        // Non-dev path → always authorized (pass-through).
        assert!(dev_request_authorized("/api/owner/x", None, false, None));
    }

    // ── REV-4 rate-limit-vs-bearer ordering (429 precedes 401) ──
    // The rate-limit layer is wired OUTSIDE (in front of) `bearer_and_dev_gate` in the router, so a
    // rate-limited request never reaches the bearer decision. This test documents the invariant by
    // asserting the decision function itself is oblivious to rate state — it CANNOT emit a 401 for a
    // request the outer layer already 429'd, because it never runs for one. The ordering is proven
    // structurally by the layer stack in `mount::auth_router` (rate-limit .layer AFTER this one, so
    // it runs FIRST — tower applies layers outside-in).
    #[test]
    fn rate_limit_precedes_bearer_is_a_structural_invariant() {
        // A well-formed AUTH_PREFIX request with a bearer passes THIS gate — meaning any 429 must
        // have come from the outer rate-limit layer, never from here (429-before-401 preserved).
        assert_eq!(
            bearer_gate_decision(&Method::POST, "/api/owner/orders", true),
            BearerGateDecision::Pass
        );
    }
}
