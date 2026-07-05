//! S10 Plane A — the platform-ops surface (`/api/admin/*`, 6 routes). Ports
//! `apps/api/src/routes/admin/{backups,fallback,notification-audit}.ts` + the audit trail of
//! `lib/platform-admin.ts:85-141`. Docs: `docs/design/rebuild-platform-admin-s10-council/`.
//!
//! ## Auth: the RUNTIME plane-gate, NOT a per-handler check (REV-S10-1)
//! Every request to `/api/admin/*` has ALREADY passed
//! [`crate::auth::plane_gate::platform_admin_plane_gate`] (the outermost layer in `main.rs`), which
//! verified the bearer, narrowed it to `OwnerClaims`, re-read the `platform_admins` allowlist
//! (403 miss / 503 DB-blip fail-closed), and INSERTED the verified `OwnerClaims` into request
//! extensions. So these handlers bind `Extension<OwnerClaims>` for the audit actor_id — they add NO
//! second auth check (a per-handler owner check would 403 a legitimate platform-admin, the exact B4
//! bug). The structural authority is the runtime gate; this file is the 6 known handlers.
//!
//! ## The four carried/fixed controls in this file
//!  - **REV-S10-4** — the DR-drill trigger CARRIES the gate-verified actor across the (thin Rust →
//!    Node) boundary as a REQUIRED `Uuid`; a missing actor → a typed reject, NEVER `'unknown'`
//!    ([`resolve_drill_actor`], [`RestoreDrill::trigger`]).
//!  - **REV-S10-2** — [`AdminOpsRepo::fallback_health`] is the cross-tenant platform-read; its Pg SQL
//!    is marked for the `owner_notification_targets` DEFINER swap that gates the FLIP (not this dark
//!    build) — see [`PgAdminOpsRepo::fallback_health`].
//!  - **Q-BACKUP-KEY** — [`resolve_backup_key`] is env-only + FAILS LOUD on an unknown keyId, and
//!    never logs/returns the key VALUE (the secrets-exposure-incident red-line).
//!  - **Q-DRILL-RESTORE-RUNBOOK** — there is NO restore-to-prod route. The drills target a SANDBOX;
//!    a real restore-over-prod is a manual runbook and its OWN council, never an S10 side effect.
//!    Pinned by [`tests::admin_router_exposes_no_restore_to_prod_route`].
//!
//! ## Write-ahead audit (Q-ADMIN-AUDIT)
//! Before a destructive drill, [`AdminOpsRepo::audit_start`] commits a `started` row in its OWN
//! statement (so no side-effect can occur without a pre-committed trail), then the drill runs, then
//! [`AdminOpsRepo::audit_finish`] closes it completed/failed. Reads best-effort a `completed` row
//! (a read must not fail on an audit blip). Only hashed ip/ua — never raw PII.
//!
//! ## No live DB in this sandbox
//! `PgAdminOpsRepo`'s SQL-touching methods are `#[ignore]`-probed (need a real Postgres), the same
//! posture as `orders::pg`/`owner::mod`. The pure decisions (drill-actor fail-closed, backup-key
//! fail-loud, uuid validation, audit ordering, the no-restore assertion) are unit-tested with fakes.
//!
//! ## `dead_code` allowance (dark-surface scope — same rationale as `auth/mod.rs`)
//! S10 is a DARK surface built AHEAD of its launch-wiring: `resolve_backup_key`/`BackupKeyError`
//! (the fail-loud restore-key control) and `DrillError::InProgress` (the single-flight 409 a REAL
//! Node drill returns) are forward-looking API bound by the not-yet-connected Node drill seam +
//! this module's own tests. Scoped to this module only; every other lint stays `-D`.
#![allow(
    dead_code,
    reason = "dark S10 platform-ops surface — forward-looking API bound by future launch-wiring + tests"
)]

use std::sync::Arc;

use axum::extract::Extension;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use uuid::Uuid;

use crate::auth::claims::OwnerClaims;

// ─────────────────────────── Q-BACKUP-KEY: fail-loud, env-only ───────────────────────────

/// A backup-key resolution failure. Fails LOUD (the drill/restore refuses to run with the wrong key
/// and silently produce garbage — the primary restore-to-wrong-target control, S10-T5). The error
/// carries the keyId NAME, never the key VALUE (secrets-exposure-incident, Q-DRILL-REDACT).
#[derive(Debug, thiserror::Error)]
pub enum BackupKeyError {
    #[error("BACKUP_KEYRING is not valid JSON")]
    KeyringNotJson,
    #[error(
        "Unknown backup keyId '{0}': not in BACKUP_KEYRING or BACKUP_ENCRYPTION_KEY. Refusing to restore with an unverified key."
    )]
    UnknownKeyId(String),
}

