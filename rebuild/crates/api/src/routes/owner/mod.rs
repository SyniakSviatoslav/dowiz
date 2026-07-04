//! S3 catalog/admin CRUD — the OWNER-authenticated write surface. Ports
//! `apps/api/src/routes/owner/{products,categories,modifier-groups,menu-availability,themes}.ts`
//! (owner-route census rows 1-89, excluding the 🔴/S4/S5+ rows out of this build's scope — see
//! each submodule's doc for its exact op list and deferrals).
//!
//! ## Auth: reuses S2 verbatim, adds nothing new
//! Every op binds `crate::auth::extractors::OwnerClaimsExt` (S2's role-narrowed extractor — a
//! courier/customer token cannot reach a handler here, structurally: `OwnerClaimsExt` rejects
//! any non-owner claim at 401 before a handler body ever runs) plus one of the two
//! location-resolution helpers below, both P-d live-membership re-reads (ADR-0004) ported from
//! `plugins/auth.ts::requireLocationAccess` / `lib/get-owner-location.ts::getOwnerLocationId`:
//!
//!   - [`require_location_access`] — the **OWNER+LOC** pattern (`:locationId` in the URL): 404
//!     (existence-hiding, NOT 403 — carried verbatim from `requireLocationAccess`'s owner branch,
//!     `plugins/auth.ts:145-150`) when the location isn't one of the caller's LIVE active owner
//!     memberships.
//!   - [`resolve_owner_location`] — the **OWNER-only** pattern (`/api/owner/menu/*` aliases, no
//!     `:locationId` in the URL): resolves the JWT's baked `activeLocationId` re-verified live, or
//!     the first active membership; 401 UNAUTHORIZED (`getOwnerLocationId` returning `null` ->
//!     `products.ts:380` etc.'s `reply.sendError(401, 'UNAUTHORIZED', 'Unauthorized')`) when the
//!     caller holds no active owner membership at all.
//!
//! Both call `AuthState.repo.active_owner_locations(user_id)` — the SAME live P-c membership read
//! `auth::service::owner_refresh_disposition` already uses for the analogous refresh-time check
//! (`auth.ts:293-297`'s `ORDER BY created_at, location_id`, so the "first" pick is deterministic).
//! No new DB query shape, no new auth invented — exactly the S2 extractor + repo this build was
//! briefed to reuse.
//!
//! ## Known inherited deviation from TS (NOT a carry to fix here)
//! TS's `preValidation: [verifyAuth, requireRole(['owner']), requireLocationAccess]` gives a
//! **403** `{error:'Forbidden role'}` for a cryptographically-valid courier/customer token
//! (`plugins/auth.ts:109-115`). The ALREADY-BUILT (S2, council-approved) `OwnerClaimsExt`
//! extractor instead narrows role in one step and returns **401** `{error:'Authentication
//! required'}` for the same case (`auth/extractors.rs:110`, Q-LOGOUT structural narrowing). This
//! build was briefed to reuse that extractor verbatim ("use these, do not invent new auth"), so
//! S3 inherits the 401-not-403 behavior for a valid wrong-role token; it is flagged here rather
//! than silently re-added as a bug, and is a question for the S2 surface, not S3.
//!
//! ## In-transaction membership re-check (S3 breaker finding C1+H4 — 2026-07-04)
//! [`require_location_access`]/[`resolve_owner_location`] above run against `AuthState.repo`
//! (`auth::repo::AuthRepo`), a DIFFERENT connection/pool than the one a submodule's `with_user`
//! transaction later seats `app.user_id` on. Under NOBYPASSRLS that gap matters: a request could
//! pass the out-of-band pre-check and then run its actual catalog write on a transaction whose
//! own RLS-visible membership state (as seen through THAT session's GUC) was never itself
//! verified. The breaker's fix (belt-and-suspenders, safe today and post-flip): every submodule
//! MUST additionally call [`assert_active_owner_membership`] as the FIRST statement inside its
//! OWN `with_user`-seated transaction, before the op's real SQL, and treat a `false` return as a
//! 404 (OWNER+LOC) exactly like `require_location_access`'s out-of-band check would. The
//! extractor-level pre-checks stay in place too — they're a cheap fast-path that avoids opening a
//! transaction for a request that's obviously going to fail — but they are NOT, by themselves,
//! the security boundary; the in-transaction check is.
//!
//! ## Writes: `db::with_user`, never `db::with_tenant`
//! See `crate::db` module doc (Q-GUC-FAMILY, `docs/design/rebuild-catalog-s3-council/proposal.md`
//! §3) — this surface is the first Rust writer of tenant catalog tables, and MUST seat
//! `app.user_id` (`with_user`), not `app.current_tenant` (`with_tenant`, reserved for the
//! courier/service root — never reused here). Every op ALSO carries an explicit
//! `WHERE location_id = $n` (or an ownership-fold-in `INSERT ... SELECT ... WHERE location_id =
//! $n`), belt-and-suspenders that holds independent of RLS enforcement (council packet §3 clause
//! 4) — ported verbatim from each TS query, never relaxed.
//!
//! ## Per-submodule state, not one shared `OwnerState`
//! Each submodule (`products`, `categories`, `modifier_groups`, `menu_availability`, `themes`)
//! defines its OWN narrow repo trait + `Pg*`/`fake::Fake*` pair (mirroring `crate::repo`'s
//! `PublicRepo`/`PgRepo`/`FakeRepo` S1 pattern) and its OWN `*State { auth: AuthState, repo: Arc<dyn
//! *Repo> }`, rather than one god `OwnerState` naming every table this surface touches. This keeps
//! the five verticals independently buildable/testable and mirrors the fact that Node itself
//! registers them as five separate route-plugin files with no shared handle beyond `server.db`
//! (which `with_user`'s single sanctioned pool already stands in for).

