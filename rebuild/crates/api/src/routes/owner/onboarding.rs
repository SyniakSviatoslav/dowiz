//! S10 owner onboarding-progress (`apps/api/src/routes/owner/onboarding.ts` — the 4 STATE ops;
//! `/onboarding/start` provisioning is a SEPARATE slice, NOT here). Pure jsonb step-machine over
//! `locations.onboarding_state` (+ `onboarding_completed_at`), same OWNER+LOC `with_user` seam as
//! `dwell_settings`/`fallback_settings`. No money / PII / enum — a clean config-state port.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use axum::Json;
use axum::extract::{Extension, Path};
use axum::response::IntoResponse;

use domain::ErrorCode;

use crate::auth::AuthState;
use crate::auth::extractors::OwnerClaimsExt;
use crate::error::ApiError;
use crate::repo::RepoError;
use crate::routes::correlation_id_string;
use tower_http::request_id::RequestId;

use super::{assert_active_owner_membership, require_location_access};

const STEP_COUNT: i64 = 8;
/// Steps 4 (Branding), 5 (Delivery), 7 (Telegram) may be skipped (`onboarding.ts:17`).
const SKIPPABLE: [i64; 3] = [4, 5, 7];

// ── The jsonb state (stored + wired camelCase, matching Node's `JSON.stringify(state)`) ──

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct OnboardingState {
    pub v: i64,
    pub step: i64,
    #[serde(rename = "completedSteps")]
    pub completed_steps: Vec<i64>,
    #[serde(rename = "skippedSteps")]
    pub skipped_steps: Vec<i64>,
    pub data: serde_json::Value,
}

impl Default for OnboardingState {
    /// `parseState`'s default (`onboarding.ts:350`): `{v:1, step:1, [], [], {}}`.
    fn default() -> Self {
        Self {
            v: 1,
            step: 1,
            completed_steps: vec![],
            skipped_steps: vec![],
            data: serde_json::json!({}),
        }
    }
}

/// `parseState` (`onboarding.ts:337-351`) — fail-soft: any malformed/absent state → the default.
/// Node keys the parse on `state.v` being truthy; mirror that (a shape without `v` → default).
pub fn parse_state(raw: Option<&serde_json::Value>) -> OnboardingState {
    let Some(v) = raw else {
        return OnboardingState::default();
    };
    // Node also accepts a JSON string (typeof raw === 'string'); jsonb from sqlx is already a Value.
    let obj = match v {
        serde_json::Value::String(s) => serde_json::from_str::<serde_json::Value>(s).ok(),
        serde_json::Value::Object(_) => Some(v.clone()),
        _ => None,
    };
    match obj {
        Some(o) if o.get("v").is_some_and(|x| !x.is_null() && x != false) => OnboardingState {
            v: o.get("v").and_then(serde_json::Value::as_i64).unwrap_or(1),
            step: o
                .get("step")
                .and_then(serde_json::Value::as_i64)
                .filter(|&s| s != 0)
                .unwrap_or(1),
            completed_steps: int_array(o.get("completedSteps")),
            skipped_steps: int_array(o.get("skippedSteps")),
            data: o
                .get("data")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
        },
        _ => OnboardingState::default(),
    }
}

fn int_array(v: Option<&serde_json::Value>) -> Vec<i64> {
    v.and_then(serde_json::Value::as_array)
        .map(|a| a.iter().filter_map(serde_json::Value::as_i64).collect())
        .unwrap_or_default()
}

/// The advance-to-next-incomplete loop shared by complete + skip (`onboarding.ts:210-214,282-285`).
/// Returns the next step, or `None` when all steps are done.
fn advance(state: &OnboardingState, from: i64) -> Option<i64> {
    let mut next = from + 1;
    while next <= STEP_COUNT && state.completed_steps.contains(&next) {
        next += 1;
    }
    (next <= STEP_COUNT).then_some(next)
}