/// Port of `resolveBackupKey` (`workers/backup/encrypt.ts:44-72`). Resolves a keyId → its base64
/// AES-256 key from the keyring/primary. **Env-only** (reads are passed in from `std::env`, NOT the
/// config Zod schema — so the operator-gated secret VALUE can land without a code change) and
/// **fails LOUD** on an unknown keyId. Pure (env values injected) so every branch is unit-tested and
/// so the key VALUE never touches a log line here. The default `'primary'` keyId falls back to the
/// single `BACKUP_ENCRYPTION_KEY`.
pub fn resolve_backup_key(
    key_id: &str,
    keyring_json: Option<&str>,
    primary: Option<&str>,
) -> Result<String, BackupKeyError> {
    let mut keyring: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Some(raw) = keyring_json {
        let parsed: serde_json::Value =
            serde_json::from_str(raw).map_err(|_e| BackupKeyError::KeyringNotJson)?;
        if let Some(obj) = parsed.as_object() {
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    keyring.insert(k.clone(), s.to_string());
                }
            }
        }
    }
    if let Some(p) = primary {
        keyring
            .entry("primary".to_string())
            .or_insert_with(|| p.to_string());
    }
    keyring
        .get(key_id)
        .cloned()
        .ok_or_else(|| BackupKeyError::UnknownKeyId(key_id.to_string()))
}

// ─────────────────────────── REV-S10-4: drill actor, fail-closed ───────────────────────────

/// A drill rejection BEFORE any side-effect. `MissingActor` is the REV-S10-4 fail-closed: the drill
/// trigger crosses a (thin Rust → Node) process boundary; if the gate-verified actor is not carried
/// across, the drill is REJECTED — never run with an `'unknown'` actor, never a lost attribution.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DrillError {
    #[error(
        "drill actor is required (REV-S10-4: never attribute a destructive drill to 'unknown')"
    )]
    MissingActor,
    /// The single-flight advisory lock (`backup-verify.ts:65-84`) — a drill is already in flight → 409.
    #[error("another verify is already in progress")]
    InProgress,
    /// The Node drill returned a failure (or, in the dark mount, the Node seam is not wired).
    #[error("drill failed")]
    Failed,
}

/// REV-S10-4 — resolve the actor for a destructive drill. The actor MUST be the gate-verified
/// `OwnerClaims.user_id` (present in extensions behind the runtime gate). `None` (a wiring gap, or a
/// direct invoke that did not carry the request principal across the carve-out boundary) → a typed
/// `MissingActor` reject, NEVER a `'unknown'` fallback (the exact `auditCtx` `?? 'unknown'` fail-open
/// this fixes, `platform-admin.ts:104`).
pub fn resolve_drill_actor(owner: Option<&OwnerClaims>) -> Result<Uuid, DrillError> {
    owner.map(|o| o.user_id).ok_or(DrillError::MissingActor)
}

/// A completed drill's outcome (SANDBOX target — never prod; the LC7 fix-2 canonical
/// restore-to-wrong-target defect is that the smoke pool must be the sandbox, not prod).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct DrillOutcome {
    pub success: bool,
    /// Which db the drill restored into — asserted to be the SANDBOX, proving no prod side-effect.
    pub target: String,
}

/// The thin Rust → Node DR-drill trigger (Q-DRILL-NODE-CARVEOUT). The 400-line
/// subprocess/stream/crypto `runRestoreVerify` pipeline STAYS ON NODE; the Rust admin route only
/// gates + audits + validates + rate-limits, then invokes the Node drill via this trait (a job
/// enqueue / narrow internal call). **REV-S10-4: `actor` is a REQUIRED value parameter** — it is
/// carried EXPLICITLY across the process boundary, never read from an ambient request context that
/// can go missing. It is structurally impossible to trigger a drill without an actor.
#[async_trait::async_trait]
pub trait RestoreDrill: Send + Sync {
    async fn trigger(
        &self,
        actor: Uuid,
        backup_id: Option<Uuid>,
        full_hash: bool,
    ) -> Result<DrillOutcome, DrillError>;
}

/// The DARK Node-drill seam (Q-DRILL-NODE-CARVEOUT). The 400-line subprocess/stream/crypto
/// `runRestoreVerify` pipeline STAYS on Node — this thin Rust trigger only CARRIES the gate-verified
/// actor (REV-S10-4, an explicit `Uuid` value across the process boundary) + the write-ahead audit +
/// uuid-validation + kill-switch, then would invoke the Node drill. In this DARK mount the Node
/// process/queue is not connected, so a triggered drill logs the actor-attributed intent and returns
/// `Failed` — LAUNCHING the drill is the separate, explicit act of wiring the queue. The AUTHORITY is
/// Rust (and unit-proven); the pipeline is not re-ported (boring-wins, over-engineering avoided).
pub struct NodeRestoreDrill;

