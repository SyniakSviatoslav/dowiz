//! S7 courier/dispatch surface — the courier operational plane. Ports
//! `apps/api/src/routes/courier/{shifts,assignments,me,settlements}.ts` +
//! `apps/api/src/routes/owner/{couriers,courier-invites}.ts` + `lib/{dispatch,deliveryCompletion,
//! bindingRelease,shiftService,courierAssignmentService}.ts` per the council RESOLVE
//! `docs/design/rebuild-courier-s7-council/resolution.md` (REV-S7-1..9).
//!
//! ## Auth: reuses S2 verbatim, adds nothing new (REV-S7-7)
//! Every courier op binds [`crate::auth::extractors::CourierSession`] — the S2 extractor that has
//! ALREADY performed the REV-1 live session-liveness bind (present ∧ ¬revoked ∧ ¬expired ∧ courier
//! still holds membership in the token's `activeLocationId`) before a handler body ever runs. A
//! JWT-only-verify path is structurally unreachable: `CourierSession` is the only way to obtain a
//! `CourierClaims` in this module. Every owner-side courier-management op binds
//! [`crate::auth::extractors::OwnerClaimsExt`] (the same S3/S5 extractor), no second auth impl.
//!
//! ## Tenancy: `with_tenant(activeLocationId)` for the courier root (REV-S7-1/S7-6, Q6)
//! A courier is NOT an owner-membership — the `courier_tenant_update`/read policies key on
//! `app.current_tenant`, not `app.user_id`. Every courier read+write in this surface (shifts,
//! assignments, me, settlements) runs inside [`crate::db::with_tenant`] seated on the courier
//! token's `active_location_id` — this is `with_tenant`'s FIRST real caller in this crate (the
//! module was reserved for "the courier/service root" since S1, see that module's doc). This is
//! the FIX for the old bare-pool/no-`BEGIN` reads the council's complete-census (REV-S7-1) found:
//! Rust's `with_tenant` is transactional BY CONSTRUCTION, so every read this module performs
//! through it is fixed-by-port, not by a separate patch. Owner-side courier management
//! (`owner::couriers`, `owner::courier_invites`) seats `app.user_id` via `db::with_user`, mirroring
//! S3 (the OWNER root, not the courier one) — see each submodule's doc for its own seat.
//!
//! ## Per-submodule state, not one shared `CourierState` (mirrors `routes::owner`)
//! Each submodule ([`shifts`], [`assignments`], [`me`], [`settlements`]) defines its OWN narrow
//! repo trait + `Pg*`/`fake::Fake*` pair and its OWN `*State { auth: AuthState, repo: Arc<dyn *Repo>
//! }` — same reasoning as `routes::owner`'s module doc: five independently-buildable/testable
//! verticals, mirroring the fact that Node itself registers them as separate route-plugin files
//! sharing nothing but the pool.
//!
//! ## Money: NOT re-implemented (REV-S7-3, S7-T6)
//! Settlement GENERATION (`app_generate_settlements`, the SECURITY DEFINER catch-up fn migration
//! 085 rewrites) is S5/S8-owned — S7 never calls it and never re-implements the aggregation.
//! [`settlements`] is a pure READ of the shared `courier_payouts`/`settlement_items` rows the owner
//! also reads, role-scoped by `courier_id` with stricter PII redaction (no orderId/customerId).
//! **085 is an UN-APPLIED draft migration** (a cutover DoD, not a build dependency) — this build
//! reads the CURRENT (mig-078) shape of those tables, which is schema-stable across 085 (085 only
//! changes the GENERATION function's watermark logic, not the table shape S7 reads).
//!
//! ## Dispatch: honest, no fake courier (REV-S7-2, S7-T5)
//! [`dispatch::attempt_honest_dispatch`] is the honest-dispatch ENGINE (`lib/dispatch.ts`) — ported
//! standalone and independently tested (synthetic-courier exclusion FOLDED INTO the availability
//! query itself, not left roster-only as the old bug had it). **Not wired into
//! `routes::orders::pg::owner_update_status`** (S5's PATCH still stubs honest-dispatch to
//! `{dispatched:false, reason:"no_courier"}`) — see that module's doc and this build's final report
//! for why that cross-surface wiring is an explicit follow-up, not silently done here.

pub mod assignments;
pub mod dispatch;
pub mod me;
pub mod settlements;
pub mod shifts;

use axum::Router;
use axum::routing::{get, patch, post};

use crate::auth::AuthState;

/// The four per-module states the S7 courier router mounts, mirroring
/// `routes::owner::OwnerCatalogStates`. One bundle struct so `main.rs`'s construction site stays
/// readable and a future submodule is an additive field, not a signature break.
#[derive(Clone)]
pub struct CourierStates {
    pub auth: AuthState,
    pub shifts: shifts::ShiftsState,
    pub assignments: assignments::AssignmentsState,
    pub me: me::MeState,
    pub settlements: settlements::SettlementsState,
}

