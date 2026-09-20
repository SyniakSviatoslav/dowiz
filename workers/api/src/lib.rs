//! dowiz API Worker — transport and storage ONLY.
//!
//! Every order decision is made by `dowiz_kernel::json_api`. This module never
//! re-implements the FSM, a price, or a total: it moves bytes between HTTP, D1
//! and the kernel. That is the same rule `web/src/app.js` currently breaks with
//! its hand-rolled status chain, and the reason the kernel is linked directly
//! rather than reached over a network hop.
//!
//! Identity and the clock come from the EDGE, never from the kernel — see
//! `place_order_at`. A process-local counter is unsound here by construction:
//! Workers recycles isolates constantly and runs many at once.

mod booking;
mod eta;
mod live_eta;
mod social;
mod wallet;
mod accounts;
mod auth;
mod bootstrap;
mod platform;
mod courier;
mod hubdo;
mod hubstore;
mod otel;
mod owner;
mod assist;
mod extra;
mod storefront;
mod stripe;
mod project;
mod notify;
mod channels;
mod cloud;
mod mcp;
mod integrations;
mod catalog_edit;
mod recipe;
mod waitlist;
mod errlog;

use dowiz_kernel::json_api;
use serde::Deserialize;
use worker::wasm_bindgen::{JsCast, JsValue};
use worker::*;

#[derive(Deserialize)]
struct PlaceOrderBody {
    #[serde(default)]
    customer_id: Option<String>,
    items_json: String,
    #[serde(default)]
    channel: Option<String>,
}

#[derive(Deserialize)]
struct AdvanceBody {
    next_status: String,
}

/// A CSPRNG-backed order id from the platform's Web Crypto.
///
/// FAIL-CLOSED on purpose: if `crypto.randomUUID` is not reachable we refuse the
/// request rather than fall back to a weaker source. An order id that can repeat
/// is a primary-key collision between two customers, which is exactly the defect
/// the kernel's old `AtomicU64` counter produced once it left a single process.
pub fn edge_id() -> Option<String> {
    let global = js_sys::global();
    let crypto = js_sys::Reflect::get(&global, &JsValue::from_str("crypto")).ok()?;
    let f = js_sys::Reflect::get(&crypto, &JsValue::from_str("randomUUID")).ok()?;
    let f = f.dyn_ref::<js_sys::Function>()?;
    f.call0(&crypto).ok()?.as_string()
}

/// Pull `status` out of a kernel-serialized order so it can live in its own
/// column. The kernel's JSON stays the single source of truth in `order_json`;
/// this is a projection for querying, never a second authority.
fn status_of(order_json: &str) -> Result<String> {
    let v: serde_json::Value = serde_json::from_str(order_json)
        .map_err(|e| Error::RustError(format!("kernel order json unreadable: {e}")))?;
    v.get("status")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| Error::RustError("kernel order json has no status".into()))
}

/// The kernel answers `Err(String)` for malformed input and illegal transitions
/// alike; both are client errors at this boundary, never a 500.
fn kernel_reject(msg: String) -> Result<Response> {
    Response::error(msg, 400)
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
    // One span per request, continuing an incoming W3C traceparent if there is
    // one. The export happens in waitUntil AFTER the response is returned, so
    // tracing costs the customer nothing in latency.
    let method = req.method().to_string();
    let path = req.path();
    let mut trace = otel::Trace::begin(&req, &format!("{method} {path}"));
    trace.attr(0, "http.request.method", serde_json::json!(method));
    trace.attr(0, "url.path", serde_json::json!(path));

    let out = route(req, env.clone()).await.map(harden);
    let status = match &out {
        Ok(r) => r.status_code(),
        Err(_) => 500,
    };
    if let Err(e) = &out {
        trace.fail(0, &e.to_string());
    }
    ctx.wait_until(async move { trace.export(&env, status).await });
    out
}

