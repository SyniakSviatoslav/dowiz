//! REV-S10-1 (CRIT, council-required, load-bearing) — the platform-admin plane-gate as a RUNTIME,
//! request-time check, NOT a compile-time router-nesting trick.
//!
//! ## What Node does (the property to reproduce)
//! `lib/platform-admin.ts:76-83` (`registerAdminPlaneGate`) is a root-instance `onRequest` hook that
//! fires for EVERY request on the Fastify instance root and gates on the MATCHED route pattern
//! (`request.routeOptions.url`). Because a root hook flows into every route context by construction,
//! it gates children, siblings, AND future admin routes "with zero detection" — a fail-SAFE property.
//!
//! ## Why a nested Router + clippy lint is STRICTLY WEAKER (breaker C1)
//! Axum has no post-routing matched-pattern hook. A nested `Router` + `route_layer` gates ONLY the
//! routes registered INSIDE that one sub-router; a sibling admin route added via `.merge()`/`.nest()`
//! ANYWHERE else in the tree ships UNGATED, and no literal-`/api/admin`-prefix lint sees it. A single
//! sibling-closure test proves only the ONE throwaway route it declares — it cannot cover a future,
//! not-yet-written route. That is fail-OPEN future-route coverage behind an advisory lint.
//!
//! ## The Rust reproduction of Node's root hook (this module)
//! [`platform_admin_plane_gate`] is applied as the OUTERMOST `.layer()` on the FINAL, fully-merged
//! `Router` in `main.rs` — AFTER every surface's `.merge()`. tower applies layers outside-in, so the
//! last `.layer` runs FIRST: this function wraps LITERALLY EVERY request the app receives, regardless
//! of which nested/merged sub-router (if any) later handles it. It is NOT a router-nesting trick — it
//! does not care whether a matching route exists at all: it inspects the RAW request path BEFORE
//! axum's router attempts a match, so
//!   - a brand-new `/api/admin/*` route added anywhere (even a careless `.merge()` bypassing every
//!     existing nest) still passes through THIS function first, and
//!   - an entirely UNREGISTERED `/api/admin/*` probe still gets 401/403 here, never axum's default 404.
//!
//! Proven by attack: [`tests::admin_plane_gate_denies_unregistered_future_route`].
//!
//! ## RED LINE — no `PlatformAdminClaims` variant (Q1 / Q-ADMIN-NO-CLAIM)
//! Platform-admin is a SERVER-SIDE fact in the `platform_admins` allowlist, keyed on
//! `OwnerClaims.user_id`, re-read on every admin request — NOT a JWT role. The 3-role `Claims` enum
//! (`claims.rs`) stays 3-role; `unknown_role_is_rejected` already forbids a 4th. A courier/customer
//! token carries `sub` but no `user_id`, so it is not `Claims::Owner` → the gate 401s it exactly as
//! Node's "no resolvable userId → 401" (`platform-admin.ts:34-38`). Setting an owner's `revoked_at`
//! denies the NEXT request (immediate insider-removal, ADR-0004) because the allowlist is re-read,
//! never trusted from the token.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode, header::AUTHORIZATION};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use uuid::Uuid;

use super::claims::{Claims, OwnerClaims};
use super::jwt::JwtVerifier;

/// The exact allowlist point-read `PlatformAdminRepo` runs (`lib/platform-admin.ts:20`). A pure
/// constant so a unit test pins "does the SQL say what we think" without a live DB — the same
/// posture as `db::SET_TENANT_STATEMENT`. The table is non-tenant, NO-RLS (protected by GRANTs), so
/// this read returns identical rows under BYPASSRLS (today) and NOBYPASSRLS (post-B3) — genuinely
/// B3-independent, no GUC, no DEFINER fn (Q-ADMIN-NO-CLAIM clause 2). `platform_admins` is granted
/// SELECT-only to the operational role (mig `1790000000080`), so self-serve escalation via the app
/// role is structurally impossible (Q-ADMIN-GRANT — carried verbatim).
pub const PLATFORM_ADMIN_SELECT: &str =
    "SELECT 1 FROM platform_admins WHERE user_id = $1 AND revoked_at IS NULL";

