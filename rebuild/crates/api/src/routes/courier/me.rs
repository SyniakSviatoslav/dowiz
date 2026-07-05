//! S7 courier profile/messenger/audit-log/password/earnings/history — ports
//! `apps/api/src/routes/courier/me.ts`. See `crate::routes::courier` module doc for the shared
//! auth/tenancy contract (`CourierSession` extractor + `with_tenant(active_location_id)`) every
//! S7 submodule follows, and `courier::assignments` for a sibling file using the same conventions.
//!
//! ## Tenancy (REV-S7-1) — `with_tenant` uniformly, even where old Node never seated the GUC
//! `couriers`/`courier_sessions` have NO RLS at all — a bare-pool read against them alone is
//! correct either way (`get_password_hash`/`update_messenger` use the bare pool for exactly this
//! reason). Every OTHER table this file touches (`courier_locations`, `courier_audit_log`,
//! `courier_assignments`, `orders`, `customers`, `locations`, `courier_payouts`) DOES enforce RLS
//! keyed on `app.current_tenant` (several via the missing-ok `NULLIF(current_setting(...,true),'')`
//! rewrite) — every method touching one of those runs inside `with_tenant`, matching the
//! whole-surface REV-S7-1 directive in `courier::mod`'s doc.
//!
//! Old Node's `me.ts` never sets `app.current_tenant` at all (unlike `settlements.ts`, which
//! explicitly does for the identical `courier_payouts` table) — this "works" today only because
//! the operational pool's role still bypasses RLS (the same `with_tenant`-is-the-fix reasoning
//! `courier::assignments`'s module doc spells out). Under real NOBYPASSRLS enforcement, wrapping
//! `earnings_window`/`recent_payouts`/`history`/`audit_log` in `with_tenant(active_location_id)`
//! narrows each read to the courier's CURRENTLY ACTIVE location — a real behavior change from
//! Node's location-unscoped SQL predicates (none of which filter by location for these four
//! reads). This is a deliberate judgment call, not an oversight: a courier session in this rebuild
//! is single-location-scoped by construction (`CourierClaims.active_location_id` is one `Uuid`,
//! never a list), and `settlements.ts`'s OWN explicit (if bare/no-`BEGIN`) tenant-seat for the
//! identical `courier_payouts` table confirms the intent was always to scope payouts per-location
//! — Node's two files simply disagree with each other on this point, and this port picks the
//! with_tenant-everywhere side for all four reads, consistently.
//!
//! ## REV-S7-7 (security) — password change revokes all live sessions
//! `patch_password`'s Node source (`me.ts:160-164`) revokes every un-revoked `courier_sessions`
//! row for this courier in the SAME transaction as the password write, forcing a re-login
//! everywhere. Carried verbatim; `patch_password_revokes_all_live_sessions_on_success` is the
//! named DoD test.
//!
//! ## Judgment calls / flags (not silently dropped)
//! - `ip_hash`/`user_agent_hash` (`me.ts:125-126`): Node sha256-hashes the request IP / User-Agent
//!   header for the password-change audit-log row. This crate has no established axum
//!   `ConnectInfo`/header-extraction wiring for client IP anywhere yet — inventing that wiring
//!   here would be new cross-cutting surface, out of scope for a two-file S7 port. Both hashes are
//!   passed as `NULL` (`courier_audit_log.ip_hash`/`user_agent_hash` are nullable) — a deliberate
//!   simplification; the security-critical part (the actual session revocation) is fully ported
//!   and tested.
//! - `couriers.messenger_kind`/`messenger_handle`: confirmed present via
//!   `packages/db/migrations/1790000000038_messenger-deeplink.ts` (adds both columns to
//!   `customers`, `couriers`, AND `orders`) — not an invented migration.
//! - `me.ts:243`'s `reference` string slices `p.period_start`/`period_end` directly with
//!   `String.prototype.slice` — since `pg`'s default type parser returns `timestamptz` columns as
//!   JS `Date` objects (no `.slice` method), that line is a latent Node-side bug whenever a payout
//!   row actually exists (a `TypeError`, simply never hit in practice if `/me/earnings` has no
//!   payout rows yet). This port reproduces the INTENT (the first 10 chars = the date part) via
//!   `chrono`'s `%Y-%m-%d` formatting, not the crash.
//! - `crate::auth::pii::mask_str` (reused here, not reimplemented, per the build brief) implements
//!   an email-shaped "first char + @domain" mask. The actual Node `maskStr` (`lib/pii-mask.ts`) is
//!   a generic length-based substring mask (first 2 + `***` + last 2 chars) with no `@`-awareness
//!   at all — a pre-existing S2 divergence from Node found while porting this file (both
//!   `masked_email`/`masked_phone` here and `customerAddress` in `/me/history` call through it).
//!   Flagged, not fixed: `auth/pii.rs` is shared S2 surface well outside this task's two-file
//!   scope, and the task brief explicitly directs reusing it as-is.

use std::sync::Arc;

use axum::Json;
use axum::extract::Extension;
use axum::response::IntoResponse;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tower_http::request_id::RequestId;
use utoipa::ToSchema;
use uuid::Uuid;

use domain::{ErrorCode, TenantId};

use crate::auth::AuthState;
use crate::auth::extractors::CourierSession;
use crate::auth::pii::mask_str;
use crate::error::ApiError;
use crate::repo::RepoError;
use crate::routes::correlation_id_string;

// ── State + repo trait ──────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct MeState {
    pub auth: AuthState,
    pub repo: Arc<dyn MeRepo>,
}

/// `GET /me`'s row (`me.ts:40-47`) — PII columns stay encrypted at this layer; the handler
/// decrypts/masks per-field so one field's failure never fails the others.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProfileRow {
    pub email_encrypted: Vec<u8>,
    pub phone_encrypted: Option<Vec<u8>>,
    pub full_name_encrypted: Vec<u8>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub messenger_kind: Option<String>,
    pub messenger_handle: Option<String>,
    pub role: String,
}

