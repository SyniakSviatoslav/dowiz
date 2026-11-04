//! The routes W-OWNER-CPU2 moved into the venue's object: every kind of the lane has its
//! parameters and its handler; the refund (flows gate F4's 503) runs in the object and answers as
//! its body did, costing the Worker one object request and no image; the owner console's reads
//! and the public routes answer byte for byte what the handler answered from the Worker.

use super::{in_object, params, routed, routed_slug};
use crate::edge::mem::{block_on, object_paths};
use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::hubdo::command::Kind;
use crate::storefront::route_tests::{open_venue, place_pickup};
use crate::wire::{Call, Reply};
use serde_json::json;

/// Every kind this lane added, as `command.rs` lists them.
const KINDS: &[Kind] = &[
    Kind::Refund, Kind::Returned, Kind::GuestConfirm, Kind::Sitting, Kind::StaffRoom, Kind::StaffFloor, Kind::TillTips, Kind::KitchenAssist, Kind::CourierAssist,
    Kind::Customers, Kind::CustomerReveal, Kind::CustomerForget, Kind::Reveals, Kind::CustomerRecord, Kind::CustomerRekey, Kind::Reforget, Kind::CourierDetail,
    Kind::CampaignList, Kind::CampaignDefine, Kind::CampaignReport, Kind::CampaignPreview, Kind::CampaignSend, Kind::Inbox, Kind::WalletLegs, Kind::WalletLegsRepair,
    Kind::Posts, Kind::DraftPost, Kind::Graph, Kind::OwnerAssist, Kind::AnalyticsKitchen, Kind::Exceptions, Kind::Health, Kind::OptionBomRead, Kind::OptionBomWrite,
    Kind::BagCard, Kind::BagSet, Kind::BagQr, Kind::PromoCheck, Kind::Voice, Kind::Welcome, Kind::ThreadMessages, Kind::ThreadSend,
];

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}

/// The object requests `f` made, by path, and the answer.
fn paid(f: impl FnOnce() -> Reply) -> (Vec<String>, Reply) {
    let from = object_paths().len();
    let r = f();
    (object_paths()[from..].to_vec(), r)
}

#[test]
fn every_kind_of_this_lane_has_its_parameters_and_its_handler_and_no_other_kind_does() {
    let site = Site::new();
    for k in KINDS {
        assert!(params(*k).is_some(), "{k:?} names no parameters");
        assert_eq!(params(*k).unwrap(), k.params(), "{k:?}: `Kind::params` answers through `relayed::params`");
        let call = Call::new("https://alpha.dowiz.org/x", worker::Method::Get).unwrap();
        assert!(in_object(*k, call, site.ctx(&[])).is_ok(), "{k:?} has no handler in the object");
    }
    for k in [Kind::Place, Kind::ConsoleOrders, Kind::OwnerLogin, Kind::ProductEdit] {
        assert!(params(k).is_none(), "{k:?} is not this lane's");
        let call = Call::new("https://alpha.dowiz.org/x", worker::Method::Get).unwrap();
        assert!(in_object(k, call, site.ctx(&[])).is_err(), "{k:?} is handed back");
    }
    assert_eq!(params(Kind::Refund), Some(&["id"][..]));
    assert_eq!(params(Kind::CustomerReveal), Some(&["key"][..]));
    assert_eq!(params(Kind::ThreadSend), Some(&["slug", "id"][..]));
}

