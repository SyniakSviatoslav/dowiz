//! S7 courier/dispatch surface — the OWNER-side courier-invite mint/list/revoke half. Ports
//! `apps/api/src/routes/owner/courier-invites.ts` (3 ops) per the council RESOLVE
//! `docs/design/rebuild-courier-s7-council/resolution.md`. See `couriers.rs`'s module doc for the
//! full GUC-family rationale shared by both owner-side courier-management files — summary: this
//! file ALSO seats `app.current_tenant` via [`crate::db::with_tenant`], never `app.user_id`,
//! because `courier_invites`' RLS policy is `location_id = current_setting('app.current_tenant')`
//! (`packages/db/migrations/1780421031109_courier-invites.ts`, missing-ok-rewritten but
//! single-root unchanged by the later NOBYPASSRLS-phase1 migration).
//!
//! ## F4 fix, carried verbatim (🔴 security, REV-S7 — an invite must never mint anything but a
//! courier)
//! Node's OWN comment (`courier-invites.ts:33-36`) documents this as a real fix already landed:
//! `role !== 'courier'` -> 400 `INVALID_ROLE`. This is NOT a validation nicety — `courier_invites`
//! also allows `role = 'dispatcher'` at the SCHEMA level (the `CHECK` constraint), so without this
//! allow-list an owner-invite endpoint could mint a privileged dispatcher invite through a field
//! that looks like plain courier onboarding. Carried at full strength, not weakened.
//!
//! ## Mint-only: this file never verifies a code
//! `crate::auth::crypto::argon2_verify` (verify a candidate password/code against a stored PHC
//! hash) is the REDEEM-side primitive, already used by S2's `auth_courier.rs` on invite
//! acceptance. This file only ever HASHES a freshly generated code (`hash_invite_code` below, a
//! private copy of `auth_courier.rs`'s own `hash_password` — same argon2id params, kept local
//! rather than imported since that fn is private to its own module).
//!
//! ## Judgment call: the deep-link host is hardcoded, not read from the request
//! Node derives the link host from the inbound `Host` header, defaulting to `dowiz.fly.dev`
//! (`courier-invites.ts:74-76`). Reading a request header here would need a new `HeaderMap`
//! extraction this handler doesn't otherwise need; since the fallback IS the production host,
//! this port hardcodes `https://dowiz.fly.dev` unconditionally rather than wiring header
//! inspection for a value that's the same in the overwhelmingly common case. Flagged in the build
//! report, not silently narrowed.
//!
//! ## Judgment call: `ip_hash`/`user_agent_hash` are empty-string placeholders
//! Same simplification as `couriers.rs`'s `patch_courier` — see that file's module doc.
//!
//! ## Revoke has no existence-revealing branch (carried verbatim)
//! `courier-invites.ts:110-133`'s `DELETE` handler always replies `{success: true}}`, whether or
//! not a row actually matched the `UPDATE ... WHERE id=$1 AND location_id=$2 AND used_at IS NULL
//! AND revoked_at IS NULL` predicate (already-used, already-revoked, and unknown-id all look
//! identical on the wire). This port does not invent a 404 branch the source never had.
//!
//! ## R2a additions: the two OTHER invite mounts (distinct handlers in Node, ported as such)
//! The R2a batch sheet calls `POST /api/owner/courier-invites` + `POST /couriers/invites` "twin
//! mounts, one handler" — the Node tree says otherwise, and the CODE is the contract:
//!
//! 1. [`create_courier_invite_token_derived`] — `POST /api/owner/courier-invites`
//!    (`spa-proxy.ts:741-755`): a LINK STUB. Node persists NOTHING — it mints a random 8-hex code
//!    (`crypto.randomUUID().substring(0,8)`), echoes `phone || null` back, and builds a
//!    `https://{locationId}.dowiz.org/courier/join?code=...` link (the location UUID as a
//!    subdomain, verbatim quirk). A valid owner token with NO resolvable location gets the
//!    `pending: true` placeholder on `app.dowiz.org` instead of a 401 (fresh owner mid-wizard,
//!    `spa-proxy.ts:746-752`). No repo call at all.
//! 2. [`create_courier_invite_body_location`] — `POST /couriers/invites`
//!    (`apps/api/src/routes/couriers.ts:8-53`): body-`locationId`, explicit active-owner
//!    membership predicate (404 `Location not found`), then a bare
//!    `INSERT INTO courier_invites (location_id, code_hash, created_by_owner_id, expires_at)`.
//!    ⚠️ KNOWN NODE DEFECT, CARRIED VERBATIM: that column list omits the schema's NOT NULL
//!    `invited_email_hash` (`1780421031109_courier-invites.ts:11`, no default), so the INSERT
//!    always fails 23502 and the route answers 500 `INTERNAL` on the live schema. Ported with the
//!    IDENTICAL column list + 500 mapping (CARRY-VERBATIM rule — the parity oracle must see the
//!    same behavior); fixing it is a council decision, not a port liberty.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tower_http::request_id::RequestId;
use utoipa::ToSchema;
use uuid::Uuid;

use domain::{ErrorCode, TenantId};

use crate::auth::AuthState;
use crate::auth::extractors::OwnerClaimsExt;
use crate::error::ApiError;
use crate::repo::RepoError;
use crate::routes::correlation_id_string;

use super::require_location_access;

const DEEP_LINK_BASE: &str = "https://dowiz.fly.dev";

// ── State + repo trait ──────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct CourierInvitesState {
    pub auth: AuthState,
    pub repo: Arc<dyn CourierInvitesRepo>,
}

