//! S10 Plane B — the acquisition-ops surface (`/internal/acquisition/*`, 9 routes). Ports
//! `apps/api/src/modules/acquisition/{route,ops-auth,provisioning,claim,retention}.ts`. Docs:
//! `docs/design/rebuild-platform-admin-s10-council/`.
//!
//! ## Auth: the ops-secret gate — a DISTINCT gate, deliberately decoupled (Q-PROVISION-SECRET)
//! Gated SOLELY by the `x-provision-ops-secret` header (timing-safe, fail-closed-404 when unset).
//! This is a SECOND, DISTINCT authority from the platform-admin allowlist AND the dev-login family
//! (B4): enabling provisioning in prod must NOT re-arm the mock-auth owner-JWT backdoor (ADR-0003).
//! The secret is read from env (NOT the config Zod schema) so it composes without touching the
//! dev-bypass prod-offenders guard. Unlike the platform-admin plane-gate (a ROOT layer, REV-S10-1,
//! because a future `/api/admin/*` route must not escape), this gate is correctly a ROUTER-scoped
//! `.layer()` — the faithful port of Node's PLUGIN-scoped `onRequest` hook (`route.ts:56`), not a
//! root-instance one; any future `/internal/acquisition/*` route is added to THIS router and inherits
//! the layer.
//!
//! ## Carried verbatim (verified sound — no re-litigation)
//!  - **Q-PROVISION-SECRET** — [`provision_ops_authorized`]: `subtle` constant-time compare, length
//!    pre-check, fail-closed `false` (→404) when the secret is unset/empty.
//!  - **Q-PROVISION-RLS** — the shadow-spine ORDERING + `app.provision_token` GUC dance
//!    ([`PROVISION_SPINE_ORDER`], [`PgAcquisitionRepo::provision_shadow_spine`]): set_config(token,
//!    txn-local) → FOR UPDATE grant → inserts admitted by the `provision_shadow` policy → state-pinned
//!    advance(ENRICHED→PROVISIONED) → consume the grant LAST. Load-bearing: a context-free/out-of-order
//!    port matches 0 rows or admits a 2nd concurrent spine.
//!  - **Q-SHADOW-ERASE** — `hard_delete_shadow` NULLs `place_raw`/`menu_draft` + erases the shadow via
//!    the `erase_shadow_tenant` DEFINER whose `owner_id IS NULL` guard stops a CLAIMED tenant from
//!    being erased.
//!  - **Art-14 (counsel CC1)** — [`build_art14_notice`]: the hostile-recipient first-contact notice
//!    with an EQUALLY-prominent one-click decline-and-erase (no registration). Decline prominence is
//!    preserved (erase stays as easy as claim).
//!
//! ## No live DB in this sandbox
//! `PgAcquisitionRepo` SQL methods are `#[ignore]`-probed; the pure controls (ops-secret timing-safe
//! fail-closed, the spine ordering text, the Art-14 decline prominence) are unit-tested here.
//!
//! ## `dead_code` allowance (dark-surface scope — same rationale as `auth/mod.rs`)
//! `PROVISION_SPINE_ORDER` is the pinned Q-PROVISION-RLS ordering documentation, asserted by a unit
//! test; the Pg repo is bound at the `main.rs` dark mount. Scoped to this module only.
#![allow(
    dead_code,
    reason = "dark S10 acquisition-ops surface — pinned ordering constant + forward-looking API bound by tests/launch-wiring"
)]

use std::sync::Arc;

use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::config::Secret;

const OPS_HEADER: &str = "x-provision-ops-secret";

/// Q-PROVISION-SECRET — the ops-secret decision (`ops-auth.ts:17-29`). Pure so every branch is a
/// unit test. `subtle` constant-time compare gated on a length pre-check FIRST (the same two-step
/// shape `crypto.timingSafeEqual` uses); an UNSET/empty configured secret → `false` (fail-closed,
/// the whole surface then 404s — existence hidden), the safe default on any box that hasn't opted in.
pub fn provision_ops_authorized(provided: Option<&str>, secret: Option<&str>) -> bool {
    let secret = match secret {
        Some(s) if !s.is_empty() => s,
        _ => return false, // disabled / fail-closed
    };
    let provided = match provided {
        Some(p) if !p.is_empty() => p,
        _ => return false,
    };
    if provided.len() != secret.len() {
        return false;
    }
    provided.as_bytes().ct_eq(secret.as_bytes()).into()
}