/// Pure step-complete transition (`onboarding.ts:195-235`). `Err` = the 400 "not current" case
/// (returns the actual current step for the message). `Ok((state, completed, next))`.
#[allow(clippy::type_complexity)]
pub fn complete_step(
    mut state: OnboardingState,
    step: i64,
) -> Result<(OnboardingState, bool, Option<i64>), i64> {
    if state.completed_steps.contains(&step) {
        // idempotent — already completed
    } else if state.step != step {
        return Err(state.step);
    } else {
        state.completed_steps.push(step);
    }
    state.skipped_steps.retain(|&s| s != step);
    match advance(&state, step) {
        Some(next) => {
            state.step = next;
            Ok((state, false, Some(next)))
        }
        None => Ok((state, true, None)),
    }
}

/// Pure step-skip transition (`onboarding.ts:262-296`). `Err(SkipError)` covers the two 400s.
#[derive(Debug, PartialEq)]
pub enum SkipError {
    NotSkippable,
    AlreadyCompleted,
}
#[allow(clippy::type_complexity)]
pub fn skip_step(
    mut state: OnboardingState,
    step_num: i64,
) -> Result<(OnboardingState, bool, Option<i64>), SkipError> {
    if !SKIPPABLE.contains(&step_num) {
        return Err(SkipError::NotSkippable);
    }
    if state.completed_steps.contains(&step_num) {
        return Err(SkipError::AlreadyCompleted);
    }
    state.skipped_steps.push(step_num);
    state.completed_steps.push(step_num);
    match advance(&state, step_num) {
        Some(next) => {
            state.step = next;
            Ok((state, false, Some(next)))
        }
        None => Ok((state, true, None)),
    }
}

// ── State + repo ──────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct OnboardingStateSvc {
    pub auth: AuthState,
    pub repo: std::sync::Arc<dyn OnboardingRepo>,
}

/// A location row for onboarding reads. `None` outer = not an active-owner membership (404).
pub struct OnboardingRow {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub onboarding_state: Option<serde_json::Value>,
    pub completed_at: Option<String>,
}

#[async_trait::async_trait]
pub trait OnboardingRepo: Send + Sync {
    async fn get(&self, owner: Uuid, location: Uuid) -> Result<Option<OnboardingRow>, RepoError>;
    /// Read the current state, apply `f`, persist (state + optional completed-stamp) in one tx.
    /// `None` = 404. The persisted `completed` also sets `onboarding_completed_at = now()`.
    async fn persist(
        &self,
        owner: Uuid,
        location: Uuid,
        state: &OnboardingState,
        completed: bool,
    ) -> Result<Option<()>, RepoError>;
    async fn read_state(
        &self,
        owner: Uuid,
        location: Uuid,
    ) -> Result<Option<OnboardingState>, RepoError>;
    async fn completed_at(
        &self,
        owner: Uuid,
        location: Uuid,
    ) -> Result<Option<(String, Option<String>)>, RepoError>;
}

// ── Response DTOs ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct StateResponse {
    #[serde(rename = "locationId")]
    pub location_id: Uuid,
    pub slug: String,
    pub name: String,
    #[serde(rename = "onboardingState")]
    pub onboarding_state: OnboardingState,
    #[serde(rename = "currentStep")]
    pub current_step: i64,
    pub completed: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StepResponse {
    pub completed: bool,
    #[serde(rename = "currentStep")]
    pub current_step: Option<i64>,
    #[serde(rename = "onboardingState")]
    pub onboarding_state: OnboardingState,
    #[serde(rename = "skipNote", skip_serializing_if = "Option::is_none")]
    pub skip_note: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CompleteStepBody {
    pub step: i64,
}

// ── Handlers ──────────────────────────────────────────────────────────────────────────────────

/// `GET /api/owner/onboarding/{locationId}/state` (`onboarding.ts:144`).
#[utoipa::path(get, path = "/api/owner/onboarding/{locationId}/state", tag = "owner-onboarding",
    params(("locationId" = Uuid, Path)),
    responses((status = 200, body = StateResponse), (status = 404, body = domain::ErrorEnvelope)))]
