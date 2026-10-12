//! WHAT ONE ORDER COSTS IN OBJECT REQUESTS, route by route (W-BN4B).
//!
//! Every billed Durable Object request a route makes is counted by the test edge
//! (`edge::mem::object_paths`, every `Stub::Mem` fetch with its path) -- not estimated.
//! One delivery order is walked through its whole life the way the four apps drive it:
//! the storefront places it, the console lists and moves it, the kitchen sees and acks
//! it, a courier is assigned and carries it, the customer tracks it and leaves feedback.
//!
//! WALKED TWICE. `Via::Worker` calls each route's handler body (`*_at`) from the Worker,
//! as every route ran before BN4: every read and write of the venue's images is a request.
//! `Via::Object` calls the routes as the router does now (`hubdo/command.rs`): the Worker
//! picks the object and the object runs the same body in one turn. The two walks must end
//! in the same order, so the move is checked to change the bill and nothing else.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::edge::Ctx;
use crate::storefront::route_tests::open_venue;
use crate::wire::{Call, Reply};
use serde_json::{json, Value};

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}

/// The platform object's images (`platform_store`): the registry, sign-in and the roster.
/// A request for one of them is the platform's bill, not the venue's.
const PLATFORM_IMAGES: [&str; 7] = ["registry", "identity", "sessions", "couriers", "waitlist", "erasures", "errors"];

/// Is this object request one to the venue's own object?
pub(crate) fn is_venue(path: &str) -> bool {
    !path.strip_prefix("/img/").is_some_and(|img| PLATFORM_IMAGES.contains(&img))
}

/// The object requests `f` made, by path, and what it answered.
fn paid(f: impl FnOnce() -> Reply) -> (Vec<String>, Reply) {
    let from = crate::edge::mem::object_paths().len();
    let r = f();
    (crate::edge::mem::object_paths()[from..].to_vec(), r)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Via {
    /// Each handler's body run by the Worker (before BN4 / W-BN4B).
    Worker,
    /// Each route as the router runs it now.
    Object,
}

/// One step of the walk: its name, the object requests it made, its status.
pub(crate) struct Step {
    pub name: &'static str,
    pub paths: Vec<String>,
    pub status: u16,
}

impl Step {
    pub fn venue(&self) -> usize {
        self.paths.iter().filter(|p| is_venue(p)).count()
    }
}

/// The placement as the Worker ran it before BN4: `place_at` with the Worker's own env.
async fn place_from_worker(req: Call, ctx: Ctx<crate::Req>) -> worker::Result<Reply> {
    let place = crate::hubstore::Place::of_slug(&ctx, "alpha").await?;
    crate::storefront::place_at(place, req, ctx).await
}

/// `$at` from the Worker, or `$route` as routed, over the same request.
macro_rules! by {
    ($via:expr, $site:expr, $at:path, $route:path, $call:expr, $params:expr) => {
        match $via {
            Via::Worker => $site.run($at, $call, $params),
            Via::Object => $site.run($route, $call, $params),
        }
    };
}

/// Every step of one delivery order, and the order as it ended.
pub(crate) fn a_delivery_order(site: &Site, via: Via) -> (Vec<Step>, Value) {
    let (owner, dish) = open_venue(site, "alpha", "alpha@example.com");
    let (rider, rider_id) = site.courier("alpha", &owner, "+355691111111");
    let cook = site.staff("alpha", &owner, "k@x.test", "kitchen");
    let r = site.run(crate::courier::shift, post(&at("/api/courier/shift"), &json!({"open": true})).bearer(&rider).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "shift: {}", r.body_str());
    let mut steps = Vec::new();
    let mut step = |name: &'static str, f: &dyn Fn() -> Reply| -> Value {
        let (paths, r) = paid(f);
        steps.push(Step { name, paths, status: r.status_code() });
        r.body_value()
    };
    let basket = json!({
        "items": [{"product_id": dish, "quantity": 1}],
        "contact": {"name": "Guest", "phone": "+355690000002"},
        "fulfilment": {"kind": "delivery", "address": {"line": "Rruga Tregtare 1", "lat_udeg": 41_312_000, "lon_udeg": 19_446_000}},
        "payment": "cash",
    });
    let placed = step("place", &|| {
        let call = post(&at("/api/public/locations/alpha/orders"), &basket).on("alpha");
        by!(via, site, place_from_worker, crate::storefront::place, call, &[("slug", "alpha")])
    });
    let id = placed["id"].as_str().unwrap_or_else(|| panic!("no id: {placed}")).to_string();
    let key = placed["access_token"].as_str().unwrap_or("").to_string();
    let ids: &[(&str, &str)] = &[("id", id.as_str())];
    let track = || {
        let call = get(&at(&format!("/api/order/{id}"))).bearer(&key).on("alpha");
        by!(via, site, crate::services::orders::read::order_at, crate::services::orders::read::order, call, ids)
    };
    let act = |a: &str| {
        let call = post(&at(&format!("/api/owner/orders/{id}/action")), &json!({"location_id": "alpha", "action": a})).bearer(&owner).on("alpha");
        by!(via, site, crate::owner::order_action_at, crate::owner::order_action, call, ids)
    };
    step("track", &track);
    step("console list", &|| {
        let call = get(&at("/api/owner/orders")).bearer(&owner).on("alpha");
        by!(via, site, crate::owner::orders_at, crate::owner::orders, call, &[])
    });
    step("confirm", &|| act("confirm"));
    step("kitchen board", &|| {
        let call = get(&at("/api/staff/kitchen?location_id=alpha")).bearer(&cook).on("alpha");
        by!(via, site, crate::services::orders::kitchen_ack::board::kitchen_orders_at, crate::services::orders::kitchen_ack::board::kitchen_orders, call, &[])
    });
    step("kitchen ack", &|| {
        let call = post(&at(&format!("/api/staff/orders/{id}/kitchen-ack")), &json!({"location_id": "alpha"})).bearer(&cook).on("alpha");
        by!(via, site, crate::services::orders::kitchen_ack::kitchen_ack_at, crate::services::orders::kitchen_ack::kitchen_ack, call, ids)
    });
    step("preparing", &|| act("preparing"));
    step("ready", &|| act("ready"));
    step("assign", &|| {
        let call = post(&at(&format!("/api/owner/orders/{id}/assign")), &json!({"location_id": "alpha", "courier_id": rider_id})).bearer(&owner).on("alpha");
        by!(via, site, crate::owner::assign_courier_at, crate::owner::assign_courier, call, ids)
    });
    step("courier tasks", &|| {
        let call = get(&at("/api/courier/tasks")).bearer(&rider).on("alpha");
        by!(via, site, crate::courier::tasks_at, crate::courier::tasks, call, &[])
    });
    let tap = |what: &str| post(&at(&format!("/api/courier/orders/{id}/{what}")), &json!({})).bearer(&rider).on("alpha");
    step("accept", &|| by!(via, site, crate::courier::accept_at, crate::courier::accept, tap("accept"), ids));
    step("pickup", &|| by!(via, site, crate::courier::pickup_at, crate::courier::pickup, tap("pickup"), ids));
    step("position", &|| {
        let call = post(&at("/api/courier/position"), &json!({"lat": 41.3231, "lon": 19.4414, "order_id": id})).bearer(&rider).on("alpha");
        by!(via, site, crate::courier::position_at, crate::courier::position, call, &[])
    });
    step("deliver", &|| {
        let call = post(&at(&format!("/api/courier/orders/{id}/deliver")), &json!({"cash_collected": 900})).bearer(&rider).on("alpha");
        by!(via, site, crate::courier::deliver_at, crate::courier::deliver, call, ids)
    });
    step("track done", &track);
    step("stamps", &|| {
        let call = get(&at(&format!("/api/order/{id}/stamps"))).bearer(&key).on("alpha");
        by!(via, site, crate::services::loyalty::handlers::order_stamps_at, crate::services::loyalty::handlers::order_stamps, call, ids)
    });
    step("feedback", &|| {
        let call = post(&at(&format!("/api/order/{id}/feedback")), &json!({"text": "tasty"})).bearer(&key).on("alpha");
        by!(via, site, crate::services::orders::feedback::feedback_at, crate::services::orders::feedback::feedback, call, ids)
    });
    let stored = site.fold("alpha", &format!("/fold/order?id={id}")).body_value();
    let mut order: Value = serde_json::from_str(stored["order_json"].as_str().unwrap_or("{}")).unwrap_or(Value::Null);
    // Ids are minted per site: what must agree is that the order ended with THIS rider.
    order["carried_by_the_rider"] = json!(order["courier_id"] == json!(rider_id));
    (steps, order)
}