/// The `RETURNING id, expires_at` shape of op #1's `INSERT` (`courier-invites.ts:52-56`).
#[derive(Debug, Clone, Copy)]
pub struct CreatedInvite {
    pub id: Uuid,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// One active-invite row — op #2's raw shape (`courier-invites.ts:87-101`), snake_case verbatim
/// (Node sends `res.rows` straight through).
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CourierInviteRow {
    pub id: Uuid,
    pub role: String,
    #[serde(serialize_with = "crate::dto::serialize_js_instant")]
    pub expires_at: chrono::DateTime<chrono::Utc>,
    #[serde(serialize_with = "crate::dto::serialize_js_instant")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait::async_trait]
pub trait CourierInvitesRepo: Send + Sync {
    /// Op #1 (`courier-invites.ts:49-72`). `role` is always `"courier"` by the time this is
    /// called — the handler rejects anything else BEFORE hashing/minting a code, so no wasted
    /// argon2 work on a request that's already going to 400.
    #[allow(clippy::too_many_arguments)]
    async fn create(
        &self,
        location_id: Uuid,
        owner_id: Uuid,
        role: String,
        invited_email_hash: String,
        code_hash: String,
        ttl_hours: i32,
    ) -> Result<CreatedInvite, RepoError>;

    /// Op #2 (`courier-invites.ts:87-101`) — active invites only (unused, unrevoked, unexpired).
    async fn list(&self, location_id: Uuid) -> Result<Vec<CourierInviteRow>, RepoError>;

    /// Op #3 (`courier-invites.ts:104-134`). `Ok(true)` = a row actually matched and was revoked
    /// (audit-logged); `Ok(false)` = no matching row (unknown id / already used / already
    /// revoked) — the HANDLER treats both identically (`{success: true}` either way, see module
    /// doc), the distinction exists only so the repo can decide whether to audit-log.
    async fn revoke(
        &self,
        location_id: Uuid,
        invite_id: Uuid,
        owner_id: Uuid,
    ) -> Result<bool, RepoError>;

    /// R2a `POST /couriers/invites` (`apps/api/src/routes/couriers.ts:41-45`): the bare insert,
    /// column list VERBATIM — deliberately omits the NOT NULL `invited_email_hash`, so on the
    /// live schema this returns `Err` (23502) every time and the handler answers Node's 500
    /// `INTERNAL`. See module doc ("KNOWN NODE DEFECT, CARRIED VERBATIM").
    async fn create_bare(
        &self,
        location_id: Uuid,
        owner_id: Uuid,
        code_hash: String,
    ) -> Result<(), RepoError>;
}

// ── DTOs (wire shapes) ──────────────────────────────────────────────────────────────────────

/// Op #1 body. Node has no formal Zod schema for this route (manual `body?.role`/`body?.email`
/// checks, `courier-invites.ts:30-39`) — `deny_unknown_fields` is applied here anyway per the
/// build brief's "conceptually `.strict()`" instruction, tightening rather than loosening the
/// contract; every field Node actually reads is still present and optional exactly where Node
/// treats it as optional (`ttl_hours`).
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateCourierInviteRequest {
    pub role: String,
    pub email: String,
    #[serde(default)]
    pub ttl_hours: Option<i32>,
}

/// `{success: true}` (op #3's shape, and `couriers.rs`'s `patch_courier` reuses the identical
/// concept — kept as a separate local type per the "own narrow repo/DTO set per submodule"
/// convention `routes::owner`'s module doc establishes, not shared across files).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SuccessResponse {
    pub success: bool,
}

/// Op #1's response (`courier-invites.ts:78-83`). `code` is the ONLY place the plaintext invite
/// code ever appears — no DTO/row type in this file re-exposes it after this response is built.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCourierInviteResponse {
    #[serde(rename = "inviteId")]
    pub invite_id: Uuid,
    pub code: String,
    #[serde(rename = "deepLink")]
    pub deep_link: String,
    #[serde(
        rename = "expiresAt",
        serialize_with = "crate::dto::serialize_js_instant"
    )]
    #[schema(value_type = String)]
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

fn internal_error(correlation_id: String) -> ApiError {
    ApiError::new(ErrorCode::Internal, "internal_error", correlation_id)
}

/// A fresh 16-hex-char invite code — `crypto.randomBytes(8).toString('hex')`
/// (`courier-invites.ts:45`). `random_hex_32` is CSPRNG-backed (`rand::thread_rng` + `fill_bytes`,
/// see `crypto.rs`); slicing its first 16 hex chars is equivalent entropy-wise to generating 8
/// fresh random bytes directly, and reuses the one already-public, already-tested helper instead
/// of adding a second near-identical RNG call site.
fn generate_invite_code() -> String {
    crate::auth::crypto::random_hex_32()[..16].to_string()
}

/// argon2id hash of a freshly generated invite code — a local copy of `auth_courier.rs`'s private
/// `hash_password` (same params: `Params::new(65536, 3, 4, None)`, Argon2id, v0x13; see that
/// file's fn for the exact incantation this mirrors). Kept as its own private copy rather than an
/// import: that fn is private to its module, and this file's argon2 use is MINT-only (see module
/// doc) — there is no shared verify-side code path to unify it with.
fn hash_invite_code(code: &str) -> Result<String, ()> {
    use argon2::password_hash::{PasswordHasher, SaltString};
    use argon2::{Argon2, Params};
    let salt = SaltString::generate(&mut rand::thread_rng());
    let params = Params::new(65536, 3, 4, None).map_err(|_e| ())?;
    let argon = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    argon
        .hash_password(code.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_e| ())
}

