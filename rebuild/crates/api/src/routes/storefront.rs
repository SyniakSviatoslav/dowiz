//! S1 storefront-read: `getStorefrontPage`, `getStorefrontCartPage`, `getStorefrontCheckoutPage`,
//! `getStorefrontOrderPage`, `getStorefrontOrderPageLegacy`.
//! Sources: `apps/api/src/routes/public/ssr.ts`, `client-flow.ts`, `lib/spa-shell.ts`,
//! `lib/ssr-renderer.ts`, `lib/preview-render.ts`.
//!
//! ## Scope cut (flagged, not silent)
//! The Node source renders the FULL React/Preact-hydrated SPA shell (`lib/spa-shell.ts` reads
//! `apps/api/public/index.html`, a Vite build artifact) and a pixel-styled preact-rendered SSR
//! menu page with a client hydration bundle (`lib/ssr-renderer.ts`, `<script type="module"
//! src="/dist/menu/app.js">`). Neither build artifact exists in this pure-Rust workspace — and
//! per `openapi-s1-storefront-read.yaml`'s own `getStorefrontPage` description, "This is the
//! Astro handoff seam: the bot branch becomes an Astro SSR route in the rebuild." This build
//! therefore ports the REAL logic that decides behavior — bot/human branching, shadow-tenant
//! detection, the 🔴 P6-2/P6-3 privacy invariant (shadow tenants ALWAYS noindex + generic OG,
//! regardless of branch), and the per-tenant CSP — and renders a minimal-but-correct HTML
//! document (proper meta/JSON-LD/CSP/cache headers) instead of the pixel-identical hydrated
//! page. Full-fidelity HTML is the Astro lane's job; this crate is not going to grow a
//! preact-equivalent template engine to fake it. Flagged in the lane report as UNMAPPED
//! (partial) rather than silently declared BUILT-complete.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::{IntoResponse, Response};

use crate::AppState;
use crate::dto::PublicMenu;
use crate::repo::{PreviewLookup, SpaShellTenantRow};

/// Ports `BOT_UA` (`spa-shell.ts:15`) verbatim as a token list (no regex dependency needed for
/// a pure case-insensitive substring alternation).
const BOT_TOKENS: &[&str] = &[
    "bot",
    "crawl",
    "spider",
    "slurp",
    "mediapartners",
    "facebookexternalhit",
    "embedly",
    "quora",
    "pinterest",
    "whatsapp",
    "telegram",
    "slackbot",
    "twitter",
    "linkedinbot",
    "discord",
    "google",
    "bing",
    "yandex",
    "baidu",
    "duckduck",
    "applebot",
    "petalbot",
    "semrush",
    "ahrefs",
];