#[async_trait::async_trait]
impl RestoreDrill for NodeRestoreDrill {
    async fn trigger(
        &self,
        actor: Uuid,
        backup_id: Option<Uuid>,
        full_hash: bool,
    ) -> Result<DrillOutcome, DrillError> {
        // The actor is CARRIED here explicitly (REV-S10-4) — it is a value parameter, never read from
        // an ambient request context that could go missing across the Rust→Node boundary. Never logs
        // any key/secret (Q-DRILL-REDACT) — only the actor uuid + the backup id + the flag.
        tracing::warn!(
            %actor, ?backup_id, full_hash,
            "S10 DR-drill trigger invoked — Node drill seam not wired in the dark mount (actor carried; launching wires the queue)"
        );
        Err(DrillError::Failed)
    }
}

// ─────────────────────────── audit ctx (Q-ADMIN-AUDIT, hashed ip/ua) ───────────────────────────

fn sha256_hex(s: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}

/// The audit context (`auditCtx`, `platform-admin.ts:97-110`). actor_id = the gate-verified
/// `OwnerClaims.user_id` (never `'unknown'`); ip/ua are HASHED (no raw PII in the audit trail).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditCtx {
    pub actor_id: Uuid,
    pub action: String,
    pub target: Option<String>,
    pub ip_hash: Option<String>,
    pub ua_hash: Option<String>,
}

impl AuditCtx {
    fn build(actor_id: Uuid, action: &str, target: Option<String>, headers: &HeaderMap) -> Self {
        let ua = headers
            .get(axum::http::header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(sha256_hex);
        // ip is not directly available here without a ConnectInfo layer; the Node hash is of
        // `request.ip`. In this port the ip hash is carried when a forwarded-for header is present
        // (the same shape the Node proxy sets); absent → None (still no raw PII).
        let ip = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .map(|s| sha256_hex(s.split(',').next().unwrap_or(s).trim()));
        AuditCtx {
            actor_id,
            action: action.to_string(),
            target,
            ip_hash: ip,
            ua_hash: ua,
        }
    }
}

// ─────────────────────────── repo (SQL pinned; Pg #[ignore]-probed) ───────────────────────────

#[derive(Debug, thiserror::Error)]
#[error("admin ops repo error")]
pub struct AdminRepoError;

/// One backup row (`backups.ts:56-65`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct BackupRow {
    pub id: String,
    pub r#type: String,
    pub status: String,
    pub has_checksum: bool,
    pub restore_test_result: Option<String>,
}

/// One fleet fallback-health row (`fallback.ts:26-40`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct FallbackHealthRow {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub telegram_active: i64,
    pub push_active: i64,
    pub dead_channels: i64,
}

/// R2 fallback coverage (`fallback.ts:53-59`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct R2Coverage {
    pub total_locations: i64,
    pub with_fallback_phone: i64,
    pub coverage_pct: i64,
}

/// The platform-ops data authority (backups list, cross-tenant fallback reads, notification-audit)
/// + the write-ahead audit trail. Narrow, per-surface repo (the S3 convention), with a Fake for the
/// pure handler tests and a Pg impl carrying the exact SQL.
#[async_trait::async_trait]
pub trait AdminOpsRepo: Send + Sync {
    async fn list_backups(
        &self,
        type_filter: Option<String>,
        status_filter: Option<String>,
        limit: i64,
    ) -> Result<Vec<BackupRow>, AdminRepoError>;

    /// REV-S10-2 — the cross-tenant fallback-health read over ALL tenants. `owner_notification_targets`
    /// is FORCE tenant-isolated (mig 1790000000077:99), so post-B3 this needs a platform-read DEFINER
    /// or role (see the Pg impl's marker). `locations` reads fine via `public_select` — the false
    /// premise the breaker corrected (H1): the real B3 blocker is the notification-targets columns.
    async fn fallback_health(&self) -> Result<Vec<FallbackHealthRow>, AdminRepoError>;

    async fn r2_check(&self) -> Result<R2Coverage, AdminRepoError>;

    async fn notification_audit(
        &self,
        event: String,
        since_minutes: i64,
    ) -> Result<Vec<serde_json::Value>, AdminRepoError>;

    /// WRITE-AHEAD `started` row committed BEFORE a destructive drill. Returns its id.
    async fn audit_start(&self, ctx: &AuditCtx) -> Result<String, AdminRepoError>;
    /// Close a write-ahead row to completed/failed.
    async fn audit_finish(&self, id: &str, status: &str) -> Result<(), AdminRepoError>;
    /// Best-effort single `completed` row for a read (a read must not fail on an audit blip).
    async fn audit_completed(&self, ctx: &AuditCtx);
}

// ─────────────────────────── state + router ───────────────────────────

#[derive(Clone)]
pub struct PlatformAdminState {
    pub repo: Arc<dyn AdminOpsRepo>,
    pub drill: Arc<dyn RestoreDrill>,
    /// `ADMIN_DRILLS_ENABLED` — the kill-switch scopes ONLY the two heavy drills (verify, dr-report).
    /// The recovery READS (backups list, fallback/health) are NEVER darkened during an incident
    /// (Q-DRILL-HARDENING). Default false in the dark build.
    pub drills_enabled: bool,
}