/// Assemble the S7 courier-side REST surface (`/api/courier/*` — the Node `courierShiftsRoutes` /
/// `courierAssignmentsRoutes` / `courierMeRoutes` / `courierSettlementRoutes` plugins, all
/// registered at `prefix: '/api/courier'`, `bootstrap/routes.ts:127,147-149`). Courier AUTH
/// (`/api/courier/auth/*`) is NOT here — it is S2's `auth_router` (already mounted, reused
/// verbatim, REV-S7-7).
///
/// ## Layer stack (mirrors `routes::owner::owner_catalog_router`)
/// tower applies layers OUTSIDE-IN (the LAST `.layer` runs FIRST). Order (outer -> inner):
/// request-id mint -> AuthState/module-state extensions -> REV-4 bearer/dev pre-gate -> per-route
/// `CourierSession` extractor (the REV-1 live-session bind).
pub fn courier_router(states: CourierStates) -> Router {
    use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};

    let correlation_header = axum::http::HeaderName::from_static("x-correlation-id");

    Router::new()
        // ── shifts.rs (rows 129-133) ──
        .route("/api/courier/me/shift", get(shifts::get_shift))
        .route("/api/courier/me/shift/start", post(shifts::start_shift))
        .route("/api/courier/me/shift/end", post(shifts::end_shift))
        .route(
            "/api/courier/shifts/transition",
            post(shifts::shifts_transition),
        )
        .route("/api/courier/shifts/ping", post(shifts::shifts_ping))
        // ── assignments.rs (rows 114-122) ──
        .route(
            "/api/courier/me/assignments",
            get(assignments::list_assignments),
        )
        .route(
            "/api/courier/assignments/{id}",
            get(assignments::get_assignment),
        )
        .route(
            "/api/courier/assignments/{id}/accept",
            post(assignments::accept_assignment),
        )
        .route(
            "/api/courier/assignments/{id}/reject",
            post(assignments::reject_assignment),
        )
        .route(
            "/api/courier/assignments/{id}/picked-up",
            post(assignments::picked_up_assignment),
        )
        .route(
            "/api/courier/assignments/{id}/delivered",
            post(assignments::delivered_assignment),
        )
        .route(
            "/api/courier/assignments/{id}/cancel",
            post(assignments::cancel_assignment),
        )
        .route(
            "/api/courier/assignments/{id}/abort",
            post(assignments::abort_assignment),
        )
        .route(
            "/api/courier/assignments/{id}/decline",
            post(assignments::decline_assignment),
        )
        // ── me.rs (rows 123-126, 127, 128) ──
        .route("/api/courier/me", get(me::get_profile))
        .route("/api/courier/me/messenger", patch(me::patch_messenger))
        .route("/api/courier/me/audit-log", get(me::get_audit_log))
        .route("/api/courier/me/password", patch(me::patch_password))
        .route("/api/courier/me/earnings", get(me::get_earnings))
        .route("/api/courier/me/history", get(me::get_history))
        // ── settlements.rs (rows 139-140) ──
        .route("/api/courier/me/payouts", get(settlements::get_payouts))
        .route(
            "/api/courier/me/payouts/{id}",
            get(settlements::get_payout_detail),
        )
        // REV-4 pre-route gate — same position as S2/S3.
        .layer(axum::middleware::from_fn(
            crate::auth::middleware::bearer_and_dev_gate,
        ))
        .layer(axum::Extension(states.auth))
        .layer(axum::Extension(states.shifts))
        .layer(axum::Extension(states.assignments))
        .layer(axum::Extension(states.me))
        .layer(axum::Extension(states.settlements))
        .layer(PropagateRequestIdLayer::new(correlation_header.clone()))
        .layer(SetRequestIdLayer::new(correlation_header, MakeRequestUuid))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::repo::fake::FakeAuthRepo;
    use std::sync::Arc;

    fn test_states() -> CourierStates {
        let auth = AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        CourierStates {
            auth: auth.clone(),
            shifts: shifts::ShiftsState {
                auth: auth.clone(),
                repo: Arc::new(shifts::fake::FakeShiftsRepo::default()),
            },
            assignments: assignments::AssignmentsState {
                auth: auth.clone(),
                repo: Arc::new(assignments::fake::FakeAssignmentsRepo::default()),
            },
            me: me::MeState {
                auth: auth.clone(),
                repo: Arc::new(me::fake::FakeMeRepo::default()),
            },
            settlements: settlements::SettlementsState {
                auth,
                repo: Arc::new(settlements::fake::FakeSettlementsRepo::default()),
            },
        }
    }

    /// `Router::route` panics at construction on an invalid path pattern or a duplicate
    /// method-per-path registration — proves all 18 S7 courier-side ops register cleanly
    /// (parity with S2/S3/S5's own panic-freedom tests).
    #[test]
    fn courier_router_builds_without_panicking() {
        let _router = courier_router(test_states());
    }

    /// Wiring proof: a bearer-less request to an S7 courier path gets the REV-4 pre-gate's bare
    /// `401 {error:'Unauthorized'}` — the middleware tower is actually layered, not just present.
    #[tokio::test]
    async fn courier_router_pre_gates_bearerless_requests_with_bare_401() {
        use tower::ServiceExt;
        let app = courier_router(test_states());
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/courier/me")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
    }
}
