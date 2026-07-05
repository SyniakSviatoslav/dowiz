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

/// JS truthiness over a jsonb value (`ssr-renderer.ts`'s `loc.address ? ...` / `loc.geo ? ...`
/// guards) — null/false/0/"" are falsy, everything else (INCLUDING `{}` and `[]`) truthy.
fn js_truthy(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Null => false,
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(true),
        serde_json::Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

/// `toMajorUnits(price, minorUnit).toFixed(minorUnit)` (`ssr-renderer.ts:102`) as EXACT integer
/// math — the price is integer minor units, so major-units-with-`minor_unit`-decimals is an
/// exact decimal string (`1500`,0 → `"1500"`; `1500`,2 → `"15.00"`), which is what `toFixed`
/// produces for every representable price. NOT float division (this is money display).
fn major_units_string(price: i64, minor_unit: i32) -> String {
    let m = u32::try_from(minor_unit.max(0)).unwrap_or(0);
    if m == 0 {
        return price.to_string();
    }
    let pow = 10u64.checked_pow(m).unwrap_or(1);
    let sign = if price < 0 { "-" } else { "" };
    let abs = price.unsigned_abs();
    format!(
        "{sign}{}.{:0width$}",
        abs / pow,
        abs % pow,
        width = usize::try_from(m).unwrap_or(0)
    )
}

/// Ports `buildHours` (`ssr-renderer.ts:160-183`) verbatim — including its live no-op quirk:
/// the day map keys are SHORT names (`mon`..`sun`) while real `hours_json` rows key on FULL
/// names (`monday`…), so on live data no day matches and the result is `[]` (exactly what Node
/// emits today — `openingHoursSpecification: []`). CARRY-VERBATIM, not a fix: SEO output must
/// not change shape under the port. One safety deviation: Node `for..of`s a non-array `periods`
/// and would THROW on short-keyed object values; this port skips them instead (unreachable on
/// real data, and a bot page must not 500 on a malformed row).
fn build_hours(hours: &serde_json::Value) -> Vec<serde_json::Value> {
    const DAY_MAP: &[(&str, &str)] = &[
        ("mon", "Monday"),
        ("tue", "Tuesday"),
        ("wed", "Wednesday"),
        ("thu", "Thursday"),
        ("fri", "Friday"),
        ("sat", "Saturday"),
        ("sun", "Sunday"),
    ];
    let mut result = Vec::new();
    let Some(obj) = hours.as_object() else {
        return result;
    };
    for (short_day, periods) in obj {
        let lower = short_day.to_lowercase();
        let Some((_, day_name)) = DAY_MAP.iter().find(|(k, _)| *k == lower) else {
            continue;
        };
        let Some(arr) = periods.as_array() else {
            continue;
        };
        for p in arr {
            result.push(serde_json::json!({
                "@type": "OpeningHoursSpecification",
                "dayOfWeek": day_name,
                "opens": p.get("open"),
                "closes": p.get("close"),
            }));
        }
    }
    result
}

/// Ports `buildJsonLd` (`ssr-renderer.ts:66-158`) in FULL — Restaurant (servesCuisine /
/// priceRange / address / geo / openingHoursSpecification, each under the source's JS
/// truthiness guards), Menu (description + `hasMenuItem` sliced to 30, price as the
/// `toFixed(minorUnit)` STRING, the `EUR-else-ALL` priceCurrency quirk, description omitted
/// when empty), BreadcrumbList, and FAQPage when the FULL (pre-slice) item count exceeds 2.
/// The trimmed 2-entity emission this replaces dropped 2 entities + 5 Restaurant fields and
/// emitted 49 numeric prices — 62 leaf diffs on the live staging oracle (2026-07-05).
///
/// `info` carries address/geo/hours (the menu payload has no location detail); `None` (info
/// read failed) degrades to the guard-fails branch of each field — same as a Node tenant with
/// null address/geo/hours.
fn build_json_ld(
    menu: &PublicMenu,
    info: Option<&crate::repo::LocationInfoRow>,
    slug: &str,
    base_url: &str,
) -> serde_json::Value {
    let menu_url = format!("{base_url}/s/{slug}");
    let name = &menu.location_name;

    let mut restaurant = serde_json::Map::new();
    restaurant.insert("@context".into(), "https://schema.org".into());
    restaurant.insert("@type".into(), "Restaurant".into());
    restaurant.insert("name".into(), name.as_str().into());
    restaurant.insert("url".into(), menu_url.as_str().into());
    restaurant.insert("servesCuisine".into(), "Albanian".into());
    restaurant.insert("priceRange".into(), "€".into());
    if let Some(addr) = info
        .and_then(|i| i.address.as_deref())
        .filter(|a| !a.is_empty())
    {
        restaurant.insert(
            "address".into(),
            serde_json::json!({"@type": "PostalAddress", "streetAddress": addr}),
        );
    }
    if let Some(geo) = info.and_then(|i| i.geo.as_ref()).filter(|g| js_truthy(g)) {
        // JSON.stringify semantics: a MISSING key is dropped (undefined), a PRESENT key is
        // kept even when null — so `{}` emits a bare GeoCoordinates (today's live output).
        let mut g = serde_json::Map::new();
        g.insert("@type".into(), "GeoCoordinates".into());
        if let Some(lat) = geo.get("lat") {
            g.insert("latitude".into(), lat.clone());
        }
        if let Some(lng) = geo.get("lng") {
            g.insert("longitude".into(), lng.clone());
        }
        restaurant.insert("geo".into(), serde_json::Value::Object(g));
    }
    if let Some(hours) = info
        .and_then(|i| i.hours_json.as_ref())
        .filter(|h| js_truthy(h))
    {
        restaurant.insert(
            "openingHoursSpecification".into(),
            serde_json::Value::Array(build_hours(hours)),
        );
    }

    let minor_unit = menu.currency.minor_unit;
    let price_currency = if menu.currency.code == "EUR" {
        "EUR"
    } else {
        "ALL"
    };
    let items: Vec<serde_json::Value> = menu
        .categories
        .iter()
        .flat_map(|c| &c.products)
        .map(|p| {
            let mut item = serde_json::Map::new();
            item.insert("@type".into(), "MenuItem".into());
            item.insert(
                "name".into(),
                if p.name.is_empty() { "Item" } else { &p.name }.into(),
            );
            if let Some(desc) = p.description.as_deref().filter(|d| !d.is_empty()) {
                item.insert("description".into(), desc.into());
            }
            item.insert(
                "offers".into(),
                serde_json::json!({
                    "@type": "Offer",
                    "price": major_units_string(p.price, minor_unit),
                    "priceCurrency": price_currency,
                }),
            );
            serde_json::Value::Object(item)
        })
        .collect();
    let full_item_count = items.len();

    let menu_entity = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "Menu",
        "name": format!("{name} Menu"),
        "description": format!("Menu for {name}"),
        "hasMenuItem": items.into_iter().take(30).collect::<Vec<_>>(),
    });

    let breadcrumbs = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "BreadcrumbList",
        "itemListElement": [
            {"@type": "ListItem", "position": 1, "name": "Home", "item": base_url},
            {"@type": "ListItem", "position": 2, "name": name, "item": menu_url},
        ],
    });

    let mut parts = vec![
        serde_json::Value::Object(restaurant),
        menu_entity,
        breadcrumbs,
    ];
    if full_item_count > 2 {
        parts.push(serde_json::json!({
            "@context": "https://schema.org",
            "@type": "FAQPage",
            "mainEntity": [
                {
                    "@type": "Question",
                    "name": format!("What are the delivery hours for {name}?"),
                    "acceptedAnswer": {
                        "@type": "Answer",
                        "text": format!("Check the {name} menu page for current operating hours and delivery availability."),
                    },
                },
                {
                    "@type": "Question",
                    "name": format!("What payment methods does {name} accept?"),
                    "acceptedAnswer": {
                        "@type": "Answer",
                        "text": format!("{name} accepts cash on delivery."),
                    },
                },
            ],
        }));
    }
    serde_json::Value::Array(parts)
}

