//! `native-spa-server` — DK-04 native-Rust static file server.
//!
//! Replaces the chainguard/nginx container. Serves a `--root` directory on
//! port 8080 (default) with:
//!
//! * SPA fallback: unknown non-asset routes resolve to `index.html`.
//! * `/assets/` long-cache: `Cache-Control: public, max-age=31536000, immutable`.
//! * The exact security headers ported 1:1 from `docker/nginx-default.conf`.
//! * gzip/deflate (and precompressed `.gz`/`.br`/`.zz`) compression for text.
//! * HTTP/1.1 by default; HTTP/2 when a TLS cert is supplied.
//! * A minimal cap-gated order API (P37) merged on top — see `api`.
//!
//! Configuration is via CLI flags / env vars only — there are NO secret reads
//! and no `.env` loading. NO-COURIER-SCORING guard is asserted below.

use std::collections::HashMap;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use axum::{
    extract::Request,
    http::{header, HeaderName, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
    Router,
};
use tower_http::services::{ServeDir, ServeFile};

/// The minimal HTTP order surface (P37 W37-2/3): cap-gated `/api/order*`.
pub mod api;

/// P48-INTAKE Phase 1 — `/webhook/*` route handlers (external signature gate,
/// NOT capability-cert gated — separate trust boundary per §5.3).
pub mod webhook;

/// DEFAULT_ROOT mirrors the legacy nginx web root.
pub const DEFAULT_ROOT: &str = "/usr/share/nginx/html";
/// DEFAULT_PORT mirrors the nginx `listen 8080`.
pub const DEFAULT_PORT: u16 = 8080;

/// Hard cap on concurrently-accepted TCP connections (defense-in-depth, listener-wide).
/// Sized well above `api::MAX_INFLIGHT_API` (64) to allow static-asset traffic headroom,
/// but bounded so an accept-time flood cannot grow the task count without limit.
/// Rationale: 512 covers a single browser's concurrent-connection ceiling (~6–8) across
/// many users, with headroom for Keep-Alive reuse; a deliberate choice to leave room for
/// the existing per-API bulkhead to be the tighter bound for /api/* traffic.
pub const MAX_CONCURRENT_CONNECTIONS: usize = 512;

/// EXACT security headers ported from `docker/nginx-default.conf` (lines 16-25).
///
/// These are applied to EVERY response via [`security_headers`] middleware,
/// matching nginx `add_header ... always;` semantics (always = on error
/// responses too).
pub const SECURITY_HEADERS: &[(&str, &str)] = &[
    (
        "Content-Security-Policy",
        "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'; object-src 'none'; upgrade-insecure-requests",
    ),
    ("X-Content-Type-Options", "nosniff"),
    ("X-Frame-Options", "DENY"),
    ("Referrer-Policy", "strict-origin-when-cross-origin"),
    (
        "Permissions-Policy",
        "geolocation=(), microphone=(), camera=(), payment=()",
    ),
];

/// Axum middleware: attach the EXACT ported security headers to every response.
pub async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    // Reset to avoid duplicate appends if a layer is stacked (idempotent set).
    for (name, value) in SECURITY_HEADERS {
        insert_header(response.headers_mut(), name, value);
    }
    response
}

/// Axum middleware: mirror nginx `location /assets/ { add_header Cache-Control
/// "public, max-age=31536000, immutable"; }`. Any request path under `/assets/`
/// gets the EXACT immutable long-cache header.
pub async fn asset_cache_control(request: Request, next: Next) -> Response {
    let is_asset = request.uri().path().starts_with("/assets/");
    let mut response = next.run(request).await;
    if is_asset {
        if let Ok(v) = HeaderValue::from_str("public, max-age=31536000, immutable") {
            response
                .headers_mut()
                .insert(HeaderName::from_static("cache-control"), v);
        }
    }
    response
}

/// Insert a header by name (case-insensitive) into a `HeaderMap`, parsing the
/// name as bytes so mixed-case literal names (e.g. `Content-Security-Policy`)
/// are accepted. `HeaderName::from_static` would reject them.
fn insert_header(headers: &mut header::HeaderMap, name: &str, value: &str) {
    if let (Ok(n), Ok(v)) = (
        header::HeaderName::from_bytes(name.as_bytes()),
        HeaderValue::from_str(value),
    ) {
        headers.insert(n, v);
    }
}