/// Ports `isBot` (`spa-shell.ts:17-19`).
pub fn is_bot(user_agent: Option<&str>) -> bool {
    let ua = user_agent.unwrap_or("").to_ascii_lowercase();
    BOT_TOKENS.iter().any(|t| ua.contains(t))
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Ports the CSP template (`spa-shell.ts:159`) verbatim, parameterized on the three inputs the
/// Node function actually varies: per-location `frame-ancestors`, the R2 origin (widens
/// `img-src`/`connect-src` when configured), and the voice-control kill-switch (widens
/// `connect-src` ONLY while voice is live — R2-E gating, same predicate as `getVoiceConfig`).
pub fn build_csp(frame_ancestors: &str, r2_origin: Option<&str>, voice_enabled: bool) -> String {
    let img_extra = r2_origin.map(|o| format!(" {o}")).unwrap_or_default();
    let connect_extra = if voice_enabled {
        r2_origin.map(|o| format!(" {o}")).unwrap_or_default()
    } else {
        String::new()
    };
    format!(
        "default-src 'self'; img-src 'self' data: https:{img_extra}; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; script-src 'self' 'unsafe-inline' 'unsafe-eval' https://cdn.tailwindcss.com https://plausible.io; worker-src 'self' blob:; connect-src 'self' https://cdn.jsdelivr.net https://tiles.openfreemap.org https://router.project-osrm.org https://en.wikipedia.org https://plausible.io{connect_extra}; frame-ancestors {frame_ancestors}"
    )
}

/// True iff `owner_id` is absent — the P6-2/P6-3 shadow-tenant discriminator
/// (`spa-shell.ts:136`, `ssr.ts`'s `read_preview_menu` non-null check). Never advertise a
/// shadow's real name/logo to humans or unfurlers.
fn is_shadow(tenant: &SpaShellTenantRow) -> bool {
    tenant.owner_id.is_none()
}

/// The minimal SPA-shell placeholder this build serves for humans (see module doc's scope cut).
/// Still tenant-aware: injects the same `<title>`/OG tags Node's `buildTenantMeta`
/// (`spa-shell.ts:65-91`) would, so link-unfurl parity holds even without the hydrated bundle.
fn spa_shell_html(
    tenant: Option<&SpaShellTenantRow>,
    slug: &str,
    is_shadow_tenant: bool,
) -> String {
    let t = match tenant {
        Some(t) if !is_shadow_tenant => t,
        _ => {
            return "<!doctype html>\n<html><head><meta charset=\"utf-8\" /><meta name=\"robots\" content=\"noindex, nofollow\" /><title>Dowiz</title></head><body></body></html>".to_string();
        }
    };
    let title = format!("{} — Order Online | Dowiz", t.name);
    let desc = match &t.address {
        Some(addr) => format!(
            "Order delivery from {} at {addr}. View menu, prices, and place your order online.",
            t.name
        ),
        None => format!(
            "Order delivery from {}. View menu, prices, and place your order online.",
            t.name
        ),
    };
    format!(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\" /><title>{}</title><meta name=\"description\" content=\"{}\" /><meta property=\"og:title\" content=\"{}\" /><meta property=\"og:description\" content=\"{}\" /></head><body><div id=\"root\" data-slug=\"{}\"></div></body></html>",
        escape_html(&title),
        escape_html(&desc),
        escape_html(&title),
        escape_html(&desc),
        escape_html(slug),
    )
}

/// Ports `renderShadowPreview` (`preview-render.ts:45-100`), simplified per module scope cut but
/// preserving the H3 safety invariant: generic `<title>`/OG, real name ONLY in the body banner.
fn shadow_preview_html(menu: &PublicMenu) -> String {
    let banner = "This is a preview mockup built from this restaurant's public website — it is NOT a live store and cannot take orders.";
    let mut sections = String::new();
    for cat in &menu.categories {
        sections.push_str(&format!("<section><h2>{}</h2><ul>", escape_html(&cat.name)));
        for p in &cat.products {
            sections.push_str(&format!("<li>{}</li>", escape_html(&p.name)));
        }
        sections.push_str("</ul></section>");
    }
    format!(
        "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\" /><meta name=\"robots\" content=\"noindex, nofollow\" /><title>Restaurant preview · Dowiz</title><meta property=\"og:title\" content=\"Restaurant menu preview · Dowiz\" /><meta property=\"og:description\" content=\"An unclaimed menu preview on Dowiz.\" /></head><body><div class=\"banner\">{}</div><h1>{}</h1>{}</body></html>",
        escape_html(banner),
        escape_html(&menu.location_name),
        sections,
    )
}

/// Ports the schema.org `Restaurant` + `Menu` JSON-LD core fields (`ssr-renderer.ts:66-158`,
/// trimmed to the fields this crate has data for — full FAQPage/BreadcrumbList emission is part
/// of the Astro-lane full-fidelity port, see module doc).
fn build_json_ld(menu: &PublicMenu, slug: &str, base_url: &str) -> serde_json::Value {
    let menu_url = format!("{base_url}/s/{slug}");
    let items: Vec<_> = menu
        .categories
        .iter()
        .flat_map(|c| &c.products)
        .map(|p| {
            serde_json::json!({
                "@type": "MenuItem",
                "name": p.name,
                "offers": {"@type": "Offer", "price": p.price, "priceCurrency": menu.currency.code},
            })
        })
        .collect();
    serde_json::json!([
        {
            "@context": "https://schema.org",
            "@type": "Restaurant",
            "name": menu.location_name,
            "url": menu_url,
        },
        {
            "@context": "https://schema.org",
            "@type": "Menu",
            "name": format!("{} Menu", menu.location_name),
            "hasMenuItem": items,
        }
    ])
}

fn bot_menu_html(menu: &PublicMenu, slug: &str, base_url: &str) -> String {
    let title = format!("{} — Order Online | Dowiz", menu.location_name);
    let json_ld = build_json_ld(menu, slug, base_url).to_string();
    format!(
        "<!doctype html>\n<html lang=\"{}\"><head><meta charset=\"utf-8\" /><title>{}</title><meta property=\"og:title\" content=\"{}\" /><script type=\"application/ld+json\">{}</script></head><body><h1>{}</h1></body></html>",
        menu.default_locale,
        escape_html(&title),
        escape_html(&title),
        json_ld,
        escape_html(&menu.location_name),
    )
}

fn user_agent(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
}

fn html_response(body: String, cache_control: &str, extra: &[(&str, &str)]) -> Response {
    let mut response = axum::response::Html(body).into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_str(cache_control)
            .unwrap_or_else(|_| HeaderValue::from_static("no-store")),
    );
    for (name, value) in extra {
        if let (Ok(name), Ok(value)) = (
            axum::http::HeaderName::try_from(*name),
            HeaderValue::from_str(value),
        ) {
            response.headers_mut().insert(name, value);
        }
    }
    response
}