/// The `</script>`-breakout escape (`ssr-renderer.ts:152-157`): `<`, `>`, `&` → `\uXXXX` inside
/// the serialized JSON (still valid JSON, parses back to the same chars).
fn escape_json_ld(serialized: &str) -> String {
    serialized
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

fn bot_menu_html(
    menu: &PublicMenu,
    info: Option<&crate::repo::LocationInfoRow>,
    slug: &str,
    base_url: &str,
) -> String {
    let title = format!("{} — Order Online | Dowiz", menu.location_name);
    let json_ld = escape_json_ld(&build_json_ld(menu, info, slug, base_url).to_string());
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
                // Second read for the location detail (address/geo/hours) the menu payload
                // doesn't carry — Node gets it in one `read_public_menu_all_locales` call; the
                // OUTPUT is what must match, and a failed info read degrades to the same
                // emission as a tenant with null address/geo/hours (never a 5xx bot page).
                let info = state.repo.location_info(&slug).await.ok().flatten();
                let body = bot_menu_html(&menu, info.as_ref(), &slug, &state.app_base_url);
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
    use uuid::Uuid;

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

    fn ld_menu(products: usize) -> PublicMenu {
        serde_json::from_value(serde_json::json!({
            "menu_version": 1,
            "location_id": "00000000-0000-0000-0000-000000000000",
            "locationId": "00000000-0000-0000-0000-000000000000",
            "location_name": "Eljo's Pizza",
            "default_locale": "sq",
            "supported_locales": ["sq"],
            "currency": {"code": "ALL", "minor_unit": 0},
            "categories": [{
                "id": "00000000-0000-0000-0000-000000000000",
                "name": "Pizza",
                "sort_order": 0,
                "products": (0..products).map(|i| serde_json::json!({
                    "id": "00000000-0000-0000-0000-000000000000",
                    "name": format!("P{i}"),
                    "description": if i == 0 { serde_json::Value::String(String::new()) } else { serde_json::Value::String(format!("desc {i}")) },
                    "price": 1500,
                    "available": true,
                    "image_key": null,
                    "primary_media_id": null,
                    "imageUrl": null,
                    "attributes": null,
                    "prep_time_minutes": null,
                    "modifier_groups": [],
                })).collect::<Vec<_>>(),
            }],
        }))
        .unwrap()
    }

    fn ld_info(geo: Option<serde_json::Value>) -> crate::repo::LocationInfoRow {
        crate::repo::LocationInfoRow {
            id: Uuid::nil(),
            name: "Eljo's Pizza".to_string(),
            slug: "eljos".to_string(),
            currency_code: "ALL".to_string(),
            currency_minor_unit: 0,
            default_locale: "sq".to_string(),
            lat: None,
            lng: None,
            delivery_paused: false,
            hours_json: Some(serde_json::json!({"monday": {"open": "09:00", "close": "22:00"}})),
            geo,
            address: Some("Rruga 1, Tirana".to_string()),
            phone: None,
            kitchen_busy_until: None,
            delivery_fee_flat: None,
            free_delivery_threshold: None,
            min_order_value: None,
            tax_rate: None,
            price_includes_tax: false,
            has_distance_tiers: false,
            google_rating: None,
            google_review_count: None,
            google_maps_url: None,
            google_place_id: None,
            social_instagram: None,
            social_facebook: None,
        }
    }

    /// The full ssr-renderer parity contract (staging oracle 2026-07-05, 62-leaf-diff catch):
    /// 4 entities in source order, Restaurant carries the constants + address + the empty-`{}`
    /// geo quirk + present-but-empty hours (full-name keys never match the short-key day map),
    /// prices are `toFixed` STRINGS with the EUR-else-ALL quirk, empty description omitted.
    #[test]
    fn build_json_ld_matches_node_ssr_renderer_shape() {
        let menu = ld_menu(4);
        let info = ld_info(Some(serde_json::json!({})));
        let ld = build_json_ld(&menu, Some(&info), "eljos", "https://dowiz.fly.dev");
        let parts = ld.as_array().unwrap();
        let types: Vec<_> = parts.iter().map(|p| p["@type"].as_str().unwrap()).collect();
        assert_eq!(types, ["Restaurant", "Menu", "BreadcrumbList", "FAQPage"]);

        let rest = &parts[0];
        assert_eq!(rest["servesCuisine"], "Albanian");
        assert_eq!(rest["priceRange"], "€");
        assert_eq!(rest["address"]["streetAddress"], "Rruga 1, Tirana");
        assert_eq!(
            rest["geo"],
            serde_json::json!({"@type": "GeoCoordinates"}),
            "empty geo `{{}}` is truthy in JS → bare GeoCoordinates, the live Node emission"
        );
        assert_eq!(
            rest["openingHoursSpecification"],
            serde_json::json!([]),
            "full-name hours keys never match the short-key day map — [] like live Node"
        );

        let menu_e = &parts[1];
        assert_eq!(menu_e["description"], "Menu for Eljo's Pizza");
        let items = menu_e["hasMenuItem"].as_array().unwrap();
        assert_eq!(
            items[0]["offers"]["price"], "1500",
            "price is a toFixed STRING"
        );
        assert_eq!(items[0]["offers"]["priceCurrency"], "ALL");
        assert!(
            items[0].get("description").is_none(),
            "empty description omitted (`|| undefined`)"
        );
        assert_eq!(items[1]["description"], "desc 1");
    }

    #[test]
    fn build_json_ld_slices_items_to_30_and_gates_faq_on_full_count() {
        let menu = ld_menu(49);
        let ld = build_json_ld(&menu, None, "eljos", "https://dowiz.fly.dev");
        let parts = ld.as_array().unwrap();
        assert_eq!(parts[1]["hasMenuItem"].as_array().unwrap().len(), 30);
        assert_eq!(
            parts.len(),
            4,
            "FAQPage present (49 > 2, FULL pre-slice count)"
        );

        let two = build_json_ld(&ld_menu(2), None, "eljos", "https://dowiz.fly.dev");
        assert_eq!(
            two.as_array().unwrap().len(),
            3,
            "no FAQPage at exactly 2 items"
        );
    }

    #[test]
    fn build_json_ld_none_info_omits_address_geo_hours() {
        let ld = build_json_ld(&ld_menu(1), None, "eljos", "https://dowiz.fly.dev");
        let rest = &ld.as_array().unwrap()[0];
        assert!(rest.get("address").is_none());
        assert!(rest.get("geo").is_none());
        assert!(rest.get("openingHoursSpecification").is_none());
    }

    #[test]
    fn major_units_string_is_exact_integer_math() {
        assert_eq!(major_units_string(1500, 0), "1500");
        assert_eq!(major_units_string(1500, 2), "15.00");
        assert_eq!(major_units_string(5, 2), "0.05");
        assert_eq!(major_units_string(-1234, 2), "-12.34");
        assert_eq!(major_units_string(0, 2), "0.00");
    }

    #[test]
    fn escape_json_ld_blocks_script_breakout() {
        let escaped = escape_json_ld(r#"{"x":"</script><b>&"}"#);
        assert!(!escaped.contains("</script>"));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&escaped).unwrap()["x"],
            "</script><b>&",
            "escapes stay valid JSON parsing back to the same chars"
        );
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