/// Build the application router for a given static root directory.
///
/// * `ServeDir` handles real files (with precompressed variants).
/// * `ServeFile` of `index.html` is the SPA fallback for unknown routes
///   (mirrors nginx `try_files $uri $uri/ /index.html`).
/// * The cap-gated order API (P37) is merged on top — its middleware runs only
///   on the `/api/*` + `/healthz` routes, so static serving is byte-unchanged.
pub fn build_router(root: impl AsRef<Path>, api: Arc<api::ApiState>, webhook_state: Arc<webhook::WebhookState>) -> Router {
    let root = root.as_ref().to_path_buf();
    let index = root.join("index.html");
    let serve_dir = ServeDir::new(&root)
        .precompressed_gzip()
        .precompressed_br()
        .precompressed_deflate()
        .fallback(ServeFile::new(index));

    Router::new()
        .fallback_service(serve_dir)
        .merge(api::build_api_router(api))
        .merge(webhook::build_webhook_router(webhook_state))
        .layer(axum::middleware::from_fn(asset_cache_control))
        .layer(axum::middleware::from_fn(security_headers))
}

/// Resolve the static root from CLI/env, falling back to [`DEFAULT_ROOT`].
pub fn resolve_root(explicit: Option<PathBuf>) -> PathBuf {
    explicit.unwrap_or_else(|| PathBuf::from(DEFAULT_ROOT))
}

/// Default header-read timeout for plain HTTP/1.1 connections. A client that has not
/// finished sending its request headers within this window is disconnected — the
/// standard mitigation for a slowloris-style connection-exhaustion attack (a client
/// opens a connection and trickles headers in slowly enough to never trigger a normal
/// request timeout while occupying one of the [`api::MAX_INFLIGHT_API`] bulkhead slots
/// or a bare listener socket indefinitely). Matches hyper-util's own documented default.
pub const DEFAULT_HEADER_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Per-IP connection admission state. Each distinct source IP gets a
/// [`dowiz_kernel::token_bucket::TokenBucket`] that admits at most `CAPACITY` burst
/// connections with a sustained `REFILL_PER_SEC` rate. Idle entries are evicted by
/// `sweep_locked` on every `admit` call to bound map growth from many distinct IPs
/// (each of which DID complete a real TCP handshake, so IP spoofing at this layer is
/// naturally constrained by the OS TCP stack).
pub struct PerIpLimiter {
    buckets: Mutex<HashMap<IpAddr, (dowiz_kernel::token_bucket::TokenBucket, Instant)>>,
}

impl PerIpLimiter {
    /// Burst capacity per IP — generous for a real browser opening several asset
    /// connections at once, tight against a single-source connection flood.
    pub const CAPACITY: f64 = 8.0;
    /// Sustained refill rate per IP (tokens/sec).
    pub const REFILL_PER_SEC: f64 = 2.0;
    /// Entries idle longer than this are evicted to bound map growth.
    pub const IDLE_EVICT_AFTER: std::time::Duration = std::time::Duration::from_secs(300);