/// FLOWS GATE F4 (`refund <order> from READY: 503 <!DOCTYPE html>` on qa-durres): the refund
/// ran in the stateless Worker -- the identity image, the idempotency table twice, the merged
/// order -- and died of `exceededCpu`. Now the Worker sends ONE request and the venue's object
/// runs the same handler; its refusals and its answer are the handler's own.
#[test]
fn the_refund_runs_in_the_venues_object_and_answers_as_its_body_did() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let a = place_pickup(&site, "alpha", &dish, 1).body_value()["id"].as_str().unwrap().to_string();
    let b = place_pickup(&site, "alpha", &dish, 1).body_value()["id"].as_str().unwrap().to_string();
    let refund = |id: &str, body: serde_json::Value| post(&at(&format!("/api/staff/orders/{id}/refund")), &body).bearer(&owner).on("alpha");
    let body = json!({"location_id": "alpha", "reason": "venue_cancelled"});
    // THE ORACLE, a refusal that changes nothing: a PENDING order is not refunded, both ways.
    let direct = site.run(crate::services::orders::refund::refund, refund(&b, body.clone()), &[("id", &b)]);
    let (paths, via) = paid(|| site.run(routed(Kind::Refund), refund(&b, body.clone()), &[("id", &b)]));
    assert_eq!(direct.status_code(), 409, "{}", direct.body_str());
    assert_eq!((via.status_code(), via.body()), (direct.status_code(), direct.body()), "the same refusal, byte for byte");
    assert_eq!(paths.iter().filter(|p| !p.starts_with("/fold/verdict")).collect::<Vec<_>>(), ["/fold/command"], "the Worker's one request: {paths:?}");
    assert!(!paths.iter().any(|p| p.starts_with("/img/")), "no image crossed into the Worker: {paths:?}");
    // The real thing: confirm `a`, refund it through the object.
    let r = site.run(crate::owner::order_action, post(&at(&format!("/api/owner/orders/{a}/action")), &json!({"location_id": "alpha", "action": "confirm"})).bearer(&owner).on("alpha"), &[("id", &a)]);
    assert_eq!(r.status_code(), 200, "confirm: {}", r.body_str());
    let (paths, r) = paid(|| site.run(routed(Kind::Refund), refund(&a, body.clone()), &[("id", &a)]));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    // A cash order with no money taken is compensated in the same step; a paid one would be REFUNDING.
    let status = r.body_value()["order"]["status"].as_str().unwrap_or("").to_string();
    assert!(status == "REFUNDING" || status == "COMPENSATED_REFUND", "{}", r.body_str());
    assert_eq!(r.body_value()["order"]["refund"]["reason"], json!("venue_cancelled"), "{}", r.body_str());
    assert_eq!(paths.iter().filter(|p| !p.starts_with("/fold/verdict")).collect::<Vec<_>>(), ["/fold/command"], "{paths:?}");
    // And the order is refunded in the object's log, not only in the answer.
    let listed = site.fold("alpha", "/fold/orders").body_value();
    let a_now = listed.as_array().unwrap().iter().find(|o| o["order_id"] == json!(a)).expect("order a");
    assert!(a_now["order_json"].as_str().unwrap().contains(&format!("\"{status}\"")), "{a_now}");
    // An unknown order: the handler's 404, through the object.
    let r = site.run(routed(Kind::Refund), refund("o_nope", body), &[("id", "o_nope")]);
    assert_eq!(r.status_code(), 404, "{}", r.body_str());
}

/// The owner console's reads: the handler run in the object answers the bytes it answered from
/// the Worker, and the Worker makes one request for each (plus the verdict the object asks).
#[test]
fn the_owner_consoles_reads_answer_byte_for_byte_through_the_object() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    place_pickup(&site, "alpha", &dish, 2);
    type H = fn(Call, crate::edge::Ctx<crate::Req>) -> std::pin::Pin<Box<dyn std::future::Future<Output = worker::Result<Reply>>>>;
    let reads: Vec<(&str, Kind, H)> = vec![
        ("/api/owner/customers?location_id=alpha", Kind::Customers, |c, x| Box::pin(crate::services::customers::handlers::customers(c, x))),
        ("/api/owner/customers/reveals?location_id=alpha", Kind::Reveals, |c, x| Box::pin(crate::services::customers::handlers::reveals(c, x))),
        ("/api/owner/exceptions?location_id=alpha", Kind::Exceptions, |c, x| Box::pin(crate::exceptions::exceptions(c, x))),
        ("/api/owner/analytics/kitchen?location_id=alpha", Kind::AnalyticsKitchen, |c, x| Box::pin(crate::services::analytics::kitchen::kitchen(c, x))),
        ("/api/owner/campaigns?location_id=alpha", Kind::CampaignList, |c, x| Box::pin(crate::services::campaigns::handlers::list(c, x))),
        ("/api/owner/threads?location_id=alpha", Kind::Inbox, |c, x| Box::pin(crate::social::inbox::list(c, x))),
        ("/api/owner/wallet/legs?location_id=alpha", Kind::WalletLegs, |c, x| Box::pin(crate::services::orders::legs::audit(c, x))),
        ("/api/owner/posts?location_id=alpha", Kind::Posts, |c, x| Box::pin(crate::services::engagement::posts::posts(c, x))),
        ("/api/owner/graph?location_id=alpha", Kind::Graph, |c, x| Box::pin(crate::services::engagement::assist::graph(c, x))),
        ("/api/owner/bag?location_id=alpha", Kind::BagCard, |c, x| Box::pin(crate::services::loyalty::bag_routes::card(c, x))),
        ("/api/staff/room?location_id=alpha", Kind::StaffRoom, |c, x| Box::pin(crate::services::orders::room::handlers::room_view(c, x))),
        ("/api/staff/floor?location_id=alpha", Kind::StaffFloor, |c, x| Box::pin(crate::services::orders::room::floor::get(c, x))),
        ("/api/staff/till/tips?location_id=alpha", Kind::TillTips, |c, x| Box::pin(crate::services::orders::room::till::tips(c, x))),
    ];
    for (path, kind, handler) in reads {
        let direct = site.run(handler, get(&at(path)).bearer(&owner).on("alpha"), &[]);
        assert_eq!(direct.status_code(), 200, "{path} from the Worker: {}", direct.body_str());
        let (paths, via) = paid(|| site.run(routed(kind), get(&at(path)).bearer(&owner).on("alpha"), &[]));
        assert_eq!(via.status_code(), 200, "{path} through the object: {}", via.body_str());
        assert_eq!(via.body(), direct.body(), "{path}: byte for byte");
        assert_eq!(via.headers().get("content-type").ok().flatten(), direct.headers().get("content-type").ok().flatten(), "{path}");
        let worker_side: Vec<&String> = paths.iter().filter(|p| !p.starts_with("/fold/verdict")).collect();
        assert_eq!(worker_side, ["/fold/command"], "{path}: the Worker's one request: {paths:?}");
    }
    // THE HEALTH PANE embeds the object's own counters (`reads`, the window's `toMs`), which the
    // request itself moves, so it is compared by status and keys, not bytes.
    let url = at("/api/owner/health?location_id=alpha");
    let direct = site.run(crate::services::operations::health, get(&url).bearer(&owner).on("alpha"), &[]);
    let (paths, via) = paid(|| site.run(routed(Kind::Health), get(&url).bearer(&owner).on("alpha"), &[]));
    assert_eq!((direct.status_code(), via.status_code()), (200, 200), "{} | {}", direct.body_str(), via.body_str());
    let keys = |r: &Reply| r.body_value().as_object().unwrap().keys().cloned().collect::<Vec<_>>();
    assert_eq!(keys(&via), keys(&direct), "health: the same pane");
    assert_eq!(via.body_value()["verdict"], direct.body_value()["verdict"]);
    assert_eq!(paths.iter().filter(|p| !p.starts_with("/fold/verdict")).collect::<Vec<_>>(), ["/fold/command"], "{paths:?}");
}

