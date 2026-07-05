//! S1 storefront-read axum entrypoint. Boots config (fail-fast) -> connects the two sqlx pools
//! (fail-fast) -> serves the full S1 storefront-read surface (`/healthz`, `/livez`,
//! `/openapi.json` + the 20 `openapi-s1-storefront-read.yaml` operations) -> shuts down
//! gracefully on SIGTERM/SIGINT within a bounded deadline.
#![forbid(unsafe_code)]
// See domain/src/lib.rs for why `unwrap`/`expect` are relaxed in `#[cfg(test)]` only.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod auth;
mod cache;
mod config;
mod db;
mod dto;
mod error;
mod jobs;
mod media;
mod middleware;
mod openapi;
mod repo;
mod routes;
mod service;
mod storage;
mod ws;
// Test-only: throwaway RSA keypairs generated at runtime (crates/api/src/test_support.rs) —
// replaces committed test_keys/*.pem so no key material ever enters the tree (secrets hygiene).
#[cfg(test)]
pub(crate) mod test_support;

use std::sync::Arc;
use std::time::Duration;

use axum::error_handling::HandleErrorLayer;
use axum::extract::Extension;
use axum::http::{HeaderName, StatusCode};
use axum::routing::get;
use axum::{BoxError, Json, Router};
use tower::ServiceBuilder;
use tower::timeout::TimeoutLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;

use config::Config;
use db::Pools;
use middleware::ratelimit::RateLimitLayer;
use repo::{CachedRepo, PgRepo, PublicRepo};
use storage::{LocalFsStorage, R2Storage, Storage};

/// How long in-flight requests get to finish once a shutdown signal arrives before the process
/// force-exits. Chosen well under Fly's default stop-signal-to-SIGKILL grace period.
const SHUTDOWN_DEADLINE: Duration = Duration::from_secs(10);

/// Per-request timeout — the tower layer requested by the build brief.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

const CORRELATION_ID_HEADER: &str = "x-correlation-id";

/// S1 follow-up #3 (rebuild/README.md "New follow-ups": rate-limiting middleware) — matches
/// Node's global limiter (`apps/api/src/server.ts:360-376`: `max: 100, timeWindow: '1 minute'`).
const RATE_LIMIT_MAX_REQUESTS: u32 = 100;
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);

/// Shared app state for every S1 handler. `media_rich_enabled`/`app_base_url`/`r2_public_url`
/// are raw env reads at boot (see `routes/voice_config.rs`'s module doc for why these stay raw
/// rather than joining `config::Config`'s strict-validated surface — Node itself never validates
/// them either, CARRY-VERBATIM of the actual un-migrated behavior).
pub struct AppState {
    pub repo: Arc<dyn PublicRepo>,
    pub storage: Arc<dyn Storage>,
    pub media_rich_enabled: bool,
    pub app_base_url: String,
    pub r2_public_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = Config::from_env().map_err(|err| {
        tracing::error!(%err, "boot failed: invalid configuration");
        err
    })?;

    let pools = Pools::connect(&config).await.map_err(|err| {
        tracing::error!(%err, "boot failed: could not connect database pools");
        err
    })?;

    // Built ONCE and shared across S1 (`AppState`) and every S4 owner/public media state — a
    // single object-storage handle for the whole process, matching Node's single `storage`
    // instance threaded through every plugin registration (`server.ts`).
    let storage = build_storage();
    let state = Arc::new(build_app_state(&pools, storage.clone()));
    let mut app = build_router(state);