/// The ops-secret router layer state (the configured secret, env-sourced).
#[derive(Clone)]
pub struct OpsGateState {
    pub secret: Option<Arc<Secret>>,
}

/// The ops-secret gate middleware — applied as a `.layer()` on the acquisition router. Fail-closed
/// 404 (hide existence) unless the header matches the configured secret.
pub async fn ops_secret_gate(
    State(gate): State<OpsGateState>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let provided = request
        .headers()
        .get(OPS_HEADER)
        .and_then(|v| v.to_str().ok());
    if provision_ops_authorized(provided, gate.secret.as_deref().map(Secret::expose)) {
        next.run(request).await
    } else {
        (
            StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({ "error": "NOT_FOUND" })),
        )
            .into_response()
    }
}

// ─────────────────────────── Q-PROVISION-RLS: the pinned spine ordering ───────────────────────────

/// Q-PROVISION-RLS — the load-bearing ORDER of the shadow-spine transaction (`provisioning.ts:148-198`).
/// Pinned as a constant so a unit test proves the ordering is the dedup chokepoint even without a live
/// DB (the `db::SET_TENANT_STATEMENT` posture). Out-of-order = matches 0 rows OR admits a 2nd spine.
pub const PROVISION_SPINE_ORDER: &[&str] = &[
    "SELECT set_config('app.provision_token', $token, true)", // txn-local GUC the policy reads
    "SELECT id FROM provision_grants WHERE token_hash = encode(sha256($token::bytea),'hex') AND consumed_at IS NULL AND expires_at > now() FOR UPDATE",
    "INSERT INTO organizations (id, name, owner_id) VALUES ($orgId, $name, NULL)", // owner_id NULL shadow
    "INSERT INTO locations (id, org_id, slug, name, phone, status, published_at) VALUES (..., 'closed', NULL)", // non-live
    "INSERT INTO menu_versions (location_id, version) VALUES ($locationId, 1)",
    "advance(ENRICHED -> PROVISIONED)", // state-pinned; a racing 2nd runner gets 0 rows -> ROLLBACK
    "UPDATE provision_grants SET consumed_at = now() WHERE ... AND consumed_at IS NULL", // consume LAST
];

// ─────────────────────────── Art-14 hostile-recipient notice (CC1) ───────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct Art14Notice {
    pub subject: String,
    pub body: String,
}

/// Port of `buildArt14Notice` (`claim.ts:174-200`). Written for the HOSTILE recipient, NOT a
/// growth-hack CTA: honest identity/purpose/source/retention/rights AND an EQUALLY-prominent
/// one-click decline-and-erase (no registration). Counsel CC1 — decline stays as easy as claim.
pub fn build_art14_notice(preview_url: &str, claim_url: &str, decline_url: &str) -> Art14Notice {
    Art14Notice {
        subject:
            "We built a preview of your restaurant from your public website — your options inside"
                .to_string(),
        body: format!(
            "We built a non-live PREVIEW of your menu from your restaurant's PUBLIC website and public \
Google Places listing. It is NOT a live store and cannot take orders.\n\n\
Preview: {preview_url}\n\n\
Your options, both one click:\n\
  • CLAIM it (free) — review, correct, and decide whether to go live: {claim_url}\n\
  • DELETE it — remove the preview and erase the data we used, no account needed: {decline_url}\n\n\
Your rights: access, correct, or erase this data, and complain to your data protection authority. \
If you do nothing, the preview is automatically deleted shortly."
        ),
    }
}

// ─────────────────────────── repo (SQL ordering carried; Pg #[ignore]) ───────────────────────────

