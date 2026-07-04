//! Phase A axum entrypoint. Boots config (fail-fast) -> connects the two sqlx pools (fail-fast)
//! -> serves `/healthz`, `/livez`, `/openapi.json`, and the (stubbed) public menu route -> shuts
//! down gracefully on SIGTERM/SIGINT within a bounded deadline.
#![forbid(unsafe_code)]
// See domain/src/lib.rs for why `unwrap`/`expect` are relaxed in `#[cfg(test)]` only.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod config;
mod db;
mod error;
mod openapi;
mod routes;

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

/// How long in-flight requests get to finish once a shutdown signal arrives before the process
/// force-exits. Chosen well under Fly's default stop-signal-to-SIGKILL grace period.
const SHUTDOWN_DEADLINE: Duration = Duration::from_secs(10);

/// Per-request timeout — the tower layer requested by the build brief. Deliberately generous for
/// Phase A (no real DB-backed route yet); this will need per-route tuning once the menu query
/// lands (a slow storefront read should fail faster than a slow admin report).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

const CORRELATION_ID_HEADER: &str = "x-correlation-id";

#[allow(dead_code)] // wired at boot; not read directly by routes yet (Phase A has no DB-backed route)
struct AppState {
    pools: Pools,
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

    let state = Arc::new(AppState { pools });
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!(port = config.port, "listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn build_router(state: Arc<AppState>) -> Router {
    let correlation_header = HeaderName::from_static(CORRELATION_ID_HEADER);

    Router::new()
        .route("/healthz", get(routes::health::healthz))
        .route("/livez", get(routes::health::livez))
        .route("/openapi.json", get(openapi::openapi_json))
        .route(
            "/api/v1/public/menu/{slug}",
            get(routes::menu::get_public_menu),
        )
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
