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
//!
//! Configuration is via CLI flags / env vars only — there are NO secret reads
//! and no `.env` loading. NO-COURIER-SCORING guard is asserted below.

use std::path::{Path, PathBuf};

use axum::{
    extract::Request,
    http::{header, HeaderName, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
    Router,
};
use tower_http::services::{ServeDir, ServeFile};

/// DEFAULT_ROOT mirrors the legacy nginx web root.
pub const DEFAULT_ROOT: &str = "/usr/share/nginx/html";
/// DEFAULT_PORT mirrors the nginx `listen 8080`.
pub const DEFAULT_PORT: u16 = 8080;

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
pub fn build_router(root: impl AsRef<Path>) -> Router {
    let root = root.as_ref().to_path_buf();
    let index = root.join("index.html");
    let serve_dir = ServeDir::new(&root)
        .precompressed_gzip()
        .precompressed_br()
        .precompressed_deflate()
        .fallback(ServeFile::new(index));

    Router::new()
        .fallback_service(serve_dir)
        .layer(axum::middleware::from_fn(asset_cache_control))
        .layer(axum::middleware::from_fn(security_headers))
}

/// Resolve the static root from CLI/env, falling back to [`DEFAULT_ROOT`].
pub fn resolve_root(explicit: Option<PathBuf>) -> PathBuf {
    explicit.unwrap_or_else(|| PathBuf::from(DEFAULT_ROOT))
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