pub async fn get_state(
    Extension(svc): Extension<OnboardingStateSvc>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let cid = correlation_id_string(&request_id);
    require_location_access(&svc.auth, &owner, location_id, &cid).await?;
    let row = svc
        .repo
        .get(owner.user_id, location_id)
        .await
        .map_err(|_e| internal(cid.clone()))?
        .ok_or_else(|| not_found(cid))?;
    let state = parse_state(row.onboarding_state.as_ref());
    Ok(Json(StateResponse {
        location_id: row.id,
        slug: row.slug,
        name: row.name,
        current_step: state.step,
        completed: row.completed_at.is_some(),
        onboarding_state: state,
    }))
}

/// `POST /api/owner/onboarding/{locationId}/step/complete` (`onboarding.ts:174`).
#[utoipa::path(post, path = "/api/owner/onboarding/{locationId}/step/complete", tag = "owner-onboarding",
    params(("locationId" = Uuid, Path)), request_body = CompleteStepBody,
    responses((status = 200, body = StepResponse), (status = 400, body = domain::ErrorEnvelope), (status = 404, body = domain::ErrorEnvelope)))]
pub async fn complete_step_handler(
    Extension(svc): Extension<OnboardingStateSvc>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<CompleteStepBody>,
) -> Result<impl IntoResponse, ApiError> {
    let cid = correlation_id_string(&request_id);
    require_location_access(&svc.auth, &owner, location_id, &cid).await?;
    if !(1..=STEP_COUNT).contains(&body.step) {
        return Err(ApiError::validation_failed_400("step out of range", cid));
    }
    let state = svc
        .repo
        .read_state(owner.user_id, location_id)
        .await
        .map_err(|_e| internal(cid.clone()))?
        .ok_or_else(|| not_found(cid.clone()))?;
    let (new_state, completed, next) = complete_step(state, body.step).map_err(|current| {
        ApiError::validation_failed_400(
            format!(
                "Step {} is not current. Current step is {current}",
                body.step
            ),
            cid.clone(),
        )
    })?;
    svc.repo
        .persist(owner.user_id, location_id, &new_state, completed)
        .await
        .map_err(|_e| internal(cid.clone()))?
        .ok_or_else(|| not_found(cid))?;
    Ok(Json(StepResponse {
        completed,
        current_step: next,
        onboarding_state: new_state,
        skip_note: None,
    }))
}

/// `POST /api/owner/onboarding/{locationId}/step/{stepNum}/skip` (`onboarding.ts:247`).
#[utoipa::path(post, path = "/api/owner/onboarding/{locationId}/step/{stepNum}/skip", tag = "owner-onboarding",
    params(("locationId" = Uuid, Path), ("stepNum" = i64, Path)),
    responses((status = 200, body = StepResponse), (status = 400, body = domain::ErrorEnvelope), (status = 404, body = domain::ErrorEnvelope)))]
pub async fn skip_step_handler(
    Extension(svc): Extension<OnboardingStateSvc>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, step_num)): Path<(Uuid, i64)>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let cid = correlation_id_string(&request_id);
    require_location_access(&svc.auth, &owner, location_id, &cid).await?;
    let state = svc
        .repo
        .read_state(owner.user_id, location_id)
        .await
        .map_err(|_e| internal(cid.clone()))?
        .ok_or_else(|| not_found(cid.clone()))?;
    let (new_state, completed, next) = skip_step(state, step_num).map_err(|e| match e {
        SkipError::NotSkippable => ApiError::validation_failed_400(
            format!("Step {step_num} cannot be skipped"),
            cid.clone(),
        ),
        // Node `sendError(400, 'STEP_ALREADY_COMPLETED')` — 400, not the ValidationFailed 422 default.
        SkipError::AlreadyCompleted => ApiError::validation_failed_400(
            format!("Step {step_num} already completed"),
            cid.clone(),
        ),
    })?;
    svc.repo
        .persist(owner.user_id, location_id, &new_state, completed)
        .await
        .map_err(|_e| internal(cid.clone()))?
        .ok_or_else(|| not_found(cid))?;
    let skip_note = match step_num {
        4 => Some("Branding skipped — default theme applied".to_string()),
        5 => Some("Delivery skipped — pickup-only mode, no delivery radius".to_string()),
        7 => Some(
            "Telegram skipped — you'll still receive alerts on the dashboard and via push"
                .to_string(),
        ),
        _ => None,
    };
    Ok(Json(StepResponse {
        completed,
        current_step: next,
        onboarding_state: new_state,
        skip_note,
    }))
}

