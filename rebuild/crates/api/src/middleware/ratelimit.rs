//! S1 follow-up #3 (rebuild/README.md "New follow-ups": rate-limiting middleware) — a tower
//! `Layer` gating every route the way Node's globally-registered `@fastify/rate-limit` plugin
//! does (`apps/api/src/server.ts:360-376`): `max: 100` requests per `timeWindow: '1 minute'`,
//! keyed on the REAL client IP.
//!
//! Keying mirrors `apps/api/src/lib/client-ip.ts` exactly on the one property that actually
//! matters for brute-force/abuse resistance: trust ONLY the `Fly-Client-IP` header (the Fly edge
//! sets and overwrites it on every ingress path — not client-injectable) and NEVER
//! `X-Forwarded-For` (client-controllable; trusting it would let a client rotate the header to
//! fragment its own bucket and evade the limit). A missing header fails to a single shared
//! bucket rather than trusting a spoofable one — same fail-closed posture as Node's prod branch
//! (`client-ip.ts:55-60`); this crate has no dev/prod distinction yet, so there is no
//! `request.ip` dev-mode fallback here (flagged: Node's non-prod branch degrades to the raw
//! socket IP instead — irrelevant for abuse-resistance, only for local-dev ergonomics).
//!
//! Algorithm: a fixed-window counter per IP key (reset when the window elapses), NOT a strict
//! token bucket — simpler, and sufficient for the guarantee this follow-up actually needs
//! ("Nth request in the window is allowed, N+1th is rejected with a `Retry-After`"). Named
//! `RateLimitLayer`/`RateLimitService` rather than "token bucket" in code to avoid overclaiming.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode, header};
use axum::response::{IntoResponse, Response};
use tower::{Layer, Service};

use domain::{ErrorCode, ErrorEnvelope};

const FLY_IP_HEADER: &str = "fly-client-ip";
/// Fail-closed bucket for the (should-be-rare, edge-guaranteed) case where `Fly-Client-IP` is
/// absent — every such request shares ONE bucket rather than trusting any client-supplied header
/// (`client-ip.ts:14-15`'s prod posture; see module doc for why there's no dev-mode fallback).
const SHARED_BUCKET_KEY: &str = "shared:no-fly-ip";

struct Bucket {
    count: u32,
    window_start: Instant,
}

struct RateLimitState {
    max_requests: u32,
    window: Duration,
    buckets: Mutex<HashMap<String, Bucket>>,
}

impl RateLimitState {
    fn check(&self, key: &str) -> Result<(), Duration> {
        let now = Instant::now();
        let mut buckets = self.buckets.lock().unwrap_or_else(PoisonError::into_inner);
        let bucket = buckets.entry(key.to_string()).or_insert_with(|| Bucket {
            count: 0,
            window_start: now,
        });

        if now.duration_since(bucket.window_start) >= self.window {
            bucket.window_start = now;
            bucket.count = 0;
        }

        bucket.count += 1;
        if bucket.count > self.max_requests {
            let elapsed = now.duration_since(bucket.window_start);
            Err(self.window.saturating_sub(elapsed))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone)]
pub struct RateLimitLayer {
    state: Arc<RateLimitState>,
}

impl RateLimitLayer {
    pub fn new(max_requests: u32, window: Duration) -> Self {
        RateLimitLayer {
            state: Arc::new(RateLimitState {
                max_requests,
                window,
                buckets: Mutex::new(HashMap::new()),
            }),
        }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            state: self.state.clone(),
        }
    }
}

#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    state: Arc<RateLimitState>,
}

impl<S> Service<Request<Body>> for RateLimitService<S>
where
    S: Service<Request<Body>, Response = Response, Error = std::convert::Infallible>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = std::convert::Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let key = client_ip_key(req.headers());
        let decision = self.state.check(&key);
        // Standard tower "clone-and-swap" so `self.inner` stays ready for the NEXT `call` while
        // this request's work runs against an owned clone inside the returned future.
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            match decision {
                Ok(()) => inner.call(req).await,
                Err(retry_after) => Ok(too_many_requests(retry_after)),
            }
        })
    }
}

/// `client-ip.ts::clientIp` + `normalizeIp`, minus the dev-mode `request.ip` fallback (see module
/// doc). `HeaderMap::get` is already case-insensitive, so the lowercase constant is enough.
fn client_ip_key(headers: &HeaderMap) -> String {
    match headers.get(FLY_IP_HEADER).and_then(|v| v.to_str().ok()) {
        Some(raw) if !raw.is_empty() => normalize_ip(raw),
        _ => SHARED_BUCKET_KEY.to_string(),
    }
}

/// Ports `client-ip.ts::normalizeIp` verbatim: lowercase, strip `[...]` brackets, strip an IPv6
/// zone id, collapse an IPv4-mapped IPv6 address to bare IPv4 — so the same client can never
/// fragment across multiple buckets by casing/representation alone.
fn normalize_ip(raw: &str) -> String {
    let mut ip = raw.trim().to_ascii_lowercase();
    if let Some(rest) = ip.strip_prefix('[') {
        ip = match rest.find(']') {
            Some(end) => rest[..end].to_string(),
            None => rest.to_string(),
        };
    }
    if let Some(pos) = ip.find('%') {
        ip.truncate(pos);
    }
    if let Some(mapped) = ip.strip_prefix("::ffff:") {
        if is_ipv4_shape(mapped) {
            return mapped.to_string();
        }
    }
    ip
}

fn is_ipv4_shape(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() == 4
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.parse::<u8>().is_ok())
}