/// A DB blip on the allowlist re-check. The gate maps it to **503, fail CLOSED** — never fail-open
/// at the top privilege tier (`platform-admin.ts:44-47`, Q-ADMIN-FAILCLOSED).
#[derive(Debug, thiserror::Error)]
#[error("platform-admin allowlist re-check failed")]
pub struct PlatformAdminDbError;

/// The `platform_admins` allowlist re-read authority (ports `isPlatformAdmin`, `platform-admin.ts:20`).
#[async_trait::async_trait]
pub trait PlatformAdminRepo: Send + Sync {
    /// `Ok(true)` = allowlisted + not revoked; `Ok(false)` = miss (→403); `Err` = DB blip (→503
    /// fail-closed). NEVER returns `Ok(false)` for a DB error — a swallowed error would fail OPEN.
    async fn is_platform_admin(&self, user_id: Uuid) -> Result<bool, PlatformAdminDbError>;
}

/// The cloneable gate context inserted as the outermost layer's `State` in `main.rs`. Holds the S2
/// JWT verifier (to authenticate the bearer at request time) + the allowlist repo. `Arc` everywhere
/// so cloning per request is cheap.
#[derive(Clone)]
pub struct PlatformAdminGateState {
    pub verifier: Arc<JwtVerifier>,
    pub repo: Arc<dyn PlatformAdminRepo>,
}

impl PlatformAdminGateState {
    pub fn new(verifier: Arc<JwtVerifier>, repo: Arc<dyn PlatformAdminRepo>) -> Self {
        PlatformAdminGateState { verifier, repo }
    }
}

/// The plane predicate (REV-S10-1). Node keys on the MATCHED pattern (`routeOptions.url`), immune to
/// case / `%2e` / `%2f` / trailing-slash / the `/api/administrators` lookalike by construction. This
/// gate runs BEFORE axum routing, so it must normalize the RAW path itself:
///   percent-decode (so `%2f`→`/`, `%2e`→`.`) → lowercase → collapse repeated slashes → match
///   `/api/admin` or `/api/admin/…`.
/// Normalization is deliberately CONSERVATIVE: it errs toward GATING (a non-admin path that decodes
/// to admin-looking gets a 401/403 instead of a 404 — no security loss, just a different deny code)
/// so that no REAL admin path can ever slip past to a handler ungated. The `/api/administrators`
/// lookalike is excluded because after `/api/admin` there must be either end-of-string or a `/`.
pub fn is_admin_scoped_path(raw_path: &str) -> bool {
    let decoded = percent_decode(raw_path);
    let lowered = decoded.to_ascii_lowercase();
    let collapsed = collapse_slashes(&lowered);
    collapsed == "/api/admin" || collapsed.starts_with("/api/admin/")
}

/// Decode `%XX` escapes to their byte values (so an attacker can't smuggle `/api/admin%2fsecret`
/// past the gate). Anything that is not a well-formed `%XX` passes through unchanged. Invalid UTF-8
/// after decoding is lossily replaced — irrelevant here (we only string-match an ASCII prefix).
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        // A well-formed `%XX`: parse the two hex chars as one byte (no `as` casts). i+2 < len
        // guarantees the `[i+1..i+3]` slice is in-bounds.
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
            {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Collapse runs of `/` into a single `/` (so `/api//admin/x` normalizes to `/api/admin/x`).
fn collapse_slashes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_slash = false;
    for c in s.chars() {
        if c == '/' {
            if !prev_slash {
                out.push(c);
            }
            prev_slash = true;
        } else {
            out.push(c);
            prev_slash = false;
        }
    }
    out
}

/// Pull a non-empty `Authorization: Bearer <token>` (mirrors `extractors::bearer_token`).
fn bearer(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let rest = value.strip_prefix("Bearer ")?;
    if rest.is_empty() {
        None
    } else {
        Some(rest.to_string())
    }
}