/// `GET /api/owner/onboarding/{locationId}/complete` (`onboarding.ts:315`).
#[utoipa::path(get, path = "/api/owner/onboarding/{locationId}/complete", tag = "owner-onboarding",
    params(("locationId" = Uuid, Path)),
    responses((status = 200), (status = 400, body = domain::ErrorEnvelope), (status = 404, body = domain::ErrorEnvelope)))]
pub async fn get_complete(
    Extension(svc): Extension<OnboardingStateSvc>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let cid = correlation_id_string(&request_id);
    require_location_access(&svc.auth, &owner, location_id, &cid).await?;
    let (slug, completed_at) = svc
        .repo
        .completed_at(owner.user_id, location_id)
        .await
        .map_err(|_e| internal(cid.clone()))?
        .ok_or_else(|| not_found(cid.clone()))?;
    if completed_at.is_none() {
        // Node `sendError(400, 'ONBOARDING_INCOMPLETE')` — 400, not the ValidationFailed 422 default.
        return Err(ApiError::validation_failed_400(
            "Onboarding not yet completed",
            cid,
        ));
    }
    Ok(Json(
        serde_json::json!({ "slug": slug, "dashboardUrl": "/admin/dashboard.html" }),
    ))
}

fn internal(cid: String) -> ApiError {
    ApiError::new(ErrorCode::Internal, "internal_error", cid)
}
fn not_found(cid: String) -> ApiError {
    ApiError::new(ErrorCode::NotFound, "Not found", cid)
}

// ── PgOnboardingRepo ──────────────────────────────────────────────────────────────────────────