/// Provisioning/claim errors → the wire codes the Node routes return (409 for conflicts, etc.).
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AcquisitionError {
    #[error("{0}")]
    Conflict(String),
    #[error("db error")]
    Db,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct SpineResult {
    pub org_id: Uuid,
    pub location_id: Uuid,
}

#[async_trait::async_trait]
pub trait AcquisitionRepo: Send + Sync {
    async fn create_source(&self, place_id: &str) -> Result<Uuid, AcquisitionError>;
    async fn mint_provision_token(&self, source_id: Uuid) -> Result<String, AcquisitionError>;
    /// Q-PROVISION-RLS — the spine written THROUGH the `provision_shadow` policy in the pinned order.
    async fn provision_shadow_spine(
        &self,
        source_id: Uuid,
        token: &str,
        name: &str,
        slug: &str,
        phone: Option<&str>,
    ) -> Result<SpineResult, AcquisitionError>;
    /// Q-SHADOW-ERASE — NULLs place_raw/menu_draft + erase_shadow_tenant (owner_id IS NULL guard).
    async fn hard_delete_shadow(&self, source_id: Uuid) -> Result<(), AcquisitionError>;
    async fn mark_verified(&self, source_id: Uuid) -> Result<(), AcquisitionError>;
    async fn mint_claim_invite(
        &self,
        source_id: Uuid,
        invited_contact: Option<&str>,
    ) -> Result<String, AcquisitionError>;
    async fn retention_sweep(&self) -> Result<u64, AcquisitionError>;
}

#[derive(Clone)]
pub struct AcquisitionState {
    pub repo: Arc<dyn AcquisitionRepo>,
    /// APP_BASE_URL for the claim/decline links in the Art-14 notice.
    pub base_url: String,
}

/// Assemble the 9-route acquisition surface behind the ops-secret gate. The gate is a router layer
/// (Node's plugin `onRequest`, `route.ts:56`), applied by `mount_acquisition_router` in `main.rs`.
pub fn acquisition_router(state: AcquisitionState) -> axum::Router {
    use axum::routing::post;
    axum::Router::new()
        .route("/internal/acquisition", post(create))
        .route("/internal/acquisition/extract", post(extract))
        .route("/internal/acquisition/provision/mint", post(provision_mint))
        .route(
            "/internal/acquisition/provision/spine",
            post(provision_spine),
        )
        .route(
            "/internal/acquisition/provision/hard-delete",
            post(provision_hard_delete),
        )
        .route("/internal/acquisition/claim/verify", post(claim_verify))
        .route("/internal/acquisition/claim/mint", post(claim_mint))
        .route("/internal/acquisition/complaint", post(complaint))
        .route(
            "/internal/acquisition/retention/sweep",
            post(retention_sweep),
        )
        .layer(axum::Extension(state))
}

fn ok_json(status: StatusCode, v: serde_json::Value) -> Response {
    (status, axum::Json(v)).into_response()
}
fn bad(code: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        axum::Json(serde_json::json!({ "error": code })),
    )
        .into_response()
}
fn conflict(code: &str) -> Response {
    (
        StatusCode::CONFLICT,
        axum::Json(serde_json::json!({ "error": code })),
    )
        .into_response()
}

fn valid_slug(s: &str) -> bool {
    s.len() >= 2
        && s.len() <= 100
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// `POST /internal/acquisition` — idempotent create-source from a place_id (`route.ts:62`).
#[utoipa::path(post, path = "/internal/acquisition", tag = "acquisition",
    responses((status = 201, description = "source"), (status = 400, description = "VALIDATION_FAILED"), (status = 404, description = "ops gate")))]
pub async fn create(
    Extension(state): Extension<AcquisitionState>,
    body: axum::Json<serde_json::Value>,
) -> Response {
    let place_id = body
        .0
        .get("place_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .unwrap_or("");
    if place_id.is_empty() || place_id.len() > 512 {
        return bad("VALIDATION_FAILED");
    }
    match state.repo.create_source(place_id).await {
        Ok(id) => ok_json(StatusCode::CREATED, serde_json::json!({ "id": id })),
        Err(_) => conflict("CREATE_FAILED"),
    }
}

/// `POST /internal/acquisition/extract` — SSRF-guarded locate → AI-parse. Dark: the parser seam is
/// unwired in this port → 503 EXTRACTION_UNAVAILABLE (mirrors `route.ts:83` when no parser is bound).
#[utoipa::path(post, path = "/internal/acquisition/extract", tag = "acquisition",
    responses((status = 503, description = "extraction unavailable (parser unwired)"), (status = 400, description = "VALIDATION_FAILED")))]