/// Verify the bearer and narrow to `OwnerClaims` — the ONLY variant carrying `user_id`. A
/// missing/invalid token OR a courier/customer token (no `user_id`) yields `None` → the gate 401s,
/// exactly as Node's `requirePlatformAdmin` 401s a request with no resolvable `userId`.
fn verify_owner(verifier: &JwtVerifier, headers: &HeaderMap) -> Option<OwnerClaims> {
    let token = bearer(headers)?;
    match verifier.verify(&token).ok()? {
        Claims::Owner(o) => Some(o),
        _ => None,
    }
}

/// The bare `{error}` deny shape (matches Node `platform-admin.ts` `reply.send({error})` — a bare
/// non-envelope body, NOT the ADR-0010 envelope, so the admin FE tolerates it identically).
fn deny(status: StatusCode, msg: &str) -> Response {
    (status, axum::Json(serde_json::json!({ "error": msg }))).into_response()
}

/// REV-S10-1 runtime plane-gate — see the module doc for the full rationale. Applied as the
/// OUTERMOST `.layer()` on the merged app in `main.rs` (`from_fn_with_state`).
///
/// Failure-first, in order (`platform-admin.ts` §3):
///   1. non-admin path        → `next.run` untouched (one normalize, no DB, no auth — hot-path cheap).
///   2. no verifiable owner    → **401** (courier/customer/no-token — "no resolvable userId").
///   3. allowlist miss         → **403**.
///   4. allowlist DB blip      → **503**, fail CLOSED.
///   5. allowlisted + !revoked → insert the verified `OwnerClaims` into extensions, `next.run`.
///
/// The verified `OwnerClaims` is handed to the admin handlers via extensions so the audit actor_id
/// (REV-S10-4) is the gate-verified user — it crosses no boundary un-carried and can never fall back
/// to a `'unknown'` string.
pub async fn platform_admin_plane_gate(
    State(state): State<PlatformAdminGateState>,
    mut request: Request,
    next: Next,
) -> Response {
    if !is_admin_scoped_path(request.uri().path()) {
        return next.run(request).await; // not admin — pass without friction
    }
    let owner = match verify_owner(&state.verifier, request.headers()) {
        Some(o) => o,
        None => return deny(StatusCode::UNAUTHORIZED, "Unauthorized"),
    };
    match state.repo.is_platform_admin(owner.user_id).await {
        Ok(true) => {
            request.extensions_mut().insert(owner);
            next.run(request).await
        }
        Ok(false) => deny(StatusCode::FORBIDDEN, "Forbidden"),
        Err(_) => deny(StatusCode::SERVICE_UNAVAILABLE, "admin_unavailable"),
    }
}

/// The runtime `sqlx` allowlist read (`PLATFORM_ADMIN_SELECT`). No `query!` macro (no compile-time
/// DATABASE_URL, per this crate's convention); a plain parameterized query. A DB error becomes
/// `PlatformAdminDbError` → the gate's 503 fail-closed. `#[ignore]`-probed (needs a live Postgres);
/// the gate LOGIC is fully unit-tested via the fake below.
pub struct PgPlatformAdminRepo {
    pool: sqlx::PgPool,
}

impl PgPlatformAdminRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        PgPlatformAdminRepo { pool }
    }
}

#[async_trait::async_trait]
impl PlatformAdminRepo for PgPlatformAdminRepo {
    async fn is_platform_admin(&self, user_id: Uuid) -> Result<bool, PlatformAdminDbError> {
        let row: Option<(i32,)> = sqlx::query_as(PLATFORM_ADMIN_SELECT)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|_e| PlatformAdminDbError)?; // DB blip → 503 fail CLOSED (never Ok(false))
        Ok(row.is_some())
    }
}

#[cfg(test)]
pub(crate) mod fake {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Test double for the allowlist. `admins` maps user_id → allowlisted; `fail` forces a DB blip
    /// (to exercise the 503 fail-closed path).
    #[derive(Default)]
    pub struct FakePlatformAdminRepo {
        pub admins: Mutex<HashMap<Uuid, bool>>,
        pub fail: Mutex<bool>,
    }