use uuid::Uuid;

use crate::auth::AuthState;
use crate::auth::claims::OwnerClaims;
use crate::error::ApiError;

pub mod categories;
pub mod menu_availability;
pub mod modifier_groups;
pub mod product_image;
pub mod product_media;
pub mod products;
pub mod themes;

/// OWNER+LOC (`requireLocationAccess`, `plugins/auth.ts:117-152`, owner branch only — the
/// customer/courier branches in that TS function don't apply here, this surface only ever mounts
/// `OwnerClaimsExt`): 404 (existence-hiding, never 403) unless `location_id` is one of the
/// caller's LIVE active owner memberships. P-d: a live read every call, never a trust of the
/// JWT's baked claim (a removed/downgraded owner holding a valid <=24h token is denied
/// per-request, ADR-0004).
pub async fn require_location_access(
    auth: &AuthState,
    owner: &OwnerClaims,
    location_id: Uuid,
    correlation_id: &str,
) -> Result<(), ApiError> {
    let active = auth
        .repo
        .active_owner_locations(owner.user_id)
        .await
        .map_err(|_err| {
            ApiError::new(
                domain::ErrorCode::Internal,
                "internal_error",
                correlation_id,
            )
        })?;
    if active.contains(&location_id) {
        Ok(())
    } else {
        Err(ApiError::new(
            domain::ErrorCode::NotFound,
            "Not found",
            correlation_id,
        ))
    }
}

