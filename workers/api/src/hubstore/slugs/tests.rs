//! A4 (W-CHEAP): a public slug costs the platform object ONE request per isolate, not one per
//! public request -- and only an identity answer is ever remembered.

use super::*;
use crate::edge::mem::object_requests;
use crate::edge::site::{get, As, Site};
use crate::hubstore::Place;
use crate::edge::mem::block_on;

fn of_slug(site: &Site, slug: &str) -> (String, usize) {
    let before = object_requests();
    let p = block_on(Place::of_slug(&site.ctx(&[]), slug)).expect("of_slug");
    (p.venue, object_requests() - before)
}

/// A registry record `slug -> id` written directly, so no route has learned it yet.
fn register(site: &Site, slug: &str, id: &str) {
    let (s, i) = (slug.to_string(), id.to_string());
    block_on(crate::identity_store::with_registry(&site.env(), move |t| {
        let k = crate::identity_store::loc_by_slug(&s);
        t.put(crate::identity_store::K_LOC, &i, &format!(r#"{{"id":"{i}","slug":"{s}"}}"#), &[(k, i.clone())], &[])
            .map_err(|e| worker::Error::RustError(format!("{e:?}")))
    }))
    .expect("registry write");
}

#[test]
fn a_known_slug_asks_the_platform_object_once_not_on_every_request() {
    let site = Site::new();
    register(&site, "alpha", "alpha");
    let (v, first) = of_slug(&site, "alpha");
    assert_eq!((v.as_str(), first), ("alpha", 1), "the first lookup reads the registry");
    let (v, again) = of_slug(&site, "alpha");
    assert_eq!((v.as_str(), again), ("alpha", 0), "the second is answered without any object request");
    let (v, cased) = of_slug(&site, "Alpha");
    assert_eq!((v.as_str(), cased), ("alpha", 0), "the registry folds case, and so does the memory");
}

#[test]
fn the_menu_route_costs_one_object_request_once_the_slug_is_known() {
    let site = Site::new();
    crate::storefront::route_tests::open_venue(&site, "alpha", "a@x.test");
    let menu = || {
        let before = object_requests();
        let r = site.run(
            crate::storefront::menu,
            get("https://alpha.dowiz.org/api/public/locations/alpha/menu?fresh=1").on("alpha"),
            &[("slug", "alpha")],
        );
        assert_eq!(r.status_code(), 200, "{}", r.body_str());
        object_requests() - before
    };
    let first = menu();
    let second = menu();
    eprintln!("menu object requests: first {first}, then {second}");
    assert_eq!(second, 1, "the venue's own object, and no registry read");
    assert_eq!(first, second, "the slug was already learned while the venue was opened");
}

#[test]
fn an_unknown_slug_is_never_remembered_so_a_venue_created_later_is_found() {
    let site = Site::new();
    site.venue("alpha", "a@x.test");
    let (v, n) = of_slug(&site, "gamma");
    assert_eq!((v.as_str(), n), (crate::hubstore::UNNAMED_VENUE, 1));
    let (_, n) = of_slug(&site, "gamma");
    assert_eq!(n, 1, "no venue is not an answer to keep");
    site.venue("gamma", "g@x.test");
    let (v, _) = of_slug(&site, "gamma");
    assert_eq!(v, "gamma", "created a moment later, and found");
}

#[test]
fn a_slug_that_names_another_id_is_asked_every_time() {
    let site = Site::new();
    register(&site, "zeta", "zeta-1");
    let (v, n) = of_slug(&site, "zeta");
    assert_eq!((v.as_str(), n), ("zeta-1", 1));
    let (v, n) = of_slug(&site, "zeta");
    assert_eq!((v.as_str(), n), ("zeta-1", 1), "a non-identity answer could go stale: never remembered");
}

#[test]
fn one_world_never_answers_for_another() {
    let a = Site::new();
    register(&a, "alpha", "alpha");
    assert_eq!(of_slug(&a, "alpha"), ("alpha".to_string(), 1));
    let b = Site::new();
    assert_eq!(of_slug(&b, "alpha"), (crate::hubstore::UNNAMED_VENUE.to_string(), 1), "b has no alpha");
}

#[test]
fn only_identity_answers_are_rememberable() {
    assert!(rememberable("alpha", "alpha"));
    assert!(rememberable("Alpha ", "alpha"));
    assert!(!rememberable("alpha", "alpha-2"));
    assert!(!rememberable("alpha", ""));
}

/// THE COST OF ONE PICKUP ORDER, COUNTED (W-CHEAP item 3): billed object requests and storage
/// rows written (one row per key put: `MemHost::writes`) per step, venue and platform objects
/// apart. Printed for the report; the asserts are CEILINGS at the measured values, so a change
/// that adds a request or a row to an order goes red here and has to say why.
#[test]
fn one_pickup_order_costs_counted_per_step() {
    let site = Site::new();
    let (t, dish) = crate::storefront::route_tests::open_venue(&site, "alpha", "a@x.test");
    let venue = site.world.host("alpha");
    let platform = site.world.host(crate::platform_store::PLATFORM);
    let snap = || (object_requests(), venue.writes.borrow().len(), platform.writes.borrow().len());
    let mut rows: Vec<(String, usize, usize, usize)> = Vec::new();
    let s0 = snap();
    let placed = crate::storefront::route_tests::place_pickup(&site, "alpha", &dish, 1);
    assert_eq!(placed.status_code(), 200, "{}", placed.body_str());
    let s1 = snap();
    rows.push(("place".into(), s1.0 - s0.0, s1.1 - s0.1, s1.2 - s0.2));
    let v = placed.body_value();
    let id = ["id", "order_id", "orderId"].iter().find_map(|k| v[*k].as_str().or(v["order"][*k].as_str())).expect("order id").to_string();
    for action in ["confirm", "preparing", "ready", "collected"] {
        let a = snap();
        let r = site.run(
            crate::owner::order_action,
            crate::edge::site::post(
                &format!("https://alpha.dowiz.org/api/owner/orders/{id}/action"),
                &serde_json::json!({"location_id": "alpha", "action": action}),
            )
            .bearer(&t)
            .on("alpha"),
            &[("id", &id)],
        );
        assert_eq!(r.status_code(), 200, "{action}: {}", r.body_str());
        let b = snap();
        rows.push((action.into(), b.0 - a.0, b.1 - a.1, b.2 - a.2));
    }
    let total = rows.iter().fold((0, 0, 0), |s, r| (s.0 + r.1, s.1 + r.2, s.2 + r.3));
    for r in &rows {
        eprintln!("order cost {:10} object requests {:2}  venue rows {:2}  platform rows {:2}", r.0, r.1, r.2, r.3);
    }
    eprintln!("order cost TOTAL      object requests {:2}  venue rows {:2}  platform rows {:2}", total.0, total.1, total.2);
    assert_eq!(total.2, 0, "an order writes nothing to the platform object");
    assert!(total.0 <= ORDER_REQ_CEILING, "object requests per pickup order: {} > {ORDER_REQ_CEILING}", total.0);
    assert!(total.1 <= ORDER_ROWS_CEILING, "venue rows per pickup order: {} > {ORDER_ROWS_CEILING}", total.1);
}

/// Measured in this lane (W-CHEAP) -- see the test's printout; lower them when a change earns it.
const ORDER_REQ_CEILING: usize = 18; // measured 18 with A4 switched off (place 10 + 4 advances x 2)
const ORDER_ROWS_CEILING: usize = 14; // measured 14 (place 6 + 4 advances x 2)