    // ── S2 auth surface + S3 catalog/admin CRUD surface + S4 media surface (all dark) ──
    // Mount the auth router ONLY when the JWT/auth env is present (AuthConfig fail-fasts on a
    // prod box carrying dev-auth vars — boot-guard D). When the auth env is absent (e.g. an
    // S1-only boot), the auth routes stay DARK (unmounted) — the openapi document still lists
    // them (openapi.rs) so `openapi-diff` is satisfied, but they are not served. Launching S2 is
    // the separate, explicit act of providing the JWT keys.
    //
    // S3 rides the same gate: every S3 op binds the S2 `OwnerClaimsExt` extractor (nothing can
    // authenticate without the JWT verifier), so "auth env present" is exactly the S3
    // precondition too — the catalog surface is dark precisely when the auth surface is.
    //
    // S4 (media) rides the SAME gate too, by build-brief instruction — even though its
    // UNAUTHENTICATED half (`routes::media_public`, entry-photo + the token-proxy-PUT endpoint)
    // needs no JWT at all, it mounts "dark exactly when S2 is dark" rather than unconditionally,
    // so the whole media surface (owner-authenticated + public) launches as one unit.
    match build_auth_state(&pools) {
        Ok(auth_state) => {
            // Captured BEFORE `auth_state` is moved into S7's `build_courier_states` below — the S8
            // telegram webhook needs it to fail-closed on an unset secret in prod (guardian fix).
            let is_production = auth_state.config.node_env.is_production();
            // Captured BEFORE the same S7 move — the S10 runtime plane-gate (REV-S10-1) verifies the
            // bearer at request time and so needs the JWT verifier at the OUTERMOST layer below.
            let auth_verifier = auth_state.verifier.clone();
            app = app.merge(auth::auth_router(auth_state.clone()));
            tracing::info!("S2 auth surface mounted");
            app = app.merge(routes::owner::owner_catalog_router(build_owner_states(
                auth_state.clone(),
                &pools,
                storage.clone(),
                &config,
            )));
            tracing::info!("S3 catalog/admin CRUD surface mounted");
            app = app.merge(routes::media_public::media_public_router(
                build_media_public_state(storage, &config),
                config.media.entry_photo_global_cap_per_minute,
            ));
            tracing::info!("S4 media surface mounted");
            // ── S5 orders/money surface (docs/design/rebuild-orders-s5-council/) — the crown-jewel
            // red-line. Rides the SAME auth-env gate: the owner/customer ops bind the S2 extractors,
            // so "auth env present" is the S5 precondition too. Dark (mounted, not launched) — the
            // openapi document lists the ops (openapi.rs) so openapi-diff is satisfied.
            app = app.merge(routes::orders::orders_router(build_orders_state(
                auth_state.clone(),
                &pools,
            )));
            tracing::info!("S5 orders/money surface mounted");
            // ── S6 realtime-WS surface (docs/design/rebuild-realtime-s6-council/) — the 🔴
            // realtime-authz + cross-tenant fan-out red-line. Rides the SAME auth-env gate. Mounting
            // `/ws` and SPAWNING the `PgListener` fan-out are two halves of one dark launch (REV-S6-5).
            let (ws_state, ws_lifecycle_rx) = ws::WsState::build(
                auth_state.clone(),
                Arc::new(ws::repo::PgWsAuthzRepo::new(pools.operational.clone())),
                std::env::var("WS_URL_TOKEN_ACCEPT")
                    .map(|v| v != "false")
                    .unwrap_or(true),
            );
            app = app.merge(ws::ws_router(ws_state.clone()));
            tokio::spawn(ws::run_fanout(ws_state, config.clone(), ws_lifecycle_rx));
            tracing::info!("S6 realtime-WS surface mounted (PgListener fan-out task spawned)");
            // ── S7 courier/dispatch surface (docs/design/rebuild-courier-s7-council/) — the courier
            // operational plane. Same auth-env gate; every courier op binds the S2 CourierSession
            // extractor; owner-side courier management lives inside the S3/S4 owner_catalog_router.
            // Takes the final `auth_state` move (S6 above cloned it).
            app = app.merge(routes::courier::courier_router(build_courier_states(
                auth_state, &pools,
            )));
            tracing::info!("S7 courier/dispatch surface mounted");

            // ── S8 jobs/notifications surface (docs/design/rebuild-jobs-s8-council/) — the
            // background-work RUNTIME, not axum routes (`crate::jobs` module doc): the SKIP
            // LOCKED claim-loop worker + the cron fleet this build owns. Rides the same
            // auth-env gate as S3-S7 — money/PII-adjacent crons need the same
            // "fully configured" precondition every other post-S1 surface does.
            //
            // The ONE S8 axum route (REV-S8-2 🔴, fail-closed webhook) — its own tiny
            // `Extension` layer, since `TelegramWebhookState` is unrelated to every other
            // surface's state.
            app = app.merge(
                Router::new()
                    .route(
                        "/webhook/telegram/{secret}",
                        axum::routing::post(routes::telegram_webhook::telegram_webhook),
                    )
                    .layer(Extension(routes::telegram_webhook::TelegramWebhookState {
                        bot_secret: config
                            .notifications
                            .telegram_bot_secret
                            .clone()
                            .map(Arc::new),
                        // Guardian fix: prod + unset secret → fail-closed (reject), never accept-anyone.
                        require_secret: is_production,
                    })),
            );
            let push_sender = build_push_sender(&config);
            jobs::worker::spawn(pools.operational.clone(), push_sender);
            let mut spawned_crons: Vec<&str> = Vec::new();
            jobs::crons::order_timeout_sweep::spawn(pools.operational.clone());
            spawned_crons.push("order.timeout_sweep");
            jobs::crons::settlement::spawn(pools.operational.clone(), |now| {
                (now - chrono::Duration::days(1), now)
            });
            spawned_crons.push("settlement.generate");
            jobs::crons::refund_reconciler::spawn(pools.operational.clone());
            spawned_crons.push("refund_due.reconcile");
            jobs::crons::reconciliation::spawn(pools.operational.clone());
            spawned_crons.push("reconciliation.nightly");
            jobs::crons::gdpr_sweep::spawn(pools.operational.clone());
            spawned_crons.push("anonymizer.gdpr");
            jobs::crons::liveness::spawn(pools.operational.clone());
            spawned_crons.push("liveness.check");
            // Q-BOOT-ASSERT extended to every cron this build owns (not 2/23) — a visible red
            // deploy instead of a silently-forgotten cron. `anonymizer.gdpr` IS spawned (S8 owns
            // its timing/single-flight per §2); it now drives the real S9 erasure semantics
            // (`jobs::gdpr_erasure`, `jobs::crons::gdpr_sweep` module doc) rather than a no-op.
            if let Err(missing) = jobs::cron::assert_full_roster_spawned(&spawned_crons) {
                panic!("S8 boot-assert: cron(s) never spawned: {missing:?}");
            }
            tracing::info!(
                "S8 jobs/notifications surface mounted (claim-loop worker + cron fleet spawned)"
            );

            // ── S10 platform-admin/provisioning surface (docs/design/rebuild-platform-admin-s10-council/)
            // — the LAST strangler surface + highest-privilege plane. Rides the SAME auth-env gate as
            // S3-S8. Dark (mounted, not launched). Two planes with two DISTINCT gates:
            //  • Plane A (`/api/admin/*`) — the 6 platform-ops routes, protected by the REV-S10-1 RUNTIME
            //    plane-gate applied as the OUTERMOST layer below.
            //  • Plane B (`/internal/acquisition/*`) — 9 provisioning/claim routes behind the ops-secret
            //    gate (a ROUTER-scoped layer — the faithful port of Node's PLUGIN `onRequest`, not root).
            app = app.merge(routes::admin::admin_router(
                routes::admin::PlatformAdminState {
                    repo: Arc::new(routes::admin::PgAdminOpsRepo::new(
                        pools.operational.clone(),
                    )),
                    // Q-DRILL-NODE-CARVEOUT: the thin Rust trigger carries the actor to the (dark) Node drill.
                    drill: Arc::new(routes::admin::NodeRestoreDrill),
                    // ADMIN_DRILLS_ENABLED scopes ONLY the two heavy drills; the recovery reads never darken.
                    drills_enabled: std::env::var("ADMIN_DRILLS_ENABLED").as_deref() == Ok("true"),
                },
            ));
            app = app.merge(
                routes::internal_acquisition::acquisition_router(
                    routes::internal_acquisition::AcquisitionState {
                        repo: Arc::new(routes::internal_acquisition::PgAcquisitionRepo::new(
                            pools.operational.clone(),
                        )),
                        base_url: std::env::var("APP_BASE_URL")
                            .unwrap_or_else(|_| "https://dowiz.fly.dev".to_string()),
                    },
                )
                // Q-PROVISION-SECRET: the ops-secret gate, fail-closed-404 when PROVISION_OPS_SECRET is
                // unset (read from env, NOT the config Zod schema — decoupled from dev-login, B4).
                .layer(axum::middleware::from_fn_with_state(
                    routes::internal_acquisition::OpsGateState {
                        secret: std::env::var("PROVISION_OPS_SECRET")
                            .ok()
                            .filter(|s| !s.is_empty())
                            .map(|s| Arc::new(config::Secret::new(s))),
                    },
                    routes::internal_acquisition::ops_secret_gate,
                )),
            );
            // REV-S10-1 (CRIT, LOAD-BEARING): the runtime platform-admin plane-gate as the OUTERMOST
            // layer on the FULLY-MERGED app — the axum analogue of Node's root-instance `onRequest` hook
            // (`platform-admin.ts:76`). Applied LAST so it runs FIRST on EVERY request: any `/api/admin/*`
            // path (registered, sibling, future, or entirely unregistered) is 401/403'd HERE before axum
            // routing, so no admin route can escape the `platform_admins` allowlist gate — the fail-SAFE
            // property a nested-Router+lint cannot give (breaker C1). Proven by attack:
            // `auth::plane_gate::tests::admin_plane_gate_denies_unregistered_future_route`.
            app = app.layer(axum::middleware::from_fn_with_state(
                auth::plane_gate::PlatformAdminGateState::new(
                    auth_verifier,
                    Arc::new(auth::plane_gate::PgPlatformAdminRepo::new(
                        pools.operational.clone(),
                    )),
                ),
                auth::plane_gate::platform_admin_plane_gate,
            ));
            tracing::info!(
                "S10 platform-admin/provisioning surface mounted (RUNTIME plane-gate armed as outermost layer)"
            );
        }
        Err(err) => {
            tracing::warn!(%err, "S2 auth + S3 catalog + S4 media + S5 orders + S6 WS + S7 courier + S8 jobs + S10 platform-admin surfaces DARK — JWT/auth env not configured; not mounting them");
        }
    }

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!(port = config.port, "listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Builds `AppState` from connected `Pools` + a shared `storage` handle + raw env reads. Split
/// out from `main()` so it's exercised (minus the live-DB pool) by `build_router`'s wiring test
/// below without needing a live Postgres.
fn build_app_state(pools: &Pools, storage: Arc<dyn Storage>) -> AppState {
    AppState {
        // S1 follow-up #1: `CachedRepo` (repo.rs) wraps the real `PgRepo` with the TTL+SWR cache
        // — `AppState.repo` stays `Arc<dyn PublicRepo>` either way, so every route handler and
        // every route module's test fixtures are untouched by this.
        repo: Arc::new(CachedRepo::new(Arc::new(PgRepo::new(
            pools.operational.clone(),
        )))),
        storage,
        media_rich_enabled: std::env::var("MEDIA_RICH_ENABLED").as_deref() == Ok("true"),
        app_base_url: std::env::var("APP_BASE_URL")
            .unwrap_or_else(|_| "https://dowiz.fly.dev".to_string()),
        r2_public_url: std::env::var("R2_PUBLIC_URL")
            .ok()
            .filter(|s| !s.is_empty()),
    }
}

/// S1 follow-up #2: `STORAGE_BACKEND=r2` selects the real R2 client (`storage::R2Storage`);
/// anything else (including unset, the safe default for local/dev) keeps `LocalFsStorage`. This
/// is a NEW, explicit selector env var — Node instead infers R2 implicitly from
/// `R2_BUCKET && R2_ENDPOINT` both being set (`server.ts:306`) with no separate flag. Flagged
/// deviation: this follow-up's brief asked for `STORAGE_BACKEND=local|r2` specifically, so an
/// explicit flag was built rather than Node's implicit-presence gate; whoever wires a live deploy
/// should pick ONE of the two conventions, not carry both.
///
/// Fails fast (panics before ever listening) if `STORAGE_BACKEND=r2` is set but any of the 4
/// `R2_*` vars is missing — same boot philosophy as `Config::from_env`, and matches Node's own
/// `R2StorageProvider` constructor throwing on a missing `R2_BUCKET`/`R2_ENDPOINT`
/// (`r2-storage.ts:18-20`).
fn build_storage() -> Arc<dyn Storage> {
    match std::env::var("STORAGE_BACKEND").as_deref() {
        Ok("r2") => match R2Storage::from_env() {
            Ok(r2) => Arc::new(r2),
            Err(err) => {
                tracing::error!(%err, "boot failed: STORAGE_BACKEND=r2 misconfigured");
                panic!("boot failed: STORAGE_BACKEND=r2 misconfigured: {err}");
            }
        },
        _ => Arc::new(LocalFsStorage::new(
            std::env::var("LOCAL_STORAGE_DIR").unwrap_or_else(|_| "tmp/imports".to_string()),
        )),
    }
}

/// Build the S2 `AuthState` from env + pools. `Err` when the JWT/auth env is missing or invalid
/// (the auth surface then stays dark — see the call site). Store/Google default to in-memory /
/// null (Redis + a real Google client are prod wirings behind those seams, A19). The PII cipher
/// loads from `COURIER_PII_ENCRYPTION_KEY` when present; without it courier redeem returns a typed
/// 500 rather than writing plaintext PII.
fn build_auth_state(pools: &Pools) -> Result<auth::AuthState, Box<dyn std::error::Error>> {
    let cfg = auth::config::AuthConfig::from_env()?;
    let verifier = Arc::new(auth::jwt::JwtVerifier::from_config(&cfg)?);
    let pii_cipher = std::env::var("COURIER_PII_ENCRYPTION_KEY")
        .ok()
        .filter(|s| !s.is_empty())
        .and_then(|k| auth::pii::PiiCipher::from_base64(&k).ok())
        .map(Arc::new);
    Ok(auth::AuthState::new(
        verifier,
        Arc::new(auth::repo::PgAuthRepo::new(pools.operational.clone())),
        Arc::new(cfg),
        Arc::new(auth::store::InMemoryStore::default()),
        Arc::new(auth::store::NullGoogleClient),
        pii_cipher,
    ))
}

/// Build the S3 per-module states PLUS the S4 owner-authenticated media states (each a
/// `Pg*Repo` over the operational pool — every query inside goes through `db::with_user`, never
/// a raw pool read; see `routes/owner/mod.rs`). `app_base_url`/`r2_public_url` mirror
/// `build_app_state`'s S1 raw env reads — the `/api/owner/menu/products*` alias ops (and the S4
/// media ops) thread them into the same `get_image_url` mapping the S1 menu read uses
/// (`product-mapper.ts` parity).
fn build_owner_states(
    auth: auth::AuthState,
    pools: &Pools,
    storage: Arc<dyn Storage>,
    config: &config::Config,
) -> routes::owner::OwnerCatalogStates {
    let app_base_url =
        std::env::var("APP_BASE_URL").unwrap_or_else(|_| "https://dowiz.fly.dev".to_string());
    let r2_public_url = std::env::var("R2_PUBLIC_URL")
        .ok()
        .filter(|s| !s.is_empty());
    let token_signer = config
        .media
        .upload_token_secret_hex
        .as_deref()
        .and_then(|hex| media::upload_token::UploadTokenSigner::from_hex(hex).ok())
        .map(Arc::new);

    routes::owner::OwnerCatalogStates {
        auth: auth.clone(),
        products: routes::owner::products::ProductsState {
            auth: auth.clone(),
            repo: Arc::new(routes::owner::products::PgProductsRepo::new(
                pools.operational.clone(),
            )),
            app_base_url: app_base_url.clone(),
            r2_public_url: r2_public_url.clone(),
        },
        categories: routes::owner::categories::CategoriesState {
            auth: auth.clone(),
            repo: Arc::new(routes::owner::categories::PgCategoriesRepo::new(
                pools.operational.clone(),
            )),
        },
        modifier_groups: routes::owner::modifier_groups::ModifierGroupsState {
            auth: auth.clone(),
            repo: Arc::new(routes::owner::modifier_groups::PgModifierGroupsRepo::new(
                pools.operational.clone(),
            )),
        },
        menu_availability: routes::owner::menu_availability::MenuAvailabilityState {
            auth: auth.clone(),
            repo: Arc::new(
                routes::owner::menu_availability::PgMenuAvailabilityRepo::new(
                    pools.operational.clone(),
                ),
            ),
        },
        themes: routes::owner::themes::ThemesState {
            auth: auth.clone(),
            repo: Arc::new(routes::owner::themes::PgThemesRepo::new(
                pools.operational.clone(),
            )),
            storage: storage.clone(),
            processor: Arc::new(media::processor::RustImageProcessor),
            app_base_url: app_base_url.clone(),
        },
        product_media: routes::owner::product_media::ProductMediaState {
            auth: auth.clone(),
            repo: Arc::new(routes::owner::product_media::PgProductMediaRepo::new(
                pools.operational.clone(),
            )),
            storage: storage.clone(),
            token_signer,
            app_base_url: app_base_url.clone(),
        },
        product_image: routes::owner::product_image::ProductImageState {
            auth: auth.clone(),
            repo: Arc::new(routes::owner::product_image::PgProductImageRepo::new(
                pools.operational.clone(),
            )),
            storage,
            processor: Arc::new(media::processor::RustImageProcessor),
            app_base_url,
        },
        // S7 owner-side courier management — same auth-env gate, same OwnerClaimsExt layer as S3/S4;
        // the repos seat `app.current_tenant` via `with_tenant` (the courier/service RLS root), NOT
        // `with_user` (see routes/owner/couriers.rs & courier_invites.rs module docs).
        couriers: routes::owner::couriers::CouriersState {
            auth: auth.clone(),
            repo: Arc::new(routes::owner::couriers::PgCouriersRepo::new(
                pools.operational.clone(),
            )),
        },
        courier_invites: routes::owner::courier_invites::CourierInvitesState {
            auth: auth.clone(),
            repo: Arc::new(routes::owner::courier_invites::PgCourierInvitesRepo::new(
                pools.operational.clone(),
            )),
        },
        // S9 GDPR/compliance (docs/design/rebuild-gdpr-s9-council/) — the request-lifecycle/reads
        // surface only; the erasure ENGINE (crate::jobs::gdpr_erasure) is wired into the S8 cron
        // fleet separately, below.
        gdpr: routes::owner::gdpr::GdprState {
            auth,
            repo: Arc::new(routes::owner::gdpr::PgGdprRepo::new(
                pools.operational.clone(),
            )),
        },
    }
}

/// Build the S7 courier-side operational state (`docs/design/rebuild-courier-s7-council/`). Each
/// submodule repo is a `Pg*Repo` over the operational pool; every courier read+write inside routes
/// through `db::with_tenant(activeLocationId)` (the courier/service RLS root — REV-S7-1 complete
/// seat census). Reuses the S2 `AuthState` verbatim for the `CourierSession` extractor (the
/// per-request session-liveness bind, REV-S7-7) — no new auth.
fn build_courier_states(auth: auth::AuthState, pools: &Pools) -> routes::courier::CourierStates {
    routes::courier::CourierStates {
        auth: auth.clone(),
        shifts: routes::courier::shifts::ShiftsState {
            auth: auth.clone(),
            repo: Arc::new(routes::courier::shifts::PgShiftsRepo::new(
                pools.operational.clone(),
            )),
        },
        assignments: routes::courier::assignments::AssignmentsState {
            auth: auth.clone(),
            repo: Arc::new(routes::courier::assignments::PgAssignmentsRepo::new(
                pools.operational.clone(),
            )),
        },
        me: routes::courier::me::MeState {
            auth: auth.clone(),
            repo: Arc::new(routes::courier::me::PgMeRepo::new(
                pools.operational.clone(),
            )),
        },
        settlements: routes::courier::settlements::SettlementsState {
            auth,
            repo: Arc::new(routes::courier::settlements::PgSettlementsRepo::new(
                pools.operational.clone(),
            )),
        },
    }
}

/// Build the S5 orders/money state — a `PgOrdersRepo` over the operational pool (the create tx runs
/// a GUC-less `pool.begin()`, the owner-status tx a `with_user`, the customer-cancel tx a
/// `with_tenant`; see `routes/orders/pg.rs`). Reuses the S2 `AuthState` verbatim for the
/// owner/customer extractors — no new auth.
fn build_orders_state(auth: auth::AuthState, pools: &Pools) -> routes::orders::OrdersState {
    routes::orders::OrdersState {
        auth,
        repo: Arc::new(routes::orders::pg::PgOrdersRepo::new(
            pools.operational.clone(),
        )),
    }
}

/// Build the S4 UNAUTHENTICATED media state (entry-photo + the token-proxy-PUT endpoint) — no
/// DB pool at all, matching `spa-proxy.ts:268-293`'s design (entry-photo writes no row).
fn build_media_public_state(
    storage: Arc<dyn Storage>,
    config: &config::Config,
) -> routes::media_public::MediaPublicState {
    let token_signer = config
        .media
        .upload_token_secret_hex
        .as_deref()
        .and_then(|hex| media::upload_token::UploadTokenSigner::from_hex(hex).ok())
        .map(Arc::new);
    routes::media_public::MediaPublicState {
        storage,
        processor: Arc::new(media::processor::RustImageProcessor),
        token_signer,
        app_base_url: std::env::var("APP_BASE_URL")
            .unwrap_or_else(|_| "https://dowiz.fly.dev".to_string()),
        entry_photo_enabled: config.media.entry_photo_enabled,
    }
}

/// Builds the S8 VAPID push adapter — `None` when either VAPID key is absent
/// (`NotificationsConfig::vapid_ready`, `bootstrap/notifications.ts:52` CARRY) so the caller
/// stays dark exactly like every other optional channel in this surface, or when the
/// `reqwest::Client` itself fails to build (an environment-level TLS/DNS-resolver init failure,
/// not a config-shape one — logged, not fatal, since the rest of S8 has no hard dependency on
/// push specifically).
fn build_push_sender(
    config: &config::Config,
) -> Option<Arc<jobs::channels::push::VapidPushSender>> {
    if !config.notifications.vapid_ready() {
        tracing::info!("VAPID keys not configured — push adapter stays dark");
        return None;
    }
    #[allow(
        clippy::unwrap_used,
        reason = "vapid_ready() above already proved vapid_private_key is Some"
    )]
    let private_key = config.notifications.vapid_private_key.clone().unwrap();
    match jobs::channels::push::VapidPushSender::new(
        private_key,
        config.notifications.vapid_subject.clone(),
    ) {
        Ok(sender) => Some(Arc::new(sender)),
        Err(err) => {
            tracing::error!(%err, "VAPID push adapter failed to build its HTTP client — push stays dark");
            None
        }
    }
}

