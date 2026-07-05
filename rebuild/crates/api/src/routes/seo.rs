//! S1 storefront-read: `getRobotsTxt`, `getSitemapIndex`, `getSitemapShard`.
//! Source: `apps/api/src/routes/public/seo.ts`.

use std::sync::Arc;

use axum::extract::{Extension, Path, State};
use axum::http::{HeaderMap, header};
use axum::response::IntoResponse;
use tower_http::request_id::RequestId;

use domain::ErrorCode;

use crate::AppState;
use crate::error::ApiError;
use crate::repo::SitemapLocationRow;
use crate::routes::correlation_id_string;

const MAX_URLS_PER_SHARD: usize = 50_000;

/// Ports the exact `robots.txt` body (`seo.ts:50-82`), parameterized only by the derived
/// `sitemapUrl` — the rest is a fixed template, kept byte-identical (the AI-bot allowlist is a
/// real business decision, not incidental formatting).
pub fn build_robots_txt(sitemap_url: &str) -> String {
    format!(
        "User-agent: *\nAllow: /$\nAllow: /s/\nAllow: /public/\nDisallow: /s/*/cart\nDisallow: /s/*/checkout\nDisallow: /s/*/order/\nDisallow: /*?embed=true\nDisallow: /*?preview=true\nDisallow: /admin/\nDisallow: /courier/\nDisallow: /onboarding\nDisallow: /api/\nDisallow: /dist/\nDisallow: /icons/\n\n# AI answer engines — allowed for citation\nUser-agent: GPTBot\nAllow: /s/\nUser-agent: OAI-SearchBot\nAllow: /s/\nUser-agent: ChatGPT-User\nAllow: /s/\nUser-agent: ClaudeBot\nAllow: /s/\nUser-agent: Claude-Web\nAllow: /s/\nUser-agent: PerplexityBot\nAllow: /s/\nUser-agent: Google-Extended\nAllow: /s/\n\nSitemap: {sitemap_url}"
    )
}

/// Derives `${protocol}://${host}` from request headers, mirroring Fastify's
/// `request.hostname`/`request.protocol` (`seo.ts:46-48`). Behind Fly's proxy `X-Forwarded-Proto`
/// carries the original scheme; `https` is the safe default absent that header (this deploy is
/// never plain-HTTP in production).
pub fn base_url_from_headers(headers: &HeaderMap) -> String {
    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("dowiz.fly.dev");
    let protocol = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("https");
    format!("{protocol}://{host}")
}

/// `GET /robots.txt` — source: `seo.ts:45-86`. Always 200.
#[utoipa::path(
    get,
    path = "/robots.txt",
    responses((status = 200, description = "Robots directives incl. AI-bot allowlist")),
    tag = "seo"
)]
pub async fn get_robots_txt(headers: HeaderMap) -> impl IntoResponse {
    let base_url = base_url_from_headers(&headers);
    let body = build_robots_txt(&format!("{base_url}/sitemap.xml"));
    (
        [
            (header::CONTENT_TYPE, "text/plain"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        body,
    )
}

/// Ports the sitemap-index shard-count math (`seo.ts:96`): at least 1 shard, else
/// `ceil(total / MAX_URLS_PER_SHARD)`.
pub fn shard_count(total_with_products: usize) -> usize {
    total_with_products.div_ceil(MAX_URLS_PER_SHARD).max(1)
}

pub fn build_sitemap_index(base_url: &str, shards: usize) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<sitemapindex xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for i in 1..=shards {
        xml.push_str(&format!(
            "  <sitemap>\n    <loc>{base_url}/sitemap-locations-{i}.xml</loc>\n  </sitemap>\n"
        ));
    }
    xml.push_str("</sitemapindex>");
    xml
}

/// `GET /sitemap.xml` — source: `seo.ts:90-118`.
#[utoipa::path(
    get,
    path = "/sitemap.xml",
    responses(
        (status = 200, description = "Sitemap index XML"),
        (status = 500, description = "DB failure", body = domain::ErrorEnvelope),
    ),
    tag = "seo"
)]
pub async fn get_sitemap_index(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let rows = state.repo.active_locations().await.map_err(|err| {
        tracing::warn!(%err, "active_locations query failed while building the sitemap index");
        ApiError::new(
            ErrorCode::Internal,
            "internal_error",
            correlation_id_string(&request_id),
        )
    })?;
    let total = rows.iter().filter(|r| r.has_products).count();
    let base_url = base_url_from_headers(&headers);
    let xml = build_sitemap_index(&base_url, shard_count(total));
    Ok((
        [
            (header::CONTENT_TYPE, "application/xml"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        xml,
    ))
}

/// Ports `buildUrlTag` (`seo.ts:35-43`) verbatim — one `<url>` entry with per-locale
/// `hreflang` alternates plus an `x-default`.
pub fn build_url_tag(loc: &str, lastmod: &str, supported_locales: Option<&[String]>) -> String {
    let mut xml = format!("  <url>\n    <loc>{loc}</loc>\n    <lastmod>{lastmod}</lastmod>\n");
    let default_locales = ["sq".to_string(), "en".to_string()];
    let locales = supported_locales
        .filter(|l| !l.is_empty())
        .unwrap_or(&default_locales);
    for locale in locales {
        xml.push_str(&format!(
            "    <xhtml:link rel=\"alternate\" hreflang=\"{locale}\" href=\"{loc}?locale={locale}\" />\n"
        ));
    }
    xml.push_str(&format!(
        "    <xhtml:link rel=\"alternate\" hreflang=\"x-default\" href=\"{loc}\" />\n"
    ));
    xml.push_str("  </url>\n");
    xml
}

pub fn build_sitemap_shard(base_url: &str, rows: &[SitemapLocationRow]) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" xmlns:xhtml=\"http://www.w3.org/1999/xhtml\">\n",
    );
    for row in rows {
        let loc = format!("{base_url}/s/{}", row.slug);
        let lastmod = row.lastmod.format("%Y-%m-%d").to_string();
        xml.push_str(&build_url_tag(
            &loc,
            &lastmod,
            row.supported_locales.as_deref(),
        ));
    }
    xml.push_str("</urlset>");
    xml
}