/// `GET /s/{slug}` — source: `ssr.ts:18-52`. See module doc for the scope cut on HTML fidelity.
#[utoipa::path(
    get,
    path = "/s/{slug}",
    params(("slug" = String, Path)),
    responses((status = 200, description = "HTML (SSR menu / shadow preview / SPA shell — by UA and tenant kind)")),
    tag = "storefront"
)]
pub async fn get_storefront_page(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    let bot = is_bot(user_agent(&headers));

    // Shadow-tenant branch (ssr.ts:32-44): read_preview_menu non-null -> ALWAYS noindex,
    // regardless of bot/human (🔴 P6-2/P6-3 privacy invariant).
    if let Ok(PreviewLookup::Found(preview)) = state.repo.read_preview_menu(&slug).await {
        let menu = crate::service::adapt_preview_menu(&preview);
        let body = if bot {
            shadow_preview_html(&menu)
        } else {
            spa_shell_html(None, &slug, true)
        };
        return html_response(
            body,
            "no-cache, no-store, must-revalidate",
            &[("X-Robots-Tag", "noindex, nofollow")],
        );
    }

    if bot {
        // Reuses `read_public_menu` (default-locale, `locale=""`) rather than porting a second
        // DB entry point for `read_public_menu_all_locales` (`ssr-renderer.ts:297-300`, which
        // returns every locale's translations so the SSR page could hreflang-switch): the
        // JSON-LD/OG this simplified render emits (see module doc) doesn't branch on locale, so
        // the all-locales variant has no consumer in this build yet. Flagged as a follow-up if
        // the Astro lane's full-fidelity SSR page needs the multi-locale payload.
        if let Ok(Some(raw)) = state.repo.read_public_menu(&slug, "").await {
            if let Ok(menu) = serde_json::from_value::<PublicMenu>(raw) {
                let body = bot_menu_html(&menu, &slug, &state.app_base_url);
                return html_response(body, "no-cache, no-store, must-revalidate", &[]);
            }
        }
    }

    serve_spa_shell(&state, &slug).await
}