/// Assemble the 6-route platform-ops surface at the SAME paths the Node API serves. Auth is the
/// runtime plane-gate (`main.rs` outermost layer), NOT a router-level layer here — nesting a gate
/// on THIS router is exactly the weaker mechanism breaker-C1 rejected; the gate must wrap the whole
/// app so a future sibling admin route cannot escape it.
pub fn admin_router(state: PlatformAdminState) -> axum::Router {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/admin/backups", get(list_backups))
        .route("/api/admin/backups/verify", post(verify_backup))
        .route("/api/admin/backups/dr-report", get(dr_report))
        .route("/api/admin/fallback/health", get(fallback_health))
        .route("/api/admin/fallback/r2-check", post(r2_check))
        .route("/api/admin/notification-audit", get(notification_audit))
        .layer(axum::Extension(state))
}

fn err(status: StatusCode, code: &str) -> Response {
    (status, axum::Json(serde_json::json!({ "error": code }))).into_response()
}

// ─────────────────────────── handlers ───────────────────────────

/// `GET /api/admin/backups` — list recent backups + restore-test results (read). A recovery read:
/// NEVER kill-switched.
#[utoipa::path(get, path = "/api/admin/backups", tag = "platform-admin",
    responses((status = 200, description = "backup list"), (status = 503, description = "read error")))]
pub async fn list_backups(
    Extension(state): Extension<PlatformAdminState>,
    Extension(owner): Extension<OwnerClaims>,
    headers: HeaderMap,
) -> Response {
    match state.repo.list_backups(None, None, 50).await {
        Ok(backups) => {
            state
                .repo
                .audit_completed(&AuditCtx::build(
                    owner.user_id,
                    "backups.list",
                    None,
                    &headers,
                ))
                .await;
            axum::Json(serde_json::json!({ "backups": backups })).into_response()
        }
        Err(_) => err(StatusCode::SERVICE_UNAVAILABLE, "backups_unavailable"),
    }
}

/// `POST /api/admin/backups/verify` — trigger a manual restore DRILL (weaponizable). Q-DRILL-HARDENING:
/// kill-switch → uuid-validate `backupId` → resolve the actor (REV-S10-4) → WRITE-AHEAD audit →
/// invoke the Node drill → finish audit. Single-flight 409 surfaces from the trigger.
#[utoipa::path(post, path = "/api/admin/backups/verify", tag = "platform-admin",
    responses((status = 200, description = "drill result"), (status = 400, description = "VALIDATION_FAILED"),
        (status = 403, description = "drills disabled"), (status = 409, description = "drill_in_progress"),
        (status = 500, description = "actor required (REV-S10-4)")))]
pub async fn verify_backup(
    Extension(state): Extension<PlatformAdminState>,
    owner: Option<Extension<OwnerClaims>>,
    headers: HeaderMap,
    body: Option<axum::Json<serde_json::Value>>,
) -> Response {
    let backup_id = match body
        .as_ref()
        .and_then(|b| b.0.get("backupId"))
        .and_then(|v| v.as_str())
    {
        Some(s) => match Uuid::parse_str(s) {
            Ok(u) => Some(u),
            Err(_) => return err(StatusCode::BAD_REQUEST, "VALIDATION_FAILED"),
        },
        None => None,
    };
    run_drill(
        &state,
        owner.map(|e| e.0),
        &headers,
        backup_id,
        false,
        "backups.verify",
    )
    .await
}

/// `GET /api/admin/backups/dr-report` — fleet DR-drill report (heavy, `fullHash:true`).
#[utoipa::path(get, path = "/api/admin/backups/dr-report", tag = "platform-admin",
    responses((status = 200, description = "dr report"), (status = 403, description = "drills disabled"),
        (status = 409, description = "drill_in_progress"), (status = 500, description = "actor required")))]
pub async fn dr_report(
    Extension(state): Extension<PlatformAdminState>,
    owner: Option<Extension<OwnerClaims>>,
    headers: HeaderMap,
) -> Response {
    run_drill(
        &state,
        owner.map(|e| e.0),
        &headers,
        None,
        true,
        "backups.dr_report",
    )
    .await
}