// ── Handlers ─────────────────────────────────────────────────────────────────────────────────

/// `POST /api/owner/locations/{locationId}/courier-invites` (op #1, `courier-invites.ts:27-84`)
/// -> 200, or 400 VALIDATION_FAILED / INVALID_ROLE.
#[utoipa::path(
    post,
    path = "/api/owner/locations/{locationId}/courier-invites",
    params(("locationId" = Uuid, Path)),
    request_body = CreateCourierInviteRequest,
    responses(
        (status = 200, description = "Invite minted — the plaintext code is returned exactly once", body = CreateCourierInviteResponse),
        (status = 400, description = "Missing role/email, or role is not 'courier'", body = domain::ErrorEnvelope),
        (status = 404, description = "Foreign/unknown location", body = domain::ErrorEnvelope),
    ),
    tag = "owner-courier-invites"
)]
pub async fn create_courier_invite(
    Extension(state): Extension<CourierInvitesState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<CreateCourierInviteRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    if body.role.trim().is_empty() || body.email.trim().is_empty() {
        return Err(ApiError::validation_failed_400(
            "role and email are required",
            correlation_id,
        ));
    }
    // F4 (🔴 security, carried verbatim): an invite must never mint anything but a courier —
    // `courier_invites.role` also permits 'dispatcher' at the schema/CHECK level.
    if body.role != "courier" {
        return Err(ApiError::new(
            ErrorCode::InvalidRole,
            "role must be 'courier'",
            correlation_id,
        ));
    }

    let email = body.email.to_lowercase().trim().to_string();
    let ttl_hours = body.ttl_hours.unwrap_or(48);

    let code = generate_invite_code();
    let code_hash = hash_invite_code(&code).map_err(|_e| internal_error(correlation_id.clone()))?;
    let email_hash = crate::auth::crypto::sha256_hex(&email);

    let created = state
        .repo
        .create(
            location_id,
            owner.user_id,
            body.role,
            email_hash,
            code_hash,
            ttl_hours,
        )
        .await
        .map_err(|_err| internal_error(correlation_id))?;

    let deep_link = format!("{DEEP_LINK_BASE}/courier-invite/{}", created.id);

    Ok(Json(CreateCourierInviteResponse {
        invite_id: created.id,
        code,
        deep_link,
        expires_at: created.expires_at,
    }))
}

/// `GET /api/owner/locations/{locationId}/courier-invites` (op #2, `courier-invites.ts:87-101`)
/// -> 200.
#[utoipa::path(
    get,
    path = "/api/owner/locations/{locationId}/courier-invites",
    params(("locationId" = Uuid, Path)),
    responses(
        (status = 200, description = "Active (unused, unrevoked, unexpired) invites for this location"),
        (status = 404, description = "Foreign/unknown location", body = domain::ErrorEnvelope),
    ),
    tag = "owner-courier-invites"
)]
pub async fn list_courier_invites(
    Extension(state): Extension<CourierInvitesState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let rows = state
        .repo
        .list(location_id)
        .await
        .map_err(|_err| internal_error(correlation_id))?;

    Ok(Json(serde_json::json!({ "invites": rows })))
}

/// `DELETE /api/owner/locations/{locationId}/courier-invites/{inviteId}` (op #3,
/// `courier-invites.ts:104-134`) -> 200 `{success: true}` always (see module doc).
#[utoipa::path(
    delete,
    path = "/api/owner/locations/{locationId}/courier-invites/{inviteId}",
    params(("locationId" = Uuid, Path), ("inviteId" = Uuid, Path)),
    responses(
        (status = 200, description = "Always success, whether or not a row matched", body = SuccessResponse),
        (status = 404, description = "Foreign/unknown location", body = domain::ErrorEnvelope),
    ),
    tag = "owner-courier-invites"
)]
pub async fn revoke_courier_invite(
    Extension(state): Extension<CourierInvitesState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, invite_id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let _revoked = state
        .repo
        .revoke(location_id, invite_id, owner.user_id)
        .await
        .map_err(|_err| internal_error(correlation_id))?;

    Ok(Json(SuccessResponse { success: true }))
}

/// JS truthiness over a JSON value — `phone || null` (`spa-proxy.ts:751,754`): `null`, `false`,
/// `0`, and `""` are falsy; everything else passes through verbatim.
fn js_truthy(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Null => false,
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64().is_none_or(|f| f != 0.0),
        serde_json::Value::String(s) => !s.is_empty(),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => true,
    }
}

/// R2a: `POST /api/owner/courier-invites` (`spa-proxy.ts:741-755`) — the token-derived invite
/// LINK STUB (see module doc: Node persists NOTHING here; no repo call). Body is optional-JSON
/// (`{phone?}`); a valid owner token with no resolvable location gets the `pending: true`
/// placeholder instead of 401 (fresh owner mid-wizard).
///
/// Known edge divergence (flagged): Node destructures `request.body` unconditionally, so a POST
/// with NO parseable JSON body 500s there; here an absent/unparseable body degrades to
/// `phone: null`. Every real client sends JSON.
#[utoipa::path(
    post,
    path = "/api/owner/courier-invites",
    responses(
        (status = 200, description = "Invite link stub — `{link, code, phone}` (+ `pending: true` when the owner has no location yet)"),
        (status = 401, description = "Not a valid owner token", body = domain::ErrorEnvelope),
    ),
    tag = "owner-courier-invites"
)]
pub async fn create_courier_invite_token_derived(
    Extension(state): Extension<CourierInvitesState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Extension(request_id): Extension<RequestId>,
    body: Option<Json<serde_json::Value>>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let phone = body
        .as_ref()
        .and_then(|Json(v)| v.get("phone"))
        .filter(|v| js_truthy(v))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    // `crypto.randomUUID().substring(0, 8)` — the first 8 hex chars of a v4 UUID.
    let code = uuid::Uuid::new_v4().to_string()[..8].to_string();

    match super::spa_proxy_location_id(&state.auth, &owner, &correlation_id).await? {
        Some(location_id) => Ok(Json(serde_json::json!({
            // Verbatim quirk: the location UUID as a subdomain (`spa-proxy.ts:753`).
            "link": format!("https://{location_id}.dowiz.org/courier/join?code={code}"),
            "code": code,
            "phone": phone,
        }))),
        None => Ok(Json(serde_json::json!({
            "link": format!("https://app.dowiz.org/courier/join?code={code}"),
            "code": code,
            "phone": phone,
            "pending": true,
        }))),
    }
}

