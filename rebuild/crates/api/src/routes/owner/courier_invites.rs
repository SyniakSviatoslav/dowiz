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
    pub expires_at: chrono::DateTime<chrono::Utc>,
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
    #[serde(rename = "expiresAt")]
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
}