fn build_router(state: Arc<AppState>) -> Router {
    let correlation_header = HeaderName::from_static(CORRELATION_ID_HEADER);

    Router::new()
        .route("/healthz", get(routes::health::healthz))
        .route("/livez", get(routes::health::livez))
        .route("/openapi.json", get(openapi::openapi_json))
        // S1 storefront-read — openapi-s1-storefront-read.yaml (20 operations).
        .route(
            "/public/locations/{locationIdOrSlug}/menu",
            get(routes::menu::get_public_menu),
        )
        .route(
            "/public/locations/{slug}/info",
            get(routes::menu::get_public_location_info),
        )
        .route(
            "/public/locations/{slug}/products/{productId}/media",
            get(routes::menu::get_product_media),
        )
        .route(
            "/api/public/theme/{slug}",
            get(routes::theme::get_public_theme),
        )
        .route(
            "/public/locations/{locationId}/theme.css",
            get(routes::theme::get_theme_css),
        )
        .route("/s/{slug}", get(routes::storefront::get_storefront_page))
        .route(
            "/s/{slug}/cart",
            get(routes::storefront::get_storefront_cart_page),
        )
        .route(
            "/s/{slug}/checkout",
            get(routes::storefront::get_storefront_checkout_page),
        )
        .route(
            "/s/{slug}/order/{id}",
            get(routes::storefront::get_storefront_order_page),
        )
        .route(
            "/s/{slug}/orders/{orderId}",
            get(routes::storefront::get_storefront_order_page_legacy),
        )
        .route(
            "/s/{slug}/manifest.webmanifest",
            get(routes::manifest::get_web_manifest),
        )
        .route(
            "/api/public/locations/{slug}/fallback-config",
            get(routes::fallback_config::get_fallback_config),
        )
        .route("/images/{*key}", get(routes::media_proxy::get_image))
        .route("/media/{*key}", get(routes::media_proxy::get_media_object))
        .route(
            "/api/public/voice-config",
            get(routes::voice_config::get_voice_config),
        )
        .route(
            "/api/push/vapid-public-key",
            get(routes::vapid::get_vapid_public_key),
        )
        .route("/v1/rates", get(routes::rates::get_exchange_rate))
        .route("/robots.txt", get(routes::seo::get_robots_txt))
        .route("/sitemap.xml", get(routes::seo::get_sitemap_index))
        // Wire note (see `routes::seo::parse_sitemap_shard_filename`'s doc): axum/matchit
        // cannot register `/sitemap-locations-{shard}.xml` directly (mixed literal+capture in
        // one segment) — `{filename}` captures the whole segment and the handler parses the
        // `sitemap-locations-<N>.xml` shape itself. Static routes (`/robots.txt`, `/sitemap.xml`
        // above) still take routing priority over this single-segment capture.
        .route("/{filename}", get(routes::seo::get_sitemap_shard))
        // TimeoutLayer's inner service can itself fail (the timeout elapsing is an Err, not a
        // Response) — axum requires every layered service's Error to be Into<Infallible>, so a
        // HandleErrorLayer must sit in front of it to turn that Err into a real Response.
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(handle_timeout_error))
                .layer(TimeoutLayer::new(REQUEST_TIMEOUT)),
        )
        .layer(PropagateRequestIdLayer::new(correlation_header.clone()))
        .layer(TraceLayer::new_for_http())
        .layer(SetRequestIdLayer::new(correlation_header, MakeRequestUuid))
        // S1 follow-up #3: outermost layer (added last) — gates every route the same way Node's
        // globally-registered `fastifyRateLimit` does (`server.ts:360-376`), before request-id
        // assignment even runs (the layer mints its own correlation id per rejected request; see
        // `middleware/ratelimit.rs`).
        .layer(RateLimitLayer::new(
            RATE_LIMIT_MAX_REQUESTS,
            RATE_LIMIT_WINDOW,
        ))
        .with_state(state)
}