/// `GET /me/audit-log`'s row (`me.ts:98-102`).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuditLogRow {
    pub action: String,
    pub actor_kind: String,
    pub created_at: DateTime<Utc>,
}

/// One of the three identical-shaped earnings queries (`me.ts:187-215`).
#[derive(Debug, Clone, Copy, Default)]
pub struct EarningsWindow {
    pub amount: i64,
    pub deliveries: i64,
    pub tips: i64,
}

/// Which of the three earnings windows (`me.ts:187/197/207`) — selects the SQL date predicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EarningsWindowKind {
    Today,
    Week,
    Month,
}

/// `courier_payouts` row for the earnings summary (`me.ts:217-223`).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PayoutSummaryRow {
    pub id: Uuid,
    pub amount: i64,
    pub status: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    /// Selected for the `ORDER BY created_at DESC` (and mapped by `FromRow`) but not read in the
    /// response: Node's `date: p.period_end || p.created_at` uses `created_at` only as a fallback
    /// for a NULL `period_end`, but `courier_payouts.period_end` is `NOT NULL` — so the fallback is
    /// unreachable and the Rust handler reads `period_end` directly.
    #[allow(
        dead_code,
        reason = "selected for ORDER BY; period_end is NOT NULL so the Node created_at fallback is unreachable"
    )]
    pub created_at: DateTime<Utc>,
}

/// `GET /me/history`'s enriched join (`me.ts:252-264`), BEFORE the separate best-effort ratings
/// join — `rating`/`feedback` are folded in by the handler, not this repo row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct HistoryRow {
    pub id: Uuid,
    pub order_id: Uuid,
    pub status: String,
    pub delivered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub cash_amount: Option<i64>,
    pub total: i64,
    pub customer_name: String,
    pub location_name: String,
}

