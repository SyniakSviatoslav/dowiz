//! S2 auth surface (`openapi-s2-auth.yaml`, 20 operations, 19 built + 1 RETIRED). Ports
//! `packages/platform/src/auth/jwt.ts`, `apps/api/src/routes/auth*`, `courier/auth.ts`,
//! `customer/track.ts`, `public/claim.ts`, `dev/mock-auth.ts`, and `plugins/auth.ts` — carrying
//! every behavior VERBATIM except the council FIXes (RETIRE courier-activate, REV-1 courier
//! has_location, REV-3 customer-scope) folded in on the record (docs/design/rebuild-auth-s2-council).
//!
//! Module map:
//!   - `claims`     — the strict discriminated-union JWT claims (legacy.ts port).
//!   - `jwt`        — RS256 double-pin sign/verify + kid select + body-kid round-trip.
//!   - `config`     — the JWT/dev-auth env slice + boot-guard D.
//!   - `crypto`     — sha256-hex / opaque-token / argon2 / timing-safe helpers.
//!   - `service`    — pure disposition logic (owner refresh, courier bind, customer scope).
//!   - `repo`       — the AuthRepo trait + PgAuthRepo (runtime sqlx) + FakeAuthRepo.
//!   - `dto`        — wire request/response shapes.
//!   - `error`      — the ADR-0010 envelope + the 4 divergent non-envelope shapes (Q4 carry).
//!   - `extractors` — the type-state auth family (Owner/Customer/CourierSession).
//!   - `middleware` — the REV-4 pre-route tower (dev-404, bearer gate, OPTIONS, NO_AUTH/OTP).
//!   - `mount`      — AuthState + the router assembly (RETIRE decision applied).
//!
//! ## `dead_code` allowance (dark-surface scope)
//! S2 is a DARK surface built AHEAD of its consumers: the REV-1 `CourierSession` + REV-3
//! `CustomerClaimsExt` extractors (and their `service`/`repo` deps) are bound by the AUTHENTICATED
//! S5/S6 surfaces (orders/WS), not by any S2 route; the dev-kid mint constants + `MockAuth*` DTOs
//! are bound only in a `dev-routes` build; `pii::decrypt` is the read-side pair of the write-only
//! redeem path. Every one is exercised by THIS module's tests (proving the council invariants),
//! so `dead_code` here is a binary-crate false-positive for forward-looking pub API. The
//! `clippy --all-targets -D warnings` gate stays fully strict for all non-auth code and for every
//! other lint (this allow is scoped to the auth module tree only).
#![allow(
    dead_code,
    reason = "dark S2 surface — forward-looking API bound by future S5/S6/dev consumers; test-exercised"
)]

pub mod claims;
pub mod config;
pub mod crypto;
pub mod dto;
pub mod error;
pub mod extractors;
pub mod jwt;
pub mod middleware;
pub mod mount;
pub mod pii;
// S10 platform-admin plane-gate (docs/design/rebuild-platform-admin-s10-council/, REV-S10-1) — the
// RUNTIME root-hook analogue that gates every `/api/admin/*` request (registered/sibling/future/
// unregistered) on the `platform_admins` allowlist. Lives in `auth` because it is an auth boundary
// (like the S2 extractors), but adds NO new claim variant (RED LINE, Q-ADMIN-NO-CLAIM).
pub mod plane_gate;
pub mod repo;
pub mod service;
pub mod store;

pub use mount::{AuthState, auth_router};