pub async fn extract(body: axum::Json<serde_json::Value>) -> Response {
    if body
        .0
        .get("acquisition_source_id")
        .and_then(|v| v.as_str())
        .is_none()
    {
        return bad("VALIDATION_FAILED");
    }
    (
        StatusCode::SERVICE_UNAVAILABLE,
        axum::Json(serde_json::json!({ "error": "EXTRACTION_UNAVAILABLE" })),
    )
        .into_response()
}

fn parse_source_id(body: &serde_json::Value) -> Option<Uuid> {
    body.get("acquisition_source_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
}

/// `POST /internal/acquisition/provision/mint` — mint a single-use provisioning token (`route.ts:90`).
#[utoipa::path(post, path = "/internal/acquisition/provision/mint", tag = "acquisition",
    responses((status = 201, description = "token"), (status = 400, description = "VALIDATION_FAILED"), (status = 409, description = "ACTIVE_GRANT_EXISTS")))]
pub async fn provision_mint(
    Extension(state): Extension<AcquisitionState>,
    body: axum::Json<serde_json::Value>,
) -> Response {
    let Some(source_id) = parse_source_id(&body.0) else {
        return bad("VALIDATION_FAILED");
    };
    match state.repo.mint_provision_token(source_id).await {
        Ok(token) => ok_json(StatusCode::CREATED, serde_json::json!({ "token": token })),
        Err(AcquisitionError::Conflict(c)) => conflict(&c),
        Err(_) => conflict("MINT_FAILED"),
    }
}

/// `POST /internal/acquisition/provision/spine` — write the shadow spine THROUGH the provision_shadow
/// RLS policy (Q-PROVISION-RLS, one tx, consume-LAST) (`route.ts:107`).
#[utoipa::path(post, path = "/internal/acquisition/provision/spine", tag = "acquisition",
    responses((status = 201, description = "spine"), (status = 400, description = "VALIDATION_FAILED"), (status = 409, description = "INVALID_OR_EXPIRED_TOKEN")))]
pub async fn provision_spine(
    Extension(state): Extension<AcquisitionState>,
    body: axum::Json<serde_json::Value>,
) -> Response {
    let b = &body.0;
    let Some(source_id) = parse_source_id(b) else {
        return bad("VALIDATION_FAILED");
    };
    let token = b.get("token").and_then(|v| v.as_str()).unwrap_or("");
    let name = b
        .get("name")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .unwrap_or("");
    let slug = b
        .get("slug")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .unwrap_or("");
    let phone = b.get("phone").and_then(|v| v.as_str());
    if token.is_empty()
        || token.len() > 256
        || name.is_empty()
        || name.len() > 256
        || !valid_slug(slug)
    {
        return bad("VALIDATION_FAILED");
    }
    match state
        .repo
        .provision_shadow_spine(source_id, token, name, slug, phone)
        .await
    {
        Ok(r) => ok_json(
            StatusCode::CREATED,
            serde_json::json!({ "org_id": r.org_id, "location_id": r.location_id }),
        ),
        Err(AcquisitionError::Conflict(c)) => conflict(&c),
        Err(_) => conflict("SPINE_FAILED"),
    }
}

/// `POST /internal/acquisition/provision/hard-delete` — born-with-erasure day-one delete (`route.ts:130`).
#[utoipa::path(post, path = "/internal/acquisition/provision/hard-delete", tag = "acquisition",
    responses((status = 200, description = "deleted"), (status = 400, description = "VALIDATION_FAILED")))]
pub async fn provision_hard_delete(
    Extension(state): Extension<AcquisitionState>,
    body: axum::Json<serde_json::Value>,
) -> Response {
    let Some(source_id) = parse_source_id(&body.0) else {
        return bad("VALIDATION_FAILED");
    };
    match state.repo.hard_delete_shadow(source_id).await {
        Ok(()) => ok_json(StatusCode::OK, serde_json::json!({ "deleted": true })),
        Err(_) => conflict("DELETE_FAILED"),
    }
}