#[async_trait::async_trait]
pub trait MeRepo: Send + Sync {
    /// `GET /me` (`me.ts:36-71`). `Ok(None)` = no row (unknown courier OR not a member of this
    /// location) — 404.
    async fn get_profile(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<ProfileRow>, RepoError>;

    /// `PATCH /me/messenger` (`me.ts:76-91`) — writes exactly the two values the handler already
    /// resolved (both-or-neither, pre-applied by the caller; `None` clears the column).
    async fn update_messenger(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
        kind: Option<String>,
        handle: Option<String>,
    ) -> Result<(), RepoError>;

    /// `GET /me/audit-log` (`me.ts:94-107`).
    async fn audit_log(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<Vec<AuditLogRow>, RepoError>;

    /// `couriers.password_hash` read for the verify step (`me.ts:130-133`). `Ok(None)` = 404.
    async fn get_password_hash(&self, courier_id: Uuid) -> Result<Option<String>, RepoError>;

    /// The ONE transaction (`me.ts:150-166`): password write + audit-log insert + REV-S7-7
    /// all-sessions revoke.
    async fn change_password(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
        new_hash: String,
    ) -> Result<(), RepoError>;

    /// `locations.currency_code` (`me.ts:181-182`). `Ok(None)` -> handler defaults `"ALL"`.
    async fn location_currency(&self, location_id: Uuid) -> Result<Option<String>, RepoError>;

    /// One of today/week/month (`me.ts:187-215`) — `window` selects the SQL date predicate.
    async fn earnings_window(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
        window: EarningsWindowKind,
    ) -> Result<EarningsWindow, RepoError>;

    /// `courier_payouts` list, newest 20 (`me.ts:217-223`).
    async fn recent_payouts(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<Vec<PayoutSummaryRow>, RepoError>;

    /// `GET /me/history`'s base join (`me.ts:252-264`), ratings NOT yet attached.
    async fn history(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<Vec<HistoryRow>, RepoError>;

    /// Best-effort ratings by order id (`me.ts:266-275`) — degrades to `vec![]` on ANY error
    /// (including `order_ratings` not yet migrated), never propagated as a hard failure. No
    /// `Result` in the signature by design: there is no failure mode this trait exposes to a
    /// caller — the impl swallows it internally, matching Node's `try {...} catch { /* not yet
    /// migrated */ }`.
    async fn ratings_for_orders(
        &self,
        order_ids: &[Uuid],
    ) -> Vec<(Uuid, Option<i32>, Option<String>)>;
}

// ── DTOs ─────────────────────────────────────────────────────────────────────────────────────

/// `me.ts:61-70` — snake_case, Node's raw `reply.send({...})`. No `ToSchema` derive (see the
/// `AuditLogEntry` doc comment below for why: `last_login_at`'s raw `DateTime<Utc>` isn't
/// utoipa-schema-able without a workspace Cargo.toml feature change out of this port's scope).
#[derive(Debug, Serialize)]
pub struct ProfileResponse {
    pub id: Uuid,
    pub full_name: String,
    pub masked_email: String,
    pub masked_phone: Option<String>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub messenger_kind: Option<String>,
    pub messenger_handle: Option<String>,
    pub active_location: ActiveLocation,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ActiveLocation {
    pub id: Uuid,
    pub role: String,
}

/// `PATCH /me/messenger` request enum (`me.ts:79`).
#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessengerKind {
    Telegram,
    Whatsapp,
    Viber,
}

impl MessengerKind {
    fn as_db_str(self) -> &'static str {
        match self {
            MessengerKind::Telegram => "telegram",
            MessengerKind::Whatsapp => "whatsapp",
            MessengerKind::Viber => "viber",
        }
    }
}

/// `.strict()` body (`me.ts:78-81`). Node's tri-state (`.nullable().optional()`) collapses to a
/// plain `Option<T>` here: the handler's own `both = b.messenger_kind && b.messenger_handle && ...`
/// treats an ABSENT field identically to an explicit `null` (both are falsy in JS) — there is no
/// third "leave unchanged" case to preserve.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PatchMessengerRequest {
    #[serde(default)]
    pub messenger_kind: Option<MessengerKind>,
    #[serde(default)]
    pub messenger_handle: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PatchMessengerResponse {
    pub messenger_kind: Option<String>,
    pub messenger_handle: Option<String>,
}

// `ToSchema` is deliberately NOT derived on `AuditLogEntry`/`AuditLogResponse` (nor the other
// DateTime-bearing DTOs below) — `chrono::DateTime<Utc>` doesn't implement utoipa's
// `PartialSchema` without the workspace-wide "chrono" utoipa feature, which isn't enabled
// (`crates/api/Cargo.toml` is shared, out of this two-file port's scope to add a feature to).
// `crate::routes::owner::products::ProductRow` (which also carries a raw `created_at:
// DateTime<Utc>`) already established this exact workaround: `Serialize`-only DTOs, and the
// corresponding `#[utoipa::path]` response omits `body = ...` for the 200 case (kept below).
#[derive(Debug, Serialize)]
pub struct AuditLogEntry {
    pub action: String,
    pub actor_kind: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogResponse {
    pub logs: Vec<AuditLogEntry>,
}

/// Manual-Zod-equivalent body (`me.ts:112-115`, `.strict()`).
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PatchPasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SimpleSuccessResponse {
    pub success: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EarningsSummary {
    pub today: i64,
    pub today_deliveries: i64,
    pub today_tips: i64,
    pub week: i64,
    pub week_deliveries: i64,
    pub week_tips: i64,
    pub month: i64,
    pub month_deliveries: i64,
    pub month_tips: i64,
    pub currency: String,
}

#[derive(Debug, Serialize)]
pub struct PayoutSummary {
    pub id: Uuid,
    pub date: DateTime<Utc>,
    pub amount: i64,
    pub status: String,
    pub reference: String,
}

#[derive(Debug, Serialize)]
pub struct EarningsResponse {
    pub summary: EarningsSummary,
    pub payouts: Vec<PayoutSummary>,
}

/// `me.ts:15-27`'s `mapCourierHistoryRow` wire shape — deliberately camelCase (the one endpoint in
/// this file that is), and `customerAddress` is a misleading name carried verbatim: it actually
/// holds the MASKED customer name, not an address (byte-parity wire contract, not a naming choice
/// to "fix").
#[derive(Debug, Serialize)]
pub struct HistoryEntry {
    pub id: Uuid,
    #[serde(rename = "orderId")]
    pub order_id: Uuid,
    pub date: DateTime<Utc>,
    pub restaurant: String,
    #[serde(rename = "customerAddress")]
    pub customer_address: String,
    pub amount: i64,
    pub status: String,
    pub rating: Option<i32>,
    pub feedback: Option<String>,
}

fn internal_error(correlation_id: String) -> ApiError {
    ApiError::new(ErrorCode::Internal, "internal_error", correlation_id)
}

fn not_found(correlation_id: String, message: &str) -> ApiError {
    ApiError::new(ErrorCode::NotFound, message, correlation_id)
}

/// argon2id hash (matching Node's `argon2.hash(x, {type:argon2id, memoryCost:65536, timeCost:3,
/// parallelism:4})`) — a small local equivalent of `auth_courier.rs`'s private `hash_password`
/// (that one is private to its own module, not reusable from here).
fn hash_password(password: &str) -> Result<String, ()> {
    use argon2::password_hash::{PasswordHasher, SaltString};
    use argon2::{Argon2, Params};
    let salt = SaltString::generate(&mut rand::thread_rng());
    let params = Params::new(65536, 3, 4, None).map_err(|_e| ())?;
    let argon = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    argon
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_e| ())
}

// ── Handlers ─────────────────────────────────────────────────────────────────────────────────

/// `GET /api/courier/me` (`me.ts:36-71`).
#[utoipa::path(get, path = "/api/courier/me", tag = "courier",
    responses((status = 200, description = "Profile"), (status = 404, description = "Not found", body = domain::ErrorEnvelope)))]
pub async fn get_profile(
    Extension(state): Extension<MeState>,
    CourierSession(courier): CourierSession,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let row = state
        .repo
        .get_profile(courier.sub, courier.active_location_id)
        .await
        .map_err(|_e| internal_error(correlation_id.clone()))?
        .ok_or_else(|| not_found(correlation_id, "Not found"))?;

    let cipher = state.auth.pii_cipher.as_deref();
    // Each field decrypts independently (me.ts:57-59) — a failure in one must never fail the
    // others.
    let email_plain = cipher.and_then(|c| c.decrypt(&row.email_encrypted).ok());
    let phone_plain = row
        .phone_encrypted
        .as_ref()
        .and_then(|b| cipher.and_then(|c| c.decrypt(b).ok()));
    let full_name_plain = cipher.and_then(|c| c.decrypt(&row.full_name_encrypted).ok());

    Ok(Json(ProfileResponse {
        id: courier.sub,
        full_name: full_name_plain.unwrap_or_else(|| "(decryption failed)".to_string()),
        masked_email: email_plain
            .map(|e| mask_str(&e))
            .unwrap_or_else(|| "(decryption failed)".to_string()),
        masked_phone: phone_plain.map(|p| mask_str(&p)),
        last_login_at: row.last_login_at,
        messenger_kind: row.messenger_kind,
        messenger_handle: row.messenger_handle,
        active_location: ActiveLocation {
            id: courier.active_location_id,
            role: row.role,
        },
    }))
}

/// `PATCH /api/courier/me/messenger` (`me.ts:76-91`) — both-or-neither.
#[utoipa::path(patch, path = "/api/courier/me/messenger", tag = "courier",
    request_body = PatchMessengerRequest,
    responses((status = 200, body = PatchMessengerResponse)))]
pub async fn patch_messenger(
    Extension(state): Extension<MeState>,
    CourierSession(courier): CourierSession,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<PatchMessengerRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);

    let handle_trimmed = body
        .messenger_handle
        .as_deref()
        .map(str::trim)
        .filter(|h| !h.is_empty());
    let both = body.messenger_kind.is_some() && handle_trimmed.is_some();
    let kind = if both {
        body.messenger_kind.map(MessengerKind::as_db_str)
    } else {
        None
    };
    let handle = if both {
        handle_trimmed.map(str::to_string)
    } else {
        None
    };

    state
        .repo
        .update_messenger(
            courier.sub,
            courier.active_location_id,
            kind.map(str::to_string),
            handle.clone(),
        )
        .await
        .map_err(|_e| internal_error(correlation_id))?;

    Ok(Json(PatchMessengerResponse {
        messenger_kind: kind.map(str::to_string),
        messenger_handle: handle,
    }))
}

