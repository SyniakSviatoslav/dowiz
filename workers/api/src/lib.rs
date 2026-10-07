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

/// `log_error!` / `log_line!`: the platform's console (`worker::console_error!`/`console_log!`) on
/// wasm32, stderr natively. Worker's own macros call a wasm-bindgen import, which PANICS in a native
/// test, so no code path that logged could run under `cargo test` (W-COV C2). A crate-local NAME,
/// because a `macro_rules!` of the same name is ambiguous against `use worker::*`'s glob.
#[cfg(target_arch = "wasm32")]
macro_rules! log_error {
    ($($t:tt)*) => { worker::console_error!($($t)*) };
}
#[cfg(not(target_arch = "wasm32"))]
macro_rules! log_error {
    ($($t:tt)*) => { eprintln!($($t)*) };
}
#[cfg(target_arch = "wasm32")]
macro_rules! log_line {
    ($($t:tt)*) => { worker::console_log!($($t)*) };
}
#[cfg(not(target_arch = "wasm32"))]
macro_rules! log_line {
    ($($t:tt)*) => { eprintln!($($t)*) };
}

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
mod platform_admins; // the first platform administrator, behind BOOTSTRAP_SECRET (W-ATOMIC row 4)
mod rail;
mod idempotency;
mod identity_store;
mod courier;
mod hubdo;
mod command;
mod hubstore;
mod otel;
mod outbox;
mod print_rail;
mod bell_route;
mod owner;
mod assist;
mod storefront;
mod stripe;
mod project;
mod notify;
mod channels;
mod cloud;
mod cron;
mod mcp;
mod integrations;
mod ebills;
mod catalog_edit;
mod catalog_history; // W-PITR: the menu's edit journal, its history screen and restore
/// The request body, parsed so `deny_unknown_fields` means it (gate: strict-body).
mod body;
/// A request and a response in plain Rust, so route code runs under `cargo test` (W-COV C2).
mod wire;
/// The platform bindings behind enums a test can fill (W-COV C2).
mod edge;
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
mod exceptions;
mod fiscal;
mod privacy;
mod learn;
mod version;
mod image_err;

#[cfg(target_arch = "wasm32")]
use worker::wasm_bindgen::{JsCast, JsValue};
use worker::*;



/// A CSPRNG-backed order id from the platform's Web Crypto.
///
/// FAIL-CLOSED on purpose: if `crypto.randomUUID` is not reachable we refuse the
/// request rather than fall back to a weaker source. An order id that can repeat
/// is a primary-key collision between two customers, which is exactly the defect
/// the kernel's old `AtomicU64` counter produced once it left a single process.
#[cfg(target_arch = "wasm32")]
pub fn edge_id() -> Option<String> {
    let global = js_sys::global();
    let crypto = js_sys::Reflect::get(&global, &JsValue::from_str("crypto")).ok()?;
    let f = js_sys::Reflect::get(&crypto, &JsValue::from_str("randomUUID")).ok()?;
    let f = f.dyn_ref::<js_sys::Function>()?;
    f.call0(&crypto).ok()?.as_string()
}

/// Natively (tests), the same shape from the OS CSPRNG -- still fail-closed (W-COV C2).
#[cfg(not(target_arch = "wasm32"))]
pub fn edge_id() -> Option<String> {
    let mut b = [0u8; 16];
    getrandom::getrandom(&mut b).ok()?;
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    Some(format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32]))
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
    // One span per request, continuing an incoming W3C traceparent if there is
    // one. The export happens in waitUntil AFTER the response is returned, so
    // tracing costs the customer nothing in latency.
    let method = req.method().to_string();
    let path = req.path();
    let mut trace = otel::Trace::begin(req.headers().get("traceparent").ok().flatten().as_deref(), &format!("{method} {path}"));
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
                &edge::ObjectNamespace::Live(ns),
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
    ctx.wait_until(async move { trace.export(&edge::Env::Live(env), status).await });
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
    router(Router::with_data(Req { now_ms: Date::now().as_millis() as i64 })).run(req, env).await
}

#[cfg(test)]
#[path = "routes_table/tests.rs"]
mod router_tests;