/// OWNER-only (`getOwnerLocationId`, `lib/get-owner-location.ts:4-26`): resolves the working
/// location from the JWT's baked `activeLocationId` IFF it is still a live active membership,
/// else the first active membership (deterministic — `active_owner_locations` is `ORDER BY
/// created_at, location_id`, the same tiebreak `auth::service::owner_refresh_disposition` uses).
/// `Err` (401 UNAUTHORIZED) mirrors `getOwnerLocationId` returning `null` -> the route's
/// `reply.sendError(401, 'UNAUTHORIZED', 'Unauthorized')` (`products.ts:380`, `categories.ts:194`,
/// etc.) — carried verbatim across every `/api/owner/menu/*` alias route.
pub async fn resolve_owner_location(
    auth: &AuthState,
    owner: &OwnerClaims,
    correlation_id: &str,
) -> Result<Uuid, ApiError> {
    let active = auth
        .repo
        .active_owner_locations(owner.user_id)
        .await
        .map_err(|_err| {
            ApiError::new(
                domain::ErrorCode::Internal,
                "internal_error",
                correlation_id,
            )
        })?;
    let resolved = match owner.active_location_id {
        Some(requested) if active.contains(&requested) => Some(requested),
        _ => active.first().copied(),
    };
    resolved.ok_or_else(|| {
        ApiError::new(
            domain::ErrorCode::Unauthorized,
            "Unauthorized",
            correlation_id,
        )
    })
}

/// The five per-module states the S3 catalog router mounts. One bundle struct (rather than six
/// positional `Router`-assembly arguments) so `main.rs`'s construction site stays readable and a
/// future sixth submodule is an additive field, not a signature break.
#[derive(Clone)]
pub struct OwnerCatalogStates {
    pub auth: AuthState,
    pub products: products::ProductsState,
    pub categories: categories::CategoriesState,
    pub modifier_groups: modifier_groups::ModifierGroupsState,
    pub menu_availability: menu_availability::MenuAvailabilityState,
    pub themes: themes::ThemesState,
    /// S4 media surface (`docs/design/rebuild-media-s4-council/`) — the product-media ADR-0002
    /// seam plus product-image upload. Bundled into the SAME `OwnerCatalogStates`/
    /// `owner_catalog_router` as S3 (rather than a separate merge site in `main.rs`) because
    /// every S4 owner-authenticated op reuses the identical `OwnerClaimsExt`/
    /// `bearer_and_dev_gate` layer stack — there is no reason for a second, parallel router just
    /// to hold five more routes.
    pub product_media: product_media::ProductMediaState,
    pub product_image: product_image::ProductImageState,
}