/// R2a: `POST /couriers/invites` (`apps/api/src/routes/couriers.ts:8-53`) — body-`locationId`
/// invite mint. Validation mirrors `z.object({locationId: z.string().uuid()}).strict()` at
/// Node's 400; unknown/foreign location -> 404 `Location not found`; any insert failure -> 500
/// `Internal server error` (which, on the live schema, is EVERY insert — the carried
/// `invited_email_hash` NOT NULL defect, see module doc).
#[utoipa::path(
    post,
    path = "/couriers/invites",
    responses(
        (status = 200, description = "`{code}` — 6 uppercase hex chars (unreachable on the current schema, see module doc)"),
        (status = 400, description = "Body is not exactly `{locationId: <uuid>}`", body = domain::ErrorEnvelope),
        (status = 404, description = "Not the caller's active owner location", body = domain::ErrorEnvelope),
        (status = 500, description = "Insert failed (always today: carried NOT NULL defect)", body = domain::ErrorEnvelope),
    ),
    tag = "owner-courier-invites"
)]
pub async fn create_courier_invite_body_location(
    Extension(state): Extension<CourierInvitesState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);

    // `z.object({ locationId: z.string().uuid() }).strict()` — unknown key / missing key /
    // non-uuid all 400 in Node (schema validation), matched here at the same status.
    let obj = body.as_object().ok_or_else(|| {
        ApiError::validation_failed_400("body must be an object", correlation_id.clone())
    })?;
    if obj.keys().any(|k| k != "locationId") {
        return Err(ApiError::validation_failed_400(
            "unrecognized key in body (strict schema)",
            correlation_id,
        ));
    }
    let location_id = obj
        .get("locationId")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| {
            ApiError::validation_failed_400("locationId must be a uuid", correlation_id.clone())
        })?;

    // `couriers.ts:29-36` (#7 security-hardening): an EXPLICIT live active-owner membership
    // predicate on the BODY locationId — never RLS visibility. Same live read
    // `require_location_access` uses; Node's exact 404 message carried.
    let active = state
        .auth
        .repo
        .active_owner_locations(owner.user_id)
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?;
    if !active.contains(&location_id) {
        return Err(ApiError::new(
            ErrorCode::NotFound,
            "Location not found",
            correlation_id,
        ));
    }

    // `crypto.randomBytes(4).toString('hex').slice(0, 6).toUpperCase()` — 6 uppercase hex chars;
    // the sha256 is over the UPPERCASED code (couriers.ts:38-39).
    let code = crate::auth::crypto::random_hex_32()[..6].to_uppercase();
    let code_hash = crate::auth::crypto::sha256_hex(&code);

    state
        .repo
        .create_bare(location_id, owner.user_id, code_hash)
        .await
        .map_err(|_err| {
            // `couriers.ts:49-52`: ANY failure inside the handler -> 500 INTERNAL with Node's
            // exact message (today that is every request — the carried NOT NULL defect).
            ApiError::new(
                ErrorCode::Internal,
                "Internal server error",
                correlation_id.clone(),
            )
        })?;

    Ok(Json(serde_json::json!({ "code": code })))
}

// ── PgCourierInvitesRepo ─────────────────────────────────────────────────────────────────────

/// Constructed by the lead's integration wiring — see `categories.rs`'s `PgCategoriesRepo` doc for
/// why this is genuinely unused (dead-code-allowed) until then.
#[allow(
    dead_code,
    reason = "constructed by the lead's CourierInvitesState wiring at integration — see struct doc"
)]
pub struct PgCourierInvitesRepo {
    pool: sqlx::PgPool,
}

#[allow(
    dead_code,
    reason = "constructed by the lead's CourierInvitesState wiring at integration — see struct doc"
)]
impl PgCourierInvitesRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        PgCourierInvitesRepo { pool }
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

#[async_trait::async_trait]
impl CourierInvitesRepo for PgCourierInvitesRepo {
    async fn create(
        &self,
        location_id: Uuid,
        owner_id: Uuid,
        role: String,
        invited_email_hash: String,
        code_hash: String,
        ttl_hours: i32,
    ) -> Result<CreatedInvite, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let (id, expires_at): (Uuid, chrono::DateTime<chrono::Utc>) = sqlx::query_as(
                    "INSERT INTO courier_invites (location_id, created_by_owner_id, role, invited_email_hash, code_hash, expires_at) \
                     VALUES ($1, $2, $3, $4, $5, now() + interval '1 hour' * $6) \
                     RETURNING id, expires_at",
                )
                .bind(location_id)
                .bind(owner_id)
                .bind(&role)
                .bind(&invited_email_hash)
                .bind(&code_hash)
                .bind(ttl_hours)
                .fetch_one(&mut **txn)
                .await?;

