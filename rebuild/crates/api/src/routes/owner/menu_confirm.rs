//! R2a: `POST /api/owner/locations/:locationId/products/:productId/confirm-allergens`
//! (`apps/api/src/routes/owner/menu-confirm.ts`) — the P6 CLAIM-phase allergen confirmation.
//!
//! ## ADR-0014 single-source rule (the reason this file is tiny and must stay tiny)
//! Confirmation flips `allergens_confirmed = true` and NOTHING ELSE — it never writes allergen
//! VALUES and never mutates `source` (that column preserves the 'place' provenance/liability
//! audit, and the C2 read-gate keys on it). The owner AUTHORS allergens through the normal
//! product editor; this endpoint only attests them. [`CONFIRM_SQL`] is pinned by a test below so
//! a future edit that sneaks a second SET target turns the build red.
//!
//! ## Wire quirks carried verbatim
//! - 404 body is the RAW `{"error": "PRODUCT_NOT_FOUND"}` (`menu-confirm.ts:24` uses
//!   `reply.code(404).send(...)`, NOT the sendError envelope) — reproduced exactly, not
//!   "upgraded" to the ADR-0010 envelope.
//! - 200 body is `{"confirmed": true}`.
//! - Node has NO param schema (a non-uuid id would 500 at the Postgres cast); axum's
//!   `Path<(Uuid, Uuid)>` 400s instead — the same accepted garbage-input divergence every other
//!   ported owner route carries (products/categories precedent).

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use axum::Json;
use axum::extract::{Extension, Path};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::auth::AuthState;
use crate::auth::extractors::OwnerClaimsExt;
use crate::error::ApiError;
use crate::repo::RepoError;
use crate::routes::correlation_id_string;
use tower_http::request_id::RequestId;

use super::{assert_active_owner_membership, require_location_access};

#[derive(Clone)]
pub struct MenuConfirmState {
    pub auth: AuthState,
    pub repo: std::sync::Arc<dyn MenuConfirmRepo>,
}

#[async_trait::async_trait]
pub trait MenuConfirmRepo: Send + Sync {
    /// `Ok(true)` = the product row matched and was flag-confirmed; `Ok(false)` = no row
    /// (unknown product / foreign product / membership vanished mid-flight) — the handler's
    /// raw 404.
    async fn confirm(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
    ) -> Result<bool, RepoError>;
}

/// `{confirmed: true}` (`menu-confirm.ts:25`).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConfirmedResponse {
    pub confirmed: bool,
}

/// The ONE statement this surface is allowed (ADR-0014): flag-confirm, never value-rewrite.
/// Pinned verbatim from `menu-confirm.ts:20`.
const CONFIRM_SQL: &str = "UPDATE products SET allergens_confirmed = true WHERE id = $1 AND location_id = $2 RETURNING id";

/// `POST /api/owner/locations/{locationId}/products/{productId}/confirm-allergens`
/// (`menu-confirm.ts:10-27`) -> 200 `{confirmed: true}`, or the RAW 404
/// `{"error": "PRODUCT_NOT_FOUND"}`.
#[utoipa::path(
    post,
    path = "/api/owner/locations/{locationId}/products/{productId}/confirm-allergens",
    params(("locationId" = Uuid, Path), ("productId" = Uuid, Path)),
    responses(
        (status = 200, description = "Allergens flag-confirmed", body = ConfirmedResponse),
        (status = 404, description = "Raw `{\"error\": \"PRODUCT_NOT_FOUND\"}` — not the error envelope (Node parity)"),
    ),
    tag = "owner-menu-confirm"
)]
pub async fn confirm_allergens(
    Extension(state): Extension<MenuConfirmState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, product_id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
) -> Result<axum::response::Response, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let confirmed = state
        .repo
        .confirm(owner.user_id, location_id, product_id)
        .await
        .map_err(|_err| {
            ApiError::new(
                domain::ErrorCode::Internal,
                "internal_error",
                correlation_id,
            )
        })?;

    if confirmed {
        Ok(Json(ConfirmedResponse { confirmed: true }).into_response())
    } else {
        // Raw body, verbatim — see module doc.
        Ok((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "PRODUCT_NOT_FOUND" })),
        )
            .into_response())
    }
}

// ── PgMenuConfirmRepo ────────────────────────────────────────────────────────────────────────

pub struct PgMenuConfirmRepo {
    pool: sqlx::PgPool,
}
impl PgMenuConfirmRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl MenuConfirmRepo for PgMenuConfirmRepo {
    async fn confirm(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
    ) -> Result<bool, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                // First-in-tx membership assert (session lesson / S3 C1+H4) — a `false` here is
                // the same wire outcome as a missing product (Node's FORCE-RLS UPDATE would
                // match 0 rows too).
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(false);
                }
                let row: Option<(Uuid,)> = sqlx::query_as(CONFIRM_SQL)
                    .bind(product_id)
                    .bind(location_id)
                    .fetch_optional(&mut **txn)
                    .await?;
                Ok(row.is_some())
            })
        })
        .await
        .map_err(map_txn_err)
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

// ── Fake + tests ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
pub mod fake {
    use super::{MenuConfirmRepo, RepoError};
    use std::collections::HashSet;
    use std::sync::Mutex;
    use uuid::Uuid;

    #[derive(Default)]
    pub struct FakeMenuConfirmRepo {
        /// (location_id, product_id) pairs that exist; confirm() flags and returns true.
        pub products: Mutex<HashSet<(Uuid, Uuid)>>,
        pub confirmed: Mutex<HashSet<(Uuid, Uuid)>>,
    }

