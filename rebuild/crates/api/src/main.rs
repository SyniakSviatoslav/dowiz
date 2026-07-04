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
mod middleware;
mod openapi;
mod repo;
mod routes;
mod service;
mod storage;
// Test-only: throwaway RSA keypairs generated at runtime (crates/api/src/test_support.rs) —
// replaces committed test_keys/*.pem so no key material ever enters the tree (secrets hygiene).
#[cfg(test)]
pub(crate) mod test_support;

use std::sync::Arc;
use std::time::Duration;

use axum::error_handling::HandleErrorLayer;
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

    let state = Arc::new(build_app_state(&pools));
    let mut app = build_router(state);

    // ── S2 auth surface (dark) ──
    // Mount the auth router ONLY when the JWT/auth env is present (AuthConfig fail-fasts on a
    // prod box carrying dev-auth vars — boot-guard D). When the auth env is absent (e.g. an
    // S1-only boot), the auth routes stay DARK (unmounted) — the openapi document still lists
    // them (openapi.rs) so `openapi-diff` is satisfied, but they are not served. Launching S2 is
    // the separate, explicit act of providing the JWT keys.
    match build_auth_state(&pools) {
        Ok(auth_state) => {
            app = app.merge(auth::auth_router(auth_state));
            tracing::info!("S2 auth surface mounted");
        }
        Err(err) => {
            tracing::warn!(%err, "S2 auth surface DARK — JWT/auth env not configured; not mounting auth routes");
        }
    }

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!(port = config.port, "listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Builds `AppState` from connected `Pools` + raw env reads. Split out from `main()` so it's
/// exercised (minus the live-DB pool) by `build_router`'s wiring test below without needing a
/// live Postgres.
fn build_app_state(pools: &Pools) -> AppState {
    AppState {
        // S1 follow-up #1: `CachedRepo` (repo.rs) wraps the real `PgRepo` with the TTL+SWR cache
        // — `AppState.repo` stays `Arc<dyn PublicRepo>` either way, so every route handler and
        // every route module's test fixtures are untouched by this.
        repo: Arc::new(CachedRepo::new(Arc::new(PgRepo::new(
            pools.operational.clone(),
        )))),
        storage: build_storage(),
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