/// Assemble the S3 catalog/admin CRUD surface (35 ops) PLUS the S4 media council's
/// owner-authenticated ops (theme logo, product-media presign/confirm/set-primary/reorder/
/// toggle, product-image upload — 7 more) at the SAME paths the Node API serves (each
/// submodule's `#[utoipa::path]` annotations are the per-op SSOT; this function's `.route()`
/// calls mirror them 1:1, grouped per path because axum panics on a duplicate `.route()`
/// registration for the same path). S4's UNAUTHENTICATED ops (entry-photo, the token-proxy-PUT
/// endpoint) are NOT here — see `routes/media_public.rs` and its own mount site in `main.rs`.
///
/// ## Layer stack (mirrors `auth::mount::auth_router`, S2)
/// tower applies layers OUTSIDE-IN (the LAST `.layer` runs FIRST). Order (outer -> inner):
/// request-id mint -> AuthState/module-state extensions -> REV-4 bearer/dev pre-gate -> per-route
/// extractor. The `SetRequestIdLayer` is applied HERE (not inherited) because `Router::merge`
/// does NOT extend `build_router`'s S1 layer stack over merged routes — the S2 auth router has
/// the same property (its handlers read headers directly and never bind `Extension<RequestId>`,
/// so S2 never noticed); every S3 handler binds `Extension<RequestId>`, which would be a runtime
/// 500 ("missing extension") without this layer. Rate-limit parity note: Node's global limiter
/// covers these routes; this dark mount (like the S2 auth mount today) carries none — an explicit
/// launch-wiring item, flagged in the lane report, not silently skipped.
pub fn owner_catalog_router(states: OwnerCatalogStates) -> axum::Router {
    use axum::extract::DefaultBodyLimit;
    use axum::routing::{delete, get, patch, post, put};
    use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};

    let correlation_header = axum::http::HeaderName::from_static("x-correlation-id");

    axum::Router::new()
        // ── products.rs (census rows 1-14) ──
        .route(
            "/api/owner/locations/{locationId}/products",
            post(products::create_product).get(products::list_products),
        )
        .route(
            "/api/owner/locations/{locationId}/products/{id}",
            get(products::get_product)
                .patch(products::update_product)
                .delete(products::delete_product),
        )
        .route(
            "/api/owner/locations/{locationId}/products/{id}/translations/{locale}",
            put(products::put_product_translation).delete(products::delete_product_translation),
        )
        .route(
            "/api/owner/locations/{locationId}/products/{id}/translations",
            get(products::list_product_translations),
        )
        .route(
            "/api/owner/locations/{locationId}/products/{id}/modifier-groups",
            put(products::sync_product_modifier_groups).get(products::list_product_modifier_groups),
        )
        .route(
            "/api/owner/menu/products",
            get(products::list_menu_products).post(products::create_menu_product),
        )
        .route(
            "/api/owner/menu/products/{productId}",
            patch(products::update_menu_product).delete(products::delete_menu_product),
        )
        // ── categories.rs (census rows 15-22) ──
        .route(
            "/api/owner/locations/{locationId}/categories",
            post(categories::create_category).get(categories::list_categories),
        )
        .route(
            "/api/owner/locations/{locationId}/categories/{id}",
            get(categories::get_category)
                .patch(categories::update_category)
                .delete(categories::delete_category),
        )
        .route(
            "/api/owner/menu/categories",
            get(categories::list_categories_alias).post(categories::create_category_alias),
        )
        .route(
            "/api/owner/menu/categories/{id}",
            delete(categories::delete_category_alias),
        )
        // ── modifier_groups.rs (census rows 30-36) ──
        .route(
            "/api/owner/locations/{locationId}/modifier-groups",
            post(modifier_groups::create_modifier_group).get(modifier_groups::list_modifier_groups),
        )
        .route(
            "/api/owner/locations/{locationId}/modifier-groups/{id}",
            patch(modifier_groups::update_modifier_group)
                .delete(modifier_groups::delete_modifier_group),
        )
        .route(
            "/api/owner/locations/{locationId}/modifier-groups/{groupId}/modifiers",
            post(modifier_groups::create_modifier),
        )
        .route(
            "/api/owner/locations/{locationId}/modifiers/{id}",
            patch(modifier_groups::update_modifier).delete(modifier_groups::delete_modifier),
        )
        // ── menu_availability.rs (census rows 80-83) ──
        .route(
            "/api/owner/locations/{locationId}/kitchen-busy",
            patch(menu_availability::set_kitchen_busy),
        )
        .route(
            "/api/owner/locations/{locationId}/menu-schedules",
            get(menu_availability::list_schedules).post(menu_availability::create_schedule),
        )
        .route(
            "/api/owner/locations/{locationId}/menu-schedules/{id}",
            delete(menu_availability::delete_schedule),
        )
        // ── themes.rs (census rows 84-86; row 86 logo upload is the S4 media council op) ──
        .route(
            "/api/owner/locations/{locationId}/theme",
            get(themes::get_owner_theme).put(themes::put_owner_theme),
        )
        .route(
            "/api/owner/locations/{locationId}/theme/logo",
            post(themes::upload_theme_logo)
                .layer(DefaultBodyLimit::max(themes::LOGO_MAX_UPLOAD_BYTES)),
        )
        // ── S4 media council: product_media.rs (ADR-0002 seam) + product_image.rs ──
        .route(
            "/api/owner/menu/products/{productId}/media/presign",
            post(product_media::presign_product_media),
        )
        .route(
            "/api/owner/menu/products/{productId}/media/confirm",
            post(product_media::confirm_product_media),
        )
        .route(
            "/api/owner/menu/products/{productId}/media/{mediaId}/set-primary",
            post(product_media::set_primary_product_media),
        )
        .route(
            "/api/owner/menu/products/{productId}/media/reorder",
            post(product_media::reorder_product_media),
        )
        .route(
            "/api/owner/menu/products/{productId}/media/{mediaId}",
            patch(product_media::set_product_media_available),
        )
        .route(
            "/api/owner/menu/products/{productId}/image",
            post(product_image::upload_product_image)
                .layer(DefaultBodyLimit::max(product_image::MAX_UPLOAD_BYTES)),
        )
        // REV-4 pre-route gate — innermost of the cross-cutting layers, same position as S2.
        .layer(axum::middleware::from_fn(
            crate::auth::middleware::bearer_and_dev_gate,
        ))
        // State extensions (type-keyed; order among these is immaterial).
        .layer(axum::Extension(states.auth))
        .layer(axum::Extension(states.products))
        .layer(axum::Extension(states.categories))
        .layer(axum::Extension(states.modifier_groups))
        .layer(axum::Extension(states.menu_availability))
        .layer(axum::Extension(states.themes))
        .layer(axum::Extension(states.product_media))
        .layer(axum::Extension(states.product_image))
        // Outermost: mint + propagate the correlation id (see fn doc for why this is here).
        .layer(PropagateRequestIdLayer::new(correlation_header.clone()))
        .layer(SetRequestIdLayer::new(correlation_header, MakeRequestUuid))
}