/// `GET /api/courier/me/audit-log` (`me.ts:94-107`).
#[utoipa::path(get, path = "/api/courier/me/audit-log", tag = "courier",
    responses((status = 200, description = "Audit log")))]
pub async fn get_audit_log(
    Extension(state): Extension<MeState>,
    CourierSession(courier): CourierSession,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let rows = state
        .repo
        .audit_log(courier.sub, courier.active_location_id)
        .await
        .map_err(|_e| internal_error(correlation_id))?;
    Ok(Json(AuditLogResponse {
        logs: rows
            .into_iter()
            .map(|r| AuditLogEntry {
                action: r.action,
                actor_kind: r.actor_kind,
                created_at: r.created_at,
            })
            .collect(),
    }))
}

/// `PATCH /api/courier/me/password` (`me.ts:110-174`) — REV-S7-7 revokes all live sessions.
#[utoipa::path(patch, path = "/api/courier/me/password", tag = "courier",
    request_body = PatchPasswordRequest,
    responses(
        (status = 200, body = SimpleSuccessResponse),
        (status = 400, description = "Validation failed / invalid current password", body = domain::ErrorEnvelope),
        (status = 404, description = "Courier not found", body = domain::ErrorEnvelope),
    ))]
pub async fn patch_password(
    Extension(state): Extension<MeState>,
    CourierSession(courier): CourierSession,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<PatchPasswordRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);

    // Manual-Zod parity (me.ts:112-115): current_password.min(1), new_password.min(12).
    if body.current_password.is_empty() || body.new_password.len() < 12 {
        return Err(ApiError::validation_failed_400(
            "Validation failed",
            correlation_id,
        ));
    }

    let stored_hash = state
        .repo
        .get_password_hash(courier.sub)
        .await
        .map_err(|_e| internal_error(correlation_id.clone()))?
        .ok_or_else(|| not_found(correlation_id.clone(), "Courier not found"))?;

    if !crate::auth::crypto::argon2_verify(&stored_hash, &body.current_password) {
        return Err(ApiError::validation_failed_400(
            "Invalid current password",
            correlation_id,
        ));
    }

    let new_hash =
        hash_password(&body.new_password).map_err(|_e| internal_error(correlation_id.clone()))?;

    state
        .repo
        .change_password(courier.sub, courier.active_location_id, new_hash)
        .await
        .map_err(|_e| internal_error(correlation_id))?;

    Ok(Json(SimpleSuccessResponse { success: true }))
}

/// `GET /api/courier/me/earnings` (`me.ts:177-246`).
#[utoipa::path(get, path = "/api/courier/me/earnings", tag = "courier",
    responses((status = 200, description = "Earnings summary + recent payouts")))]
pub async fn get_earnings(
    Extension(state): Extension<MeState>,
    CourierSession(courier): CourierSession,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);

    let currency = state
        .repo
        .location_currency(courier.active_location_id)
        .await
        .map_err(|_e| internal_error(correlation_id.clone()))?
        .unwrap_or_else(|| "ALL".to_string());

    let today = state
        .repo
        .earnings_window(
            courier.sub,
            courier.active_location_id,
            EarningsWindowKind::Today,
        )
        .await
        .map_err(|_e| internal_error(correlation_id.clone()))?;
    let week = state
        .repo
        .earnings_window(
            courier.sub,
            courier.active_location_id,
            EarningsWindowKind::Week,
        )
        .await
        .map_err(|_e| internal_error(correlation_id.clone()))?;
    let month = state
        .repo
        .earnings_window(
            courier.sub,
            courier.active_location_id,
            EarningsWindowKind::Month,
        )
        .await
        .map_err(|_e| internal_error(correlation_id.clone()))?;

    let payouts = state
        .repo
        .recent_payouts(courier.sub, courier.active_location_id)
        .await
        .map_err(|_e| internal_error(correlation_id))?;

    Ok(Json(EarningsResponse {
        summary: EarningsSummary {
            today: today.amount,
            today_deliveries: today.deliveries,
            today_tips: today.tips,
            week: week.amount,
            week_deliveries: week.deliveries,
            week_tips: week.tips,
            month: month.amount,
            month_deliveries: month.deliveries,
            month_tips: month.tips,
            currency,
        },
        payouts: payouts
            .into_iter()
            .map(|p| PayoutSummary {
                id: p.id,
                date: p.period_end,
                amount: p.amount,
                status: p.status,
                reference: format!(
                    "Payout {} - {}",
                    p.period_start.format("%Y-%m-%d"),
                    p.period_end.format("%Y-%m-%d")
                ),
            })
            .collect(),
    }))
}

/// `GET /api/courier/me/history` (`me.ts:249-282`) — camelCase wire shape, ratings best-effort.
#[utoipa::path(get, path = "/api/courier/me/history", tag = "courier",
    responses((status = 200, description = "Delivery history")))]
pub async fn get_history(
    Extension(state): Extension<MeState>,
    CourierSession(courier): CourierSession,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let rows = state
        .repo
        .history(courier.sub, courier.active_location_id)
        .await
        .map_err(|_e| internal_error(correlation_id))?;

    let order_ids: Vec<Uuid> = rows.iter().map(|r| r.order_id).collect();
    let ratings_map: std::collections::HashMap<Uuid, (Option<i32>, Option<String>)> = state
        .repo
        .ratings_for_orders(&order_ids)
        .await
        .into_iter()
        .map(|(id, rating, feedback)| (id, (rating, feedback)))
        .collect();

    let entries: Vec<HistoryEntry> = rows
        .into_iter()
        .map(|r| {
            let (rating, feedback) = ratings_map
                .get(&r.order_id)
                .cloned()
                .unwrap_or((None, None));
            HistoryEntry {
                id: r.id,
                order_id: r.order_id,
                date: r.delivered_at.unwrap_or(r.created_at),
                restaurant: r.location_name,
                customer_address: mask_str(&r.customer_name),
                amount: r.cash_amount.unwrap_or(r.total),
                status: match r.status.as_str() {
                    "delivered" => "DELIVERED".to_string(),
                    "cancelled" => "CANCELLED".to_string(),
                    other => other.to_string(),
                },
                rating,
                feedback,
            }
        })
        .collect();

    Ok(Json(entries))
}