/// The walk's table, printed (`--nocapture`): the lane's before/after.
fn table(via: Via, steps: &[Step]) -> usize {
    let total: usize = steps.iter().map(Step::venue).sum();
    for s in steps {
        eprintln!("COST {via:?} {:<14} {} venue {:>2} all {:>2} {:?}", s.name, s.status, s.venue(), s.paths.len(), s.paths);
    }
    eprintln!("COST {via:?} total venue-object requests {total}");
    total
}

/// The same order, the same end, both ways: only the bill moved.
fn the_order_as_it_ended(o: &Value) -> Value {
    json!({
        "status": o["status"], "rider": o["carried_by_the_rider"], "cash_collected": o["cash_collected"],
        "subtotal": o["subtotal"], "feedback": o["feedback"]["text"], "kitchen_seen": o["kitchen"]["seen"]["at"],
    })
}

/// EVERY STEP OF AN ORDER'S LIFE COSTS EXACTLY ONE REQUEST TO THE VENUE'S OBJECT, and ends
/// the order exactly where the Worker-run handlers end it.
#[test]
fn every_step_of_a_delivery_order_costs_exactly_one_venue_object_request() {
    let (before, ended_before) = a_delivery_order(&Site::new(), Via::Worker);
    let (after, ended_after) = a_delivery_order(&Site::new(), Via::Object);
    let (b, a) = (table(Via::Worker, &before), table(Via::Object, &after));
    for s in before.iter().chain(&after) {
        assert!(s.status < 300, "{} answered {}", s.name, s.status);
    }
    assert_eq!(the_order_as_it_ended(&ended_after), the_order_as_it_ended(&ended_before), "the move changed the order");
    assert_eq!((&ended_after["status"], &ended_after["carried_by_the_rider"]), (&json!("DELIVERED"), &json!(true)), "{ended_after}");
    let over: Vec<(&str, usize, &Vec<String>)> = after.iter().filter(|s| s.venue() != 1).map(|s| (s.name, s.venue(), &s.paths)).collect();
    assert!(over.is_empty(), "steps that are not one venue-object request: {over:?}");
    assert!(a < b, "after {a} is not below before {b}");
}