/// Shared drill path (verify + dr-report): the Q-DRILL-HARDENING conjunction + REV-S10-4 actor +
/// write-ahead audit. Kept in one place so both heavy drills carry the identical control set.
async fn run_drill(
    state: &PlatformAdminState,
    owner: Option<OwnerClaims>,
    headers: &HeaderMap,
    backup_id: Option<Uuid>,
    full_hash: bool,
    action: &str,
) -> Response {
    if !state.drills_enabled {
        return err(StatusCode::FORBIDDEN, "DRILLS_DISABLED");
    }
    // REV-S10-4: the actor MUST be present (gate-inserted). Missing → reject, NEVER 'unknown'.
    let actor = match resolve_drill_actor(owner.as_ref()) {
        Ok(a) => a,
        Err(_) => return err(StatusCode::INTERNAL_SERVER_ERROR, "actor_required"),
    };
    let ctx = AuditCtx::build(actor, action, backup_id.map(|u| u.to_string()), headers);
    // WRITE-AHEAD: a durable 'started' row BEFORE the destructive drill runs.
    let audit_id = match state.repo.audit_start(&ctx).await {
        Ok(id) => id,
        Err(_) => return err(StatusCode::SERVICE_UNAVAILABLE, "audit_unavailable"),
    };
    match state.drill.trigger(actor, backup_id, full_hash).await {
        Ok(outcome) => {
            let status = if outcome.success {
                "completed"
            } else {
                "failed"
            };
            let _closed = state.repo.audit_finish(&audit_id, status).await; // best-effort close
            axum::Json(outcome).into_response()
        }
        Err(DrillError::InProgress) => {
            let _closed = state.repo.audit_finish(&audit_id, "failed").await;
            err(StatusCode::CONFLICT, "drill_in_progress")
        }
        Err(_) => {
            let _closed = state.repo.audit_finish(&audit_id, "failed").await;
            err(StatusCode::INTERNAL_SERVER_ERROR, "drill_failed")
        }
    }
}

/// `GET /api/admin/fallback/health` — REV-S10-2 cross-tenant fallback health. A recovery read: NEVER
/// kill-switched. The read goes through [`AdminOpsRepo::fallback_health`] — the platform-read path
/// whose Pg SQL is marked for the `owner_notification_targets` DEFINER swap that gates the FLIP.
#[utoipa::path(get, path = "/api/admin/fallback/health", tag = "platform-admin",
    responses((status = 200, description = "fleet fallback health"), (status = 503, description = "read error")))]
pub async fn fallback_health(
    Extension(state): Extension<PlatformAdminState>,
    Extension(owner): Extension<OwnerClaims>,
    headers: HeaderMap,
) -> Response {
    match state.repo.fallback_health().await {
        Ok(locations) => {
            state
                .repo
                .audit_completed(&AuditCtx::build(
                    owner.user_id,
                    "fallback.health",
                    None,
                    &headers,
                ))
                .await;
            axum::Json(serde_json::json!({ "locations": locations })).into_response()
        }
        Err(_) => err(StatusCode::SERVICE_UNAVAILABLE, "fallback_unavailable"),
    }
}

/// `POST /api/admin/fallback/r2-check` — cross-tenant fallback coverage %.
#[utoipa::path(post, path = "/api/admin/fallback/r2-check", tag = "platform-admin",
    responses((status = 200, description = "coverage"), (status = 503, description = "read error")))]
pub async fn r2_check(
    Extension(state): Extension<PlatformAdminState>,
    Extension(owner): Extension<OwnerClaims>,
    headers: HeaderMap,
) -> Response {
    match state.repo.r2_check().await {
        Ok(cov) => {
            state
                .repo
                .audit_completed(&AuditCtx::build(
                    owner.user_id,
                    "fallback.r2_check",
                    None,
                    &headers,
                ))
                .await;
            axum::Json(cov).into_response()
        }
        Err(_) => err(StatusCode::SERVICE_UNAVAILABLE, "fallback_unavailable"),
    }
}

/// `GET /api/admin/notification-audit` — PII-free event/status/channel/count rollup. Q-ADMIN-ERR-LEAK:
/// a query error returns a GENERIC message, never `err.message` (a schema/internal detail).
#[utoipa::path(get, path = "/api/admin/notification-audit", tag = "platform-admin",
    params(("event" = String, Query, description = "event name")),
    responses((status = 200, description = "audit rollup"), (status = 400, description = "VALIDATION_FAILED"),
        (status = 500, description = "generic error (no leak)")))]
pub async fn notification_audit(
    Extension(state): Extension<PlatformAdminState>,
    Extension(owner): Extension<OwnerClaims>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<NotificationAuditQuery>,
) -> Response {
    if q.event.is_empty() || q.event.len() > 50 {
        return err(StatusCode::BAD_REQUEST, "VALIDATION_FAILED");
    }
    let since = q.since_minutes.unwrap_or(30);
    match state.repo.notification_audit(q.event.clone(), since).await {
        Ok(rows) => {
            state
                .repo
                .audit_completed(&AuditCtx::build(
                    owner.user_id,
                    "notification_audit.query",
                    None,
                    &headers,
                ))
                .await;
            axum::Json(serde_json::json!({ "audit": rows })).into_response()
        }
        // Q-ADMIN-ERR-LEAK: generic, never the underlying error detail.
        Err(_) => err(StatusCode::INTERNAL_SERVER_ERROR, "Audit query failed"),
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct NotificationAuditQuery {
    pub event: String,
    #[serde(rename = "sinceMinutes")]
    pub since_minutes: Option<i64>,
}

// ─────────────────────────── Pg impl (SQL pinned; #[ignore]-probed) ───────────────────────────

pub struct PgAdminOpsRepo {
    pool: sqlx::PgPool,
}

impl PgAdminOpsRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        PgAdminOpsRepo { pool }
    }
}