/// THE ROUTE TABLE, apart from the request (W-COV C2): a test builds it natively, which is where
/// two routes that conflict are found -- the router refuses such a pair when it is built, and on
/// the Worker that is every request's first line.
pub(crate) fn router(r: Router<'static, Req>) -> Router<'static, Req> {
    r.get("/healthz", |_, _| Response::ok("ok"))
        // W-DEPLOY: which commit this build is; tools/deploy/deploy.sh reads it back after a rollout.
        .get("/api/version", version::serve)
        // ── public storefront ──
        .get_async("/api/public/locations/:slug/menu", |r, c| edge::run(r, c, storefront::menu))
        .get_async("/api/public/locations/:slug/menu/week", |r, c| edge::run(r, c, services::analytics::week_route::week))
        .get_async("/api/public/locations/:slug/context", |r, c| edge::run(r, c, services::venue::context::context)) // W-SENSE
        .get_async("/manifest.webmanifest", |r, c| edge::run(r, c, storefront::manifest))
        // P8/P9: the venue's privacy notice (venue from the Host) and the DPA text.
        .get_async("/privacy", |r, c| edge::run(r, c, privacy::notice::serve))
        .get_async("/dpa", |r, c| edge::run(r, c, privacy::dpa::page))
        .post_async("/api/public/locations/:slug/orders", |r, c| edge::run(r, c, storefront::place))
        // ── reservations: the transport for `dowiz_kernel::reservation` ──
        .get_async("/api/public/locations/:slug/reservations", |r, c| edge::run(r, c, booking::list))
        .post_async("/api/public/locations/:slug/reservations", |r, c| edge::run(r, c, booking::create))
        .get_async("/api/public/locations/:slug/reservations/:id", |r, c| edge::run(r, c, booking::detail))
        .post_async("/api/public/locations/:slug/reservations/:id/action", |r, c| edge::run(r, c, booking::action))
        .get_async("/api/public/locations/:slug/reservations/:id/pass", |r, c| edge::run(r, c, booking::issue_pass))
        .post_async("/api/public/locations/:slug/pass/verify", |r, c| edge::run(r, c, booking::verify_pass))
        // ── THE FLOOR: which tables are free FOR A SLOT ──
        //
        // PUBLIC, DELIBERATELY. It is the venue's own furniture and the "is
        // there room tonight" a phone call already answers; it carries no name,
        // no party and no reservation id. It REFUSES without `slotMin`,
        // because a table is free or taken only for a slot and a plan drawn
        // without one is a picture of a lie.
        .get_async("/api/public/locations/:slug/tables", |r, c| edge::run(r, c, booking::availability))
        // ── threads: the transport for `dowiz_kernel::thread` ──
        .get_async("/api/public/locations/:slug/threads/:id", |r, c| edge::run(r, c, social::messages))
        .post_async("/api/public/locations/:slug/threads/:id/messages", |r, c| edge::run(r, c, social::send))
        // ── the wallet journal: `dowiz_kernel::ledger_account` ──
        .get_async("/api/public/locations/:slug/wallet", |r, c| edge::run(r, c, wallet::balance))
        .get_async("/api/public/locations/:slug/wallet/statement", |r, c| edge::run(r, c, wallet::statement))
        .post_async("/api/public/locations/:slug/wallet/topup", |r, c| edge::run(r, c, wallet::top_up))
        // ── the delivery estimate: `dowiz_kernel::eta` ──
        .post_async("/api/public/locations/:slug/eta", |r, c| edge::run(r, c, eta::quote))
        .post_async("/api/promo/check", |r, c| edge::run(r, c, services::ordering::preview::promo_check))
        .get_async("/api/public/reach", |r, c| edge::run(r, c, services::venue::zones::reach))
        .get_async("/api/public/rates", |r, c| edge::run(r, c, services::ordering::rates::rates))
        .post_async("/api/voice", |r, c| edge::run(r, c, services::engagement::voice::voice))
        .post_async("/api/owner/zones", |r, c| edge::run(r, c, services::venue::zones::set_zones))
        // The room as DATA an owner can edit, beside the delivery zones it
        // sits next to in the catalogue. Refused on write when it does not
        // parse back, naming the zone, the table and the rule.
        .post_async("/api/owner/floorplan", |r, c| edge::run(r, c, booking::set_plan))
        // The console's floor editor opens the plan as stored.
        .get_async("/api/owner/floorplan", |r, c| edge::run(r, c, booking::get_plan))
        // The console's Bookings: the day's list, and the venue's moves on
        // one (confirm, decline, seat, no-show, cancel -- all FSM events).
        .get_async("/api/owner/reservations", |r, c| edge::run(r, c, booking::venue_day))
        .post_async("/api/owner/reservations/:id/action", |r, c| edge::run(r, c, booking::venue_action))
        .post_async("/api/owner/branding/extract", |r, c| edge::run(r, c, services::venue::brand_extract::extract_branding))
        .post_async("/api/order/:id/feedback", |r, c| edge::run(r, c, services::orders::feedback::feedback))
        // ── accounts ──
        .post_async("/api/bootstrap", |r, c| edge::run(r, c, bootstrap::seed))
        // The waiting list: the landing page's one form. Public to write,
        // administrators only to read -- see `waitlist`.
        .post_async("/api/waitlist", |r, c| edge::run(r, c, waitlist::join))
        .get_async("/api/platform/waitlist", |r, c| edge::run(r, c, waitlist::list))
        // The main hub. Platform administrators only -- see `platform`.
        .get_async("/api/platform/errors", |r, c| edge::run(r, c, platform::errors))
        .get_async("/api/platform/hubs", |r, c| edge::run(r, c, platform::hubs))
        .post_async("/api/platform/hubs", |r, c| edge::run(r, c, platform::create_hub))
        .post_async("/api/platform/admins", |r, c| edge::run(r, c, platform_admins::create))
        .post_async("/api/platform/compact", |r, c| edge::run(r, c, hubdo::compact::fan::platform_compact))
        .post_async("/api/webhooks/stripe", |r, c| edge::run(r, c, stripe::webhook))
        .post_async("/api/auth/login", |r, c| edge::run(r, c, accounts::owner_login))
        .post_async("/api/auth/refresh", |r, c| edge::run(r, c, accounts::owner_refresh))
        .post_async("/api/auth/logout", |r, c| edge::run(r, c, accounts::owner_logout))
        .post_async("/api/courier/auth/login", |r, c| edge::run(r, c, accounts::courier_login))
        .post_async("/api/courier/auth/claim", |r, c| edge::run(r, c, accounts::courier_claim))
        // THE ROOM (docs/design/BLUEPRINT-POS-THE-ROOM-2026-09-22.md).
        .post_async("/api/staff/login", |r, c| edge::run(r, c, services::identity::staff::staff_login))
        .post_async("/api/staff/claim", |r, c| edge::run(r, c, services::identity::staff::staff_claim))
        .post_async("/api/staff/password", |r, c| edge::run(r, c, services::identity::staff::staff_password))
        .get_async("/api/owner/staff", |r, c| edge::run(r, c, services::identity::staff_admin::list_staff))
        .post_async("/api/owner/staff/invite", |r, c| edge::run(r, c, services::identity::staff_admin::invite_staff))
        .post_async("/api/owner/staff/:id", |r, c| edge::run(r, c, services::identity::staff_admin::set_staff))
        .post_async("/api/owner/staff/:id/password", |r, c| edge::run(r, c, services::identity::staff::password::owner_reset))
        .get_async("/api/staff/room", |r, c| edge::run(r, c, services::orders::room::handlers::room_view))
        .post_async("/api/staff/orders/:id/amend", |r, c| edge::run(r, c, services::orders::room::handlers::amend))
        .post_async("/api/staff/orders/:id/pay", |r, c| edge::run(r, c, services::orders::room::pay::pay))
        .post_async("/api/staff/orders/:id/kitchen-ack", |r, c| edge::run(r, c, services::orders::kitchen_ack::kitchen_ack))
        .get_async("/api/staff/kitchen", |r, c| edge::run(r, c, services::orders::kitchen_ack::board::kitchen_orders))
        .get_async("/api/staff/kitchen/prep", |r, c| edge::run(r, c, services::analytics::forecast::prep)) // W-PREP P6
        .post_async("/api/staff/assist", |r, c| edge::run(r, c, services::engagement::assist::kitchen::kitchen_assist))
        .post_async("/api/print/poll", |r, c| edge::run(r, c, services::orders::print::poll))
        .get_async("/api/print/job/:token", |r, c| edge::run(r, c, services::orders::print::job))
        .delete_async("/api/print/job/:token", |r, c| edge::run(r, c, services::orders::print::ack))
        .get_async("/api/owner/print/jobs", |r, c| edge::run(r, c, services::orders::print::jobs))
        .get_async("/api/owner/wallet/legs", |r, c| edge::run(r, c, services::orders::legs::audit))
        .post_async("/api/owner/wallet/legs/repair", |r, c| edge::run(r, c, services::orders::legs::repair))
        .post_async("/api/staff/orders/aggregator", |r, c| edge::run(r, c, services::orders::aggregator::enter))
        .post_async("/api/staff/orders/:id/refund", |r, c| edge::run(r, c, services::orders::refund::refund))
        .post_async("/api/staff/orders/:id/returned", |r, c| edge::run(r, c, services::orders::refund::returned))
        .post_async("/api/staff/orders/:id/transfer", |r, c| edge::run(r, c, services::orders::room::transfer::transfer))
        .post_async("/api/staff/sittings/:id/move", |r, c| edge::run(r, c, services::orders::room::transfer::move_sitting))
        .post_async("/api/staff/till/open", |r, c| edge::run(r, c, services::orders::room::till::open))
        .post_async("/api/staff/till/count", |r, c| edge::run(r, c, services::orders::room::till::count))
        .post_async("/api/staff/till/close", |r, c| edge::run(r, c, services::orders::room::till::close))
        .post_async("/api/staff/till/pay_in", |r, c| edge::run(r, c, services::orders::room::till::pay_in))
        .post_async("/api/staff/till/pay_out", |r, c| edge::run(r, c, services::orders::room::till::pay_out))
        .get_async("/api/staff/till/tips", |r, c| edge::run(r, c, services::orders::room::till::tips))
        .post_async("/api/staff/offline_sales", |r, c| edge::run(r, c, services::orders::offline_sale::handler::sell))
        .get_async("/api/staff/floor", |r, c| edge::run(r, c, services::orders::room::floor::get))
        .post_async("/api/staff/floor/:sitting/cleared", |r, c| edge::run(r, c, services::orders::room::floor::post_cleared))
        // A9: a table's QR code, a guest's round answered by the room, and the
        // guest's read-only view of the table's bill.
        .get_async("/api/owner/tables/qr", |r, c| edge::run(r, c, services::orders::room::table_qr::list))
        .get_async("/api/owner/tables/:zone/:n/qr.svg", |r, c| edge::run(r, c, services::orders::room::table_qr::one))
        .post_async("/api/staff/orders/:id/guest", |r, c| edge::run(r, c, services::orders::room::guest_round::confirm))
        .get_async("/api/order/:id/sitting", |r, c| edge::run(r, c, services::orders::room::guest_round::sitting_bill))
        .get_async("/api/order/:id/stamps", |r, c| edge::run(r, c, services::loyalty::handlers::order_stamps))
        .get_async("/api/order/:id/taste", |r, c| edge::run(r, c, services::customers::taste_routes::guest_view))
        .post_async("/api/order/:id/taste/withdraw", |r, c| edge::run(r, c, services::customers::taste_routes::guest_withdraw))
        .get_async("/api/order/:id/taste/for-you", |r, c| edge::run(r, c, services::customers::taste_routes::for_you)) // W-TASTE2
        // ── owner ──
        .get_async("/api/owner/orders", |r, c| edge::run(r, c, owner::orders))
        .post_async("/api/owner/orders/:id/action", |r, c| edge::run(r, c, owner::order_action))
        .post_async("/api/owner/orders/:id/assign", |r, c| edge::run(r, c, owner::assign_courier))
        .get_async("/api/owner/couriers/:id", |r, c| edge::run(r, c, services::courier::console::courier_detail))
        .get_async("/api/owner/dashboard", |r, c| edge::run(r, c, owner::dashboard))
        .post_async("/api/owner/products", |r, c| edge::run(r, c, catalog_edit::create_product))
        .post_async("/api/owner/products/:id/delete", |r, c| edge::run(r, c, catalog_edit::delete_product))
        .get_async("/api/owner/categories", |r, c| edge::run(r, c, catalog_edit::list_categories))
        .post_async("/api/owner/categories", |r, c| edge::run(r, c, catalog_edit::set_category))
        .post_async("/api/owner/categories/:id/delete", |r, c| edge::run(r, c, catalog_edit::delete_category))
        .post_async("/api/owner/products/:id", |r, c| edge::run(r, c, owner::update_product))
        .post_async("/api/owner/products/:id/sense/suggest", |r, c| edge::run(r, c, services::catalogue::sense::suggest)) // W-SENSE
        .post_async("/api/owner/location", |r, c| edge::run(r, c, owner::update_location))
        .post_async("/api/owner/i18n", |r, c| edge::run(r, c, owner::write_translations))
        // ── ported from the native adapter, on the SAME dowiz-hub logic ──
        .get_async("/api/owner/analytics", |r, c| edge::run(r, c, services::analytics::analytics))
        .get_async("/api/owner/analytics/kitchen", |r, c| edge::run(r, c, services::analytics::kitchen::kitchen))
        .post_async("/api/owner/analytics/history", |r, c| edge::run(r, c, services::analytics::handler::history))
        .get_async("/api/owner/exceptions", |r, c| edge::run(r, c, exceptions::exceptions))
        .get_async("/api/owner/promotions", |r, c| edge::run(r, c, services::ordering::promotions::promotions))
        .post_async("/api/owner/promotions", |r, c| edge::run(r, c, services::ordering::promotions::set_promotion))
        .post_async("/api/owner/promotions/:code/delete", |r, c| edge::run(r, c, services::ordering::promotions::delete_promotion))
        .get_async("/api/owner/activation", |r, c| edge::run(r, c, services::venue::activation::activation))
        .get_async("/api/owner/branding", |r, c| edge::run(r, c, services::venue::brand::branding))
        .post_async("/api/owner/branding", |r, c| edge::run(r, c, services::venue::brand::set_branding))
        .post_async("/api/owner/branding/preset", |r, c| edge::run(r, c, services::venue::brand::set_preset))
        // C6 campaigns: segment, preview the count, queue through the outbox.
        .get_async("/api/owner/campaigns", |r, c| edge::run(r, c, services::campaigns::handlers::list))
        .post_async("/api/owner/campaigns", |r, c| edge::run(r, c, services::campaigns::handlers::define))
        .get_async("/api/owner/campaigns/:id", |r, c| edge::run(r, c, services::campaigns::handlers::report))
        .post_async("/api/owner/campaigns/:id/preview", |r, c| edge::run(r, c, services::campaigns::handlers::preview))
        .post_async("/api/owner/campaigns/:id/send", |r, c| edge::run(r, c, services::campaigns::handlers::send_now))
        .get_async("/api/owner/customers", |r, c| edge::run(r, c, services::customers::handlers::customers))
        .post_async("/api/owner/customers/:key/reveal", |r, c| edge::run(r, c, services::customers::handlers::reveal_customer))
        .post_async("/api/owner/customers/:key/forget", |r, c| edge::run(r, c, services::customers::forget::forget_customer))
        .get_async("/api/owner/customers/:key/taste", |r, c| edge::run(r, c, services::customers::taste_routes::owner_view))
        .get_async("/api/owner/customers/taste/segments", |r, c| edge::run(r, c, services::customers::taste_routes::owner_segments))
        .get_async("/api/owner/customers/taste/builder", |r, c| edge::run(r, c, services::customers::taste_builder::builder)) // W-SENSE
        .get_async("/api/owner/snn", |r, c| edge::run(r, c, services::customers::snn_routes::owner_view)) // W-SNN
        .post_async("/api/owner/snn", |r, c| edge::run(r, c, services::customers::snn_routes::owner_set)) // W-SNN
        .get_async("/api/owner/customers/reveals", |r, c| edge::run(r, c, services::customers::handlers::reveals))
        .put_async("/api/owner/customers/:key/record", |r, c| edge::run(r, c, services::customers::record_routes::put_record))
        .post_async("/api/owner/customers/rekey", |r, c| edge::run(r, c, services::customers::record_routes::rekey))
        .post_async("/api/owner/customers/reforget", |r, c| edge::run(r, c, services::customers::forget::run::reforget))
        .post_async("/api/owner/customers/:key/consent", |r, c| edge::run(r, c, services::customers::consent_routes::owner_act))
        .post_async("/api/owner/customers/:key/link", |r, c| edge::run(r, c, services::customers::alias_routes::link))
        .post_async("/api/owner/customers/:key/unlink", |r, c| edge::run(r, c, services::customers::alias_routes::unlink))
        .get_async("/api/public/consent/wordings", |r, c| edge::run(r, c, services::customers::consent_routes::wordings))
        .get_async("/api/owner/stock", |r, c| edge::run(r, c, services::operations::stock::stock))
        .post_async("/api/owner/stock/:kind", |r, c| edge::run(r, c, services::operations::stock::stock_move))
        .get_async("/api/owner/stock/waste", |r, c| edge::run(r, c, services::operations::waste::waste_report))
        .get_async("/api/owner/stock/haccp", |r, c| edge::run(r, c, services::operations::stock::haccp::export))
        .post_async("/api/owner/supplies", |r, c| edge::run(r, c, services::operations::supplies::set_supply))
        .post_async("/api/owner/supplies/:id/retire", |r, c| edge::run(r, c, services::operations::supplies::retire_supply))
        .post_async("/api/owner/supplies/bulk", |r, c| edge::run(r, c, services::operations::supplies::quick::add_supplies))
        .post_async("/api/owner/supplies/delete", |r, c| edge::run(r, c, services::operations::supplies::delete::delete_supplies))
        .post_async("/api/owner/preps", |r, c| edge::run(r, c, services::operations::preps::set_prep))
        .get_async("/api/owner/preps", |r, c| edge::run(r, c, services::operations::preps::list_preps))
        .get_async("/api/owner/supplies/:id/uses", |r, c| edge::run(r, c, services::operations::preps::uses))
        .get_async("/api/owner/products/:id/takes", |r, c| edge::run(r, c, services::operations::preps::takes_of))
        .post_async("/api/owner/products/delete", |r, c| edge::run(r, c, catalog_edit::delete_products))
        .post_async("/api/owner/ingredients/reset", |r, c| edge::run(r, c, services::operations::ingredients_reset::reset_ingredients))
        // F1: supplies and recipes in bulk, dry run first; the dishes as stored.
        .post_async("/api/owner/supplies/import", |r, c| edge::run(r, c, services::catalogue::import::bulk::import_supplies))
        .post_async("/api/owner/recipes/import", |r, c| edge::run(r, c, services::catalogue::import::bulk::import_recipes))
        .get_async("/api/owner/products", |r, c| edge::run(r, c, services::catalogue::import::bulk::owner_products))
        .get_async("/api/owner/features", |r, c| edge::run(r, c, services::venue::settings::features))
        .post_async("/api/owner/features", |r, c| edge::run(r, c, services::venue::settings::set_feature))
        .get_async("/api/owner/settings", |r, c| edge::run(r, c, services::venue::settings::settings))
        .post_async("/api/owner/settings", |r, c| edge::run(r, c, services::venue::settings::set_setting))
        // BN2: what the venue's object put on the CDN for the storefront, and publish now (W-PUBUI).
        .get_async("/api/owner/publish", |r, c| edge::run(r, c, services::venue::publish::status))
        .post_async("/api/owner/publish", |r, c| edge::run(r, c, services::venue::publish::publish_now))
        .post_async("/api/owner/notify/test", |r, c| edge::run(r, c, notify::test))
        .get_async("/api/push/key", |r, c| edge::run(r, c, notify::push::routes::key))
        .post_async("/api/push/subscribe", |r, c| edge::run(r, c, notify::push::routes::subscribe))
        .post_async("/api/push/unsubscribe", |r, c| edge::run(r, c, notify::push::routes::unsubscribe))
        .post_async("/api/push/state", |r, c| edge::run(r, c, notify::push::routes::state))
        .post_async("/api/webhooks/telegram", |r, c| edge::run(r, c, notify::hook::webhook))
        .get_async("/api/owner/telegram", |r, c| edge::run(r, c, notify::hook::owner::state))
        .post_async("/api/owner/telegram/connect", |r, c| edge::run(r, c, notify::hook::owner::connect))
        .post_async("/api/owner/telegram/link", |r, c| edge::run(r, c, notify::hook::owner::link))
        .post_async("/api/owner/telegram/group", |r, c| edge::run(r, c, notify::hook::owner::group))
        .post_async("/api/owner/telegram/test", |r, c| edge::run(r, c, notify::hook::owner::test))
        .post_async("/api/owner/telegram/unlink", |r, c| edge::run(r, c, notify::hook::owner::unlink))
        .get_async("/api/owner/inbox", |r, c| edge::run(r, c, channels::inbox))
        .get_async("/api/owner/threads", |r, c| edge::run(r, c, social::inbox::list))
        .get_async("/api/owner/inbox/:peer", |r, c| edge::run(r, c, channels::thread))
        .post_async("/api/owner/inbox/:peer", |r, c| edge::run(r, c, channels::reply))
        .get_async("/api/owner/backup/cloud", |r, c| edge::run(r, c, cloud::status))
        .post_async("/api/owner/backup/cloud", |r, c| edge::run(r, c, cloud::push))
        .get_async("/api/webhooks/meta", |r, c| edge::run(r, c, channels::webhook_verify))
        .post_async("/api/webhooks/meta", |r, c| edge::run(r, c, channels::webhook))
        .get_async("/api/owner/integrations", |r, c| edge::run(r, c, integrations::status))
        .get_async("/api/owner/dpa", |r, c| edge::run(r, c, privacy::dpa::read))
        .post_async("/api/owner/dpa/accept", |r, c| edge::run(r, c, privacy::dpa::accept))
        // L7: lesson videos from R2 `dowiz-learn`, staff/owner/courier Bearer only, Range-aware.
        .get_async("/api/learn/manifest", |r, c| edge::run(r, c, learn::manifest))
        .get_async("/api/learn/media/*key", learn::media)
        .post_async("/api/owner/integrations/check", |r, c| edge::run(r, c, integrations::check))
        .get_async("/api/owner/ebills", |r, c| edge::run(r, c, ebills::routes::status))
        .post_async("/api/owner/ebills/config", |r, c| edge::run(r, c, ebills::routes::config))
        .post_async("/api/owner/ebills/map", |r, c| edge::run(r, c, ebills::routes::map))
        .get_async("/api/owner/sms", |r, c| edge::run(r, c, notify::sms::routes::status)).post_async("/api/owner/sms", |r, c| edge::run(r, c, notify::sms::routes::set)) // W-SMS
        .post_async("/api/owner/sms/test", |r, c| edge::run(r, c, notify::sms::routes::test)).post_async("/api/owner/sms/stop", |r, c| edge::run(r, c, notify::sms::routes::stop))
        .get_async("/api/public/locations/:slug/sms", |r, c| edge::run(r, c, notify::sms::routes::box_for))
        .get_async("/api/owner/bag", |r, c| edge::run(r, c, services::loyalty::bag_routes::card)).post_async("/api/owner/bag", |r, c| edge::run(r, c, services::loyalty::bag_routes::set)) // W-QR
        .get_async("/api/owner/bag/qr.svg", |r, c| edge::run(r, c, services::loyalty::bag_routes::qr))
        .get_async("/api/public/locations/:slug/welcome", |r, c| edge::run(r, c, services::loyalty::bag_routes::public))
        .get_async("/api/owner/fiscal", |r, c| edge::run(r, c, fiscal::routes::status))
        .get_async("/api/owner/offline_sales", |r, c| edge::run(r, c, services::orders::offline_sale::handler::pane))
        .post_async("/api/owner/fiscal/ebills", |r, c| edge::run(r, c, fiscal::routes::set))
        .get_async("/api/owner/orders/:id/receipt", |r, c| edge::run(r, c, fiscal::routes::receipt))
        .get_async("/api/mcp", |r, c| edge::run(r, c, mcp::describe))
        .post_async("/api/mcp", |r, c| edge::run(r, c, mcp::rpc))
        // A person's own agent key (mcp/keys.rs): minted in their app, seen and ended by the owner.
        .get_async("/api/staff/mcp/keys", |r, c| edge::run(r, c, mcp::staff_list))
        .post_async("/api/staff/mcp/keys", |r, c| edge::run(r, c, mcp::staff_mint))
        .post_async("/api/staff/mcp/keys/revoke", |r, c| edge::run(r, c, mcp::staff_revoke))
        .get_async("/api/courier/mcp/keys", |r, c| edge::run(r, c, mcp::courier_list))
        .post_async("/api/courier/mcp/keys", |r, c| edge::run(r, c, mcp::courier_mint))
        .post_async("/api/courier/mcp/keys/revoke", |r, c| edge::run(r, c, mcp::courier_revoke))
        .get_async("/api/owner/mcp/keys", |r, c| edge::run(r, c, mcp::owner_list))
        .post_async("/api/owner/mcp/keys/revoke", |r, c| edge::run(r, c, mcp::owner_revoke))
        .post_async("/api/owner/menu/import", |r, c| edge::run(r, c, services::catalogue::import::import_menu))
        .get_async("/api/owner/menu/history", |r, c| edge::run(r, c, catalog_history::list)) // W-PITR
        .post_async("/api/owner/menu/history/restore", |r, c| edge::run(r, c, catalog_history::restore)) // W-PITR
        .get_async("/api/owner/couriers", |r, c| edge::run(r, c, services::courier::console::couriers))
        .post_async("/api/owner/couriers/invite", |r, c| edge::run(r, c, services::courier::hiring::invite_courier))
        .post_async("/api/owner/couriers/:id/uninvite", |r, c| edge::run(r, c, services::courier::hiring::uninvite_courier))
        .post_async("/api/owner/couriers/:id/active", |r, c| edge::run(r, c, services::courier::hiring::set_courier_active))
        .get_async("/api/owner/posts", |r, c| edge::run(r, c, services::engagement::posts::posts))
        .post_async("/api/owner/posts/draft", |r, c| edge::run(r, c, services::engagement::posts::draft_post))
        .post_async("/api/owner/posts/:id/approve", |r, c| edge::run(r, c, services::engagement::verdict::approve_post))
        .post_async("/api/owner/posts/:id/reject", |r, c| edge::run(r, c, services::engagement::verdict::reject_post))
        .get_async("/api/owner/graph", |r, c| edge::run(r, c, services::engagement::assist::graph))
        .get_async("/api/live", |r, c| edge::run(r, c, live::connect))
        .get_async("/api/owner/health", |r, c| edge::run(r, c, services::operations::health))
        .get_async("/api/owner/history", |r, c| edge::run(r, c, services::operations::history))
        .post_async("/api/owner/hub/rotate", |r, c| edge::run(r, c, services::operations::rotate_now))
        .get_async("/api/owner/backup", |r, c| edge::run(r, c, services::operations::backup))
        .post_async("/api/owner/restore", |r, c| edge::run(r, c, services::operations::restore))
        .post_async("/api/owner/assist", |r, c| edge::run(r, c, services::engagement::assist::owner_assist))
        .get_async("/api/owner/ai", |r, c| edge::run(r, c, services::engagement::ai::status))
        .post_async("/api/owner/ai/test", |r, c| edge::run(r, c, services::engagement::ai::test))
        .post_async("/api/owner/ai/ask", |r, c| edge::run(r, c, services::engagement::ai::ask))
        .get_async("/api/owner/ai/explain", |r, c| edge::run(r, c, services::engagement::ai::explain))
        .post_async("/api/courier/assist", |r, c| edge::run(r, c, services::engagement::assist::courier_assist))
        .get_async("/api/owner/apikeys", |r, c| edge::run(r, c, services::identity::keys::list_api_keys))
        .post_async("/api/owner/apikeys", |r, c| edge::run(r, c, services::identity::keys::create_api_key))
        .post_async("/api/owner/apikeys/revoke", |r, c| edge::run(r, c, services::identity::keys::revoke_api_key))
        .post_async("/api/owner/products/:id/image", |r, c| edge::run(r, c, services::catalogue::media::set_product_image))
        .post_async("/api/owner/products/:id/image/clear", |r, c| edge::run(r, c, services::catalogue::media::clear_product_image))
        // R13 (W-LOST): an option's recipe -- what "extra salmon" takes off the shelf.
        .get_async("/api/owner/products/:id/option-bom", |r, c| edge::run(r, c, services::catalogue::option_bom::read))
        .post_async("/api/owner/products/:id/option-bom", |r, c| edge::run(r, c, services::catalogue::option_bom::write))
        // The venue's own mark, stored the way its dishes' photographs are.
        .post_async("/api/owner/place", |r, c| edge::run(r, c, services::venue::place::set_place))
        .post_async("/api/owner/logo", |r, c| edge::run(r, c, services::catalogue::media::set_venue_logo))
        .post_async("/api/owner/logo/clear", |r, c| edge::run(r, c, services::catalogue::media::clear_venue_logo))
        .get_async("/media/:name", |r, c| edge::run(r, c, services::catalogue::media::media))
        // ── courier ──
        .get_async("/api/courier/tasks", |r, c| edge::run(r, c, courier::tasks))
        .post_async("/api/courier/shift", |r, c| edge::run(r, c, courier::shift))
        .post_async("/api/courier/orders/:id/accept", |r, c| edge::run(r, c, courier::accept))
        .post_async("/api/courier/orders/:id/pickup", |r, c| edge::run(r, c, courier::pickup))
        .post_async("/api/courier/orders/:id/deliver", |r, c| edge::run(r, c, courier::deliver))
        .post_async("/api/courier/orders/:id/refused", |r, c| edge::run(r, c, courier::refused))
        .post_async("/api/courier/position", |r, c| edge::run(r, c, courier::position))
        .get_async("/api/courier/earnings", |r, c| edge::run(r, c, courier::earnings))
        .get_async("/api/courier/history", |r, c| edge::run(r, c, services::courier::history::courier_history))
        .get_async("/api/order/:id", |r, c| edge::run(r, c, services::orders::read::order))
        // The customer <-> courier chat of one order (W-URGENT 2026-10-02): the two parties, the owner read-only.
        .get_async("/api/order/:id/chat", |r, c| edge::run(r, c, services::orders::chat::read))
        .post_async("/api/order/:id/chat", |r, c| edge::run(r, c, services::orders::chat::send))
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
}

/// The cron in wrangler.toml (`cloud::NIGHTLY_CRON`): every venue with a
/// bucket gets its nightly copy, and every venue's alarm is checked.
#[event(scheduled)]
pub async fn scheduled(event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    // ONE CRON (DAG Phase 2, 2026-09-28). The minute one is gone: a venue's
    // timed work -- outbox, e-bills, fiscal -- is woken by its own object's
    // alarm (`cron.rs`, `hubdo/timer.rs`). An expression this build does not
    // know is said, never run as the nightly.
    // ONE CLOCK READ PER INVOCATION, for the reason `Req` gives on the fetch
    // path: a job whose parts each ask the wall clock has as many answers as it
    // has parts. `tools/gates/clock.sh` allows this line and the router's, and
    // nothing else on either entry point.
    let now_ms = Date::now().as_millis() as i64;
    let env = edge::Env::Live(env);
    if event.cron() == cloud::NIGHTLY_CRON {
        // ONE request: the platform object fans the night out to every venue's
        // own alarm and re-arms a venue with work due and no alarm (W-LOOP).
        cron::nightly(&env, now_ms).await;
    } else {
        log_error!("scheduled: unknown cron {:?} -- nothing run", event.cron());
    }
}
