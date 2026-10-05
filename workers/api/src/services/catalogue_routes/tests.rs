//! THE MOVE CANNOT CHANGE AN ANSWER (BN1, lane W-BN1A, 2026-10-02): the fifteen
//! handlers that pulled the catalogue image across the Worker<->object hop,
//! pinned on one fixture venue BEFORE the derivations moved into the object
//! (`/fold/*`), through the route seam (W-COV C2). Each pin is the handler's
//! status and body as the platform in memory answered it at HEAD c0262881+,
//! with only the generated ids (dish, orders, people, tokens) scrubbed, since
//! `getrandom` mints them. The body is compared through `serde_json::Value`,
//! so key order is canonical on both sides of the move.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::{open_venue, place_pickup};
use crate::wire::Reply;
use serde_json::{json, Value};

fn at(slug: &str, path: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}{path}")
}

/// The fixture venue: a dish with a recipe and a cooking time, a supply on the
/// shelf, a promo code, one pickup order placed by a guest, a waiter and a
/// counter-manager. Returns what the pins scrub.
struct Venue {
    owner: String,
    owner_id: String,
    dish: String,
    waiter: String,
    waiter_id: String,
    manager: String,
    manager_id: String,
    order: String,
    placed: Reply,
}

impl Venue {
    fn ids(&self) -> Vec<(String, &'static str)> {
        vec![
            (self.dish.clone(), "DISH"),
            (self.order.clone(), "ORDER"),
            (self.owner_id.clone(), "OWNER"),
            (self.waiter_id.clone(), "WAITER"),
            (self.manager_id.clone(), "MANAGER"),
        ]
    }
}

fn user_of(site: &Site, token: &str) -> String {
    match crate::auth::verify(&site.env(), token, site.now_ms).expect("a genuine token") {
        crate::auth::Claims::Owner { user_id, .. } => user_id,
        other => panic!("not an owner: {other:?}"),
    }
}

fn fixture(site: &Site) -> Venue {
    let (owner, dish) = open_venue(site, "alpha", "a@x.test");
    let r = site.run(
        crate::services::operations::supplies::set_supply,
        post(&at("alpha", "/api/owner/supplies"), &json!({"id": "salmon", "name": "Salmon", "unit": "g", "costPerBasis": 180, "lowAt": 100})).bearer(&owner).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "supply: {}", r.body_str());
    let r = site.run(
        crate::owner::update_product,
        post(&at("alpha", &format!("/api/owner/products/{dish}")), &json!({"location_id": "alpha", "cooking_min": 12, "bom": [{"supply": "salmon", "qty": 100}]}))
            .bearer(&owner)
            .on("alpha"),
        &[("id", &dish)],
    );
    assert_eq!(r.status_code(), 200, "recipe: {}", r.body_str());
    let r = site.run(
        crate::services::ordering::promotions::set_promotion,
        post(&at("alpha", "/api/owner/promotions"), &json!({"code": "SAVE10", "kind": "percent", "value": 10, "active": true})).bearer(&owner).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "promo: {}", r.body_str());
    let r = site.run(
        crate::services::operations::stock::stock_move,
        post(&at("alpha", "/api/owner/stock/received"), &json!({"item": "salmon", "qty": 5000})).bearer(&owner).on("alpha"),
        &[("kind", "received")],
    );
    assert_eq!(r.status_code(), 200, "received: {}", r.body_str());
    let (waiter, waiter_id) = site.staff_full("alpha", &owner, "w@x.test", "waiter");
    let (manager, manager_id) = site.staff_full("alpha", &owner, "m@x.test", "counter-manager");
    let placed = place_pickup(site, "alpha", &dish, 2);
    assert_eq!(placed.status_code(), 200, "{}", placed.body_str());
    let order = placed.body_value()["id"].as_str().expect("order id").to_string();
    let owner_id = user_of(site, &owner);
    Venue { owner, owner_id, dish, waiter, waiter_id, manager, manager_id, order, placed }
}