/// The ADR-0010 envelope shape for a 429 (`apps/api/src/lib/api-error.ts::rateLimitError`):
/// `code: RATE_LIMIT`, `retryAfterMs`, plus a `Retry-After` header. This layer can run BEFORE
/// `SetRequestIdLayer` mints the per-request id (it is the outermost layer, `main.rs`), so it
/// mints its own correlation id rather than reading a request extension that may not exist yet.
fn too_many_requests(retry_after: Duration) -> Response {
    let retry_after_secs = retry_after.as_secs().max(1);
    let mut envelope = ErrorEnvelope::new(
        ErrorCode::RateLimit,
        format!("Too many requests. Try again in {retry_after_secs}s."),
        uuid::Uuid::new_v4().to_string(),
    );
    envelope.retry_after_ms = Some(u64::try_from(retry_after.as_millis()).unwrap_or(u64::MAX));

    (
        StatusCode::TOO_MANY_REQUESTS,
        [(header::RETRY_AFTER, retry_after_secs.to_string())],
        axum::Json(envelope),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use tower::{ServiceBuilder, ServiceExt};

    /// A trivial always-200 inner service, concrete (not `impl Trait`/closure-based) so its
    /// `Future`/`Clone` impls are unambiguous for `RateLimitService`'s generic bounds.
    #[derive(Clone)]
    struct OkService;

    impl Service<Request<Body>> for OkService {
        type Response = Response;
        type Error = std::convert::Infallible;
        type Future =
            Pin<Box<dyn Future<Output = Result<Response, std::convert::Infallible>> + Send>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _req: Request<Body>) -> Self::Future {
            Box::pin(async { Ok(Response::new(Body::empty())) })
        }
    }

    fn ok_service() -> OkService {
        OkService
    }

    fn request_from(ip: &str) -> Request<Body> {
        Request::builder()
            .uri("/public/locations/x/menu")
            .header(FLY_IP_HEADER, ip)
            .body(Body::empty())
            .unwrap()
    }

    fn request_with_xff(fly_ip: &str, xff: &str) -> Request<Body> {
        Request::builder()
            .uri("/public/locations/x/menu")
            .header(FLY_IP_HEADER, fly_ip)
            .header("x-forwarded-for", xff)
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn nth_plus_one_request_is_rate_limited() {
        let layer = RateLimitLayer::new(2, Duration::from_secs(60));
        let svc = ServiceBuilder::new().layer(layer).service(ok_service());

        for _ in 0..2 {
            let resp = svc.clone().oneshot(request_from("1.2.3.4")).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
        }
        let third = svc.clone().oneshot(request_from("1.2.3.4")).await.unwrap();
        assert_eq!(third.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(third.headers().get(header::RETRY_AFTER).is_some());
    }

    #[tokio::test]
    async fn different_ips_have_independent_buckets() {
        let layer = RateLimitLayer::new(1, Duration::from_secs(60));
        let svc = ServiceBuilder::new().layer(layer).service(ok_service());

        let a1 = svc.clone().oneshot(request_from("1.1.1.1")).await.unwrap();
        assert_eq!(a1.status(), StatusCode::OK);
        let b1 = svc.clone().oneshot(request_from("2.2.2.2")).await.unwrap();
        assert_eq!(
            b1.status(),
            StatusCode::OK,
            "a different IP must not share 1.1.1.1's bucket"
        );

        let a2 = svc.clone().oneshot(request_from("1.1.1.1")).await.unwrap();
        assert_eq!(
            a2.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "1.1.1.1's own second request must be limited"
        );
    }

    #[tokio::test]
    async fn spoofed_x_forwarded_for_does_not_rekey_the_bucket() {
        let layer = RateLimitLayer::new(1, Duration::from_secs(60));
        let svc = ServiceBuilder::new().layer(layer).service(ok_service());

        let first = svc
            .clone()
            .oneshot(request_with_xff("9.9.9.9", "10.0.0.1"))
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::OK);

        // Same Fly-Client-IP, a DIFFERENT X-Forwarded-For each time — must still hit the SAME
        // bucket (the whole point of never trusting XFF for the key).
        let second = svc
            .clone()
            .oneshot(request_with_xff("9.9.9.9", "10.0.0.2"))
            .await
            .unwrap();
        assert_eq!(
            second.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "a spoofed X-Forwarded-For must not create a fresh bucket"
        );
    }

    #[tokio::test]
    async fn missing_fly_client_ip_falls_back_to_one_shared_bucket() {
        let layer = RateLimitLayer::new(1, Duration::from_secs(60));
        let svc = ServiceBuilder::new().layer(layer).service(ok_service());

        let no_header = Request::builder().uri("/x").body(Body::empty()).unwrap();
        let first = svc.clone().oneshot(no_header).await.unwrap();
        assert_eq!(first.status(), StatusCode::OK);

        let no_header_again = Request::builder().uri("/x").body(Body::empty()).unwrap();
        let second = svc.clone().oneshot(no_header_again).await.unwrap();
        assert_eq!(
            second.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "every header-less request shares the same fail-closed bucket"
        );
    }

    #[test]
    fn normalize_ip_matches_client_ip_ts_shapes() {
        assert_eq!(normalize_ip("2001:DB8::1"), "2001:db8::1");
        assert_eq!(normalize_ip("[2001:db8::1]"), "2001:db8::1");
        assert_eq!(normalize_ip("fe80::1%eth0"), "fe80::1");
        assert_eq!(normalize_ip("::ffff:1.2.3.4"), "1.2.3.4");
        assert_eq!(normalize_ip("1.2.3.4"), "1.2.3.4");
    }

    #[test]
    fn header_value_casing_does_not_matter_for_lookup() {
        let mut headers = HeaderMap::new();
        headers.insert("Fly-Client-IP", HeaderValue::from_static("5.5.5.5"));
        assert_eq!(client_ip_key(&headers), "5.5.5.5");
    }
}