/// The headers every API response carries, set in ONE place.
///
/// Per-handler headers were never going to hold: there are 223 places in this
/// crate that build an error response, and a policy that has to be remembered
/// 223 times is a policy with holes in it. The static surfaces get theirs from
/// `public/_headers`, because `[assets]` answers those without ever calling
/// this function.
///
/// A header already set by the handler WINS. `/media` sets a year of immutable
/// caching and `/api/order/:id` sets `no-store`; overwriting either from here
/// would be this function quietly undoing a decision made where the content was
/// actually known.
fn harden(mut res: Response) -> Response {
    let h = res.headers_mut();
    for (k, v) in [
        ("strict-transport-security", "max-age=31536000; includeSubDomains"),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "strict-origin-when-cross-origin"),
        // An API response is JSON, never a document. No script can run from it,
        // so the policy only has to say that nothing may be loaded at all.
        ("content-security-policy", "default-src 'none'; frame-ancestors 'none'"),
        // Every API answer is specific to who asked. A shared cache holding one
        // would hand an owner's dashboard to the next person through the same
        // proxy; handlers that know better set their own and keep it.
        ("cache-control", "private, no-store"),
    ] {
        if h.get(k).ok().flatten().is_none() {
            let _ = h.set(k, v);
        }
    }
    res
}

/// Which page the ROOT of a host is.
///
/// One Worker serves two different front doors and they share a path. On a
/// client host, `sushi-durres.dowiz.org/` is that venue's storefront. On the
/// platform's own host, `dowiz.org/` is the main hub -- the console where a
/// client's hub is created -- and a storefront there would be a shop with no
/// venue behind it, which is why the apex used to resolve to the `demo` slug
/// and show nothing.
///
/// `run_worker_first = ["/"]` in `wrangler.toml` is what lets this run at all:
/// assets otherwise answer `/` before the Worker is reached, and the decision
/// needs the Host header. Only `/` is taken back -- every other asset is still
/// served directly, so the storefront's own JS, CSS and images cost nothing.
async fn serve_root(req: &Request, env: &Env) -> Result<Response> {
    let host = req
        .headers()
        .get("host")
        .ok()
        .flatten()
        .unwrap_or_default()
        .split(':')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let platform = env
        .var("PLATFORM_HOST")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "dowiz.org".to_string());
    // NEITHER OF THESE MAY BE `/index.html`, AND THAT IS NOT A STYLE CHOICE.
    // Cloudflare's asset layer normalises `/index.html` to `/`, `/` is
    // `run_worker_first`, and so asking assets for the storefront re-entered
    // this function: the Worker fetched itself until the edge cut it off with
    // error 1042 and the storefront root served nothing at all. Both documents
    // therefore live one directory down, where the normalised form (`/store/`,
    // `/platform/`) is a path the Worker does not take back.
    let page = if host == platform || host == format!("www.{platform}") {
        "/platform/index.html"
    } else {
        "/store/index.html"
    };
    let mut url = req.url()?;
    url.set_path(page);
    env.assets("ASSETS")?.fetch(url.to_string(), None).await
}