/// REV-S10-2 — the cross-tenant fallback-health read (`fallback.ts:14-24`). `locations` reads fine
/// post-B3 via `public_select USING(true)`; the FORCE-isolated `owner_notification_targets` columns
/// (telegram_active/push_active/dead_channels) go to 0 under NOBYPASSRLS.
// S10 REV-S10-2 SWAPPED (2026-07-05, staging placement done): the ont-joined aggregate now runs
// through `platform_fallback_health()` — a SECURITY DEFINER platform-read (pinned search_path,
// table-owner execution, EXECUTE granted to dowiz_app, inner SQL verbatim the previous constant).
// Under FORCE RLS + NOBYPASSRLS a direct read here silently zeroed every tenant's channel counts
// (the council's false-green). DoD: fallback/health returns fleet counts (not 0) — proven on
// staging (27 rows). The `locations` half needed no change (public_select).
pub const FALLBACK_HEALTH_SQL: &str = "SELECT * FROM platform_fallback_health()";

pub const R2_CHECK_SQL: &str = "SELECT COUNT(*)::bigint AS total_locations,
        COUNT(*) FILTER (WHERE fallback_config->>'phone' IS NOT NULL AND fallback_config->>'phone' != '')::bigint AS with_fallback_phone
   FROM locations";

#[async_trait::async_trait]
impl AdminOpsRepo for PgAdminOpsRepo {
    async fn list_backups(
        &self,
        _type_filter: Option<String>,
        _status_filter: Option<String>,
        limit: i64,
    ) -> Result<Vec<BackupRow>, AdminRepoError> {
        let rows: Vec<(String, String, String, bool)> = sqlx::query_as(
            "SELECT id::text, type, status, checksum_sha256 IS NOT NULL AS has_checksum
               FROM backup_metadata ORDER BY created_at DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|_e| AdminRepoError)?;
        Ok(rows
            .into_iter()
            .map(|(id, r#type, status, has_checksum)| BackupRow {
                id,
                r#type,
                status,
                has_checksum,
                restore_test_result: None,
            })
            .collect())
    }

    async fn fallback_health(&self) -> Result<Vec<FallbackHealthRow>, AdminRepoError> {
        let rows: Vec<(Uuid, String, String, i64, i64, i64)> = sqlx::query_as(FALLBACK_HEALTH_SQL)
            .fetch_all(&self.pool)
            .await
            .map_err(|_e| AdminRepoError)?;
        Ok(rows
            .into_iter()
            .map(
                |(id, name, slug, telegram_active, push_active, dead_channels)| FallbackHealthRow {
                    id,
                    name,
                    slug,
                    telegram_active,
                    push_active,
                    dead_channels,
                },
            )
            .collect())
    }

    async fn r2_check(&self) -> Result<R2Coverage, AdminRepoError> {
        let (total, with_phone): (i64, i64) = sqlx::query_as(R2_CHECK_SQL)
            .fetch_one(&self.pool)
            .await
            .map_err(|_e| AdminRepoError)?;
        let pct = if total > 0 {
            (with_phone * 100) / total
        } else {
            0
        };
        Ok(R2Coverage {
            total_locations: total,
            with_fallback_phone: with_phone,
            coverage_pct: pct,
        })
    }

    async fn notification_audit(
        &self,
        event: String,
        since_minutes: i64,
    ) -> Result<Vec<serde_json::Value>, AdminRepoError> {
        let rows: Vec<(String, String, Option<String>, i64)> = sqlx::query_as(
            "SELECT event, status, channel, count(*)::bigint AS cnt
               FROM notification_outbox_audit
              WHERE event = $1 AND created_at > now() - ($2 || ' minutes')::interval
              GROUP BY event, status, channel ORDER BY cnt DESC LIMIT 20",
        )
        .bind(&event)
        .bind(since_minutes.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|_e| AdminRepoError)?;
        Ok(rows
            .into_iter()
            .map(|(event, status, channel, cnt)| {
                serde_json::json!({ "event": event, "status": status, "channel": channel, "cnt": cnt })
            })
            .collect())
    }

    async fn audit_start(&self, ctx: &AuditCtx) -> Result<String, AdminRepoError> {
        let row: (Uuid,) = sqlx::query_as(
            "INSERT INTO platform_admin_audit_log (actor_id, action, target, status, ip_hash, user_agent_hash)
             VALUES ($1, $2, $3, 'started', $4, $5) RETURNING id",
        )
        .bind(ctx.actor_id)
        .bind(&ctx.action)
        .bind(&ctx.target)
        .bind(&ctx.ip_hash)
        .bind(&ctx.ua_hash)
        .fetch_one(&self.pool)
        .await
        .map_err(|_e| AdminRepoError)?;
        Ok(row.0.to_string())
    }