                sqlx::query(
                    "INSERT INTO courier_audit_log (location_id, action, actor_kind, actor_id, ip_hash, user_agent_hash) \
                     VALUES ($1, 'invite.created', 'owner', $2, $3, $4)",
                )
                .bind(location_id)
                .bind(owner_id)
                .bind("")
                .bind("")
                .execute(&mut **txn)
                .await?;

                Ok(CreatedInvite { id, expires_at })
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn list(&self, location_id: Uuid) -> Result<Vec<CourierInviteRow>, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let rows: Vec<CourierInviteRow> = sqlx::query_as(
                    "SELECT id, role, expires_at, created_at \
                     FROM courier_invites \
                     WHERE location_id = $1 AND used_at IS NULL AND revoked_at IS NULL AND expires_at > now()",
                )
                .bind(location_id)
                .fetch_all(&mut **txn)
                .await?;
                Ok(rows)
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn revoke(
        &self,
        location_id: Uuid,
        invite_id: Uuid,
        owner_id: Uuid,
    ) -> Result<bool, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let result = sqlx::query(
                    "UPDATE courier_invites SET revoked_at = now() \
                     WHERE id = $1 AND location_id = $2 AND used_at IS NULL AND revoked_at IS NULL",
                )
                .bind(invite_id)
                .bind(location_id)
                .execute(&mut **txn)
                .await?;

                let revoked = result.rows_affected() == 1;
                if revoked {
                    sqlx::query(
                        "INSERT INTO courier_audit_log (location_id, action, actor_kind, actor_id, ip_hash, user_agent_hash) \
                         VALUES ($1, 'invite.revoked', 'owner', $2, $3, $4)",
                    )
                    .bind(location_id)
                    .bind(owner_id)
                    .bind("")
                    .bind("")
                    .execute(&mut **txn)
                    .await?;
                }
                Ok(revoked)
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn create_bare(
        &self,
        location_id: Uuid,
        owner_id: Uuid,
        code_hash: String,
    ) -> Result<(), RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                // `couriers.ts:41-45` VERBATIM — including the missing NOT NULL
                // `invited_email_hash` (see module doc: this fails 23502 on the live schema and
                // the handler maps it to Node's 500). Do NOT "fix" the column list here.
                sqlx::query(
                    "INSERT INTO courier_invites (location_id, code_hash, created_by_owner_id, expires_at) \
                     VALUES ($1, $2, $3, now() + interval '7 days')",
                )
                .bind(location_id)
                .bind(&code_hash)
                .bind(owner_id)
                .execute(&mut **txn)
                .await?;
                Ok(())
            })
        })
        .await
        .map_err(map_txn_err)
    }
}

// ── FakeCourierInvitesRepo (test-only) ───────────────────────────────────────────────────────

#[cfg(test)]
pub mod fake {
    //! Mutex<HashMap>-backed stub, mirroring `couriers.rs`'s `fake::FakeCouriersRepo`.

    use super::{CourierInviteRow, CourierInvitesRepo, CreatedInvite, RepoError};
    use std::collections::HashMap;
    use std::sync::Mutex;
    use uuid::Uuid;

    #[derive(Debug, Clone)]
    pub struct StoredInvite {
        pub location_id: Uuid,
        pub role: String,
        pub expires_at: chrono::DateTime<chrono::Utc>,
        pub created_at: chrono::DateTime<chrono::Utc>,
        pub used_at: Option<chrono::DateTime<chrono::Utc>>,
        pub revoked_at: Option<chrono::DateTime<chrono::Utc>>,
    }

    #[derive(Default)]
    pub struct FakeCourierInvitesRepo {
        pub invites: Mutex<HashMap<Uuid, StoredInvite>>,
        /// `create_bare` attempts: `(location_id, owner_id, code_hash)`. The REAL repo always
        /// fails on the live schema (carried NOT NULL defect); the fake defaults to SUCCESS so
        /// the happy-path wire shape stays testable, and flips to failure via `fail_bare`.
        pub bare_attempts: Mutex<Vec<(Uuid, Uuid, String)>>,
        pub fail_bare: std::sync::atomic::AtomicBool,
    }

    impl FakeCourierInvitesRepo {
        /// Seed a fixture invite directly (bypassing `create`) so list/revoke tests can control
        /// `used_at`/`revoked_at`/`expires_at` precisely.
        pub fn seed(&self, id: Uuid, invite: StoredInvite) {
            self.invites.lock().unwrap().insert(id, invite);
        }
    }

    #[async_trait::async_trait]
    impl CourierInvitesRepo for FakeCourierInvitesRepo {
        async fn create(
            &self,
            location_id: Uuid,
            _owner_id: Uuid,
            role: String,
            _invited_email_hash: String,
            _code_hash: String,
            ttl_hours: i32,
        ) -> Result<CreatedInvite, RepoError> {
            let id = Uuid::new_v4();
            let now = chrono::Utc::now();
            let expires_at = now + chrono::Duration::hours(i64::from(ttl_hours));
            self.invites.lock().unwrap().insert(
                id,
                StoredInvite {
                    location_id,
                    role,
                    expires_at,
                    created_at: now,
                    used_at: None,
                    revoked_at: None,
                },
            );
            Ok(CreatedInvite { id, expires_at })
        }