// ── PgMeRepo ─────────────────────────────────────────────────────────────────────────────────

pub struct PgMeRepo {
    pool: sqlx::PgPool,
}

impl PgMeRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        PgMeRepo { pool }
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
impl MeRepo for PgMeRepo {
    async fn get_profile(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<ProfileRow>, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let row: Option<ProfileRow> = sqlx::query_as(
                    "SELECT c.email_encrypted, c.phone_encrypted, c.full_name_encrypted, c.last_login_at, \
                     c.messenger_kind, c.messenger_handle, cl.role \
                     FROM couriers c JOIN courier_locations cl ON c.id = cl.courier_id \
                     WHERE c.id = $1 AND cl.location_id = $2",
                )
                .bind(courier_id)
                .bind(location_id)
                .fetch_optional(&mut **txn)
                .await?;
                Ok(row)
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn update_messenger(
        &self,
        courier_id: Uuid,
        _location_id: Uuid,
        kind: Option<String>,
        handle: Option<String>,
    ) -> Result<(), RepoError> {
        // `couriers` has no RLS (module doc) — a bare-pool write is correct.
        sqlx::query("UPDATE couriers SET messenger_kind = $2, messenger_handle = $3 WHERE id = $1")
            .bind(courier_id)
            .bind(kind)
            .bind(handle)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn audit_log(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<Vec<AuditLogRow>, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let rows: Vec<AuditLogRow> = sqlx::query_as(
                    "SELECT action, actor_kind, created_at FROM courier_audit_log \
                     WHERE courier_id = $1 ORDER BY created_at DESC LIMIT 50",
                )
                .bind(courier_id)
                .fetch_all(&mut **txn)
                .await?;
                Ok(rows)
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn get_password_hash(&self, courier_id: Uuid) -> Result<Option<String>, RepoError> {
        // `couriers` has no RLS (module doc) — a bare-pool read is correct.
        let row: Option<(String,)> =
            sqlx::query_as("SELECT password_hash FROM couriers WHERE id = $1")
                .bind(courier_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(|(h,)| h))
    }

    async fn change_password(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
        new_hash: String,
    ) -> Result<(), RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                sqlx::query("UPDATE couriers SET password_hash = $1 WHERE id = $2")
                    .bind(&new_hash)
                    .bind(courier_id)
                    .execute(&mut **txn)
                    .await?;
                // ip_hash/user_agent_hash: deliberate NULL — see module doc judgment call.
                sqlx::query(
                    "INSERT INTO courier_audit_log \
                     (courier_id, location_id, action, actor_kind, actor_id, ip_hash, user_agent_hash) \
                     VALUES ($1, $2, 'password.changed', 'courier', $1, $3, $4)",
                )
                .bind(courier_id)
                .bind(location_id)
                .bind(Option::<String>::None)
                .bind(Option::<String>::None)
                .execute(&mut **txn)
                .await?;
                // REV-S7-7: revoke every live session so the courier is forced to re-login.
                sqlx::query(
                    "UPDATE courier_sessions SET revoked_at = now() WHERE courier_id = $1 AND revoked_at IS NULL",
                )
                .bind(courier_id)
                .execute(&mut **txn)
                .await?;
                Ok(())
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn location_currency(&self, location_id: Uuid) -> Result<Option<String>, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let row: Option<(String,)> =
                    sqlx::query_as("SELECT currency_code FROM locations WHERE id = $1")
                        .bind(location_id)
                        .fetch_optional(&mut **txn)
                        .await?;
                Ok(row.map(|(c,)| c))
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn earnings_window(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
        window: EarningsWindowKind,
    ) -> Result<EarningsWindow, RepoError> {
        #[derive(sqlx::FromRow)]
        struct EarningsSqlRow {
            amount: i64,
            deliveries: i32,
            tips: i64,
        }

        let predicate = match window {
            EarningsWindowKind::Today => "CURRENT_DATE",
            EarningsWindowKind::Week => "date_trunc('week', CURRENT_DATE)",
            EarningsWindowKind::Month => "date_trunc('month', CURRENT_DATE)",
        };
        let sql = format!(
            "SELECT COALESCE(SUM(ca.cash_amount),0)::bigint AS amount, COUNT(*)::int AS deliveries, \
             COALESCE(SUM(o.tip_amount),0)::bigint AS tips \
             FROM courier_assignments ca LEFT JOIN orders o ON o.id = ca.order_id \
             WHERE ca.courier_id=$1 AND ca.status='delivered' AND ca.delivered_at >= {predicate} \
             AND ca.location_id=$2"
        );
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let row: EarningsSqlRow = sqlx::query_as(&sql)
                    .bind(courier_id)
                    .bind(location_id)
                    .fetch_one(&mut **txn)
                    .await?;
                Ok(row)
            })
        })
        .await
        .map(|row| EarningsWindow {
            amount: row.amount,
            deliveries: i64::from(row.deliveries),
            tips: row.tips,
        })
        .map_err(map_txn_err)
    }

    async fn recent_payouts(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<Vec<PayoutSummaryRow>, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let rows: Vec<PayoutSummaryRow> = sqlx::query_as(
                    "SELECT id, total_earned::bigint AS amount, status, period_start, period_end, created_at \
                     FROM courier_payouts WHERE courier_id = $1 ORDER BY created_at DESC LIMIT 20",
                )
                .bind(courier_id)
                .fetch_all(&mut **txn)
                .await?;
                Ok(rows)
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn history(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<Vec<HistoryRow>, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let rows: Vec<HistoryRow> = sqlx::query_as(
                    "SELECT a.id, a.order_id, a.status, a.delivered_at, a.created_at, a.cash_amount::bigint, \
                     o.total::bigint, c.name AS customer_name, l.name AS location_name \
                     FROM courier_assignments a \
                     JOIN orders o ON o.id = a.order_id \
                     JOIN customers c ON c.id = o.customer_id \
                     JOIN locations l ON l.id = o.location_id \
                     WHERE a.courier_id = $1 AND a.status IN ('delivered','cancelled') \
                     ORDER BY a.delivered_at DESC NULLS LAST, a.created_at DESC LIMIT 50",
                )
                .bind(courier_id)
                .fetch_all(&mut **txn)
                .await?;
                Ok(rows)
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn ratings_for_orders(
        &self,
        order_ids: &[Uuid],
    ) -> Vec<(Uuid, Option<i32>, Option<String>)> {
        if order_ids.is_empty() {
            return Vec::new();
        }
        // Best-effort: ANY error (including "relation order_ratings does not exist" pre-migration)
        // degrades to an empty map — never fails `/me/history` (me.ts:266-275).
        let result: Result<Vec<(Uuid, Option<i32>, Option<String>)>, sqlx::Error> = sqlx::query_as(
            "SELECT order_id, rating, feedback FROM order_ratings WHERE order_id = ANY($1)",
        )
        .bind(order_ids)
        .fetch_all(&self.pool)
        .await;
        result.unwrap_or_default()
    }
}

// ── FakeMeRepo (test-only) ───────────────────────────────────────────────────────────────────

#[cfg(test)]
pub mod fake {
    use super::{
        AuditLogRow, EarningsWindow, EarningsWindowKind, HistoryRow, MeRepo, PayoutSummaryRow,
        ProfileRow, RepoError,
    };
    use std::collections::HashMap;
    use std::sync::Mutex;
    use uuid::Uuid;

    /// `(courier_id, messenger_kind, messenger_handle)` — one recorded `update_messenger` call.
    /// Named so clippy's `type_complexity` lint (workspace `deny`) doesn't flag the field below.
    type MessengerUpdate = (Uuid, Option<String>, Option<String>);
    /// `(rating, feedback)` — one `order_ratings` fixture row.
    type RatingFixture = (Option<i32>, Option<String>);

    #[derive(Default)]
    pub struct FakeMeRepo {
        /// (courier_id, location_id) -> row. Absent = 404 (no membership at this location).
        pub profiles: Mutex<HashMap<(Uuid, Uuid), ProfileRow>>,
        pub messenger_updates: Mutex<Vec<MessengerUpdate>>,
        pub audit_logs: Mutex<HashMap<Uuid, Vec<AuditLogRow>>>,
        /// courier_id -> password_hash. Absent = 404 courier not found.
        pub password_hashes: Mutex<HashMap<Uuid, String>>,
        /// courier_ids whose sessions `change_password` revoked — the REV-S7-7 test hook.
        pub revoked_sessions_for: Mutex<Vec<Uuid>>,
        pub location_currencies: Mutex<HashMap<Uuid, String>>,
        pub earnings: Mutex<HashMap<(Uuid, EarningsWindowKind), EarningsWindow>>,
        pub payouts: Mutex<HashMap<Uuid, Vec<PayoutSummaryRow>>>,
        pub history_rows: Mutex<HashMap<Uuid, Vec<HistoryRow>>>,
        pub ratings: Mutex<HashMap<Uuid, RatingFixture>>,
        /// When true, `ratings_for_orders` simulates the "order_ratings not migrated" case.
        pub ratings_table_missing: Mutex<bool>,
    }

    // A test fixture deliberately exposes a COMPLETE seed API (one seeder per table it models); a
    // given test suite uses a subset, so a few seeders are legitimately unused here. Allowed at the
    // impl level rather than per-method — the fixture's contract is the whole seed surface.
    #[allow(
        dead_code,
        reason = "test fixture exposes a full seed API; this suite uses a subset"
    )]
    impl FakeMeRepo {
        pub fn seed_profile(&self, courier_id: Uuid, location_id: Uuid, row: ProfileRow) {
            self.profiles
                .lock()
                .unwrap()
                .insert((courier_id, location_id), row);
        }
        pub fn seed_password(&self, courier_id: Uuid, hash: String) {
            self.password_hashes
                .lock()
                .unwrap()
                .insert(courier_id, hash);
        }
        pub fn seed_currency(&self, location_id: Uuid, currency: String) {
            self.location_currencies
                .lock()
                .unwrap()
                .insert(location_id, currency);
        }
        pub fn seed_earnings(
            &self,
            courier_id: Uuid,
            window: EarningsWindowKind,
            w: EarningsWindow,
        ) {
            self.earnings
                .lock()
                .unwrap()
                .insert((courier_id, window), w);
        }
        pub fn seed_payouts(&self, courier_id: Uuid, rows: Vec<PayoutSummaryRow>) {
            self.payouts.lock().unwrap().insert(courier_id, rows);
        }
        pub fn seed_history(&self, courier_id: Uuid, rows: Vec<HistoryRow>) {
            self.history_rows.lock().unwrap().insert(courier_id, rows);
        }
        pub fn seed_rating(&self, order_id: Uuid, rating: Option<i32>, feedback: Option<String>) {
            self.ratings
                .lock()
                .unwrap()
                .insert(order_id, (rating, feedback));
        }
        pub fn set_ratings_table_missing(&self) {
            *self.ratings_table_missing.lock().unwrap() = true;
        }
    }

    #[async_trait::async_trait]
    impl MeRepo for FakeMeRepo {
        async fn get_profile(
            &self,
            courier_id: Uuid,
            location_id: Uuid,
        ) -> Result<Option<ProfileRow>, RepoError> {
            Ok(self
                .profiles
                .lock()
                .unwrap()
                .get(&(courier_id, location_id))
                .cloned())
        }

        async fn update_messenger(
            &self,
            courier_id: Uuid,
            _location_id: Uuid,
            kind: Option<String>,
            handle: Option<String>,
        ) -> Result<(), RepoError> {
            self.messenger_updates
                .lock()
                .unwrap()
                .push((courier_id, kind, handle));
            Ok(())
        }

        async fn audit_log(
            &self,
            courier_id: Uuid,
            _location_id: Uuid,
        ) -> Result<Vec<AuditLogRow>, RepoError> {
            Ok(self
                .audit_logs
                .lock()
                .unwrap()
                .get(&courier_id)
                .cloned()
                .unwrap_or_default())
        }

        async fn get_password_hash(&self, courier_id: Uuid) -> Result<Option<String>, RepoError> {
            Ok(self
                .password_hashes
                .lock()
                .unwrap()
                .get(&courier_id)
                .cloned())
        }

        async fn change_password(
            &self,
            courier_id: Uuid,
            _location_id: Uuid,
            new_hash: String,
        ) -> Result<(), RepoError> {
            self.password_hashes
                .lock()
                .unwrap()
                .insert(courier_id, new_hash);
            self.revoked_sessions_for.lock().unwrap().push(courier_id);
            Ok(())
        }

        async fn location_currency(&self, location_id: Uuid) -> Result<Option<String>, RepoError> {
            Ok(self
                .location_currencies
                .lock()
                .unwrap()
                .get(&location_id)
                .cloned())
        }

        async fn earnings_window(
            &self,
            courier_id: Uuid,
            _location_id: Uuid,
            window: EarningsWindowKind,
        ) -> Result<EarningsWindow, RepoError> {
            Ok(self
                .earnings
                .lock()
                .unwrap()
                .get(&(courier_id, window))
                .copied()
                .unwrap_or_default())
        }

        async fn recent_payouts(
            &self,
            courier_id: Uuid,
            _location_id: Uuid,
        ) -> Result<Vec<PayoutSummaryRow>, RepoError> {
            Ok(self
                .payouts
                .lock()
                .unwrap()
                .get(&courier_id)
                .cloned()
                .unwrap_or_default())
        }

        async fn history(
            &self,
            courier_id: Uuid,
            _location_id: Uuid,
        ) -> Result<Vec<HistoryRow>, RepoError> {
            Ok(self
                .history_rows
                .lock()
                .unwrap()
                .get(&courier_id)
                .cloned()
                .unwrap_or_default())
        }

        async fn ratings_for_orders(
            &self,
            order_ids: &[Uuid],
        ) -> Vec<(Uuid, Option<i32>, Option<String>)> {
            if *self.ratings_table_missing.lock().unwrap() {
                return Vec::new();
            }
            let ratings = self.ratings.lock().unwrap();
            order_ids
                .iter()
                .filter_map(|id| ratings.get(id).map(|(r, f)| (*id, *r, f.clone())))
                .collect()
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::fake::FakeMeRepo;
    use super::*;
    use crate::auth::claims::CourierClaims;
    use crate::auth::repo::fake::FakeAuthRepo;
    use axum::body::to_bytes;
    use axum::http::StatusCode;
    use axum::response::Response;
    use std::sync::Arc;

    fn request_id() -> RequestId {
        RequestId::new(axum::http::HeaderValue::from_static("corr-1"))
    }

    fn courier_session(courier_id: Uuid, location_id: Uuid) -> CourierSession {
        CourierSession(CourierClaims::new(courier_id, location_id, None))
    }

    fn state_with(repo: FakeMeRepo) -> MeState {
        MeState {
            auth: AuthState::test_state(Arc::new(FakeAuthRepo::default())),
            repo: Arc::new(repo),
        }
    }

    async fn json_body(resp: Response) -> (StatusCode, serde_json::Value) {
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    /// The same fixed test key `AuthState::test_state` wires into `pii_cipher` — lets a test
    /// encrypt fixture PII that the handler under test can then correctly decrypt.
    fn test_cipher() -> crate::auth::pii::PiiCipher {
        use base64::Engine;
        let key_b64 = base64::engine::general_purpose::STANDARD.encode([7u8; 32]);
        crate::auth::pii::PiiCipher::from_base64(&key_b64).unwrap()
    }

    // ── get_profile ──

    #[tokio::test]
    async fn get_profile_404_when_courier_has_no_membership_at_this_location() {
        let state = state_with(FakeMeRepo::default());
        let courier_id = Uuid::new_v4();
        let location = Uuid::new_v4();

        let err = crate::error::expect_err(
            get_profile(
                Extension(state),
                courier_session(courier_id, location),
                Extension(request_id()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn get_profile_masks_email_and_phone_but_returns_plaintext_full_name() {
        let courier_id = Uuid::new_v4();
        let location = Uuid::new_v4();
        let cipher = test_cipher();
        let repo = FakeMeRepo::default();
        repo.seed_profile(
            courier_id,
            location,
            ProfileRow {
                email_encrypted: cipher.encrypt("rider@example.com").unwrap(),
                phone_encrypted: Some(cipher.encrypt("+15551234567").unwrap()),
                full_name_encrypted: cipher.encrypt("Jane Rider").unwrap(),
                last_login_at: None,
                messenger_kind: None,
                messenger_handle: None,
                role: "courier".to_string(),
            },
        );
        let state = state_with(repo);

        let resp = get_profile(
            Extension(state),
            courier_session(courier_id, location),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        let (_, json) = json_body(resp).await;

        // The courier sees their OWN plaintext name — only email/phone are masked (me.ts:63-65).
        assert_eq!(json["full_name"], "Jane Rider");
        assert_eq!(json["masked_email"], mask_str("rider@example.com"));
        assert_ne!(json["masked_email"], "rider@example.com");
        assert_eq!(json["masked_phone"], mask_str("+15551234567"));
        assert_ne!(json["masked_phone"], "+15551234567");
        assert_eq!(json["active_location"]["role"], "courier");
    }

    // ── patch_messenger ──

    #[test]
    fn patch_messenger_request_rejects_an_unknown_field() {
        let json = serde_json::json!({ "messenger_kind": "telegram", "extra": "nope" });
        assert!(serde_json::from_value::<PatchMessengerRequest>(json).is_err());
    }

    #[tokio::test]
    async fn patch_messenger_clears_both_when_handle_is_blank() {
        // Both-or-neither (me.ts:86-88): a kind with a blank/whitespace-only handle clears both.
        let courier_id = Uuid::new_v4();
        let location = Uuid::new_v4();
        let state = state_with(FakeMeRepo::default());

        let resp = patch_messenger(
            Extension(state),
            courier_session(courier_id, location),
            Extension(request_id()),
            Json(PatchMessengerRequest {
                messenger_kind: Some(MessengerKind::Telegram),
                messenger_handle: Some("   ".to_string()),
            }),
        )
        .await
        .unwrap()
        .into_response();
        let (_, json) = json_body(resp).await;
        assert_eq!(json["messenger_kind"], serde_json::Value::Null);
        assert_eq!(json["messenger_handle"], serde_json::Value::Null);
    }

    // ── patch_password ──

    #[tokio::test]
    async fn patch_password_rejects_new_password_under_12_chars() {
        let courier_id = Uuid::new_v4();
        let location = Uuid::new_v4();
        let state = state_with(FakeMeRepo::default());

        let err = crate::error::expect_err(
            patch_password(
                Extension(state),
                courier_session(courier_id, location),
                Extension(request_id()),
                Json(PatchPasswordRequest {
                    current_password: "whatever".to_string(),
                    new_password: "short".to_string(),
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::ValidationFailed);
        assert_eq!(err.envelope.status, 400);
    }

    #[tokio::test]
    async fn patch_password_wrong_current_password_is_400() {
        let courier_id = Uuid::new_v4();
        let location = Uuid::new_v4();
        let repo = FakeMeRepo::default();
        repo.seed_password(courier_id, hash_password("correct-horse-1").unwrap());
        let state = state_with(repo);

        let err = crate::error::expect_err(
            patch_password(
                Extension(state),
                courier_session(courier_id, location),
                Extension(request_id()),
                Json(PatchPasswordRequest {
                    current_password: "totally-wrong".to_string(),
                    new_password: "a-new-password-123".to_string(),
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::ValidationFailed);
        assert_eq!(err.envelope.status, 400);
    }

    /// 🔴 REV-S7-7 DoD: a successful password change revokes every live session.
    #[tokio::test]
    async fn patch_password_revokes_all_live_sessions_on_success() {
        let courier_id = Uuid::new_v4();
        let location = Uuid::new_v4();
        let repo = Arc::new(FakeMeRepo::default());
        repo.seed_password(courier_id, hash_password("correct-horse-1").unwrap());
        let state = MeState {
            auth: AuthState::test_state(Arc::new(FakeAuthRepo::default())),
            repo: repo.clone(),
        };

        let resp = patch_password(
            Extension(state),
            courier_session(courier_id, location),
            Extension(request_id()),
            Json(PatchPasswordRequest {
                current_password: "correct-horse-1".to_string(),
                new_password: "a-new-password-123".to_string(),
            }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(resp.status(), StatusCode::OK);

        // The security-critical assertion: the fake recorded a session-revoke call for THIS
        // courier (the real repo's `change_password` runs it in the SAME transaction as the
        // password write — REV-S7-7).
        assert_eq!(
            repo.revoked_sessions_for.lock().unwrap().as_slice(),
            &[courier_id]
        );
    }

    // ── get_history ──

    fn fixture_history_row(order_id: Uuid, customer_name: &str) -> HistoryRow {
        HistoryRow {
            id: Uuid::new_v4(),
            order_id,
            status: "delivered".to_string(),
            delivered_at: Some(Utc::now()),
            created_at: Utc::now(),
            cash_amount: Some(1200),
            total: 1500,
            customer_name: customer_name.to_string(),
            location_name: "Eljo's Pizza".to_string(),
        }
    }

    #[tokio::test]
    async fn get_history_masks_customer_name_via_mask_str() {
        let courier_id = Uuid::new_v4();
        let location = Uuid::new_v4();
        let order_id = Uuid::new_v4();
        let repo = FakeMeRepo::default();
        repo.seed_history(courier_id, vec![fixture_history_row(order_id, "Jane Doe")]);
        let state = state_with(repo);

        let resp = get_history(
            Extension(state),
            courier_session(courier_id, location),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        let (_, json) = json_body(resp).await;
        let entry = &json.as_array().unwrap()[0];
        assert_eq!(entry["customerAddress"], mask_str("Jane Doe"));
        assert_ne!(entry["customerAddress"], "Jane Doe");
        assert_eq!(entry["status"], "DELIVERED");
        assert_eq!(entry["amount"], 1200);
    }

    #[tokio::test]
    async fn get_history_survives_a_missing_order_ratings_table() {
        let courier_id = Uuid::new_v4();
        let location = Uuid::new_v4();
        let order_id = Uuid::new_v4();
        let repo = FakeMeRepo::default();
        repo.seed_history(courier_id, vec![fixture_history_row(order_id, "Jane Doe")]);
        repo.set_ratings_table_missing();
        let state = state_with(repo);

        let resp = get_history(
            Extension(state),
            courier_session(courier_id, location),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        let (_, json) = json_body(resp).await;
        let entry = &json.as_array().unwrap()[0];
        assert_eq!(entry["rating"], serde_json::Value::Null);
        assert_eq!(entry["feedback"], serde_json::Value::Null);
    }

    // ── get_earnings ──

    #[tokio::test]
    async fn get_earnings_defaults_currency_to_all_when_location_missing() {
        let courier_id = Uuid::new_v4();
        let location = Uuid::new_v4();
        let state = state_with(FakeMeRepo::default());

        let resp = get_earnings(
            Extension(state),
            courier_session(courier_id, location),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        let (_, json) = json_body(resp).await;
        assert_eq!(json["summary"]["currency"], "ALL");
        assert_eq!(json["summary"]["today"], 0);
    }
}