    #[async_trait::async_trait]
    impl MenuConfirmRepo for FakeMenuConfirmRepo {
        async fn confirm(
            &self,
            _owner: Uuid,
            location_id: Uuid,
            product_id: Uuid,
        ) -> Result<bool, RepoError> {
            if self
                .products
                .lock()
                .unwrap()
                .contains(&(location_id, product_id))
            {
                self.confirmed
                    .lock()
                    .unwrap()
                    .insert((location_id, product_id));
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::FakeMenuConfirmRepo;
    use super::*;
    use crate::auth::claims::OwnerClaims;
    use crate::auth::repo::fake::FakeAuthRepo;
    use std::sync::{Arc, Mutex};

    fn request_id() -> RequestId {
        RequestId::new(axum::http::HeaderValue::from_static("corr-1"))
    }

    fn owner_with_location(user_id: Uuid, loc: Uuid) -> AuthState {
        AuthState::test_state(Arc::new(FakeAuthRepo {
            active_owner_locations: Mutex::new([(user_id, vec![loc])].into_iter().collect()),
            ..Default::default()
        }))
    }

    /// ADR-0014 guardrail: the ONE allowed statement — flag-confirm only, no allergen-value or
    /// `source` writes, ever. If someone widens the SET list this pin goes red.
    #[test]
    fn confirm_sql_flips_only_the_confirmed_flag_adr_0014() {
        assert_eq!(
            CONFIRM_SQL,
            "UPDATE products SET allergens_confirmed = true WHERE id = $1 AND location_id = $2 RETURNING id"
        );
        assert!(
            !CONFIRM_SQL.contains("allergens ")
                && !CONFIRM_SQL.contains("allergens,")
                && !CONFIRM_SQL.contains("allergens ="),
            "must never rewrite allergen VALUES"
        );
        assert!(
            !CONFIRM_SQL.contains("source"),
            "must never mutate provenance"
        );
    }

    #[tokio::test]
    async fn confirm_allergens_200_flags_the_product() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let product = Uuid::new_v4();
        let repo = Arc::new(FakeMenuConfirmRepo::default());
        repo.products.lock().unwrap().insert((loc, product));
        let state = MenuConfirmState {
            auth: owner_with_location(user_id, loc),
            repo: repo.clone(),
        };

        let response = confirm_allergens(
            Extension(state),
            OwnerClaimsExt(OwnerClaims::new(user_id, Some(loc))),
            Path((loc, product)),
            Extension(request_id()),
        )
        .await
        .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body, serde_json::json!({ "confirmed": true }));
        assert!(repo.confirmed.lock().unwrap().contains(&(loc, product)));
    }

    /// The RAW 404 body — `{"error": "PRODUCT_NOT_FOUND"}`, no envelope fields (Node parity,
    /// `menu-confirm.ts:24`).
    #[tokio::test]
    async fn confirm_allergens_unknown_product_is_the_raw_404_body() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = MenuConfirmState {
            auth: owner_with_location(user_id, loc),
            repo: Arc::new(FakeMenuConfirmRepo::default()),
        };

        let response = confirm_allergens(
            Extension(state),
            OwnerClaimsExt(OwnerClaims::new(user_id, Some(loc))),
            Path((loc, Uuid::new_v4())),
            Extension(request_id()),
        )
        .await
        .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            body,
            serde_json::json!({ "error": "PRODUCT_NOT_FOUND" }),
            "raw body, NOT the ADR-0010 envelope"
        );
    }

    /// Foreign location -> the envelope 404 from `require_location_access` (Node's
    /// `requireLocationAccess` preValidation), BEFORE any repo call.
    #[tokio::test]
    async fn confirm_allergens_foreign_location_is_the_envelope_404() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = MenuConfirmState {
            auth: owner_with_location(user_id, mine),
            repo: Arc::new(FakeMenuConfirmRepo::default()),
        };

        let err = crate::error::expect_err(
            confirm_allergens(
                Extension(state),
                OwnerClaimsExt(OwnerClaims::new(user_id, Some(mine))),
                Path((theirs, Uuid::new_v4())),
                Extension(request_id()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, domain::ErrorCode::NotFound);
    }

    /// Requires a live Postgres — proves the pinned UPDATE binds/parses against the real schema
    /// (#77 class); random ids match no row (membership assert denies first), so nothing is
    /// written.
    #[tokio::test]
    #[ignore = "requires a live Postgres — set DATABASE_URL_OPERATIONAL/DATABASE_URL_SESSION and run with --ignored"]
    async fn live_pg_confirm_denies_random_ids_without_writing() {
        let config =
            crate::config::Config::from_env().expect("env must be valid to run this ignored test");
        let pools = crate::db::Pools::connect(&config)
            .await
            .expect("pools must connect");
        let repo = PgMenuConfirmRepo::new(pools.operational.clone());
        let confirmed = repo
            .confirm(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4())
            .await
            .expect("statement must bind and parse against the live schema");
        assert!(!confirmed, "random ids can never flag-confirm anything");

        // The membership assert denies before the UPDATE runs above, so ALSO prove the pinned
        // UPDATE itself parses/binds: random ids match no row -> nothing written, decode proven.
        let row: Option<(Uuid,)> = sqlx::query_as(CONFIRM_SQL)
            .bind(Uuid::new_v4())
            .bind(Uuid::new_v4())
            .fetch_optional(&pools.operational)
            .await
            .expect("the pinned CONFIRM_SQL must parse and bind against the live schema");
        assert!(row.is_none());
    }
}