/// `POST /internal/acquisition/claim/verify` — PROVISIONED→VERIFIED (`route.ts:142`).
#[utoipa::path(post, path = "/internal/acquisition/claim/verify", tag = "acquisition",
    responses((status = 200, description = "verified"), (status = 400, description = "VALIDATION_FAILED"), (status = 409, description = "NOT_VERIFIABLE")))]
pub async fn claim_verify(
    Extension(state): Extension<AcquisitionState>,
    body: axum::Json<serde_json::Value>,
) -> Response {
    let Some(source_id) = parse_source_id(&body.0) else {
        return bad("VALIDATION_FAILED");
    };
    match state.repo.mark_verified(source_id).await {
        Ok(()) => ok_json(StatusCode::OK, serde_json::json!({ "verified": true })),
        Err(AcquisitionError::Conflict(c)) => conflict(&c),
        Err(_) => conflict("VERIFY_FAILED"),
    }
}

/// `POST /internal/acquisition/claim/mint` — mint a single-use claim invite + the Art-14 notice
/// (`route.ts:159`). The notice carries the EQUALLY-prominent decline link (counsel CC1).
#[utoipa::path(post, path = "/internal/acquisition/claim/mint", tag = "acquisition",
    responses((status = 201, description = "invite + Art-14 notice"), (status = 400, description = "VALIDATION_FAILED"), (status = 409, description = "NOT_OFFERABLE")))]
pub async fn claim_mint(
    Extension(state): Extension<AcquisitionState>,
    body: axum::Json<serde_json::Value>,
) -> Response {
    let b = &body.0;
    let Some(source_id) = parse_source_id(b) else {
        return bad("VALIDATION_FAILED");
    };
    let invited_contact = b.get("invited_contact").and_then(|v| v.as_str());
    match state
        .repo
        .mint_claim_invite(source_id, invited_contact)
        .await
    {
        Ok(token) => {
            let base = &state.base_url;
            // §6 token-safe transport: the token rides the URL FRAGMENT (#token=), never the query.
            let notice = build_art14_notice(
                &format!("{base}/claim?preview={source_id}"),
                &format!("{base}/claim#token={token}"),
                &format!("{base}/claim#token={token}"),
            );
            ok_json(
                StatusCode::CREATED,
                serde_json::json!({ "token": token, "notice": notice }),
            )
        }
        Err(AcquisitionError::Conflict(c)) => conflict(&c),
        Err(_) => conflict("MINT_FAILED"),
    }
}

/// `POST /internal/acquisition/complaint` — record a C&D (structured-log health signal) (`route.ts:186`).
#[utoipa::path(post, path = "/internal/acquisition/complaint", tag = "acquisition",
    responses((status = 200, description = "recorded"), (status = 400, description = "VALIDATION_FAILED")))]
pub async fn complaint(body: axum::Json<serde_json::Value>) -> Response {
    let place_id = body
        .0
        .get("place_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .unwrap_or("");
    if place_id.is_empty() || place_id.len() > 512 {
        return bad("VALIDATION_FAILED");
    }
    // CC4: a complaint is a structured-log health signal (decline-WITHOUT-complaint is the key metric).
    tracing::info!(event = "acquisition.complaint", place_id = %place_id, "complaint recorded");
    ok_json(StatusCode::OK, serde_json::json!({ "recorded": true }))
}

/// `POST /internal/acquisition/retention/sweep` — GDPR Art-5(e) reaper (`route.ts:199`).
#[utoipa::path(post, path = "/internal/acquisition/retention/sweep", tag = "acquisition",
    responses((status = 200, description = "swept")))]
pub async fn retention_sweep(Extension(state): Extension<AcquisitionState>) -> Response {
    match state.repo.retention_sweep().await {
        Ok(n) => ok_json(StatusCode::OK, serde_json::json!({ "reaped": n })),
        Err(_) => conflict("SWEEP_FAILED"),
    }
}