/// The public routes: the promo check and the welcome page answer the same through the object;
/// the welcome page is a slug route and the slug, not the Host, names the venue.
#[test]
fn the_public_routes_answer_the_same_through_the_object() {
    let site = Site::new();
    let (_owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let body = json!({"code": "NOPE", "items": [{"product_id": dish, "quantity": 1}]});
    let direct = site.run(crate::services::ordering::preview::promo_check, post(&at("/api/promo/check"), &body).on("alpha"), &[]);
    let (paths, via) = paid(|| site.run(routed(Kind::PromoCheck), post(&at("/api/promo/check"), &body).on("alpha"), &[]));
    assert_ne!(direct.status_code(), 500, "{}", direct.body_str());
    assert_eq!((via.status_code(), via.body()), (direct.status_code(), direct.body()), "promo check, byte for byte");
    assert_eq!(paths.iter().filter(|p| p.starts_with("/fold/command")).count(), 1, "{paths:?}");
    let url = format!("https://{PLATFORM_HOST}/api/public/locations/alpha/welcome");
    let direct = site.run(crate::services::loyalty::bag_routes::public, get(&url), &[("slug", "alpha")]);
    let (paths, via) = paid(|| site.run(routed_slug(Kind::Welcome), get(&url), &[("slug", "alpha")]));
    assert_ne!(direct.status_code(), 500, "{}", direct.body_str());
    assert_eq!((via.status_code(), via.body()), (direct.status_code(), direct.body()), "welcome, byte for byte");
    assert!(paths.contains(&"/fold/command".to_string()), "{paths:?}");
    // A slug route without a slug is the handler's own 400.
    let r = site.run(routed_slug(Kind::Welcome), get(&url), &[]);
    assert_eq!(r.status_code(), 400);
    // THE OBJECT REFUSES a kind sent to the wrong object, as it refuses every command.
    let alpha = block_on(crate::hubstore::Place::of_slug(&site.ctx(&[]), "alpha")).unwrap().venue;
    open_venue(&site, "beta", "b@x.test");
    let beta = block_on(crate::hubstore::Place::of_slug(&site.ctx(&[]), "beta")).unwrap().venue;
    let cmd = crate::hubdo::command::Command { kind: Kind::Customers, venue: alpha, params: vec![], method: "GET".into(), now_ms: site.now_ms, url: at("/api/owner/customers"), headers: vec![], body: String::new() };
    let r = block_on(site.object(&beta).route(Call::new("https://hub/fold/command", worker::Method::Post).unwrap().with_json(&cmd))).unwrap();
    assert_eq!(r.status_code(), 409, "{}", r.body_str());
}