/// Ports `parseInt((request.params as any).shard, 10) || 1` (`seo.ts:122`) as an explicit
/// filename parse: axum's matchit-based router (unlike Fastify's) does NOT support mixing a
/// literal prefix/suffix with a named capture in one path segment (`/sitemap-locations-{shard}.xml`
/// panics at `Router::route()` construction — caught by `main.rs`'s
/// `build_router_does_not_panic_and_serves_healthz`-style wiring test). So this operation is
/// registered in `main.rs` as a single-segment capture (`/{filename}`) and this function does
/// the prefix/suffix parse Fastify's router did implicitly. The WIRE contract
/// (`openapi-s1-storefront-read.yaml`'s `/sitemap-locations-{shard}.xml`) is unaffected — this is
/// purely how axum reaches that same URL shape, not a behavior change a client can observe.
pub fn parse_sitemap_shard_filename(filename: &str) -> Option<i64> {
    let digits = filename
        .strip_prefix("sitemap-locations-")?
        .strip_suffix(".xml")?;
    digits
        .parse::<i64>()
        .ok()
        .or(Some(1))
        .filter(|_| !digits.is_empty())
}

/// `GET /sitemap-locations-{shard}.xml` — source: `seo.ts:121-160`. 🔴 privacy invariant:
/// `active_locations` already excludes shadow tenants (`org.owner_id IS NOT NULL` filter,
/// repo.rs) — never re-derive that filter here.
#[utoipa::path(
    get,
    path = "/sitemap-locations-{shard}.xml",
    params(("shard" = i64, Path)),
    responses(
        (status = 200, description = "urlset XML"),
        (status = 404, description = "Empty/unknown shard", body = domain::ErrorEnvelope),
        (status = 500, description = "DB failure", body = domain::ErrorEnvelope),
    ),
    tag = "seo"
)]
pub async fn get_sitemap_shard(
    State(state): State<Arc<AppState>>,
    Path(filename): Path<String>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let Some(shard) = parse_sitemap_shard_filename(&filename) else {
        return Err(ApiError::new(
            ErrorCode::NotFound,
            "Not found",
            correlation_id,
        ));
    };
    let rows = state.repo.active_locations().await.map_err(|err| {
        tracing::warn!(%err, shard, "active_locations query failed while building a sitemap shard");
        ApiError::new(
            ErrorCode::Internal,
            "internal_error",
            correlation_id.clone(),
        )
    })?;
    let filtered: Vec<_> = rows.into_iter().filter(|r| r.has_products).collect();

    let shard: usize = usize::try_from(shard).unwrap_or(1).max(1);
    let start = (shard - 1) * MAX_URLS_PER_SHARD;
    let slice: Vec<_> = filtered
        .into_iter()
        .skip(start)
        .take(MAX_URLS_PER_SHARD)
        .collect();

    if slice.is_empty() {
        return Err(ApiError::new(
            ErrorCode::NotFound,
            "Not found",
            correlation_id,
        ));
    }

    let base_url = base_url_from_headers(&headers);
    let xml = build_sitemap_shard(&base_url, &slice);
    Ok((
        [
            (header::CONTENT_TYPE, "application/xml"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        xml,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::fake::FakeRepo;
    use crate::storage::LocalFsStorage;

    fn test_state(repo: FakeRepo) -> Arc<AppState> {
        Arc::new(AppState {
            repo: Arc::new(repo),
            storage: Arc::new(LocalFsStorage::new(std::env::temp_dir())),
            media_rich_enabled: false,
            app_base_url: "https://dowiz.fly.dev".to_string(),
            r2_public_url: None,
        })
    }

    #[test]
    fn build_robots_txt_includes_ai_bot_allowlist_and_sitemap_link() {
        let body = build_robots_txt("https://dowiz.fly.dev/sitemap.xml");
        assert!(body.contains("User-agent: ClaudeBot"));
        assert!(body.contains("User-agent: GPTBot"));
        assert!(body.contains("Disallow: /admin/"));
        assert!(body.ends_with("Sitemap: https://dowiz.fly.dev/sitemap.xml"));
    }

    #[test]
    fn base_url_from_headers_uses_forwarded_proto_and_falls_back() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, "dowiz.fly.dev".parse().unwrap());
        assert_eq!(base_url_from_headers(&headers), "https://dowiz.fly.dev");

        headers.insert("x-forwarded-proto", "http".parse().unwrap());
        assert_eq!(base_url_from_headers(&headers), "http://dowiz.fly.dev");
    }

    #[test]
    fn shard_count_is_at_least_one_and_ceils() {
        assert_eq!(shard_count(0), 1);
        assert_eq!(shard_count(1), 1);
        assert_eq!(shard_count(MAX_URLS_PER_SHARD), 1);
        assert_eq!(shard_count(MAX_URLS_PER_SHARD + 1), 2);
    }

    #[test]
    fn build_url_tag_includes_hreflang_alternates_and_x_default() {
        let tag = build_url_tag(
            "https://dowiz.fly.dev/s/eljos-pizza",
            "2026-07-04",
            Some(&["sq".to_string(), "en".to_string()]),
        );
        assert!(tag.contains("hreflang=\"sq\""));
        assert!(tag.contains("hreflang=\"en\""));
        assert!(tag.contains("hreflang=\"x-default\""));
        assert!(tag.contains("<lastmod>2026-07-04</lastmod>"));
    }

    #[test]
    fn build_url_tag_defaults_to_sq_en_when_locales_absent() {
        let tag = build_url_tag("https://dowiz.fly.dev/s/x", "2026-07-04", None);
        assert!(tag.contains("hreflang=\"sq\""));
        assert!(tag.contains("hreflang=\"en\""));
    }

    fn request_id() -> Extension<RequestId> {
        Extension(RequestId::new(axum::http::HeaderValue::from_static(
            "corr-1",
        )))
    }

    #[tokio::test]
    async fn get_sitemap_shard_404_when_empty() {
        let state = test_state(FakeRepo::default());
        let err = crate::error::expect_err(
            get_sitemap_shard(
                State(state),
                Path("sitemap-locations-1.xml".to_string()),
                HeaderMap::new(),
                request_id(),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn get_sitemap_shard_200_with_rows() {
        let repo = FakeRepo::default();
        repo.sitemap_locations
            .lock()
            .unwrap()
            .push(SitemapLocationRow {
                slug: "eljos-pizza".to_string(),
                supported_locales: Some(vec!["sq".to_string()]),
                has_products: true,
                lastmod: chrono::Utc::now(),
            });
        let state = test_state(repo);
        let response = get_sitemap_shard(
            State(state),
            Path("sitemap-locations-1.xml".to_string()),
            HeaderMap::new(),
            request_id(),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[test]
    fn parse_sitemap_shard_filename_accepts_and_rejects() {
        assert_eq!(
            parse_sitemap_shard_filename("sitemap-locations-1.xml"),
            Some(1)
        );
        assert_eq!(
            parse_sitemap_shard_filename("sitemap-locations-42.xml"),
            Some(42)
        );
        assert_eq!(parse_sitemap_shard_filename("sitemap-locations-.xml"), None);
        assert_eq!(parse_sitemap_shard_filename("robots.txt"), None);
        assert_eq!(
            parse_sitemap_shard_filename("sitemap-locations-abc.xml"),
            Some(1)
        );
    }

    #[tokio::test]
    async fn get_sitemap_index_200_empty_locations() {
        let state = test_state(FakeRepo::default());
        let response = get_sitemap_index(State(state), HeaderMap::new(), request_id())
            .await
            .unwrap()
            .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }
}