// ─────────────────────────── Pg impl (SQL carried; #[ignore]) ───────────────────────────

pub struct PgAcquisitionRepo {
    pool: sqlx::PgPool,
}

impl PgAcquisitionRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        PgAcquisitionRepo { pool }
    }
}

#[async_trait::async_trait]
impl AcquisitionRepo for PgAcquisitionRepo {
    async fn create_source(&self, place_id: &str) -> Result<Uuid, AcquisitionError> {
        // Idempotent: a repeat place_id returns the existing lifecycle row (never a 2nd).
        let row: (Uuid,) = sqlx::query_as(
            "INSERT INTO acquisition_sources (place_id, state) VALUES ($1, 'SOURCED')
             ON CONFLICT (place_id) DO UPDATE SET place_id = EXCLUDED.place_id RETURNING id",
        )
        .bind(place_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|_e| AcquisitionError::Db)?;
        Ok(row.0)
    }

    async fn mint_provision_token(&self, source_id: Uuid) -> Result<String, AcquisitionError> {
        let token = hex_token();
        sqlx::query(
            "INSERT INTO provision_grants (acquisition_source_id, token_hash, expires_at)
             VALUES ($1, $2, now() + interval '5 minutes')",
        )
        .bind(source_id)
        .bind(sha256_hex(&token))
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.as_database_error().and_then(|d| d.code()).as_deref() == Some("23505") {
                AcquisitionError::Conflict("ACTIVE_GRANT_EXISTS".to_string())
            } else {
                AcquisitionError::Db
            }
        })?;
        Ok(token)
    }

    async fn provision_shadow_spine(
        &self,
        source_id: Uuid,
        token: &str,
        name: &str,
        slug: &str,
        phone: Option<&str>,
    ) -> Result<SpineResult, AcquisitionError> {
        // Q-PROVISION-RLS: the EXACT ordering in PROVISION_SPINE_ORDER — set_config(token,txn-local)
        // → FOR UPDATE grant → INSERT org/location/menu_versions (admitted by provision_shadow) →
        // advance(ENRICHED->PROVISIONED) [state-pinned] → consume the grant LAST. Any 0-row → ROLLBACK.
        let mut tx = self.pool.begin().await.map_err(|_e| AcquisitionError::Db)?;
        sqlx::query("SELECT set_config('app.provision_token', $1, true)")
            .bind(token)
            .execute(&mut *tx)
            .await
            .map_err(|_e| AcquisitionError::Db)?;
        let grant: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM provision_grants
              WHERE token_hash = encode(sha256($1::bytea),'hex') AND consumed_at IS NULL AND expires_at > now()
              FOR UPDATE",
        )
        .bind(token)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_e| AcquisitionError::Db)?;
        if grant.is_none() {
            return Err(AcquisitionError::Conflict(
                "INVALID_OR_EXPIRED_TOKEN".to_string(),
            ));
        }
        let org_id = Uuid::new_v4();
        let location_id = Uuid::new_v4();
        sqlx::query("INSERT INTO organizations (id, name, owner_id) VALUES ($1, $2, NULL)")
            .bind(org_id)
            .bind(format!("{name} Org"))
            .execute(&mut *tx)
            .await
            .map_err(|_e| AcquisitionError::Db)?;
        sqlx::query(
            "INSERT INTO locations (id, org_id, slug, name, phone, status, published_at, widget_enabled, delivery_fee_flat)
             VALUES ($1, $2, $3, $4, $5, 'closed', NULL, false, 0)",
        )
        .bind(location_id)
        .bind(org_id)
        .bind(slug)
        .bind(name)
        .bind(phone.unwrap_or(""))
        .execute(&mut *tx)
        .await
        .map_err(|_e| AcquisitionError::Db)?;
        sqlx::query("INSERT INTO menu_versions (location_id, version) VALUES ($1, 1)")
            .bind(location_id)
            .execute(&mut *tx)
            .await
            .map_err(|_e| AcquisitionError::Db)?;
        // state-pinned advance ENRICHED->PROVISIONED (a racing runner gets 0 rows -> ROLLBACK).
        let advanced = sqlx::query(
            "UPDATE acquisition_sources SET state = 'PROVISIONED', org_id = $2, location_id = $3
              WHERE id = $1 AND state = 'ENRICHED'",
        )
        .bind(source_id)
        .bind(org_id)
        .bind(location_id)
        .execute(&mut *tx)
        .await
        .map_err(|_e| AcquisitionError::Db)?;
        if advanced.rows_affected() == 0 {
            return Err(AcquisitionError::Conflict("ILLEGAL_TRANSITION".to_string()));
        }
        // consume the grant LAST (single-use; 0 rows -> already consumed -> ROLLBACK).
        let consumed = sqlx::query(
            "UPDATE provision_grants SET consumed_at = now()
              WHERE token_hash = encode(sha256($1::bytea),'hex') AND consumed_at IS NULL",
        )
        .bind(token)
        .execute(&mut *tx)
        .await
        .map_err(|_e| AcquisitionError::Db)?;
        if consumed.rows_affected() == 0 {
            return Err(AcquisitionError::Conflict(
                "TOKEN_ALREADY_CONSUMED".to_string(),
            ));
        }
        tx.commit().await.map_err(|_e| AcquisitionError::Db)?;
        Ok(SpineResult {
            org_id,
            location_id,
        })
    }

    async fn hard_delete_shadow(&self, source_id: Uuid) -> Result<(), AcquisitionError> {
        // Q-SHADOW-ERASE: NULL place_raw+menu_draft, drop FK links, erase via erase_shadow_tenant
        // (owner_id IS NULL guard inside the DEFINER — a CLAIMED tenant is never erased here).
        let mut tx = self.pool.begin().await.map_err(|_e| AcquisitionError::Db)?;
        let src: Option<(Option<Uuid>, Option<Uuid>)> =
            sqlx::query_as("SELECT org_id, location_id FROM acquisition_sources WHERE id = $1")
                .bind(source_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|_e| AcquisitionError::Db)?;
        sqlx::query(
            "UPDATE acquisition_sources SET org_id = NULL, location_id = NULL, place_raw = NULL, menu_draft = NULL WHERE id = $1",
        )
        .bind(source_id)
        .execute(&mut *tx)
        .await
        .map_err(|_e| AcquisitionError::Db)?;
        sqlx::query("DELETE FROM provision_grants WHERE acquisition_source_id = $1")
            .bind(source_id)
            .execute(&mut *tx)
            .await
            .map_err(|_e| AcquisitionError::Db)?;
        if let Some((org_id, location_id)) = src {
            sqlx::query("SELECT erase_shadow_tenant($1, $2)")
                .bind(location_id)
                .bind(org_id)
                .execute(&mut *tx)
                .await
                .map_err(|_e| AcquisitionError::Db)?;
        }
        tx.commit().await.map_err(|_e| AcquisitionError::Db)?;
        Ok(())
    }

    async fn mark_verified(&self, source_id: Uuid) -> Result<(), AcquisitionError> {
        let n = sqlx::query(
            "UPDATE acquisition_sources SET state = 'VERIFIED'
              WHERE id = $1 AND state = 'PROVISIONED' AND org_id IS NOT NULL AND location_id IS NOT NULL",
        )
        .bind(source_id)
        .execute(&self.pool)
        .await
        .map_err(|_e| AcquisitionError::Db)?;
        if n.rows_affected() == 0 {
            return Err(AcquisitionError::Conflict("NOT_VERIFIABLE".to_string()));
        }
        Ok(())
    }

    async fn mint_claim_invite(
        &self,
        source_id: Uuid,
        invited_contact: Option<&str>,
    ) -> Result<String, AcquisitionError> {
        let token = hex_token();
        let contact_hash = invited_contact.map(|c| sha256_hex(c.trim().to_lowercase().as_str()));
        sqlx::query(
            "INSERT INTO claim_invites (acquisition_source_id, token_hash, invited_contact_hash, expires_at)
             VALUES ($1, $2, $3, now() + interval '72 hours')",
        )
        .bind(source_id)
        .bind(sha256_hex(&token))
        .bind(contact_hash)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.as_database_error().and_then(|d| d.code()).as_deref() == Some("23505") {
                AcquisitionError::Conflict("ACTIVE_INVITE_EXISTS".to_string())
            } else {
                AcquisitionError::Db
            }
        })?;
        Ok(token)
    }

    async fn retention_sweep(&self) -> Result<u64, AcquisitionError> {
        let n = sqlx::query(
            "DELETE FROM provision_grants WHERE consumed_at IS NULL AND expires_at < now() - interval '1 day'",
        )
        .execute(&self.pool)
        .await
        .map_err(|_e| AcquisitionError::Db)?;
        Ok(n.rows_affected())
    }
}