    #[async_trait::async_trait]
    impl PlatformAdminRepo for FakePlatformAdminRepo {
        async fn is_platform_admin(&self, user_id: Uuid) -> Result<bool, PlatformAdminDbError> {
            if *self.fail.lock().unwrap() {
                return Err(PlatformAdminDbError);
            }
            Ok(*self.admins.lock().unwrap().get(&user_id).unwrap_or(&false))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::FakePlatformAdminRepo;
    use super::*;
    use crate::auth::AuthState;
    use crate::auth::claims::{Claims, CustomerClaims, OwnerClaims};
    use crate::auth::repo::fake::FakeAuthRepo;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use axum::routing::get;
    use tower::ServiceExt as _;

    // ── pure normalization coverage (the %2e/%2f/case/collapse/lookalike immunity) ──

    #[test]
    fn admin_scoped_path_matches_admin_and_children_only() {
        assert!(is_admin_scoped_path("/api/admin"));
        assert!(is_admin_scoped_path("/api/admin/backups"));
        assert!(is_admin_scoped_path("/api/admin/backups/verify"));
        // NOT admin
        assert!(!is_admin_scoped_path("/api/administrators")); // the lookalike (boundary excludes it)
        assert!(!is_admin_scoped_path("/api/adminx"));
        assert!(!is_admin_scoped_path("/api/owner/products"));
        assert!(!is_admin_scoped_path("/healthz"));
    }

    #[test]
    fn admin_scoped_path_is_case_slash_and_percent_immune() {
        assert!(is_admin_scoped_path("/API/ADMIN/backups"), "case");
        assert!(is_admin_scoped_path("/api//admin/backups"), "double slash");
        assert!(is_admin_scoped_path("/api/admin%2fbackups"), "%2f -> /");
        assert!(is_admin_scoped_path("/api/%61dmin/backups"), "%61 -> a");
        // /api/administrators must NOT be gated even after normalization tricks.
        assert!(!is_admin_scoped_path("/API/ADMINISTRATORS"));
    }

    fn gate_state(auth: &AuthState, repo: Arc<dyn PlatformAdminRepo>) -> PlatformAdminGateState {
        PlatformAdminGateState::new(auth.verifier.clone(), repo)
    }

    /// Build a router whose ONLY protection is the plane-gate as the outermost layer — mirroring
    /// `main.rs`. A sentinel handler that would 200 if ever reached proves the gate short-circuits.
    fn gated_app(state: PlatformAdminGateState, route: &str) -> axum::Router {
        axum::Router::new()
            .route(route, get(|| async { "REACHED-HANDLER" }))
            .layer(axum::middleware::from_fn_with_state(
                state,
                platform_admin_plane_gate,
            ))
    }

    async fn send(app: axum::Router, uri: &str, bearer: Option<&str>) -> (StatusCode, String) {
        let mut req = Request::builder().method("GET").uri(uri);
        if let Some(t) = bearer {
            req = req.header(AUTHORIZATION, format!("Bearer {t}"));
        }
        let resp = app.oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    // ── REV-S10-1 ATTACK-PROOF (the DoD): a NEW/unknown /api/admin route with NO per-route gate
    //    still 401/403s via the top layer — closure is RUNTIME, not router-nesting. ──

    #[tokio::test]
    async fn admin_plane_gate_denies_unregistered_future_route() {
        let auth = AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let repo = Arc::new(FakePlatformAdminRepo::default());
        let state = gate_state(&auth, repo);

        // (a) a REGISTERED-but-ungated sibling admin route (the "future feature added via .merge()"
        //     that carries no per-route gate of its own) — reached with no bearer → 401, NEVER the
        //     handler body.
        let app = gated_app(state.clone(), "/api/admin/brand-new-future-route");
        let (status, body) = send(app, "/api/admin/brand-new-future-route", None).await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "an ungated future /api/admin route must still be denied by the runtime top layer"
        );
        assert!(
            !body.contains("REACHED-HANDLER"),
            "the gate must short-circuit BEFORE the handler runs"
        );

        // (b) an ENTIRELY UNREGISTERED /api/admin path — the gate fires before routing, so it is
        //     401 (not axum's default 404). This is the property a router-nest+lint cannot give.
        let app2 = gated_app(state, "/healthz"); // no /api/admin route registered at all
        let (status2, _) = send(app2, "/api/admin/does-not-exist", None).await;
        assert_eq!(
            status2,
            StatusCode::UNAUTHORIZED,
            "an unregistered /api/admin path must be gated (401), never fall through to 404"
        );
    }

    #[tokio::test]
    async fn owner_not_on_allowlist_cannot_mint_admin_403() {
        // A cryptographically-valid OWNER token whose user_id is NOT on the allowlist → 403. This is
        // "owner-cannot-mint-admin": there is no token role that grants platform-admin; only the
        // server-side allowlist does.
        let auth = AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let repo = Arc::new(FakePlatformAdminRepo::default()); // empty → every owner is a miss
        let state = gate_state(&auth, repo);
        let token = auth
            .verifier
            .mint(Claims::Owner(OwnerClaims::new(Uuid::new_v4(), None)), 3600)
            .unwrap();
        let app = gated_app(state, "/api/admin/backups");
        let (status, body) = send(app, "/api/admin/backups", Some(&token)).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert!(!body.contains("REACHED-HANDLER"));
    }

    #[tokio::test]
    async fn allowlisted_owner_passes_and_owner_reaches_handler() {
        let auth = AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let user_id = Uuid::new_v4();
        let repo = Arc::new(FakePlatformAdminRepo::default());
        repo.admins.lock().unwrap().insert(user_id, true);
        let state = gate_state(&auth, repo);
        let token = auth
            .verifier
            .mint(Claims::Owner(OwnerClaims::new(user_id, None)), 3600)
            .unwrap();
        let app = gated_app(state, "/api/admin/backups");
        let (status, body) = send(app, "/api/admin/backups", Some(&token)).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("REACHED-HANDLER"));
    }