/// Shared by `getStorefrontCartPage`/`CheckoutPage`/`OrderPage`/`OrderPageLegacy` (all four are
/// `serveSpaShell(reply, db, slug)` in Node, `client-flow.ts:13-19`) AND the human branch of
/// `getStorefrontPage` above. Ports `serveSpaShell`'s CSP + shadow/tenant-meta logic
/// (`spa-shell.ts:116-190`) — see module doc for the HTML-body scope cut.
async fn serve_spa_shell(state: &AppState, slug: &str) -> Response {
    let tenant = state.repo.spa_shell_tenant(slug).await.ok().flatten();
    let shadow = tenant.as_ref().is_some_and(is_shadow);

    let frame_ancestors = tenant
        .as_ref()
        .and_then(|t| t.frame_ancestors.as_ref())
        .filter(|fa| !fa.is_empty())
        .map(|fa| fa.join(" "))
        .unwrap_or_else(|| "'self'".to_string());

    let r2_origin = state.r2_public_url.as_deref().and_then(url_origin);
    let voice_enabled = crate::routes::voice_config::is_voice_enabled(
        std::env::var("VOICE_CONTROL_ENABLED").ok().as_deref(),
        std::env::var("VOICE_KILL").ok().as_deref(),
    );
    let csp = build_csp(&frame_ancestors, r2_origin.as_deref(), voice_enabled);

    let body = spa_shell_html(tenant.as_ref(), slug, shadow);

    let mut extra: Vec<(&str, String)> = vec![("content-security-policy", csp)];
    if shadow {
        extra.push(("X-Robots-Tag", "noindex, nofollow".to_string()));
    }
    let extra_refs: Vec<(&str, &str)> = extra.iter().map(|(k, v)| (*k, v.as_str())).collect();
    html_response(body, "no-cache, no-store, must-revalidate", &extra_refs)
}

/// Extracts `scheme://host[:port]` from a URL string without pulling in a full URL-parsing
/// dependency for this one origin check (mirrors the Node `new URL(r2PublicUrl).origin` call,
/// `spa-shell.ts:148,156`, with a hand-rolled equivalent).
fn url_origin(url: &str) -> Option<String> {
    let scheme_end = url.find("://")?;
    let after_scheme = &url[scheme_end + 3..];
    let host_end = after_scheme.find('/').unwrap_or(after_scheme.len());
    Some(format!(
        "{}{}",
        &url[..scheme_end + 3],
        &after_scheme[..host_end]
    ))
}