    async fn audit_finish(&self, id: &str, status: &str) -> Result<(), AdminRepoError> {
        let uuid = Uuid::parse_str(id).map_err(|_e| AdminRepoError)?;
        sqlx::query("UPDATE platform_admin_audit_log SET status = $2 WHERE id = $1")
            .bind(uuid)
            .bind(status)
            .execute(&self.pool)
            .await
            .map_err(|_e| AdminRepoError)?;
        Ok(())
    }

    async fn audit_completed(&self, ctx: &AuditCtx) {
        // Best-effort: a read must not fail on an audit blip.
        let _written = sqlx::query(
            "INSERT INTO platform_admin_audit_log (actor_id, action, target, status, ip_hash, user_agent_hash)
             VALUES ($1, $2, $3, 'completed', $4, $5)",
        )
        .bind(ctx.actor_id)
        .bind(&ctx.action)
        .bind(&ctx.target)
        .bind(&ctx.ip_hash)
        .bind(&ctx.ua_hash)
        .execute(&self.pool)
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use std::sync::Mutex;
    use tower::ServiceExt as _;

    // ── Q-BACKUP-KEY: fail-loud, env-only, never leaks the value ──

    #[test]
    fn backup_key_resolves_primary_by_default() {
        let key = resolve_backup_key("primary", None, Some("base64key==")).unwrap();
        assert_eq!(key, "base64key==");
    }

    #[test]
    fn backup_key_unknown_id_fails_loud_without_leaking_value() {
        let err = resolve_backup_key("rotated-2", None, Some("SECRETKEYVALUE")).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("rotated-2"), "the error names the keyId");
        assert!(
            !msg.contains("SECRETKEYVALUE"),
            "the error must NEVER contain the key VALUE (secrets-incident)"
        );
    }