    #[tokio::test]
    async fn customer_token_is_401_no_user_id() {
        // A customer token carries `sub` but no `user_id` → not Claims::Owner → 401 (never 403/200).
        // This is the structural closure of owner→admin AND courier/customer→admin escalation.
        let auth = AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let repo = Arc::new(FakePlatformAdminRepo::default());
        let state = gate_state(&auth, repo);
        let token = auth
            .verifier
            .mint(
                Claims::Customer(CustomerClaims::new(
                    Uuid::new_v4(),
                    Uuid::new_v4(),
                    Uuid::new_v4(),
                )),
                3600,
            )
            .unwrap();
        let app = gated_app(state, "/api/admin/backups");
        let (status, _) = send(app, "/api/admin/backups", Some(&token)).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn db_blip_on_allowlist_fails_closed_503() {
        // Fail CLOSED at the top tier — a DB error on the allowlist re-check is 503, NEVER a
        // fail-open pass. (Q-ADMIN-FAILCLOSED.)
        let auth = AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let user_id = Uuid::new_v4();
        let repo = Arc::new(FakePlatformAdminRepo::default());
        repo.admins.lock().unwrap().insert(user_id, true); // even an allowlisted user...
        *repo.fail.lock().unwrap() = true; // ...gets 503 when the re-check DB errors.
        let state = gate_state(&auth, repo);
        let token = auth
            .verifier
            .mint(Claims::Owner(OwnerClaims::new(user_id, None)), 3600)
            .unwrap();
        let app = gated_app(state, "/api/admin/backups");
        let (status, body) = send(app, "/api/admin/backups", Some(&token)).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(!body.contains("REACHED-HANDLER"));
    }

    #[tokio::test]
    async fn non_admin_path_passes_untouched_without_bearer() {
        // A public route is not gated — no bearer needed, handler reached.
        let auth = AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let repo = Arc::new(FakePlatformAdminRepo::default());
        let state = gate_state(&auth, repo);
        let app = gated_app(state, "/healthz");
        let (status, body) = send(app, "/healthz", None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("REACHED-HANDLER"));
    }

    #[test]
    fn allowlist_sql_is_the_revoke_aware_point_read() {
        // Pin the exact SQL (revoked_at IS NULL = immediate insider-removal; keyed on user_id).
        assert_eq!(
            PLATFORM_ADMIN_SELECT,
            "SELECT 1 FROM platform_admins WHERE user_id = $1 AND revoked_at IS NULL"
        );
    }
}