pub(crate) async fn route(req: Request, env: Env) -> Result<Response> {
    // Before the router, because this is about the HOST and not the path.
    if let Ok(u) = req.url() {
        if u.path() == "/" || u.path() == "/index.html" {
            return serve_root(&req, &env).await;
        }
    }
    Router::new()
        .get("/healthz", |_, _| Response::ok("ok"))
        // ── public storefront ──
        .get_async("/api/public/locations/:slug/menu", storefront::menu)
        .get_async("/manifest.webmanifest", storefront::manifest)
        .post_async("/api/public/locations/:slug/orders", storefront::place)
        // ── reservations: the transport for `dowiz_kernel::reservation` ──
        .get_async("/api/public/locations/:slug/reservations", booking::list)
        .post_async("/api/public/locations/:slug/reservations", booking::create)
        .get_async("/api/public/locations/:slug/reservations/:id", booking::detail)
        .post_async("/api/public/locations/:slug/reservations/:id/action", booking::action)
        .get_async("/api/public/locations/:slug/reservations/:id/pass", booking::issue_pass)
        .post_async("/api/public/locations/:slug/pass/verify", booking::verify_pass)
        // ── threads: the transport for `dowiz_kernel::thread` ──
        .get_async("/api/public/locations/:slug/threads/:id", social::messages)
        .post_async("/api/public/locations/:slug/threads/:id/messages", social::send)
        // ── the wallet journal: `dowiz_kernel::ledger_account` ──
        .get_async("/api/public/locations/:slug/wallet", wallet::balance)
        .get_async("/api/public/locations/:slug/wallet/statement", wallet::statement)
        .post_async("/api/public/locations/:slug/wallet/topup", wallet::top_up)
        // ── the delivery estimate: `dowiz_kernel::eta` ──
        .post_async("/api/public/locations/:slug/eta", eta::quote)
        .post_async("/api/promo/check", extra::promo_check)
        .get_async("/api/public/reach", extra::reach)
        .get_async("/api/public/rates", extra::rates)
        .post_async("/api/voice", extra::voice)
        .post_async("/api/owner/zones", extra::set_zones)
        .post_async("/api/owner/branding/extract", extra::extract_branding)
        .post_async("/api/order/:id/feedback", extra::feedback)
        // ── accounts ──
        .post_async("/api/bootstrap", bootstrap::seed)
        // The waiting list: the landing page's one form. Public to write,
        // administrators only to read -- see `waitlist`.
        .post_async("/api/waitlist", waitlist::join)
        .get_async("/api/platform/waitlist", waitlist::list)
        // The main hub. Platform administrators only -- see `platform`.
        .get_async("/api/platform/hubs", platform::hubs)
        .post_async("/api/platform/hubs", platform::create_hub)
        .post_async("/api/webhooks/stripe", stripe::webhook)
        .post_async("/api/auth/login", accounts::owner_login)
        .post_async("/api/auth/refresh", accounts::owner_refresh)
        .post_async("/api/auth/logout", accounts::owner_logout)
        .post_async("/api/courier/auth/login", accounts::courier_login)
        .post_async("/api/courier/auth/claim", accounts::courier_claim)
        // ── owner ──
        .get_async("/api/owner/orders", owner::orders)
        .post_async("/api/owner/orders/:id/action", owner::order_action)
        .post_async("/api/owner/orders/:id/assign", owner::assign_courier)
        .get_async("/api/owner/couriers/:id", extra::courier_detail)
        .get_async("/api/owner/dashboard", owner::dashboard)
        .post_async("/api/owner/products", catalog_edit::create_product)
        .post_async("/api/owner/products/:id/delete", catalog_edit::delete_product)
        .get_async("/api/owner/categories", catalog_edit::list_categories)
        .post_async("/api/owner/categories", catalog_edit::set_category)
        .post_async("/api/owner/categories/:id/delete", catalog_edit::delete_category)
        .post_async("/api/owner/products/:id", owner::update_product)
        .post_async("/api/owner/location", owner::update_location)
        .post_async("/api/owner/i18n", owner::write_translations)
        // ── ported from the native adapter, on the SAME dowiz-hub logic ──
        .get_async("/api/owner/analytics", extra::analytics)
        .get_async("/api/owner/promotions", extra::promotions)
        .post_async("/api/owner/promotions", extra::set_promotion)
        .post_async("/api/owner/promotions/:code/delete", extra::delete_promotion)
        .get_async("/api/owner/activation", extra::activation)
        .get_async("/api/owner/branding", extra::branding)
        .post_async("/api/owner/branding", extra::set_branding)
        .post_async("/api/owner/branding/preset", extra::set_preset)
        .get_async("/api/owner/customers", extra::customers)
        .post_async("/api/owner/customers/:key/reveal", extra::reveal_customer)
        .get_async("/api/owner/customers/reveals", extra::reveals)
        .get_async("/api/owner/stock", extra::stock)
        .post_async("/api/owner/stock/:kind", extra::stock_move)
        .post_async("/api/owner/supplies", extra::set_supply)
        .post_async("/api/owner/supplies/:id/retire", extra::retire_supply)
        .get_async("/api/owner/features", extra::features)
        .post_async("/api/owner/features", extra::set_feature)
        .get_async("/api/owner/settings", extra::settings)
        .post_async("/api/owner/settings", extra::set_setting)
        .post_async("/api/owner/notify/test", notify::test)
        .get_async("/api/owner/inbox", channels::inbox)
        .get_async("/api/owner/inbox/:peer", channels::thread)
        .post_async("/api/owner/inbox/:peer", channels::reply)
        .get_async("/api/owner/backup/cloud", cloud::status)
        .post_async("/api/owner/backup/cloud", cloud::push)
        .get_async("/api/webhooks/meta", channels::webhook_verify)
        .post_async("/api/webhooks/meta", channels::webhook)
        .get_async("/api/owner/integrations", integrations::status)
        .post_async("/api/owner/integrations/check", integrations::check)
        .get_async("/api/mcp", mcp::describe)
        .post_async("/api/mcp", mcp::rpc)
        .post_async("/api/owner/menu/import", extra::import_menu)
        .get_async("/api/owner/couriers", extra::couriers)
        .post_async("/api/owner/couriers/invite", extra::invite_courier)
        .post_async("/api/owner/couriers/:id/uninvite", extra::uninvite_courier)
        .post_async("/api/owner/couriers/:id/active", extra::set_courier_active)
        .get_async("/api/owner/posts", extra::posts)
        .post_async("/api/owner/posts/draft", extra::draft_post)
        .post_async("/api/owner/posts/:id/approve", extra::approve_post)
        .post_async("/api/owner/posts/:id/reject", extra::reject_post)
        .get_async("/api/owner/graph", extra::graph)
        .get_async("/api/owner/health", extra::health)
        .get_async("/api/owner/backup", extra::backup)
        .post_async("/api/owner/restore", extra::restore)
        .post_async("/api/owner/assist", extra::owner_assist)
        .post_async("/api/courier/assist", extra::courier_assist)
        .get_async("/api/owner/apikeys", extra::list_api_keys)
        .post_async("/api/owner/apikeys", extra::create_api_key)
        .post_async("/api/owner/apikeys/revoke", extra::revoke_api_key)
        .post_async("/api/owner/products/:id/image", extra::set_product_image)
        .post_async("/api/owner/products/:id/image/clear", extra::clear_product_image)
        // The venue's own mark, stored the way its dishes' photographs are.
        .post_async("/api/owner/place", extra::set_place)
        .post_async("/api/owner/logo", extra::set_venue_logo)
        .post_async("/api/owner/logo/clear", extra::clear_venue_logo)
        .get_async("/media/:name", extra::media)
        // ── courier ──
        .get_async("/api/courier/tasks", courier::tasks)
        .post_async("/api/courier/shift", courier::shift)
        .post_async("/api/courier/orders/:id/accept", courier::accept)
        .post_async("/api/courier/orders/:id/pickup", courier::pickup)
        .post_async("/api/courier/orders/:id/deliver", courier::deliver)
        .post_async("/api/courier/position", courier::position)
        .get_async("/api/courier/earnings", courier::earnings)
        .get_async("/api/courier/history", extra::courier_history)
        .post_async("/api/order", |mut req, ctx| async move {
        let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
            let body: PlaceOrderBody = match req.json().await {
                Ok(b) => b,
                Err(e) => return Response::error(format!("bad request body: {e}"), 400),
            };
            let Some(id) = edge_id() else {
                return Response::error("no platform CSPRNG for order id", 500);
            };
            let created_at_ms = Date::now().as_millis() as i64;

            let order_json = match json_api::place_order_at(
                id.clone(),
                body.customer_id,
                &body.items_json,
                created_at_ms,
                body.channel,
            ) {
                Ok(j) => j,
                Err(e) => return kernel_reject(e),
            };
            let status = status_of(&order_json)?;


            let seq = created_at_ms as u64;
            let ev_id = id.clone();
            let ev_json = order_json.clone();
            crate::hubstore::with_hub(&crate::hubstore::Place::of_any(&req, &ctx).await?, move |hub| {
                hub.append(dowiz_hub::EventKind::Placed, &ev_id, &ev_json, seq, [0u8; 32])
                    .map_err(|e| Error::RustError(format!("hub append failed: {e:?}")))
            })
            .await?;


            let mut res = Response::ok(order_json)?;
            res.headers_mut()
                .set("content-type", "application/json; charset=utf-8")?;
            Ok(res)
        })
        .get_async("/api/order/:id", |req, ctx| async move {
            let Some(id) = ctx.param("id").cloned() else {
                return Response::error("missing order id", 400);
            };
            // ── AN ORDER IS NOT READABLE BY WHOEVER KNOWS ITS ID ──
            //
            // This route was public. An id is not a secret -- it appears in a
            // URL, a browser history, a shared screenshot -- and behind it sat
            // the customer's name, phone and street address. That is
            // capability-by-obscurity, and it was live.
            //
            // Three principals may read one order, and each is checked against
            // THIS order rather than against a role: the customer holding the
            // key minted with it, the venue's owner, and the courier whose run
            // it actually is. A courier is not entitled to every customer's
            // address in the venue.
            let db = ctx.d1("DB")?;
            let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
            let loaded = hubstore::load(&place).await?;
            let Ok(order_json) = loaded.hub.order(&id) else {
                return Response::error("order not found", 404);
            };
            let envelope: serde_json::Value =
                serde_json::from_str(&order_json).unwrap_or(serde_json::json!({}));

            let allowed = match auth::authenticate(&req, &ctx.env, &db, Date::now().as_millis() as i64).await {
                Ok(auth::Principal::Customer { order_id, .. }) => order_id == id,
                Ok(auth::Principal::Owner { .. }) => true,
                Ok(auth::Principal::Courier { courier_id, .. }) => {
                    envelope.get("courier_id").and_then(|c| c.as_str()) == Some(courier_id.as_str())
                }
                Err(_) => false,
            };
            if !allowed {
                return Response::error("this order needs the link you were given", 401);
            }
            // The order as it stands NOW: the time that is left rides with it.
            let mut live = envelope.clone();
            live_eta::attach_one(&db, &place, &loaded, &mut live, Date::now().as_millis() as i64).await;
            let mut res = Response::ok(serde_json::to_string(&live).unwrap_or(order_json))?;
            res.headers_mut().set("content-type", "application/json; charset=utf-8")?;
            // Never cached by anything between here and the browser: it holds
            // an address.
            res.headers_mut().set("cache-control", "private, no-store")?;
            Ok(res)
        })
        .post_async("/api/order/:id/advance", |mut req, ctx| async move {
            let Some(id) = ctx.param("id").cloned() else {
                return Response::error("missing order id", 400);
            };
            let body: AdvanceBody = match req.json().await {
                Ok(b) => b,
                Err(e) => return Response::error(format!("bad request body: {e}"), 400),
            };
            let db = ctx.d1("DB")?;
            let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
            let next = body.next_status.clone();

            // One read-modify-write against the hub image, replayed if another
            // writer moved it first. The kernel decides whether the edge is
            // legal; the Worker only records its answer.
            let out = hubstore::with_hub(&place, move |hub| {
                let current = hub
                    .order(&id)
                    .map_err(|_| Error::RustError("order not found".into()))?;
                let updated = json_api::apply_event_logic(&current, &next)
                    .map_err(Error::RustError)?;
                let merged = carry_envelope(&current, &updated);
                hub.append(
                    dowiz_hub::EventKind::Advanced,
                    &id,
                    &merged,
                    Date::now().as_millis() as u64,
                    [0u8; 32],
                )
                .map_err(|e| Error::RustError(format!("hub append failed: {e:?}")))?;
                Ok(merged)
            })
            .await;

            match out {
                Ok(merged) => {
                    let mut res = Response::ok(merged)?;
                    res.headers_mut()
                        .set("content-type", "application/json; charset=utf-8")?;
                    Ok(res)
                }
                // An illegal transition is the caller's mistake, so 409 -- never
                // a 500, which would blame the server for a refusal it was right
                // to make.
                Err(e) => {
                    let msg = e.to_string();
                    let code = if msg.contains("not found") { 404 } else { 409 };
                    Response::error(msg, code)
                }
            }
        })
        .run(req, env)
        .await
}

/// Carry the fields the kernel does not model across a transition.
///
/// The kernel owns items, status, subtotal and the ledger. Delivery address,
/// contact and payment ride alongside until the aggregate's new fields reach
/// this boundary, and they have to survive every advance: losing a delivery
/// address on a status change is a silent loss that only surfaces at the door.
fn carry_envelope(old_raw: &str, updated: &str) -> String {
    let Ok(mut merged) = serde_json::from_str::<serde_json::Value>(updated) else {
        return updated.to_string();
    };
    let old: serde_json::Value = serde_json::from_str(old_raw).unwrap_or(serde_json::Value::Null);
    for k in [
        "location_id", "contact", "fulfilment", "payment", "delivery_fee", "total",
        "courier_id", "rejection_reason", "at", "accepted_at_ms", "crypto", "tip",
    ] {
        if let Some(v) = old.get(k) {
            merged[k] = v.clone();
        }
    }
    serde_json::to_string(&merged).unwrap_or_else(|_| updated.to_string())
}

/// The cron in wrangler.toml (`cloud::NIGHTLY_CRON`): every venue with a
/// bucket gets its nightly copy.
#[event(scheduled)]
pub async fn scheduled(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    cloud::nightly(&env).await;
}