/// The ADR-0010 envelope shape, hand-built here (not via `domain::ErrorEnvelope`) because a
/// tower-layer timeout is infra-level, not a domain error — it has no `OrderStatus`/order-machine
/// meaning and doesn't warrant a new `domain::ErrorCode` variant for one cross-cutting concern.
async fn handle_timeout_error(err: BoxError) -> (StatusCode, Json<serde_json::Value>) {
    if err.is::<tower::timeout::error::Elapsed>() {
        (
            StatusCode::REQUEST_TIMEOUT,
            Json(
                serde_json::json!({ "code": "REQUEST_TIMEOUT", "message": "request exceeded the timeout" }),
            ),
        )
    } else {
        tracing::error!(%err, "unhandled tower layer error");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "code": "INTERNAL", "message": "internal error" })),
        )
    }
}

/// Waits for SIGTERM (Fly/Docker/k8s stop signal) or SIGINT (Ctrl-C, local dev), then returns —
/// which is what tells `axum::serve`'s graceful shutdown to stop accepting new connections and
/// let in-flight ones finish. A second task races a hard deadline and force-exits the process if
/// graceful shutdown hasn't completed by then, so a stuck connection can never hang a restart.
async fn shutdown_signal() {
    // Failing to install a signal handler at boot is not a recoverable runtime condition — there
    // is no sensible degraded mode (the process would then never respond to a stop signal at
    // all), so panicking immediately is the correct fail-fast behavior, not a swallowed error.
    #[allow(
        clippy::expect_used,
        reason = "boot-time signal-handler install failure is fatal by design, see fn doc"
    )]
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install SIGINT handler");
    };

    #[cfg(unix)]
    #[allow(
        clippy::expect_used,
        reason = "boot-time signal-handler install failure is fatal by design, see fn doc"
    )]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    tracing::info!(
        deadline_secs = SHUTDOWN_DEADLINE.as_secs(),
        "shutdown signal received, draining in-flight requests"
    );

    // Belt-and-suspenders: if graceful drain hasn't finished by the deadline, force-exit rather
    // than let Fly/Docker's own SIGKILL do it silently with no log line. `clippy::exit` exists to
    // stop library code from short-circuiting its caller's control flow — this is the one place
    // in a binary's entrypoint where an unconditional process exit IS the intended behavior (the
    // build brief's "graceful shutdown ... deadline" requirement), so it's allowed narrowly here.
    #[allow(
        clippy::exit,
        reason = "intentional hard-deadline watchdog in the binary entrypoint, see fn doc"
    )]
    tokio::spawn(async {
        tokio::time::sleep(SHUTDOWN_DEADLINE).await;
        tracing::error!("graceful shutdown exceeded its deadline; forcing exit");
        std::process::exit(1);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;

    fn fake_state() -> Arc<AppState> {
        Arc::new(AppState {
            repo: Arc::new(repo::fake::FakeRepo::default()),
            storage: Arc::new(LocalFsStorage::new(std::env::temp_dir())),
            media_rich_enabled: false,
            app_base_url: "https://dowiz.fly.dev".to_string(),
            r2_public_url: None,
        })
    }

    /// The real point of this test: `Router::route` PANICS at construction time if a path
    /// pattern is invalid for axum's matchit-based router (e.g. mixing a literal prefix with a
    /// named capture in one segment). `build_router` registers ALL 20 S1 operations + health +
    /// openapi — if any pattern (especially `/sitemap-locations-{shard}.xml`, which mixes
    /// literal text with a capture in one segment) is invalid, this test fails LOUDLY here
    /// instead of only at `cargo run` boot time (which `cargo test`/`cargo clippy` never
    /// exercise otherwise).
    #[tokio::test]
    async fn build_router_does_not_panic_and_serves_healthz() {
        let app = build_router(fake_state());
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/healthz")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn build_router_serves_sitemap_shard_pattern() {
        let app = build_router(fake_state());
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/sitemap-locations-1.xml")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        // Empty FakeRepo -> 404 (no sitemap rows) is the CORRECT behavior here; the point of
        // this assertion is that the route matched (not a router-level 404 from no route found,
        // which would be indistinguishable at this level) — combined with the panic-freedom
        // proven by the previous test, a 404 here confirms the pattern matched and the handler
        // ran its own not-found branch, not that the route failed to register.
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn build_router_serves_wildcard_image_route() {
        let app = build_router(fake_state());
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/images/some/nested/key.webp")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        // No file on disk -> 404, but (as above) reaching the handler at all proves the
        // `{*key}` wildcard pattern registered and matched a multi-segment path.
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
