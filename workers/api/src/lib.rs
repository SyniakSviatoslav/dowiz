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
mod platform_store;
mod rail;
mod idempotency;
mod identity_store;
mod courier;
mod hubdo;
mod command;
mod hubstore;
mod otel;
mod outbox;
mod owner;
mod assist;
mod storefront;
mod stripe;
mod project;
mod notify;
mod channels;
mod cloud;
mod mcp;
mod integrations;
mod catalog_edit;
mod rebuild;
mod recipe;
mod services;
mod waitlist;
mod witness;
mod errlog;
mod gauges;
mod quarantine;
mod fold;
mod live;

use worker::wasm_bindgen::{JsCast, JsValue};
use worker::*;



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
        // THE INSTRUMENT FINALLY FIRES.
        //
        // `worker_errors` had never received a row and `sqlite_sequence` was
        // how we knew. The reason was not that nothing failed: eight of the ten
        // `loud!` sites are unreachable by configuration, and what ACTUALLY
        // fails -- every `?` in a handler, which becomes a bodyless 500 and a
        // sampled console line -- was not instrumented at all. At
        // `head_sampling_rate = 0.1` nine out of ten of those were never
        // written down anywhere.
        //
        // This is the one place every one of them passes through. It records
        // the route and the trace id, so the response the person quotes leads
        // to the failure, which is the join the black box needed.
        //
        // NO VENUE. A 500 out of `route` has already lost whatever context knew
        // which venue it was for, and guessing one would put a failure in
        // another restaurant's console. It lands platform-wide, which is where
        // an unattributed failure honestly belongs.
        if let Ok(ns) = env.durable_object("HUB") {
            crate::errlog::record(
                &ns,
                None,
                "worker.500",
                &format!("{method} {path} [trace {}]: {e}", trace.id()),
            )
            .await;
        }
    }
    // THE ID GOES BACK TO WHOEVER ASKED. It is the join key between this
    // request's spans, the failures it caused and -- once the log carries it --
    // the order it placed. Set here rather than in `harden` because this is the
    // only scope that holds the trace, and set on SUCCESS AND FAILURE alike:
    // the response a person quotes when they report a problem is usually the
    // one that went wrong.
    let out = out.map(|mut r| {
        let _ = r.headers_mut().set("x-trace-id", trace.id());
        r
    });
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

/// WHAT THE REQUEST KNOWS BEFORE ANY HANDLER RUNS.
///
/// THE CLOCK IS READ ONCE, HERE, AND TRAVELS AS DATA. Ninety-two places in this
/// crate used to ask the wall clock what time it was, and two shipped defects
/// came straight out of that: `486a5c38`, where analytics days were 24 hours
/// apart and a venue's are not, and the summer time-zone constant in
/// `dowiz-venue-timezone`. A handler that reads the clock itself cannot be TOLD
/// what time it is, so the only way to test a day boundary, a promo window or
/// an expiry was to wait for one -- which is why none of them had a test until
/// the pure part was lifted out.
///
/// IT RIDES ON `Router::with_data` rather than in a new argument, because the
/// handler signature belongs to `workers-rs` and every one of the 119 routes
/// would otherwise have to change shape rather than change a type parameter.
/// `ctx.data.now_ms` is the whole of the injection.
///
/// ONE INSTANT FOR THE WHOLE REQUEST is the property, not just the saving.
/// Three reads inside one handler are three answers to one question, and the
/// two that disagree are the ones that put an order in the wrong day.
#[derive(Clone, Copy)]
pub struct Req {
    pub now_ms: i64,
}