        async fn list(&self, location_id: Uuid) -> Result<Vec<CourierInviteRow>, RepoError> {
            let now = chrono::Utc::now();
            Ok(self
                .invites
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, inv)| {
                    inv.location_id == location_id
                        && inv.used_at.is_none()
                        && inv.revoked_at.is_none()
                        && inv.expires_at > now
                })
                .map(|(id, inv)| CourierInviteRow {
                    id: *id,
                    role: inv.role.clone(),
                    expires_at: inv.expires_at,
                    created_at: inv.created_at,
                })
                .collect())
        }

        async fn revoke(
            &self,
            location_id: Uuid,
            invite_id: Uuid,
            _owner_id: Uuid,
        ) -> Result<bool, RepoError> {
            let mut invites = self.invites.lock().unwrap();
            match invites.get_mut(&invite_id) {
                Some(inv)
                    if inv.location_id == location_id
                        && inv.used_at.is_none()
                        && inv.revoked_at.is_none() =>
                {
                    inv.revoked_at = Some(chrono::Utc::now());
                    Ok(true)
                }
                _ => Ok(false),
            }
        }

        async fn create_bare(
            &self,
            location_id: Uuid,
            owner_id: Uuid,
            code_hash: String,
        ) -> Result<(), RepoError> {
            self.bare_attempts
                .lock()
                .unwrap()
                .push((location_id, owner_id, code_hash));
            if self.fail_bare.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(RepoError(sqlx::Error::RowNotFound));
            }
            Ok(())
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::fake::{FakeCourierInvitesRepo, StoredInvite};
    use super::*;
    use crate::auth::claims::OwnerClaims;
    use crate::auth::repo::fake::FakeAuthRepo;
    use axum::http::StatusCode;
    use std::sync::Mutex;

    fn request_id() -> RequestId {
        RequestId::new(axum::http::HeaderValue::from_static("corr-1"))
    }

    fn owner_with_location(user_id: Uuid, loc: Uuid) -> AuthState {
        AuthState::test_state(Arc::new(FakeAuthRepo {
            active_owner_locations: Mutex::new([(user_id, vec![loc])].into_iter().collect()),
            ..Default::default()
        }))
    }

    fn state_with(repo: FakeCourierInvitesRepo, auth: AuthState) -> CourierInvitesState {
        CourierInvitesState {
            auth,
            repo: Arc::new(repo),
        }
    }

    #[tokio::test]
    async fn create_courier_invite_rejects_a_non_courier_role() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = state_with(
            FakeCourierInvitesRepo::default(),
            owner_with_location(user_id, loc),
        );
        let owner = OwnerClaims::new(user_id, Some(loc));

        let err = crate::error::expect_err(
            create_courier_invite(
                Extension(state),
                OwnerClaimsExt(owner),
                Path(loc),
                Extension(request_id()),
                Json(CreateCourierInviteRequest {
                    role: "dispatcher".to_string(),
                    email: "someone@example.com".to_string(),
                    ttl_hours: None,
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::InvalidRole);
        assert_eq!(err.envelope.status, 400);
    }

    #[tokio::test]
    async fn create_courier_invite_rejects_an_owner_role_too() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = state_with(
            FakeCourierInvitesRepo::default(),
            owner_with_location(user_id, loc),
        );
        let owner = OwnerClaims::new(user_id, Some(loc));

        let err = crate::error::expect_err(
            create_courier_invite(
                Extension(state),
                OwnerClaimsExt(owner),
                Path(loc),
                Extension(request_id()),
                Json(CreateCourierInviteRequest {
                    role: "owner".to_string(),
                    email: "someone@example.com".to_string(),
                    ttl_hours: None,
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::InvalidRole);
    }

    #[tokio::test]
    async fn create_courier_invite_rejects_missing_email() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = state_with(
            FakeCourierInvitesRepo::default(),
            owner_with_location(user_id, loc),
        );
        let owner = OwnerClaims::new(user_id, Some(loc));

        let err = crate::error::expect_err(
            create_courier_invite(
                Extension(state),
                OwnerClaimsExt(owner),
                Path(loc),
                Extension(request_id()),
                Json(CreateCourierInviteRequest {
                    role: "courier".to_string(),
                    email: String::new(),
                    ttl_hours: None,
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::ValidationFailed);
        assert_eq!(err.envelope.status, 400);
    }

    #[tokio::test]
    async fn create_courier_invite_404_for_a_foreign_location() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = state_with(
            FakeCourierInvitesRepo::default(),
            owner_with_location(user_id, mine),
        );
        let owner = OwnerClaims::new(user_id, Some(mine));

        let err = crate::error::expect_err(
            create_courier_invite(
                Extension(state),
                OwnerClaimsExt(owner),
                Path(theirs),
                Extension(request_id()),
                Json(CreateCourierInviteRequest {
                    role: "courier".to_string(),
                    email: "someone@example.com".to_string(),
                    ttl_hours: None,
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn create_courier_invite_returns_the_plaintext_code_exactly_once() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = state_with(
            FakeCourierInvitesRepo::default(),
            owner_with_location(user_id, loc),
        );
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = create_courier_invite(
            Extension(state),
            OwnerClaimsExt(owner),
            Path(loc),
            Extension(request_id()),
            Json(CreateCourierInviteRequest {
                role: "courier".to_string(),
                email: "Courier@Example.com".to_string(),
                ttl_hours: Some(24),
            }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        // `code` is present exactly once, in this response, in plaintext — nothing else in the
        // wire shape re-derives or re-exposes it (it's never stored anywhere but `code_hash`).
        let code = body["code"].as_str().unwrap();
        assert_eq!(code.len(), 16);
        assert!(code.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(body["inviteId"].is_string());
        assert!(
            body["deepLink"]
                .as_str()
                .unwrap()
                .ends_with(body["inviteId"].as_str().unwrap())
        );
        assert_eq!(
            body.as_object().unwrap().len(),
            4,
            "no field beyond {{inviteId,code,deepLink,expiresAt}}"
        );
    }

    #[tokio::test]
    async fn list_courier_invites_excludes_used_and_revoked_and_expired_invites() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeCourierInvitesRepo::default();
        let now = chrono::Utc::now();

        let active = Uuid::new_v4();
        repo.seed(
            active,
            StoredInvite {
                location_id: loc,
                role: "courier".to_string(),
                expires_at: now + chrono::Duration::hours(1),
                created_at: now,
                used_at: None,
                revoked_at: None,
            },
        );
        repo.seed(
            Uuid::new_v4(),
            StoredInvite {
                location_id: loc,
                role: "courier".to_string(),
                expires_at: now + chrono::Duration::hours(1),
                created_at: now,
                used_at: Some(now),
                revoked_at: None,
            },
        );
        repo.seed(
            Uuid::new_v4(),
            StoredInvite {
                location_id: loc,
                role: "courier".to_string(),
                expires_at: now + chrono::Duration::hours(1),
                created_at: now,
                used_at: None,
                revoked_at: Some(now),
            },
        );
        repo.seed(
            Uuid::new_v4(),
            StoredInvite {
                location_id: loc,
                role: "courier".to_string(),
                expires_at: now - chrono::Duration::hours(1),
                created_at: now,
                used_at: None,
                revoked_at: None,
            },
        );

        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = list_courier_invites(
            Extension(state),
            OwnerClaimsExt(owner),
            Path(loc),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let invites = body["invites"].as_array().unwrap();
        assert_eq!(invites.len(), 1);
        assert_eq!(invites[0]["id"], active.to_string());
    }

    #[tokio::test]
    async fn revoke_courier_invite_is_always_success_even_for_an_unknown_invite_id() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = state_with(
            FakeCourierInvitesRepo::default(),
            owner_with_location(user_id, loc),
        );
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = revoke_courier_invite(
            Extension(state),
            OwnerClaimsExt(owner),
            Path((loc, Uuid::new_v4())),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: SuccessResponse = serde_json::from_slice(&bytes).unwrap();
        assert!(body.success);
    }

    #[tokio::test]
    async fn revoke_courier_invite_200_happy_path_actually_revokes() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeCourierInvitesRepo::default();
        let now = chrono::Utc::now();
        let invite_id = Uuid::new_v4();
        repo.seed(
            invite_id,
            StoredInvite {
                location_id: loc,
                role: "courier".to_string(),
                expires_at: now + chrono::Duration::hours(1),
                created_at: now,
                used_at: None,
                revoked_at: None,
            },
        );
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = revoke_courier_invite(
            Extension(state.clone()),
            OwnerClaimsExt(owner),
            Path((loc, invite_id)),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        // Now the invite must no longer be listed as active.
        let owner2 = OwnerClaims::new(user_id, Some(loc));
        let response2 = list_courier_invites(
            Extension(state),
            OwnerClaimsExt(owner2),
            Path(loc),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        let bytes = axum::body::to_bytes(response2.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["invites"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn list_courier_invites_404_for_a_foreign_location() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = state_with(
            FakeCourierInvitesRepo::default(),
            owner_with_location(user_id, mine),
        );
        let owner = OwnerClaims::new(user_id, Some(mine));

        let err = crate::error::expect_err(
            list_courier_invites(
                Extension(state),
                OwnerClaimsExt(owner),
                Path(theirs),
                Extension(request_id()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    // ── R2a: POST /api/owner/courier-invites (token-derived link stub) ──────────────────────

    /// `spa-proxy.ts:753-754`: with a resolvable location — `{link, code, phone}`, NO `pending`
    /// key, the location UUID as the link subdomain, an 8-lowercase-hex code, and NO repo write.
    #[tokio::test]
    async fn invite_token_derived_returns_location_link_and_writes_nothing() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeCourierInvitesRepo::default();
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = create_courier_invite_token_derived(
            Extension(state.clone()),
            OwnerClaimsExt(owner),
            Extension(request_id()),
            Some(Json(serde_json::json!({ "phone": "+355691234567" }))),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        let code = body["code"].as_str().unwrap();
        assert_eq!(code.len(), 8);
        assert!(
            code.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
        assert_eq!(
            body["link"],
            format!("https://{loc}.dowiz.org/courier/join?code={code}")
        );
        assert_eq!(body["phone"], "+355691234567");
        assert!(
            body.get("pending").is_none(),
            "no pending key on the with-location branch"
        );
        assert_eq!(
            body.as_object().unwrap().len(),
            3,
            "exactly {{link, code, phone}} — widened nothing"
        );
    }

    /// `spa-proxy.ts:746-752`: a valid owner token with NO location (fresh signup mid-wizard) —
    /// the `pending: true` placeholder on `app.dowiz.org`, NOT a 401. Falsy phone -> `null`.
    #[tokio::test]
    async fn invite_token_derived_returns_pending_placeholder_when_owner_has_no_location() {
        let user_id = Uuid::new_v4();
        let state = state_with(
            FakeCourierInvitesRepo::default(),
            AuthState::test_state(Arc::new(FakeAuthRepo::default())),
        );
        let owner = OwnerClaims::new(user_id, None);

        let response = create_courier_invite_token_derived(
            Extension(state),
            OwnerClaimsExt(owner),
            Extension(request_id()),
            Some(Json(serde_json::json!({ "phone": "" }))),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let code = body["code"].as_str().unwrap();
        assert_eq!(
            body["link"],
            format!("https://app.dowiz.org/courier/join?code={code}")
        );
        assert_eq!(body["pending"], true);
        assert_eq!(
            body["phone"],
            serde_json::Value::Null,
            "'' is falsy -> null"
        );
    }

    // ── R2a: POST /couriers/invites (body-locationId mint) ──────────────────────────────────

    /// Happy path (`couriers.ts:38-47`): `{code}` only, 6 uppercase hex chars; the repo receives
    /// the sha256 of the UPPERCASED code.
    #[tokio::test]
    async fn invite_body_location_mints_a_6char_upper_hex_code_and_hashes_it() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = std::sync::Arc::new(FakeCourierInvitesRepo::default());
        let state = CourierInvitesState {
            auth: owner_with_location(user_id, loc),
            repo: repo.clone(),
        };
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = create_courier_invite_body_location(
            Extension(state),
            OwnerClaimsExt(owner),
            Extension(request_id()),
            Json(serde_json::json!({ "locationId": loc.to_string() })),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let code = body["code"].as_str().unwrap();
        assert_eq!(code.len(), 6);
        assert!(
            code.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_lowercase())
        );
        assert_eq!(body.as_object().unwrap().len(), 1, "exactly {{code}}");

        let attempts = repo.bare_attempts.lock().unwrap();
        assert_eq!(attempts.len(), 1);
        assert_eq!(attempts[0].0, loc);
        assert_eq!(attempts[0].1, user_id);
        assert_eq!(
            attempts[0].2,
            crate::auth::crypto::sha256_hex(code),
            "hash is over the UPPERCASED code"
        );
    }

    /// Ledger #78 class: Node's zod `.strict()` failures are 400 (never axum's 422 default) —
    /// unknown key, missing key, and non-uuid value all pinned at 400; body-locationId that
    /// isn't the caller's active membership is Node's 404 `Location not found`.
    #[tokio::test]
    async fn invite_body_location_status_code_parity_400_and_404() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let mk_state = || CourierInvitesState {
            auth: owner_with_location(user_id, loc),
            repo: std::sync::Arc::new(FakeCourierInvitesRepo::default()),
        };

        for bad_body in [
            serde_json::json!({}),
            serde_json::json!({ "locationId": loc.to_string(), "extra": 1 }),
            serde_json::json!({ "locationId": "not-a-uuid" }),
            serde_json::json!([1, 2]),
        ] {
            let err = crate::error::expect_err(
                create_courier_invite_body_location(
                    Extension(mk_state()),
                    OwnerClaimsExt(OwnerClaims::new(user_id, Some(loc))),
                    Extension(request_id()),
                    Json(bad_body),
                )
                .await,
            );
            assert_eq!(err.envelope.code, ErrorCode::ValidationFailed);
            assert_eq!(err.envelope.status, 400, "400, not the 422 default (#78)");
        }

        let foreign = Uuid::new_v4();
        let err = crate::error::expect_err(
            create_courier_invite_body_location(
                Extension(mk_state()),
                OwnerClaimsExt(OwnerClaims::new(user_id, Some(loc))),
                Extension(request_id()),
                Json(serde_json::json!({ "locationId": foreign.to_string() })),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
        assert_eq!(err.envelope.message, "Location not found");
    }

    /// `couriers.ts:49-52`: any insert failure -> 500 INTERNAL `Internal server error` — on the
    /// live schema that is EVERY request (carried `invited_email_hash` NOT NULL defect).
    #[tokio::test]
    async fn invite_body_location_maps_insert_failure_to_nodes_500_internal() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeCourierInvitesRepo::default();
        repo.fail_bare
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let state = state_with(repo, owner_with_location(user_id, loc));

        let err = crate::error::expect_err(
            create_courier_invite_body_location(
                Extension(state),
                OwnerClaimsExt(OwnerClaims::new(user_id, Some(loc))),
                Extension(request_id()),
                Json(serde_json::json!({ "locationId": loc.to_string() })),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::Internal);
        assert_eq!(err.envelope.message, "Internal server error");
    }

    /// Requires a live Postgres — pins the CARRIED DEFECT deterministically: the verbatim
    /// `create_bare` column list violates `invited_email_hash NOT NULL` (23502) on the real
    /// schema, so the insert fails and NOTHING is written (read-only in effect).
    #[tokio::test]
    #[ignore = "requires a live Postgres — set DATABASE_URL_OPERATIONAL/DATABASE_URL_SESSION and run with --ignored"]
    async fn live_pg_create_bare_fails_not_null_invited_email_hash_as_carried_from_node() {
        let config =
            crate::config::Config::from_env().expect("env must be valid to run this ignored test");
        let pools = crate::db::Pools::connect(&config)
            .await
            .expect("pools must connect");
        let repo = PgCourierInvitesRepo::new(pools.operational.clone());
        let err = repo
            .create_bare(Uuid::new_v4(), Uuid::new_v4(), "deadbeef".repeat(8))
            .await
            .expect_err("the verbatim Node column list must fail NOT NULL on the live schema");
        let RepoError(sqlx::Error::Database(db_err)) = err else {
            panic!("expected a database error, got: {err:?}");
        };
        assert_eq!(
            db_err.code().as_deref(),
            Some("23502"),
            "not_null_violation on invited_email_hash — the carried Node defect"
        );
    }
}