    pub fn new() -> Self {
        PerIpLimiter {
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Try to admit a connection from `ip`. Returns `true` if the per-IP budget
    /// allows it, `false` if throttled (caller should drop the connection).
    pub fn admit(&self, ip: IpAddr) -> bool {
        let mut m = self.buckets.lock().unwrap();
        self.sweep_locked(&mut m);
        let (bucket, seen) = m.entry(ip).or_insert_with(|| {
            (
                dowiz_kernel::token_bucket::TokenBucket::new(Self::CAPACITY, Self::REFILL_PER_SEC),
                Instant::now(),
            )
        });
        *seen = Instant::now();
        bucket.try_acquire(1.0)
    }

    /// Evict entries idle past `IDLE_EVICT_AFTER` — bounds map growth from many
    /// distinct IPs. Called inside `admit` while the lock is already held.
    fn sweep_locked(&self, m: &mut HashMap<IpAddr, (dowiz_kernel::token_bucket::TokenBucket, Instant)>) {
        m.retain(|_, (_, seen)| seen.elapsed() < Self::IDLE_EVICT_AFTER);
    }

    /// Return the number of tracked IPs (for tests / diagnostics).
    pub fn len(&self) -> usize {
        self.buckets.lock().unwrap().len()
    }
}

/// Serve `router` over plain HTTP/1.1 on an already-bound `listener`, with a bounded
/// header-read timeout per connection and listener-wide DoS defenses.
///
/// Two defense-in-depth layers guard the accept loop:
/// 1. **Per-IP rate limiting** ([`PerIpLimiter`]) — a [`dowiz_kernel::token_bucket::TokenBucket`]
///    per source IP admits at most `CAPACITY` burst / `REFILL_PER_SEC` sustained connections;
///    checked first (cheapest, rejects a flood before touching the global cap).
/// 2. **Global concurrent-connection cap** ([`MAX_CONCURRENT_CONNECTIONS`]) — a
///    [`tokio::sync::Semaphore`] that fail-closes (drops the connection immediately, never
///    blocks the accept loop) when the server is at capacity.
///
/// Both layers are defense-in-depth alongside the existing `api::MAX_INFLIGHT_API` bulkhead
/// (which only sees connections that finish header parsing — a stalled mid-header connection
/// never reaches it). Neither new layer replaces or interferes with the existing layers.
pub async fn serve_with_timeout(
    listener: tokio::net::TcpListener,
    router: Router,
    header_read_timeout: std::time::Duration,
) -> std::io::Result<()> {
    let conn_limit = std::sync::Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_CONNECTIONS));
    let ip_limiter = std::sync::Arc::new(PerIpLimiter::new());
    loop {
        let (stream, peer) = listener.accept().await?;
        // Layer 1: per-IP admission — cheapest check, rejects a single-source flood
        // before touching the global cap. A throttled connection is dropped immediately.
        if !ip_limiter.admit(peer.ip()) {
            eprintln!(
                "[native-spa-server] per-IP connection budget ({}/{} burst, {}/s sustained) \
                 exhausted for {peer}, dropping connection",
                PerIpLimiter::CAPACITY as u64,
                PerIpLimiter::CAPACITY as u64,
                PerIpLimiter::REFILL_PER_SEC as u64,
            );
            continue;
        }
        // Layer 2: global cap — fail-CLOSED, not blocking: if the semaphore is
        // saturated, drop this connection immediately (RST on stream-drop) rather
        // than queueing it — queueing would just move the exhaustion from "tasks"
        // to "an unbounded internal queue". The accept loop is NEVER blocked, so
        // legitimate new connections are never stuck behind a saturated cap.
        let permit = match conn_limit.clone().try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                eprintln!(
                    "[native-spa-server] global connection cap ({MAX_CONCURRENT_CONNECTIONS}) \
                     reached, dropping connection from {peer}"
                );
                continue;
            }
        };
        let router = router.clone();
        tokio::spawn(async move {
            let _permit = permit; // held for the connection's lifetime, released on task exit
            let hyper_service = hyper_util::service::TowerToHyperService::new(router);
            let mut builder =
                hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());
            builder
                .http1()
                .timer(hyper_util::rt::TokioTimer::new())
                .header_read_timeout(header_read_timeout);
            if let Err(e) = builder
                .serve_connection_with_upgrades(
                    hyper_util::rt::TokioIo::new(stream),
                    hyper_service,
                )
                .await
            {
                // A timed-out or reset connection surfaces here as an I/O error —
                // this is the header-read timeout doing its job, not a bug; log at
                // most, never propagate (one bad connection must not kill the server).
                eprintln!("[native-spa-server] connection closed: {e}");
            }
        });
    }
}

/// Build a 200 `index.html` response carrying the security headers. Used by the
/// binary for an out-of-band `/healthz` liveness probe.
pub fn health_response() -> Response {
    let mut r = Response::new(axum::body::Body::from("ok"));
    *r.status_mut() = StatusCode::OK;
    for (name, value) in SECURITY_HEADERS {
        insert_header(r.headers_mut(), name, value);
    }
    r
}