    #[test]
    fn backup_key_reads_keyring_json() {
        let key = resolve_backup_key("k2", Some(r#"{"k2":"kval"}"#), None).unwrap();
        assert_eq!(key, "kval");
    }

    #[test]
    fn backup_key_bad_keyring_json_fails_loud() {
        assert!(matches!(
            resolve_backup_key("primary", Some("not json"), None),
            Err(BackupKeyError::KeyringNotJson)
        ));
    }

    // ── REV-S10-4: drill actor fail-closed, never 'unknown' ──

    #[test]
    fn resolve_drill_actor_present_returns_user_id() {
        let owner = OwnerClaims::new(Uuid::new_v4(), None);
        assert_eq!(resolve_drill_actor(Some(&owner)).unwrap(), owner.user_id);
    }

    #[test]
    fn resolve_drill_actor_missing_rejects_never_unknown() {
        assert_eq!(resolve_drill_actor(None), Err(DrillError::MissingActor));
    }

    // A drill trigger that records the actor it was invoked with + a call log for ordering.
    #[derive(Default)]
    struct SpyDrill {
        actor_seen: Mutex<Option<Uuid>>,
        log: Arc<Mutex<Vec<String>>>,
        outcome_success: bool,
    }
    #[async_trait::async_trait]
    impl RestoreDrill for SpyDrill {
        async fn trigger(
            &self,
            actor: Uuid,
            _backup_id: Option<Uuid>,
            _full_hash: bool,
        ) -> Result<DrillOutcome, DrillError> {
            *self.actor_seen.lock().unwrap() = Some(actor);
            self.log.lock().unwrap().push("drill".to_string());
            Ok(DrillOutcome {
                success: self.outcome_success,
                target: "sandbox-db".to_string(),
            })
        }
    }

    #[derive(Default)]
    struct SpyRepo {
        log: Arc<Mutex<Vec<String>>>,
    }
    #[async_trait::async_trait]
    impl AdminOpsRepo for SpyRepo {
        async fn list_backups(
            &self,
            _t: Option<String>,
            _s: Option<String>,
            _l: i64,
        ) -> Result<Vec<BackupRow>, AdminRepoError> {
            Ok(vec![])
        }
        async fn fallback_health(&self) -> Result<Vec<FallbackHealthRow>, AdminRepoError> {
            self.log.lock().unwrap().push("fallback_health".to_string());
            Ok(vec![])
        }
        async fn r2_check(&self) -> Result<R2Coverage, AdminRepoError> {
            Ok(R2Coverage {
                total_locations: 0,
                with_fallback_phone: 0,
                coverage_pct: 0,
            })
        }
        async fn notification_audit(
            &self,
            _e: String,
            _m: i64,
        ) -> Result<Vec<serde_json::Value>, AdminRepoError> {
            Ok(vec![])
        }
        async fn audit_start(&self, _ctx: &AuditCtx) -> Result<String, AdminRepoError> {
            self.log.lock().unwrap().push("audit_start".to_string());
            Ok(Uuid::new_v4().to_string())
        }
        async fn audit_finish(&self, _id: &str, status: &str) -> Result<(), AdminRepoError> {
            self.log
                .lock()
                .unwrap()
                .push(format!("audit_finish:{status}"));
            Ok(())
        }
        async fn audit_completed(&self, _ctx: &AuditCtx) {
            self.log.lock().unwrap().push("audit_completed".to_string());
        }
    }

    fn state(
        drills_enabled: bool,
        success: bool,
    ) -> (PlatformAdminState, Arc<Mutex<Vec<String>>>, Arc<SpyDrill>) {
        let log = Arc::new(Mutex::new(Vec::new()));
        let drill = Arc::new(SpyDrill {
            actor_seen: Mutex::new(None),
            log: log.clone(),
            outcome_success: success,
        });
        let repo = Arc::new(SpyRepo { log: log.clone() });
        (
            PlatformAdminState {
                repo,
                drill: drill.clone(),
                drills_enabled,
            },
            log,
            drill,
        )
    }

    #[tokio::test]
    async fn drill_writes_ahead_audit_before_running_and_carries_actor() {
        // Q-ADMIN-AUDIT ordering + REV-S10-4 actor carry: audit_start (committed 'started') BEFORE
        // the drill, then audit_finish; and the drill sees the gate-verified actor.
        let (st, log, drill) = state(true, true);
        let owner = OwnerClaims::new(Uuid::new_v4(), None);
        let resp = run_drill(
            &st,
            Some(owner.clone()),
            &HeaderMap::new(),
            None,
            false,
            "backups.verify",
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            *log.lock().unwrap(),
            vec![
                "audit_start".to_string(),
                "drill".to_string(),
                "audit_finish:completed".to_string()
            ],
            "write-ahead: 'started' row is durable BEFORE the drill runs"
        );
        assert_eq!(
            *drill.actor_seen.lock().unwrap(),
            Some(owner.user_id),
            "REV-S10-4: the drill carries the gate-verified actor across the boundary"
        );
    }

    #[tokio::test]
    async fn drill_missing_actor_rejects_and_never_starts_audit() {
        // REV-S10-4 fail-closed: no actor → reject, NO write-ahead row, NO drill, NEVER 'unknown'.
        let (st, log, drill) = state(true, true);
        let resp = run_drill(&st, None, &HeaderMap::new(), None, false, "backups.verify").await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(
            log.lock().unwrap().is_empty(),
            "a drill with no actor must not audit-start or run"
        );
        assert!(drill.actor_seen.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn drills_kill_switch_blocks_drill_but_reads_are_never_darkened() {
        // ADMIN_DRILLS_ENABLED off → the drill is 403; but fallback/health (a recovery READ) still
        // serves (Q-DRILL-HARDENING: the kill-switch scopes ONLY the two heavy drills).
        let (st, _log, _drill) = state(false, true);
        let resp = run_drill(
            &st,
            Some(OwnerClaims::new(Uuid::new_v4(), None)),
            &HeaderMap::new(),
            None,
            false,
            "backups.verify",
        )
        .await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let read = fallback_health(
            Extension(st),
            Extension(OwnerClaims::new(Uuid::new_v4(), None)),
            HeaderMap::new(),
        )
        .await;
        assert_eq!(
            read.status(),
            StatusCode::OK,
            "recovery reads never darkened"
        );
    }

    #[tokio::test]
    async fn verify_backup_rejects_non_uuid_backup_id_400() {
        let (st, _log, _drill) = state(true, true);
        let resp = verify_backup(
            Extension(st),
            Some(Extension(OwnerClaims::new(Uuid::new_v4(), None))),
            HeaderMap::new(),
            Some(axum::Json(serde_json::json!({ "backupId": "not-a-uuid" }))),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // ── Q-DRILL-RESTORE-RUNBOOK: NO restore-to-prod route exists ──

    #[tokio::test]
    async fn admin_router_exposes_no_restore_to_prod_route() {
        // Coverage assertion (DoD): the S10 admin namespace exposes list/drill/dr-report only — a
        // restore-over-prod endpoint DOES NOT EXIST and is never a side effect of this port.
        let (st, _log, _drill) = state(true, true);
        let app = admin_router(st);
        for path in [
            "/api/admin/backups/restore",
            "/api/admin/restore",
            "/api/admin/backups/restore-to-prod",
        ] {
            let resp = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(path)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                resp.status(),
                StatusCode::NOT_FOUND,
                "{path} must NOT be a registered route (no restore-to-prod, Q-DRILL-RESTORE-RUNBOOK)"
            );
        }
    }

    #[test]
    fn fallback_health_sql_calls_the_platform_read_definer_never_the_table_directly() {
        // REV-S10-2: under FORCE RLS + NOBYPASSRLS a direct owner_notification_targets read
        // silently zeroes every tenant's channel counts (the council's false-green). The read
        // MUST go through the SECURITY DEFINER platform-read (which joins the right table
        // internally — verbatim the previous constant).
        assert!(FALLBACK_HEALTH_SQL.contains("platform_fallback_health()"));
        assert!(
            !FALLBACK_HEALTH_SQL.contains("owner_notification_targets"),
            "a direct table read here would silently zero post-B3 — keep it inside the DEFINER"
        );
    }
}