/// Every generated id replaced by its name, tokens blanked; then the status
/// and the canonical body.
fn scrub(v: &mut Value, ids: &[(String, &'static str)]) {
    match v {
        Value::String(s) => {
            for (id, name) in ids {
                if s.contains(id.as_str()) {
                    *s = s.replace(id.as_str(), name);
                }
            }
        }
        Value::Array(a) => a.iter_mut().for_each(|x| scrub(x, ids)),
        Value::Object(m) => {
            for (k, x) in m.iter_mut() {
                if k == "access_token" || k == "key" || k == "jti" {
                    *x = json!("TOKEN");
                } else if k == "sitting_id" {
                    // A sitting's id is minted at random when a waiter opens it.
                    *x = json!("SITTING");
                } else {
                    scrub(x, ids);
                }
            }
            // A key that IS an id (a names map, a per-dish table) is renamed too.
            let renamed: Vec<(String, String)> = m
                .keys()
                .filter_map(|k| ids.iter().find(|(id, _)| k == id).map(|(_, name)| (k.clone(), name.to_string())))
                .collect();
            for (from, to) in renamed {
                if let Some(x) = m.remove(&from) {
                    m.insert(to, x);
                }
            }
        }
        _ => {}
    }
}

fn pinned(r: &Reply, ids: &[(String, &'static str)]) -> String {
    // A refusal's body is its words, not JSON: pinned as sent.
    let Ok(mut v) = serde_json::from_slice::<Value>(r.body()) else {
        return format!("{} {}", r.status_code(), r.body_str());
    };
    scrub(&mut v, ids);
    format!("{} {}", r.status_code(), v)
}

/// The pin: the recorded answer, or -- while recording -- print it, so one
/// run of the module prints every pin (`every_pin_is_recorded` refuses an
/// empty one).
#[track_caller]
fn pin(name: &str, got: String, want: &str) {
    if want.is_empty() {
        eprintln!("PIN {name} {got}");
        return;
    }
    assert_eq!(got, want, "{name}");
}

/// A pin left empty is a test that passes without looking.
#[test]
fn every_pin_is_recorded() {
    let pins = [
        ("analytics", PIN_ANALYTICS), ("kitchen", PIN_KITCHEN), ("kitchen-staff", PIN_KITCHEN_STAFF), ("stock", PIN_STOCK),
        ("stock_move", PIN_STOCK_MOVE), ("stock_move-unknown", PIN_STOCK_MOVE_UNKNOWN), ("exceptions", PIN_EXCEPTIONS),
        ("exceptions-till", PIN_EXCEPTIONS_TILL), ("tips", PIN_TIPS), ("legs", PIN_LEGS), ("promo", PIN_PROMO),
        ("promo-unknown", PIN_PROMO_UNKNOWN), ("promo-nodish", PIN_PROMO_NODISH), ("eta", PIN_ETA), ("eta-pickup", PIN_ETA_PICKUP),
        ("eta-nocoords", PIN_ETA_NOCOORDS), ("eta-novenue", PIN_ETA_NOVENUE), ("aggregator", PIN_AGGREGATOR),
        ("aggregator-unknown", PIN_AGGREGATOR_UNKNOWN), ("place", PIN_PLACE), ("place-stored", PIN_PLACE_STORED),
        ("place-nodish", PIN_PLACE_NODISH), ("place-nopromo", PIN_PLACE_NOPROMO), ("place-promo", PIN_PLACE_PROMO),
        ("amend", PIN_AMEND), ("amend-nodish", PIN_AMEND_NODISH), ("transfer", PIN_TRANSFER), ("digest", PIN_DIGEST),
    ];
    let empty: Vec<&str> = pins.iter().filter(|(_, p)| p.is_empty()).map(|(n, _)| *n).collect();
    assert!(empty.is_empty(), "unrecorded pins: {empty:?}");
}

// ── the pins ────────────────────────────────────────────────────────────────

#[test]
fn analytics_answers_the_same_bytes() {
    let site = Site::new();
    let v = fixture(&site);
    let r = site.run(crate::services::analytics::analytics, get(&at("alpha", "/api/owner/analytics?days=7")).bearer(&v.owner).on("alpha"), &[]);
    pin("analytics", pinned(&r, &v.ids()), PIN_ANALYTICS);
}
const PIN_ANALYTICS: &str = r##"200 {"averageOrder":1800,"byChannel":{"storefront":1},"byDay":[{"at":1699398000000,"orders":0,"revenue":0},{"at":1699484400000,"orders":0,"revenue":0},{"at":1699570800000,"orders":0,"revenue":0},{"at":1699657200000,"orders":0,"revenue":0},{"at":1699743600000,"orders":0,"revenue":0},{"at":1699830000000,"orders":0,"revenue":0},{"at":1699916400000,"orders":1,"revenue":1800}],"byHour":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1],"currency":"ALL","days":7,"delivery":0,"dineIn":0,"orders":1,"pickup":1,"rejected":0,"revenue":1800,"topProducts":[{"id":"DISH","name":"Futomaki","quantity":2,"revenue":1800}]}"##;

#[test]
fn the_kitchens_numbers_answer_the_same_bytes() {
    let site = Site::new();
    let v = fixture(&site);
    let r = site.run(crate::services::analytics::kitchen::kitchen, get(&at("alpha", "/api/owner/analytics/kitchen")).bearer(&v.owner).on("alpha"), &[]);
    pin("kitchen", pinned(&r, &v.ids()), PIN_KITCHEN);
    // The kitchen's own view: the same numbers with the money stripped.
    let cook = site.staff("alpha", &v.owner, "k@x.test", "kitchen");
    let r = site.run(crate::services::analytics::kitchen::kitchen, get(&at("alpha", "/api/owner/analytics/kitchen")).bearer(&cook).on("alpha"), &[]);
    pin("kitchen-staff", pinned(&r, &v.ids()), PIN_KITCHEN_STAFF);
}
const PIN_KITCHEN: &str = r##"200 {"avt":{"countTwice":[],"notTracked":[],"revenue":1800,"rows":[],"thresholdPm":30,"top":[]},"byDay":[{"cogs":0,"day":"2023-11-08","foodCostPm":null,"orders":0,"received":0,"revenue":0,"waste":0},{"cogs":0,"day":"2023-11-09","foodCostPm":null,"orders":0,"received":0,"revenue":0,"waste":0},{"cogs":0,"day":"2023-11-10","foodCostPm":null,"orders":0,"received":0,"revenue":0,"waste":0},{"cogs":0,"day":"2023-11-11","foodCostPm":null,"orders":0,"received":0,"revenue":0,"waste":0},{"cogs":0,"day":"2023-11-12","foodCostPm":null,"orders":0,"received":0,"revenue":0,"waste":0},{"cogs":0,"day":"2023-11-13","foodCostPm":null,"orders":0,"received":0,"revenue":0,"waste":0},{"cogs":360,"day":"2023-11-14","foodCostPm":200,"orders":1,"received":0,"revenue":1800,"waste":0}],"currency":"ALL","days":["2023-11-08","2023-11-09","2023-11-10","2023-11-11","2023-11-12","2023-11-13","2023-11-14"],"dishes":[{"byDay":[0,0,0,0,0,0,2],"cogs":360,"foodCostPm":200,"hasRecipe":true,"id":"DISH","margin":1440,"marginPortion":720,"name":"Futomaki","portionCost":180,"revenue":1800,"sold":2}],"from":"2023-11-08","ingredients":[{"adu":29,"available":4800,"byDay":[0,0,0,0,0,0,200],"cleanLossG":0,"cookLossG":0,"cooked":0,"cost":360,"counted":true,"daysCover":165,"drawn":0,"drawnByDay":[0,0,0,0,0,0,0],"drawnValue":0,"drift":0,"driftValue":0,"grossG":200,"id":"salmon","made":0,"name":"Salmon","netG":200,"outG":200,"prepIn":0,"prepOut":0,"received":5000,"receivedValue":0,"reorder":null,"unit":"g","used":200,"wasted":0,"wastedValue":0}],"prices":[],"to":"2023-11-14","totals":{"cleanLossG":0,"cogs":360,"cookLossG":0,"driftValue":0,"foodCostPm":200,"margin":1440,"orders":1,"receivedValue":0,"revenue":1800,"uncosted":0,"undated":0,"unmodelled":0,"wasteValue":0},"waste":[],"yields":[]}"##;
const PIN_KITCHEN_STAFF: &str = r##"200 {"avt":{"countTwice":[],"notTracked":[],"rows":[],"thresholdPm":30,"top":[]},"byDay":[{"cogs":0,"day":"2023-11-08","orders":0,"received":0,"waste":0},{"cogs":0,"day":"2023-11-09","orders":0,"received":0,"waste":0},{"cogs":0,"day":"2023-11-10","orders":0,"received":0,"waste":0},{"cogs":0,"day":"2023-11-11","orders":0,"received":0,"waste":0},{"cogs":0,"day":"2023-11-12","orders":0,"received":0,"waste":0},{"cogs":0,"day":"2023-11-13","orders":0,"received":0,"waste":0},{"cogs":360,"day":"2023-11-14","orders":1,"received":0,"waste":0}],"currency":"ALL","days":["2023-11-08","2023-11-09","2023-11-10","2023-11-11","2023-11-12","2023-11-13","2023-11-14"],"dishes":[{"byDay":[0,0,0,0,0,0,2],"cogs":360,"hasRecipe":true,"id":"DISH","name":"Futomaki","portionCost":180,"sold":2}],"from":"2023-11-08","ingredients":[{"adu":29,"available":4800,"byDay":[0,0,0,0,0,0,200],"cleanLossG":0,"cookLossG":0,"cooked":0,"cost":360,"counted":true,"daysCover":165,"drawn":0,"drawnByDay":[0,0,0,0,0,0,0],"drawnValue":0,"drift":0,"driftValue":0,"grossG":200,"id":"salmon","made":0,"name":"Salmon","netG":200,"outG":200,"prepIn":0,"prepOut":0,"received":5000,"receivedValue":0,"reorder":null,"unit":"g","used":200,"wasted":0,"wastedValue":0}],"prices":[],"scope":"kitchen","to":"2023-11-14","totals":{"cleanLossG":0,"cogs":360,"cookLossG":0,"driftValue":0,"orders":1,"receivedValue":0,"uncosted":0,"undated":0,"unmodelled":0,"wasteValue":0},"waste":[],"yields":[]}"##;

#[test]
fn the_shelf_answers_the_same_bytes() {
    let site = Site::new();
    let v = fixture(&site);
    let r = site.run(crate::services::operations::stock::stock, get(&at("alpha", "/api/owner/stock")).bearer(&v.owner).on("alpha"), &[]);
    pin("stock", pinned(&r, &v.ids()), PIN_STOCK);
}
const PIN_STOCK: &str = r##"200 {"expiryWarnDays":2,"noRecipe":[],"orderList":{"aduDays":14,"groups":[]},"recent":[{"at":1700000000000,"by":"OWNER","doc":null,"item":"salmon","kind":"received","lot":null,"qty":5000,"seq":0,"supplier":null,"value":null}],"sessions":[],"storages":[{"archived":false,"builtIn":true,"default":true,"id":"kitchen","name":""},{"archived":false,"builtIn":true,"default":false,"id":"bar","name":""},{"archived":false,"builtIn":true,"default":false,"id":"freezer","name":""}],"stranded":[{"item":"salmon","order":"ORDER","qty":200}],"supplierAliases":{},"supplierCards":[],"suppliers":[],"supplies":[{"available":4800,"barcode":null,"byStore":{"home":"kitchen","stores":{"kitchen":5000}},"carbsPer100":null,"category":"","cleanPm":null,"code":null,"cookPm":null,"costPerBasis":180,"counted":true,"expired":0,"expiring":0,"fatPer100":null,"id":"salmon","kcalPer100":null,"kind":"food_ingredient","lastCount":null,"lots":[{"at":1700000000000,"code":"#0","daysLeft":null,"doc":null,"expiry":null,"left":5000,"received":5000,"supplier":null}],"low":false,"lowAt":100,"measuredCleanPm":null,"measuredCookPm":null,"moves":[{"at":1700000000000,"by":"OWNER","doc":null,"item":"salmon","kind":"received","lot":null,"qty":5000,"seq":0,"supplier":null,"value":null}],"name":"Salmon","needsCount":false,"nutritionBasis":null,"nutritionConfirmed":false,"onHand":5000,"packs":null,"prices":[],"proteinPer100":null,"reserved":200,"shelfDays":null,"supplier":null,"unit":"g","wac":null,"weightPerUnit":null,"yields":[]}],"today":"2023-11-14"}"##;

#[test]
fn a_stock_movement_answers_the_same_bytes() {
    let site = Site::new();
    let v = fixture(&site);
    let r = site.run(
        crate::services::operations::stock::stock_move,
        post(&at("alpha", "/api/owner/stock/wasted"), &json!({"item": "salmon", "qty": 150, "reason": "spoiled"})).bearer(&v.manager).on("alpha"),
        &[("kind", "wasted")],
    );
    pin("stock_move", pinned(&r, &v.ids()), PIN_STOCK_MOVE);
    let r = site.run(
        crate::services::operations::stock::stock_move,
        post(&at("alpha", "/api/owner/stock/received"), &json!({"item": "nobody", "qty": 1})).bearer(&v.owner).on("alpha"),
        &[("kind", "received")],
    );
    pin("stock_move-unknown", pinned(&r, &v.ids()), PIN_STOCK_MOVE_UNKNOWN);
}
const PIN_STOCK_MOVE: &str = r##"200 {"kind":"wasted","lines":[{"item":"salmon","qty":150,"value":null}],"ok":true,"value":0}"##;
const PIN_STOCK_MOVE_UNKNOWN: &str = r##"404 not found: nobody"##;

#[test]
fn the_exception_report_answers_the_same_bytes() {
    let site = Site::new();
    let v = fixture(&site);
    // One exception: the order refunded after the kitchen started it.
    for a in ["confirm", "preparing"] {
        let r = site.run(
            crate::owner::order_action,
            post(&at("alpha", &format!("/api/owner/orders/{}/action", v.order)), &json!({"location_id": "alpha", "action": a})).bearer(&v.owner).on("alpha"),
            &[("id", &v.order)],
        );
        assert_eq!(r.status_code(), 200, "{a}: {}", r.body_str());
    }
    let r = site.run(
        crate::services::orders::refund::refund,
        post(&at("alpha", &format!("/api/staff/orders/{}/refund", v.order)), &json!({"location_id": "alpha", "reason": "customer_request"})).bearer(&v.manager).on("alpha"),
        &[("id", &v.order)],
    );
    assert_eq!(r.status_code(), 200, "refund: {}", r.body_str());
    let r = site.run(crate::exceptions::exceptions, get(&at("alpha", "/api/owner/exceptions")).bearer(&v.owner).on("alpha"), &[]);
    pin("exceptions", pinned(&r, &v.ids()), PIN_EXCEPTIONS);
    let r = site.run(crate::exceptions::exceptions, get(&at("alpha", "/api/owner/exceptions?period=till")).bearer(&v.owner).on("alpha"), &[]);
    pin("exceptions-till", pinned(&r, &v.ids()), PIN_EXCEPTIONS_TILL);
}
const PIN_EXCEPTIONS: &str = r##"200 {"alertChat":false,"from":1699913600000,"groups":[{"amount":{"ALL":0},"count":1,"kind":"refund","reason":"customer_request"}],"lateMinutes":30,"names":{"MANAGER":"Staff"},"rounds":[{"kinds":["refund"],"orderId":"ORDER"}],"rows":[{"amount":0,"at":1700000000000,"by":"MANAGER","currency":"ALL","kind":"refund","order_id":"ORDER","reason":"customer_request","till_id":null}],"threshold":3,"to":1700000000000,"venue":"alpha"}"##;
const PIN_EXCEPTIONS_TILL: &str = r##"200 {"alertChat":false,"from":1699913600000,"groups":[{"amount":{"ALL":0},"count":1,"kind":"refund","reason":"customer_request"}],"lateMinutes":30,"names":{"MANAGER":"Staff"},"rounds":[{"kinds":["refund"],"orderId":"ORDER"}],"rows":[{"amount":0,"at":1700000000000,"by":"MANAGER","currency":"ALL","kind":"refund","order_id":"ORDER","reason":"customer_request","till_id":null}],"threshold":3,"to":1700000000000,"venue":"alpha"}"##;

#[test]
fn the_tips_answer_the_same_bytes() {
    let site = Site::new();
    let v = fixture(&site);
    let r = site.run(crate::services::orders::room::till::tips, get(&at("alpha", "/api/staff/till/tips?location_id=alpha")).bearer(&v.manager).on("alpha"), &[]);
    pin("tips", pinned(&r, &v.ids()), PIN_TIPS);
}
const PIN_TIPS: &str = r##"200 {"day":true,"from_ms":1699916400000,"names":{},"tips":[],"to_ms":1700000000000}"##;

#[test]
fn the_wallet_legs_audit_answers_the_same_bytes() {
    let site = Site::new();
    let v = fixture(&site);
    let r = site.run(crate::services::orders::legs::audit, get(&at("alpha", "/api/owner/wallet/legs")).bearer(&v.owner).on("alpha"), &[]);
    pin("legs", pinned(&r, &v.ids()), PIN_LEGS);
}
const PIN_LEGS: &str = r##"200 {"audit":{"mismatched":[],"missing":[],"orphans":[]},"handBack":[],"holds":true,"spends":[],"venue":"alpha"}"##;

#[test]
fn the_promo_preview_answers_the_same_bytes() {
    let site = Site::new();
    let v = fixture(&site);
    let check = |code: &str, qty: i64| {
        site.run(
            crate::services::ordering::preview::promo_check,
            post(&at("alpha", "/api/promo/check"), &json!({"code": code, "items": [{"product_id": v.dish, "quantity": qty}]})).on("alpha"),
            &[],
        )
    };
    pin("promo", pinned(&check("save10", 2), &v.ids()), PIN_PROMO);
    pin("promo-unknown", pinned(&check("NOPE", 1), &v.ids()), PIN_PROMO_UNKNOWN);
    let r = site.run(
        crate::services::ordering::preview::promo_check,
        post(&at("alpha", "/api/promo/check"), &json!({"code": "SAVE10", "items": [{"product_id": "nobody", "quantity": 1}]})).on("alpha"),
        &[],
    );
    pin("promo-nodish", pinned(&r, &v.ids()), PIN_PROMO_NODISH);
}
const PIN_PROMO: &str = r##"200 {"code":"SAVE10","discount":180,"subtotal":1800,"total":1620}"##;
const PIN_PROMO_UNKNOWN: &str = r##"400 unknown code"##;
const PIN_PROMO_NODISH: &str = r##"400 unknown product: nobody"##;

#[test]
fn the_estimate_answers_the_same_bytes() {
    let site = Site::new();
    let v = fixture(&site);
    let quote = |body: Value| site.run(crate::eta::quote, post(&at("alpha", "/api/public/locations/alpha/eta"), &body).on("alpha"), &[("slug", "alpha")]);
    // The dish's own cooking time (12) comes from the catalogue.
    pin("eta", pinned(&quote(json!({"items": [{"id": v.dish, "quantity": 2}], "distanceM": 2500})), &v.ids()), PIN_ETA);
    pin("eta-pickup", pinned(&quote(json!({"items": [{"id": v.dish}], "pickup": true})), &v.ids()), PIN_ETA_PICKUP);
    pin("eta-nocoords", pinned(&quote(json!({"items": [{"id": v.dish}], "latUdeg": 1, "lonUdeg": 1})), &v.ids()), PIN_ETA_NOCOORDS);
    let r = site.run(crate::eta::quote, post(&at("alpha", "/api/public/locations/nobody/eta"), &json!({"items": [{"id": v.dish}]})).on("nobody"), &[("slug", "nobody")]);
    pin("eta-novenue", pinned(&r, &v.ids()), PIN_ETA_NOVENUE);
}
const PIN_ETA: &str = r##"200 {"distanceM":2500,"highMin":47,"lowMin":31,"ordersAhead":0,"parts":{"overheadMin":8,"prepMin":14,"queueMin":0,"travelMin":9},"range":"31–47"}"##;
const PIN_ETA_PICKUP: &str = r##"200 {"distanceM":0,"highMin":18,"lowMin":12,"ordersAhead":0,"parts":{"overheadMin":0,"prepMin":12,"queueMin":0,"travelMin":0},"range":"12–18"}"##;
const PIN_ETA_NOCOORDS: &str = r##"409 this venue has not set its own location, so a distance cannot be measured"##;
const PIN_ETA_NOVENUE: &str = r##"404 not found"##;

#[test]
fn an_aggregator_order_answers_the_same_bytes() {
    let site = Site::new();
    let v = fixture(&site);
    let enter = |lines: Value| {
        site.run(
            crate::services::orders::aggregator::enter,
            post(&at("alpha", "/api/staff/orders/aggregator"), &json!({"location_id": "alpha", "channel": "wolt", "external_id": "W-1", "lines": lines, "total": 1000}))
                .bearer(&v.waiter)
                .on("alpha"),
            &[],
        )
    };
    pin("aggregator", pinned(&enter(json!([{"product_id": v.dish, "quantity": 1, "unit_price": 1000}])), &v.ids()), PIN_AGGREGATOR);
    pin("aggregator-unknown", pinned(&enter(json!([{"product_id": "nobody", "quantity": 1, "unit_price": 1000}])), &v.ids()), PIN_AGGREGATOR_UNKNOWN);
}
const PIN_AGGREGATOR: &str = r##"200 {"existing":false,"order":{"at":{"CONFIRMED":1700000000000},"cash_pay_with":null,"channel":"wolt","contact":{"name":"","phone":""},"created_at_ms":1700000000000,"currency":"ALL","customer_id":null,"delivery_fee":0,"discount":0,"entered_by":"WAITER","external":{"order_id":"W-1","source":"wolt"},"fulfilment":{"code":"","fee":0,"kind":"pickup"},"id":"wolt-W-1","items":[{"modifier_ids":[],"name":"Futomaki","product_id":"DISH","quantity":1,"unit_price":1000}],"location_id":"alpha","payment":"platform","payment_status":"paid","price_trusted":false,"status":"CONFIRMED","subtotal":1000,"tip":0,"total":1000}}"##;
const PIN_AGGREGATOR_UNKNOWN: &str = r##"400 nobody is not on this venue's menu"##;

#[test]
fn a_placement_answers_the_same_bytes_and_reserves_the_shelf() {
    let site = Site::new();
    let v = fixture(&site);
    pin("place", pinned(&v.placed, &v.ids()), PIN_PLACE);
    // What the object stored for it, and what the shelf holds against it.
    let stored = site.fold("alpha", &format!("/fold/order?id={}", v.order));
    pin("place-stored", pinned(&stored, &v.ids()), PIN_PLACE_STORED);
    let shelf = site.run(crate::services::operations::stock::stock, get(&at("alpha", "/api/owner/stock")).bearer(&v.owner).on("alpha"), &[]);
    assert_eq!(shelf.body_value()["supplies"][0]["reserved"], json!(200), "2 x 100 g of salmon held: {}", shelf.body_str());
    // The refusals the catalogue decides: a dish that is not on the menu, a code that is not one.
    let r = site.run(
        crate::storefront::place,
        post(&at("alpha", "/api/public/locations/alpha/orders"), &json!({"items": [{"product_id": "nobody", "quantity": 1}], "contact": {"name": "G", "phone": "+355690000001"}, "fulfilment": {"kind": "pickup"}, "payment": "cash"})).on("alpha"),
        &[("slug", "alpha")],
    );
    pin("place-nodish", pinned(&r, &v.ids()), PIN_PLACE_NODISH);
    let r = site.run(
        crate::storefront::place,
        post(&at("alpha", "/api/public/locations/alpha/orders"), &json!({"items": [{"product_id": v.dish, "quantity": 1}], "contact": {"name": "G", "phone": "+355690000001"}, "fulfilment": {"kind": "pickup"}, "payment": "cash", "promo": "NOPE"})).on("alpha"),
        &[("slug", "alpha")],
    );
    pin("place-nopromo", pinned(&r, &v.ids()), PIN_PLACE_NOPROMO);
    let r = site.run(
        crate::storefront::place,
        post(&at("alpha", "/api/public/locations/alpha/orders"), &json!({"items": [{"product_id": v.dish, "quantity": 1}], "contact": {"name": "G", "phone": "+355690000001"}, "fulfilment": {"kind": "pickup"}, "payment": "cash", "promo": "save10"})).on("alpha"),
        &[("slug", "alpha")],
    );
    let ids = {
        let mut i = v.ids();
        if let Some(id) = r.body_value()["id"].as_str() {
            i.push((id.to_string(), "ORDER2"));
        }
        i
    };
    pin("place-promo", pinned(&r, &ids), PIN_PLACE_PROMO);
}
const PIN_PLACE: &str = r##"200 {"access_token":"TOKEN","cash_pay_with":null,"channel":"storefront","contact":{"name":"Guest","phone":"+355690000001"},"created_at_ms":1700000000000,"customer_id":null,"delivery_fee":0,"fulfilment":{"address":null,"fee":0,"kind":"pickup","note":null,"table":null},"id":"ORDER","items":[{"modifier_ids":[],"name":"Futomaki","product_id":"DISH","quantity":2,"unit_price":900}],"location_id":"alpha","payment":"cash","status":"PENDING","subtotal":1800,"tip":0,"total":1800}"##;
const PIN_PLACE_STORED: &str = r##"200 {"kind":1,"order_id":"ORDER","order_json":"{\"cash_pay_with\":null,\"channel\":\"storefront\",\"contact\":{\"name\":\"Guest\",\"phone\":\"+355690000001\"},\"created_at_ms\":1700000000000,\"customer_id\":null,\"delivery_fee\":0,\"fulfilment\":{\"address\":null,\"fee\":0,\"kind\":\"pickup\",\"note\":null,\"table\":null},\"id\":\"ORDER\",\"items\":[{\"modifier_ids\":[],\"name\":\"Futomaki\",\"product_id\":\"DISH\",\"quantity\":2,\"unit_price\":900}],\"location_id\":\"alpha\",\"payment\":\"cash\",\"status\":\"PENDING\",\"subtotal\":1800,\"tip\":0,\"total\":1800}","seq":1700000000000}"##;
const PIN_PLACE_NODISH: &str = r##"400 unknown product: nobody"##;
const PIN_PLACE_NOPROMO: &str = r##"400 unknown code"##;
const PIN_PLACE_PROMO: &str = r##"200 {"access_token":"TOKEN","cash_pay_with":null,"channel":"storefront","contact":{"name":"G","phone":"+355690000001"},"created_at_ms":1700000000000,"customer_id":null,"delivery_fee":0,"discount":90,"fulfilment":{"address":null,"fee":0,"kind":"pickup","note":null,"table":null},"id":"ORDER2","items":[{"modifier_ids":[],"name":"Futomaki","product_id":"DISH","quantity":1,"unit_price":900}],"location_id":"alpha","payment":"cash","promo":{"code":"SAVE10","discount":90},"status":"PENDING","subtotal":900,"tip":0,"total":810}"##;

fn dine_in(site: &Site, v: &Venue, table: &str) -> String {
    let r = site.run(
        crate::storefront::place,
        post(
            &at("alpha", "/api/public/locations/alpha/orders"),
            &json!({"items": [{"product_id": v.dish, "quantity": 1}], "contact": {"name": "Table", "phone": ""}, "fulfilment": {"kind": "dine_in", "table": table}, "payment": "cash"}),
        )
        .bearer(&v.waiter)
        .on("alpha"),
        &[("slug", "alpha")],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_value()["id"].as_str().expect("round id").to_string()
}

fn seq_of(site: &Site, id: &str) -> u64 {
    site.fold("alpha", &format!("/fold/order?id={id}")).body_value()["seq"].as_u64().unwrap_or(0)
}

#[test]
fn an_amendment_answers_the_same_bytes() {
    let site = Site::new();
    let v = fixture(&site);
    let round = dine_in(&site, &v, "T1");
    let mut ids = v.ids();
    ids.push((round.clone(), "ROUND"));
    let amend = |ops: Value| {
        site.run(
            crate::services::orders::room::handlers::amend,
            post(&at("alpha", &format!("/api/staff/orders/{round}/amend")), &json!({"location_id": "alpha", "base_seq": seq_of(&site, &round), "ops": ops})).bearer(&v.waiter).on("alpha"),
            &[("id", &round)],
        )
    };
    pin("amend", pinned(&amend(json!([{"op": "add", "product_id": v.dish, "quantity": 2}])), &ids), PIN_AMEND);
    pin("amend-nodish", pinned(&amend(json!([{"op": "add", "product_id": "nobody", "quantity": 1}])), &ids), PIN_AMEND_NODISH);
    let shelf = site.run(crate::services::operations::stock::stock, get(&at("alpha", "/api/owner/stock")).bearer(&v.owner).on("alpha"), &[]);
    assert_eq!(shelf.body_value()["supplies"][0]["reserved"], json!(500), "the round's recipe is held too: {}", shelf.body_str());
}
const PIN_AMEND: &str = r##"200 {"order":{"amended":[{"at":1700000000000,"by":"WAITER","ops":[{"line":{"modifier_ids":[],"name":"Futomaki","product_id":"DISH","quantity":2,"unit_price":900},"op":"add"}],"reason":null}],"cash_pay_with":null,"channel":"console","contact":{"name":"Table","phone":""},"created_at_ms":1700000000000,"customer_id":null,"delivery_fee":0,"discount":0,"fulfilment":{"address":null,"fee":0,"kind":"dine_in","note":null,"table":"T1"},"id":"ROUND","items":[{"modifier_ids":[],"name":"Futomaki","product_id":"DISH","quantity":1,"unit_price":900},{"modifier_ids":[],"name":"Futomaki","product_id":"DISH","quantity":2,"unit_price":900}],"location_id":"alpha","payment":"cash","placed_by":"WAITER","sitting_id":"SITTING","status":"PENDING","subtotal":2700,"tip":0,"total":2700},"seq":1700000000001}"##;
const PIN_AMEND_NODISH: &str = r##"400 unknown product: nobody"##;

#[test]
fn a_transfer_answers_the_same_bytes() {
    let site = Site::new();
    let v = fixture(&site);
    let a = dine_in(&site, &v, "T1");
    let b = dine_in(&site, &v, "T2");
    let mut ids = v.ids();
    ids.push((a.clone(), "ROUND-A"));
    ids.push((b.clone(), "ROUND-B"));
    let r = site.run(
        crate::services::orders::room::handlers::amend,
        post(&at("alpha", &format!("/api/staff/orders/{a}/amend")), &json!({"location_id": "alpha", "base_seq": seq_of(&site, &a), "ops": [{"op": "add", "product_id": v.dish, "quantity": 2}]})).bearer(&v.waiter).on("alpha"),
        &[("id", &a)],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let r = site.run(
        crate::services::orders::room::transfer::transfer,
        post(&at("alpha", &format!("/api/staff/orders/{a}/transfer")), &json!({"location_id": "alpha", "to_order_id": b, "from_base_seq": seq_of(&site, &a), "to_base_seq": seq_of(&site, &b), "lines": [1]}))
            .bearer(&v.waiter)
            .on("alpha"),
        &[("id", &a)],
    );
    pin("transfer", pinned(&r, &ids), PIN_TRANSFER);
    let shelf = site.run(crate::services::operations::stock::stock, get(&at("alpha", "/api/owner/stock")).bearer(&v.owner).on("alpha"), &[]);
    assert_eq!(shelf.body_value()["supplies"][0]["reserved"], json!(600), "the shelf still holds every line: {}", shelf.body_str());
}
const PIN_TRANSFER: &str = r##"200 {"from":{"order":{"amended":[{"at":1700000000000,"by":"WAITER","ops":[{"line":{"modifier_ids":[],"name":"Futomaki","product_id":"DISH","quantity":2,"unit_price":900},"op":"add"}],"reason":null},{"at":1700000000000,"by":"WAITER","transfer":{"amount":1800,"comp":0,"dir":"out","id":"ROUND-A>ROUND-B@1700000000000","lines":1,"other":"ROUND-B"}}],"cash_pay_with":null,"channel":"console","contact":{"name":"Table","phone":""},"created_at_ms":1700000000000,"customer_id":null,"delivery_fee":0,"discount":0,"fulfilment":{"address":null,"fee":0,"kind":"dine_in","note":null,"table":"T1"},"id":"ROUND-A","items":[{"modifier_ids":[],"name":"Futomaki","product_id":"DISH","quantity":1,"unit_price":900}],"location_id":"alpha","payment":"cash","placed_by":"WAITER","sitting_id":"SITTING","status":"PENDING","subtotal":900,"tip":0,"total":900},"seq":1700000000002},"to":{"order":{"amended":[{"at":1700000000000,"by":"WAITER","transfer":{"amount":1800,"comp":0,"dir":"in","id":"ROUND-A>ROUND-B@1700000000000","lines":1,"other":"ROUND-A"}}],"cash_pay_with":null,"channel":"console","contact":{"name":"Table","phone":""},"created_at_ms":1700000000000,"customer_id":null,"delivery_fee":0,"discount":0,"fulfilment":{"address":null,"fee":0,"kind":"dine_in","note":null,"table":"T2"},"id":"ROUND-B","items":[{"modifier_ids":[],"name":"Futomaki","product_id":"DISH","quantity":1,"unit_price":900},{"modifier_ids":[],"name":"Futomaki","product_id":"DISH","quantity":2,"unit_price":900}],"location_id":"alpha","payment":"cash","placed_by":"WAITER","sitting_id":"SITTING","status":"PENDING","subtotal":2700,"tip":0,"total":2700},"seq":1700000000001}}"##;

#[test]
fn the_digests_fold_answers_the_same_names_and_currency() {
    let site = Site::new();
    let v = fixture(&site);
    let place = crate::hubstore::Place::of_authorised(&site.ctx(&[]), "alpha").unwrap();
    let mut f = crate::outbox::digest_rail::Folded::default();
    crate::edge::mem::block_on(crate::outbox::digest_rail::fold(&place, &mut f)).unwrap();
    let mut got = json!({ "orders": f.orders.len(), "names": f.names, "currency": f.currency, "loaded": f.loaded });
    scrub(&mut got, &v.ids());
    pin("digest", got.to_string(), PIN_DIGEST);
}
const PIN_DIGEST: &str = r##"{"currency":"ALL","loaded":true,"names":{"DISH":"Futomaki"},"orders":1}"##;

/// The basket's ledger record is the recipe EXPANDED through its semi-finished
/// products (`prep::for_ledger`), not the stored one. The fixture venue above
/// has only raw lines, where the two are the same bytes -- the first mutation
/// check of W-BN1A (expansion skipped) passed through every pin -- so this is
/// the twin that tells them apart, against the real `for_ledger`.
#[test]
fn the_basket_expands_a_semi_finished_recipe_for_the_ledger() {
    use crate::services::ordering::basket::answer;
    let mut cat = dowiz_hub::catalog::Catalog::create().unwrap();
    cat.set_supply("rice", r#"{"id":"rice","name":"Rice","unit":"g","kind":"food_ingredient","costPerBasis":20}"#);
    cat.set_supply("vinegar", r#"{"id":"vinegar","name":"Vinegar","unit":"ml","kind":"food_ingredient","costPerBasis":15}"#);
    cat.set_supply(
        "rice-seasoned",
        r#"{"id":"rice-seasoned","name":"Seasoned rice","unit":"g","kind":"prep","card":{"lines":[{"item":"rice","qty":1000},{"item":"vinegar","qty":100}],"yield":1100}}"#,
    );
    cat.set_product("roll", r#"{"id":"roll","name":"Roll","price":650,"bom":[{"supply":"rice-seasoned","qty":110}]}"#);
    let v = answer(&cat, &["roll".into(), "nobody".into()], Some("NOPE"));
    let stored = cat.product("roll").unwrap();
    let ledger = v["ledger"]["roll"].as_str().expect("a ledger record for the roll");
    assert_ne!(ledger, stored, "the ledger record is the expansion, not the stored recipe");
    assert_eq!(ledger, dowiz_hub::prep::for_ledger(&|s| cat.supply(s), &stored).0);
    let leaves = dowiz_hub::stock::bom_of(ledger);
    assert!(leaves.iter().any(|l| l.supply == "rice") && leaves.iter().any(|l| l.supply == "vinegar"), "{ledger}");
    assert!(leaves.iter().all(|l| l.supply != "rice-seasoned"), "{ledger}");
    // What is not there is not invented: an unknown dish, an unknown code, no venue record.
    assert!(v["products"].get("nobody").is_none() && v["ledger"].get("nobody").is_none());
    assert_eq!((v["promo"].clone(), v["venue"].clone()), (Value::Null, Value::Null));
    assert_eq!(v["products"]["roll"]["price"], json!(650));
}