pub struct PgOnboardingRepo {
    pool: sqlx::PgPool,
}
impl PgOnboardingRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl OnboardingRepo for PgOnboardingRepo {
    async fn get(&self, owner: Uuid, location: Uuid) -> Result<Option<OnboardingRow>, RepoError> {
        crate::db::with_user(&self.pool, owner, move |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner, location).await? {
                    return Ok(None);
                }
                let row: Option<(
                    Uuid,
                    String,
                    String,
                    Option<serde_json::Value>,
                    Option<String>,
                )> = sqlx::query_as(
                    "SELECT id, slug, name, onboarding_state, onboarding_completed_at::text \
                         FROM locations WHERE id = $1",
                )
                .bind(location)
                .fetch_optional(&mut **txn)
                .await?;
                Ok(row.map(|(id, slug, name, st, ca)| OnboardingRow {
                    id,
                    slug,
                    name,
                    onboarding_state: st,
                    completed_at: ca,
                }))
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn read_state(
        &self,
        owner: Uuid,
        location: Uuid,
    ) -> Result<Option<OnboardingState>, RepoError> {
        Ok(self
            .get(owner, location)
            .await?
            .map(|r| parse_state(r.onboarding_state.as_ref())))
    }

    async fn persist(
        &self,
        owner: Uuid,
        location: Uuid,
        state: &OnboardingState,
        completed: bool,
    ) -> Result<Option<()>, RepoError> {
        let json = serde_json::to_value(state).unwrap_or_else(|_| serde_json::json!({}));
        crate::db::with_user(&self.pool, owner, move |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner, location).await? {
                    return Ok(None);
                }
                let sql = if completed {
                    "UPDATE locations SET onboarding_state = $1::jsonb, onboarding_completed_at = now() WHERE id = $2 RETURNING id"
                } else {
                    "UPDATE locations SET onboarding_state = $1::jsonb WHERE id = $2 RETURNING id"
                };
                let row: Option<(Uuid,)> = sqlx::query_as(sql)
                    .bind(json)
                    .bind(location)
                    .fetch_optional(&mut **txn)
                    .await?;
                Ok(row.map(|_| ()))
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn completed_at(
        &self,
        owner: Uuid,
        location: Uuid,
    ) -> Result<Option<(String, Option<String>)>, RepoError> {
        Ok(self
            .get(owner, location)
            .await?
            .map(|r| (r.slug, r.completed_at)))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn s(step: i64, done: &[i64]) -> OnboardingState {
        OnboardingState {
            v: 1,
            step,
            completed_steps: done.to_vec(),
            skipped_steps: vec![],
            data: serde_json::json!({}),
        }
    }

    #[test]
    fn parse_state_defaults_when_absent_or_versionless() {
        assert_eq!(parse_state(None), OnboardingState::default());
        assert_eq!(
            parse_state(Some(&serde_json::json!({"step": 3}))),
            OnboardingState::default()
        ); // no v
        let st = parse_state(Some(
            &serde_json::json!({"v":1,"step":3,"completedSteps":[1,2]}),
        ));
        assert_eq!(st.step, 3);
        assert_eq!(st.completed_steps, vec![1, 2]);
    }

    #[test]
    fn complete_step_advances_and_is_idempotent() {
        let (st, done, next) = complete_step(s(1, &[]), 1).unwrap();
        assert!(!done);
        assert_eq!(next, Some(2));
        assert_eq!(st.completed_steps, vec![1]);
        // idempotent re-complete of an already-done step advances, does not double-push
        let (st2, _, _) = complete_step(s(2, &[1]), 1).unwrap();
        assert_eq!(st2.completed_steps, vec![1]);
    }

    #[test]
    fn complete_step_rejects_non_current() {
        assert_eq!(complete_step(s(1, &[]), 3), Err(1));
    }

    #[test]
    fn complete_last_step_marks_completed() {
        let (_, done, next) = complete_step(s(8, &[1, 2, 3, 4, 5, 6, 7]), 8).unwrap();
        assert!(done);
        assert_eq!(next, None);
    }

    #[test]
    fn skip_only_skippable_and_advances() {
        assert!(matches!(
            skip_step(s(1, &[]), 1),
            Err(SkipError::NotSkippable)
        ));
        let (st, done, next) = skip_step(s(4, &[1, 2, 3]), 4).unwrap();
        assert!(!done);
        assert_eq!(next, Some(5));
        assert!(st.skipped_steps.contains(&4) && st.completed_steps.contains(&4));
        assert!(matches!(
            skip_step(s(5, &[4]), 4),
            Err(SkipError::AlreadyCompleted)
        ));
    }
}

#[cfg(test)]
pub mod fake {
    use super::{OnboardingRepo, OnboardingRow, OnboardingState, RepoError};
    use std::sync::Mutex;
    use uuid::Uuid;

    #[derive(Default)]
    pub struct FakeOnboardingRepo {
        pub row: Mutex<Option<(Uuid, serde_json::Value, Option<String>)>>,
    }
    #[async_trait::async_trait]
    impl OnboardingRepo for FakeOnboardingRepo {
        async fn get(&self, _o: Uuid, _l: Uuid) -> Result<Option<OnboardingRow>, RepoError> {
            Ok(self
                .row
                .lock()
                .unwrap()
                .clone()
                .map(|(id, st, ca)| OnboardingRow {
                    id,
                    slug: "demo".into(),
                    name: "Demo".into(),
                    onboarding_state: Some(st),
                    completed_at: ca,
                }))
        }
        async fn read_state(&self, o: Uuid, l: Uuid) -> Result<Option<OnboardingState>, RepoError> {
            Ok(self
                .get(o, l)
                .await?
                .map(|r| super::parse_state(r.onboarding_state.as_ref())))
        }
        async fn persist(
            &self,
            _o: Uuid,
            _l: Uuid,
            _s: &OnboardingState,
            _c: bool,
        ) -> Result<Option<()>, RepoError> {
            Ok(self.row.lock().unwrap().is_some().then_some(()))
        }
        async fn completed_at(
            &self,
            o: Uuid,
            l: Uuid,
        ) -> Result<Option<(String, Option<String>)>, RepoError> {
            Ok(self.get(o, l).await?.map(|r| (r.slug, r.completed_at)))
        }
    }
}