#[utoipa::path(get, path = "/s/{slug}/cart", params(("slug" = String, Path)), responses((status = 200, description = "SPA shell HTML")), tag = "storefront")]
pub async fn get_storefront_cart_page(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Response {
    serve_spa_shell(&state, &slug).await
}

#[utoipa::path(get, path = "/s/{slug}/checkout", params(("slug" = String, Path)), responses((status = 200, description = "SPA shell HTML")), tag = "storefront")]
pub async fn get_storefront_checkout_page(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Response {
    serve_spa_shell(&state, &slug).await
}

#[utoipa::path(get, path = "/s/{slug}/order/{id}", params(("slug" = String, Path), ("id" = String, Path)), responses((status = 200, description = "SPA shell HTML")), tag = "storefront")]
pub async fn get_storefront_order_page(
    State(state): State<Arc<AppState>>,
    Path((slug, _id)): Path<(String, String)>,
) -> Response {
    serve_spa_shell(&state, &slug).await
}

/// x-quirk (`client-flow.ts:18`): a duplicate legacy alias handler, kept as its own operation
/// (not collapsed into a redirect) per the contract's own note that this is a port decision row,
/// not a behavior to silently change.
#[utoipa::path(get, path = "/s/{slug}/orders/{orderId}", params(("slug" = String, Path), ("orderId" = String, Path)), responses((status = 200, description = "SPA shell HTML")), tag = "storefront")]
pub async fn get_storefront_order_page_legacy(
    State(state): State<Arc<AppState>>,
    Path((slug, _order_id)): Path<(String, String)>,
) -> Response {
    serve_spa_shell(&state, &slug).await
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
    fn is_bot_matches_known_crawlers_case_insensitively() {
        assert!(is_bot(Some("Mozilla/5.0 (compatible; Googlebot/2.1)")));
        assert!(is_bot(Some("ClaudeBot/1.0")));
        assert!(is_bot(Some("facebookexternalhit/1.1")));
        assert!(!is_bot(Some(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15) AppleWebKit/605"
        )));
        assert!(!is_bot(None));
    }

    #[test]
    fn build_csp_widens_connect_src_only_when_voice_enabled() {
        let without_voice = build_csp("'self'", Some("https://cdn.r2.dev"), false);
        assert!(!without_voice.contains("connect-src 'self' https://cdn.jsdelivr.net https://tiles.openfreemap.org https://router.project-osrm.org https://en.wikipedia.org https://plausible.io https://cdn.r2.dev;"));
        let with_voice = build_csp("'self'", Some("https://cdn.r2.dev"), true);
        assert!(with_voice.contains("https://cdn.r2.dev; frame-ancestors"));
    }

    #[test]
    fn build_csp_uses_custom_frame_ancestors() {
        let csp = build_csp("https://embed.example.com", None, false);
        assert!(csp.ends_with("frame-ancestors https://embed.example.com"));
    }

    #[test]
    fn url_origin_strips_path() {
        assert_eq!(
            url_origin("https://cdn.r2.dev/bucket/path"),
            Some("https://cdn.r2.dev".to_string())
        );
        assert_eq!(
            url_origin("https://cdn.r2.dev"),
            Some("https://cdn.r2.dev".to_string())
        );
        assert_eq!(url_origin("not-a-url"), None);
    }

    #[test]
    fn shadow_preview_html_never_puts_real_name_in_title_or_og() {
        let menu = crate::service::adapt_preview_menu(&serde_json::json!({
            "name": "Real Secret Restaurant Name",
            "categories": [],
        }));
        let html = shadow_preview_html(&menu);
        assert!(html.contains("<title>Restaurant preview · Dowiz</title>"));
        assert!(!html.contains("<title>Real Secret Restaurant Name"));
        assert!(html.contains("og:title\" content=\"Restaurant menu preview"));
        assert!(
            html.contains("Real Secret Restaurant Name"),
            "real name still appears in the BODY banner"
        );
        assert!(html.contains("noindex, nofollow"));
    }

    #[tokio::test]
    async fn get_storefront_page_shadow_tenant_always_noindex_even_for_humans() {
        let repo = FakeRepo::default();
        repo.preview_menus.lock().unwrap().insert(
            "shadow-cafe".to_string(),
            serde_json::json!({"name": "Shadow Cafe", "categories": []}),
        );
        let state = test_state(repo);
        let mut headers = HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            "Mozilla/5.0 real browser".parse().unwrap(),
        );

        let response =
            get_storefront_page(State(state), Path("shadow-cafe".to_string()), headers).await;
        assert_eq!(
            response.headers().get("X-Robots-Tag").unwrap(),
            "noindex, nofollow"
        );
    }

    #[tokio::test]
    async fn get_storefront_page_human_gets_no_cache_shell() {
        let state = test_state(FakeRepo::default());
        let mut headers = HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            "Mozilla/5.0 real browser".parse().unwrap(),
        );
        let response =
            get_storefront_page(State(state), Path("eljos-pizza".to_string()), headers).await;
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-cache, no-store, must-revalidate"
        );
    }

    #[tokio::test]
    async fn get_storefront_cart_page_sets_csp_header() {
        let state = test_state(FakeRepo::default());
        let response =
            get_storefront_cart_page(State(state), Path("eljos-pizza".to_string())).await;
        assert!(response.headers().get("content-security-policy").is_some());
    }
}