pub(crate) async fn route(req: Request, env: Env) -> Result<Response> {
    // Before the router, because this is about the HOST and not the path.
    if let Ok(u) = req.url() {
        if u.path() == "/" || u.path() == "/index.html" {
            return serve_root(&req, &env).await;
        }
    }
    // THE ONE CLOCK READ ON THE REQUEST PATH. `tools/gates/clock.sh` allows
    // this line by name; everything below is handed the answer.
    Router::with_data(Req { now_ms: Date::now().as_millis() as i64 })
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
        // ── THE FLOOR: which tables are free FOR A SLOT ──
        //
        // PUBLIC, DELIBERATELY. It is the venue's own furniture and the "is
        // there room tonight" a phone call already answers; it carries no name,
        // no party and no reservation id. It REFUSES without `slotMin`,
        // because a table is free or taken only for a slot and a plan drawn
        // without one is a picture of a lie.
        .get_async("/api/public/locations/:slug/tables", booking::availability)
        // ── threads: the transport for `dowiz_kernel::thread` ──
        .get_async("/api/public/locations/:slug/threads/:id", social::messages)
        .post_async("/api/public/locations/:slug/threads/:id/messages", social::send)
        // ── the wallet journal: `dowiz_kernel::ledger_account` ──
        .get_async("/api/public/locations/:slug/wallet", wallet::balance)
        .get_async("/api/public/locations/:slug/wallet/statement", wallet::statement)
        .post_async("/api/public/locations/:slug/wallet/topup", wallet::top_up)
        // ── the delivery estimate: `dowiz_kernel::eta` ──
        .post_async("/api/public/locations/:slug/eta", eta::quote)
        .post_async("/api/promo/check", services::ordering::preview::promo_check)
        .get_async("/api/public/reach", services::venue::zones::reach)
        .get_async("/api/public/rates", services::ordering::rates::rates)
        .post_async("/api/voice", services::engagement::voice::voice)
        .post_async("/api/owner/zones", services::venue::zones::set_zones)
        // The room as DATA an owner can edit, beside the delivery zones it
        // sits next to in the catalogue. Refused on write when it does not
        // parse back, naming the zone, the table and the rule.
        .post_async("/api/owner/floorplan", booking::set_plan)
        .post_async("/api/owner/branding/extract", services::venue::brand_extract::extract_branding)
        .post_async("/api/order/:id/feedback", services::orders::feedback::feedback)
        // ── accounts ──
        .post_async("/api/bootstrap", bootstrap::seed)
        // The waiting list: the landing page's one form. Public to write,
        // administrators only to read -- see `waitlist`.
        .post_async("/api/waitlist", waitlist::join)
        .get_async("/api/platform/waitlist", waitlist::list)
        // The main hub. Platform administrators only -- see `platform`.
        .get_async("/api/platform/errors", platform::errors)
        .get_async("/api/platform/hubs", platform::hubs)
        .post_async("/api/platform/hubs", platform::create_hub)
        .post_async("/api/webhooks/stripe", stripe::webhook)
        .post_async("/api/auth/login", accounts::owner_login)
        .post_async("/api/auth/refresh", accounts::owner_refresh)
        .post_async("/api/auth/logout", accounts::owner_logout)
        .post_async("/api/courier/auth/login", accounts::courier_login)
        .post_async("/api/courier/auth/claim", accounts::courier_claim)
        // THE ROOM (docs/design/BLUEPRINT-POS-THE-ROOM-2026-09-22.md).
        .post_async("/api/staff/login", services::identity::staff::staff_login)
        .post_async("/api/staff/claim", services::identity::staff::staff_claim)
        .get_async("/api/owner/staff", services::identity::staff_admin::list_staff)
        .post_async("/api/owner/staff/invite", services::identity::staff_admin::invite_staff)
        .post_async("/api/owner/staff/:id", services::identity::staff_admin::set_staff)
        .get_async("/api/staff/room", services::orders::room::handlers::room_view)
        .post_async("/api/staff/orders/:id/amend", services::orders::room::handlers::amend)
        .post_async("/api/staff/orders/:id/pay", services::orders::room::pay::pay)
        // ── owner ──
        .get_async("/api/owner/orders", owner::orders)
        .post_async("/api/owner/orders/:id/action", owner::order_action)
        .post_async("/api/owner/orders/:id/assign", owner::assign_courier)
        .get_async("/api/owner/couriers/:id", services::courier::console::courier_detail)
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
        .get_async("/api/owner/analytics", services::analytics::analytics)
        .get_async("/api/owner/promotions", services::ordering::promotions::promotions)
        .post_async("/api/owner/promotions", services::ordering::promotions::set_promotion)
        .post_async("/api/owner/promotions/:code/delete", services::ordering::promotions::delete_promotion)
        .get_async("/api/owner/activation", services::venue::activation::activation)
        .get_async("/api/owner/branding", services::venue::brand::branding)
        .post_async("/api/owner/branding", services::venue::brand::set_branding)
        .post_async("/api/owner/branding/preset", services::venue::brand::set_preset)
        .get_async("/api/owner/customers", services::customers::handlers::customers)
        .post_async("/api/owner/customers/:key/reveal", services::customers::handlers::reveal_customer)
        .get_async("/api/owner/customers/reveals", services::customers::handlers::reveals)
        .put_async("/api/owner/customers/:key/record", services::customers::record_routes::put_record)
        .post_async("/api/owner/customers/rekey", services::customers::record_routes::rekey)
        .post_async("/api/owner/customers/:key/consent", services::customers::consent_routes::owner_act)
        .get_async("/api/public/consent/wordings", services::customers::consent_routes::wordings)
        .get_async("/api/owner/stock", services::operations::stock::stock)
        .post_async("/api/owner/stock/:kind", services::operations::stock::stock_move)
        .post_async("/api/owner/supplies", services::operations::supplies::set_supply)
        .post_async("/api/owner/supplies/:id/retire", services::operations::supplies::retire_supply)
        .get_async("/api/owner/features", services::venue::settings::features)
        .post_async("/api/owner/features", services::venue::settings::set_feature)
        .get_async("/api/owner/settings", services::venue::settings::settings)
        .post_async("/api/owner/settings", services::venue::settings::set_setting)
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
        .post_async("/api/owner/menu/import", services::catalogue::import::import_menu)
        .get_async("/api/owner/couriers", services::courier::console::couriers)
        .post_async("/api/owner/couriers/invite", services::courier::hiring::invite_courier)
        .post_async("/api/owner/couriers/:id/uninvite", services::courier::hiring::uninvite_courier)
        .post_async("/api/owner/couriers/:id/active", services::courier::hiring::set_courier_active)
        .get_async("/api/owner/posts", services::engagement::posts::posts)
        .post_async("/api/owner/posts/draft", services::engagement::posts::draft_post)
        .post_async("/api/owner/posts/:id/approve", services::engagement::verdict::approve_post)
        .post_async("/api/owner/posts/:id/reject", services::engagement::verdict::reject_post)
        .get_async("/api/owner/graph", services::engagement::assist::graph)
        .get_async("/api/live", live::connect)
        .get_async("/api/owner/health", services::operations::health)
        .get_async("/api/owner/history", services::operations::history)
        .post_async("/api/owner/hub/rotate", services::operations::rotate_now)
        .get_async("/api/owner/backup", services::operations::backup)
        .post_async("/api/owner/restore", services::operations::restore)
        .post_async("/api/owner/assist", services::engagement::assist::owner_assist)
        .post_async("/api/courier/assist", services::engagement::assist::courier_assist)
        .get_async("/api/owner/apikeys", services::identity::keys::list_api_keys)
        .post_async("/api/owner/apikeys", services::identity::keys::create_api_key)
        .post_async("/api/owner/apikeys/revoke", services::identity::keys::revoke_api_key)
        .post_async("/api/owner/products/:id/image", services::catalogue::media::set_product_image)
        .post_async("/api/owner/products/:id/image/clear", services::catalogue::media::clear_product_image)
        // The venue's own mark, stored the way its dishes' photographs are.
        .post_async("/api/owner/place", services::venue::place::set_place)
        .post_async("/api/owner/logo", services::catalogue::media::set_venue_logo)
        .post_async("/api/owner/logo/clear", services::catalogue::media::clear_venue_logo)
        .get_async("/media/:name", services::catalogue::media::media)
        // ── courier ──
        .get_async("/api/courier/tasks", courier::tasks)
        .post_async("/api/courier/shift", courier::shift)
        .post_async("/api/courier/orders/:id/accept", courier::accept)
        .post_async("/api/courier/orders/:id/pickup", courier::pickup)
        .post_async("/api/courier/orders/:id/deliver", courier::deliver)
        .post_async("/api/courier/position", courier::position)
        .get_async("/api/courier/earnings", courier::earnings)
        .get_async("/api/courier/history", services::courier::history::courier_history)
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
            let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
            let Some(order_json) = hubstore::order(&place, &id).await? else {
                return Response::error("order not found", 404);
            };
            let envelope: serde_json::Value =
                serde_json::from_str(&order_json).unwrap_or(serde_json::json!({}));

            let allowed = match auth::authenticate(&req, &ctx.env, ctx.data.now_ms).await {
                Ok(auth::Principal::Customer { order_id, .. }) => order_id == id,
                // AN OWNER OF THIS VENUE, not an owner of any venue.
                //
                // `authenticate`'s owner check asks "is this user an owner
                // somewhere", because that is all a role needs. Here the
                // question is about a VENUE: a platform-admin token carries no
                // location at all and this route used to answer `true` for it,
                // so it read any venue's orders -- name, phone and address --
                // on the venue's own host, which `accounts.rs` states in as
                // many words that it cannot do. The claim has to name THIS
                // hub.
                Ok(auth::Principal::Owner { active_location_id, .. }) => {
                    active_location_id.as_deref() == Some(place.venue.as_str())
                }
                Ok(auth::Principal::Courier { courier_id, .. }) => {
                    envelope.get("courier_id").and_then(|c| c.as_str()) == Some(courier_id.as_str())
                }
                // Staff of THIS venue who may move or take orders read them.
                Ok(auth::Principal::Staff { active_location_id, caps, .. }) => {
                    active_location_id == place.venue
                        && (caps.allows(auth::Cap::Advance) || caps.allows(auth::Cap::TakeOrders))
                }
                Err(_) => false,
            };
            if !allowed {
                return Response::error("this order needs the link you were given", 401);
            }
            // The order as it stands NOW: the time that is left rides with it.
            let mut live = envelope.clone();
            live_eta::attach_one(&place, &mut live, ctx.data.now_ms).await;
            let mut res = Response::ok(serde_json::to_string(&live).unwrap_or(order_json))?;
            res.headers_mut().set("content-type", "application/json; charset=utf-8")?;
            // Never cached by anything between here and the browser: it holds
            // an address.
            res.headers_mut().set("cache-control", "private, no-store")?;
            Ok(res)
        })
        // ── TWO LEGACY WRITE ROUTES, DELETED 2026-09-21 ──
        //
        // `POST /api/order` and `POST /api/order/:id/advance` took NO
        // authentication of any kind. Anyone who knew a venue's host could
        // append orders to its log, and anyone who had an order id -- which is
        // in a URL, a browser history, a shared tracking link -- could walk
        // that order to CANCELLED, or to DELIVERED, past the courier's cash
        // handover. A red-team pass found them; nothing in `public/` had
        // called either since the real routes existed.
        //
        // What replaces them, and always did: placement is
        // `POST /api/public/locations/:slug/orders` (`storefront::place`),
        // which re-derives every price from the catalogue, validates the
        // address, reserves the stock and mints the customer's key; transitions
        // are `owner::order_action` (owner token, membership scoped to the
        // venue, a whitelist of five actions) and the courier routes (an
        // assignment row the courier must own). A duplicate write path with
        // weaker authentication is not a convenience, it is the hole.
        //
        // `scripts/smoke.sh` used them and now speaks the authenticated ones.
        .run(req, env)
        .await
}

/// The cron in wrangler.toml (`cloud::NIGHTLY_CRON`): every venue with a
/// bucket gets its nightly copy.
#[event(scheduled)]
pub async fn scheduled(event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    // TWO CRONS, AND THE EXPRESSION SAYS WHICH. The nightly is a long job over
    // every venue; the minute one exists to deliver a message a kitchen is
    // waiting for, and running the nightly every minute would be absurd.
    // ONE CLOCK READ PER INVOCATION, for the reason `Req` gives on the fetch
    // path: a job whose parts each ask the wall clock has as many answers as it
    // has parts. `tools/gates/clock.sh` allows this line and the router's, and
    // nothing else on either entry point.
    let now_ms = Date::now().as_millis() as i64;
    if event.cron().starts_with("17 3") {
        cloud::nightly(&env, now_ms).await;
    } else {
        outbox::sweep(&env, now_ms).await;
    }
}