/// The in-transaction membership guard (S3 breaker finding C1+H4). Ports `plugins/auth.ts:148-151`
/// (`requireLocationAccess`'s owner-branch SQL) VERBATIM — same predicate, same column order — but
/// run on the SAME connection/transaction a submodule's `with_user` call already seated
/// `app.user_id` on, not on `AuthState.repo`'s separate pool. `Ok(true)` = the caller is a live
/// active owner member of `location_id`; `Ok(false)` = treat as 404 (existence-hiding, never 403),
/// exactly like [`require_location_access`]. Every S3 submodule calls this as the first statement
/// inside its own `with_user` closure — see that module's doc for why the out-of-band pre-checks
/// above are a fast-path only, not the security boundary.
pub async fn assert_active_owner_membership(
    txn: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    location_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let row: Option<i32> = sqlx::query_scalar(
        "SELECT 1 FROM memberships WHERE location_id = $1 AND user_id = $2 AND role = 'owner' AND status = 'active'",
    )
    .bind(location_id)
    .bind(user_id)
    .fetch_optional(&mut **txn)
    .await?;
    Ok(row.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::repo::fake::FakeAuthRepo;
    use std::sync::Arc;

    fn owner_claims(user_id: Uuid, active_location_id: Option<Uuid>) -> OwnerClaims {
        OwnerClaims::new(user_id, active_location_id)
    }

    #[tokio::test]
    async fn require_location_access_ok_when_location_is_an_active_membership() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = Arc::new(FakeAuthRepo {
            active_owner_locations: std::sync::Mutex::new(
                [(user_id, vec![loc])].into_iter().collect(),
            ),
            ..Default::default()
        });
        let state = AuthState::test_state(repo);
        let owner = owner_claims(user_id, None);
        assert!(
            require_location_access(&state, &owner, loc, "corr-1")
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn require_location_access_404_for_a_foreign_location() {
        // P-d/existence-hiding: a location the caller has no active membership on is 404, not
        // 403 — indistinguishable from a nonexistent location (requireLocationAccess owner
        // branch, plugins/auth.ts:145-150).
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let repo = Arc::new(FakeAuthRepo {
            active_owner_locations: std::sync::Mutex::new(
                [(user_id, vec![mine])].into_iter().collect(),
            ),
            ..Default::default()
        });
        let state = AuthState::test_state(repo);
        let owner = owner_claims(user_id, None);
        let err = require_location_access(&state, &owner, theirs, "corr-1")
            .await
            .unwrap_err();
        assert_eq!(err.envelope.code, domain::ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn require_location_access_404_when_owner_has_no_memberships_at_all() {
        let user_id = Uuid::new_v4();
        let state = AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let owner = owner_claims(user_id, None);
        let err = require_location_access(&state, &owner, Uuid::new_v4(), "corr-1")
            .await
            .unwrap_err();
        assert_eq!(err.envelope.code, domain::ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn resolve_owner_location_prefers_the_baked_claim_when_still_active() {
        let user_id = Uuid::new_v4();
        let baked = Uuid::new_v4();
        let other = Uuid::new_v4();
        let repo = Arc::new(FakeAuthRepo {
            active_owner_locations: std::sync::Mutex::new(
                [(user_id, vec![other, baked])].into_iter().collect(),
            ),
            ..Default::default()
        });
        let state = AuthState::test_state(repo);
        let owner = owner_claims(user_id, Some(baked));
        let resolved = resolve_owner_location(&state, &owner, "corr-1")
            .await
            .unwrap();
        assert_eq!(resolved, baked);
    }

    #[tokio::test]
    async fn resolve_owner_location_falls_back_to_first_active_when_baked_claim_is_stale() {
        // The baked activeLocationId is no longer an active membership (e.g. removed) — falls
        // back to the deterministic first active pick rather than trusting the stale JWT claim.
        let user_id = Uuid::new_v4();
        let stale = Uuid::new_v4();
        let first_active = Uuid::new_v4();
        let repo = Arc::new(FakeAuthRepo {
            active_owner_locations: std::sync::Mutex::new(
                [(user_id, vec![first_active])].into_iter().collect(),
            ),
            ..Default::default()
        });
        let state = AuthState::test_state(repo);
        let owner = owner_claims(user_id, Some(stale));
        let resolved = resolve_owner_location(&state, &owner, "corr-1")
            .await
            .unwrap();
        assert_eq!(resolved, first_active);
    }

    #[tokio::test]
    async fn resolve_owner_location_401_when_no_active_membership_exists() {
        let user_id = Uuid::new_v4();
        let state = AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let owner = owner_claims(user_id, None);
        let err = resolve_owner_location(&state, &owner, "corr-1")
            .await
            .unwrap_err();
        assert_eq!(err.envelope.code, domain::ErrorCode::Unauthorized);
    }

    fn test_states() -> OwnerCatalogStates {
        let auth = AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        OwnerCatalogStates {
            auth: auth.clone(),
            products: products::ProductsState {
                auth: auth.clone(),
                repo: Arc::new(products::fake::FakeProductsRepo::default()),
                app_base_url: "https://dowiz.fly.dev".to_string(),
                r2_public_url: None,
            },
            categories: categories::CategoriesState {
                auth: auth.clone(),
                repo: Arc::new(categories::fake::FakeCategoriesRepo::default()),
            },
            modifier_groups: modifier_groups::ModifierGroupsState {
                auth: auth.clone(),
                repo: Arc::new(modifier_groups::fake::FakeModifierGroupsRepo::default()),
            },
            menu_availability: menu_availability::MenuAvailabilityState {
                auth: auth.clone(),
                repo: Arc::new(menu_availability::fake::FakeMenuAvailabilityRepo::default()),
            },
            themes: themes::ThemesState {
                auth: auth.clone(),
                repo: Arc::new(themes::fake::FakeThemesRepo::default()),
                storage: Arc::new(crate::storage::LocalFsStorage::new(std::env::temp_dir())),
                processor: Arc::new(crate::media::processor::RustImageProcessor),
                app_base_url: "https://dowiz.fly.dev".to_string(),
            },
            product_media: product_media::ProductMediaState {
                auth: auth.clone(),
                repo: Arc::new(product_media::fake::FakeProductMediaRepo::default()),
                storage: Arc::new(crate::storage::LocalFsStorage::new(std::env::temp_dir())),
                token_signer: Some(Arc::new(
                    crate::media::upload_token::UploadTokenSigner::new(vec![3u8; 32]),
                )),
                app_base_url: "https://dowiz.fly.dev".to_string(),
            },
            product_image: product_image::ProductImageState {
                auth,
                repo: Arc::new(product_image::fake::FakeProductImageRepo::default()),
                storage: Arc::new(crate::storage::LocalFsStorage::new(std::env::temp_dir())),
                processor: Arc::new(crate::media::processor::RustImageProcessor),
                app_base_url: "https://dowiz.fly.dev".to_string(),
            },
        }
    }

    /// `Router::route` panics at construction on an invalid path pattern or a duplicate
    /// method-per-path registration — this proves all 35 S3 + 7 S4 owner-authenticated
    /// operations register cleanly (parity with S1's `build_router` / S2's `auth_router`
    /// panic-freedom tests).
    #[test]
    fn owner_catalog_router_builds_without_panicking() {
        let _router = owner_catalog_router(test_states());
    }

    /// Wiring proof: a bearer-less request to an S3 path gets the REV-4 pre-gate's bare
    /// `401 {error:'Unauthorized'}` (Q-BEARERGATE) — i.e. the middleware tower is actually
    /// layered onto these routes, not just present in the source.
    #[tokio::test]
    async fn owner_catalog_router_pre_gates_bearerless_requests_with_bare_401() {
        use tower::ServiceExt;
        let app = owner_catalog_router(test_states());
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/owner/menu/products")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body, serde_json::json!({ "error": "Unauthorized" }));
    }

    /// Wiring proof for the full happy path THROUGH the router (not a direct handler call):
    /// a real minted owner token + an active membership + the request-id layer all compose —
    /// the op reaches the handler and returns its domain response. This is the test that would
    /// have caught a missing `SetRequestIdLayer` (500 "missing extension") on the merged router.
    #[tokio::test]
    async fn owner_catalog_router_serves_an_authenticated_op_end_to_end() {
        use tower::ServiceExt;
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let auth = AuthState::test_state(Arc::new(FakeAuthRepo {
            active_owner_locations: std::sync::Mutex::new(
                [(user_id, vec![loc])].into_iter().collect(),
            ),
            ..Default::default()
        }));
        let mut states = test_states();
        states.auth = auth.clone();
        states.categories.auth = auth.clone();
        let token = auth
            .verifier
            .mint(
                crate::auth::claims::Claims::Owner(OwnerClaims::new(user_id, Some(loc))),
                3600,
            )
            .unwrap();

        let app = owner_catalog_router(states);
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/api/owner/locations/{loc}/categories"))
                    .header("authorization", format!("Bearer {token}"))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::OK,
            "authenticated list-categories through the FULL router (bearer gate + extensions + \
             request-id layer + extractor) must reach the handler"
        );
    }

    /// Requires a live Postgres — same posture as `db.rs`'s `with_user_scopes_and_resets_the_guc`.
    /// Pins the ACTUAL predicate (not just the SQL text) against a real `memberships` table shape.
    #[tokio::test]
    #[ignore = "requires a live Postgres — set DATABASE_URL_OPERATIONAL/DATABASE_URL_SESSION and run with --ignored"]
    async fn assert_active_owner_membership_matches_status_active_owner_rows_only() {
        let config =
            crate::config::Config::from_env().expect("env must be valid to run this ignored test");
        let pools = crate::db::Pools::connect(&config)
            .await
            .expect("pools must connect");
        let user_id = Uuid::new_v4();
        let location_id = Uuid::new_v4();

        let found = crate::db::with_user(&pools.operational, user_id, |txn| {
            Box::pin(async move { assert_active_owner_membership(txn, user_id, location_id).await })
        })
        .await
        .expect("with_user should succeed");
        assert!(
            !found,
            "no membership row exists for this random (user_id, location_id) pair"
        );
    }
}