fn hex_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn sha256_hex(s: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Q-PROVISION-SECRET: timing-safe, fail-closed-404 ──

    #[test]
    fn ops_secret_matches_only_on_exact_equal_length_value() {
        assert!(provision_ops_authorized(
            Some("s3cret-value"),
            Some("s3cret-value")
        ));
        assert!(
            !provision_ops_authorized(Some("s3cret-valuE"), Some("s3cret-value")),
            "content"
        );
        assert!(
            !provision_ops_authorized(Some("short"), Some("s3cret-value")),
            "length"
        );
        assert!(
            !provision_ops_authorized(Some("s3cret-value-longer"), Some("s3cret-value")),
            "length"
        );
    }

    #[test]
    fn ops_secret_fails_closed_when_unset_or_empty() {
        // Unset or empty configured secret → the whole surface is disabled (→404). The safe default.
        assert!(!provision_ops_authorized(Some("anything"), None));
        assert!(!provision_ops_authorized(Some("anything"), Some("")));
        assert!(!provision_ops_authorized(None, Some("configured")));
        assert!(!provision_ops_authorized(Some(""), Some("configured")));
    }

    #[tokio::test]
    async fn ops_gate_404s_without_the_secret_header() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt as _;
        // A router with the ops gate + a sentinel route: no/wrong header → 404 (existence hidden).
        let gate = OpsGateState {
            secret: Some(Arc::new(Secret::new("the-secret"))),
        };
        let app = axum::Router::new()
            .route(
                "/internal/acquisition",
                axum::routing::post(|| async { "REACHED" }),
            )
            .layer(axum::middleware::from_fn_with_state(gate, ops_secret_gate));
        // no header
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/internal/acquisition")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        // correct header → reaches the handler
        let resp2 = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/internal/acquisition")
                    .header(OPS_HEADER, "the-secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp2.status(), StatusCode::OK);
    }

    // ── Q-PROVISION-RLS: the ordering is the dedup chokepoint ──

    #[test]
    fn spine_order_seats_guc_first_and_consumes_grant_last() {
        assert!(
            PROVISION_SPINE_ORDER[0].contains("set_config('app.provision_token'"),
            "GUC seated FIRST"
        );
        assert!(
            PROVISION_SPINE_ORDER[1].contains("FOR UPDATE"),
            "grant locked before writes"
        );
        let last = PROVISION_SPINE_ORDER.last().unwrap();
        assert!(
            last.contains("provision_grants SET consumed_at"),
            "grant consumed LAST (single-use)"
        );
        // advance must precede the consume (state-pinned chokepoint before single-use burn).
        let advance_i = PROVISION_SPINE_ORDER
            .iter()
            .position(|s| s.contains("advance"))
            .unwrap();
        let consume_i = PROVISION_SPINE_ORDER.len() - 1;
        assert!(advance_i < consume_i);
    }

    // ── Art-14: equally-prominent decline (CC1) ──

    #[test]
    fn art14_notice_carries_equally_prominent_decline() {
        let n = build_art14_notice(
            "https://x/claim?preview=1",
            "https://x/claim#token=abc",
            "https://x/claim#token=abc",
        );
        assert!(n.body.contains("CLAIM it"), "claim option present");
        assert!(
            n.body.contains("DELETE it"),
            "decline/erase option present + prominent"
        );
        assert!(
            n.body.contains("no account needed"),
            "decline needs no registration (as easy as claim)"
        );
        assert!(
            n.body.to_lowercase().contains("erase"),
            "erasure right stated"
        );
    }
}
