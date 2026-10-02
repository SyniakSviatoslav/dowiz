//! The platform's and the public's routes (W-COV C2): the waiting list, the administrator's
//! views, the privacy pages, the estimate, the wallet, the rates and the bootstrap door.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::open_venue;
use serde_json::json;

fn apex(path: &str) -> String {
    format!("https://{PLATFORM_HOST}{path}")
}
fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}

#[test]
fn the_waiting_list_takes_an_address_and_only_an_administrator_reads_it() {
    let site = Site::new();
    let owner = site.venue("alpha", "a@x.test");
    let bad = site.run(crate::waitlist::join, post(&apex("/api/waitlist"), &json!({"email": "not an address"})), &[]);
    assert_eq!(bad.status_code(), 400, "{}", bad.body_str());
    let r = site.run(crate::waitlist::join, post(&apex("/api/waitlist"), &json!({"email": "chef@x.test", "venue": "Chef's", "lang": "en"})), &[]);
    assert_eq!(r.status_code(), 204, "a joined address answers No Content: {}", r.body_str());
    let list = site.run(crate::waitlist::list, get(&apex("/api/platform/waitlist")).bearer(&site.admin_token()), &[]);
    assert_eq!(list.status_code(), 200, "{}", list.body_str());
    assert!(list.body_str().contains("chef@x.test"), "{}", list.body_str());
    let not = site.run(crate::waitlist::list, get(&apex("/api/platform/waitlist")).bearer(&owner), &[]);
    assert!(not.status_code() >= 400 && !not.body_str().contains("chef@x.test"), "{}", not.body_str());
}

#[test]
fn the_administrator_lists_hubs_and_errors_and_an_owner_cannot() {
    let site = Site::new();
    let owner = site.venue("alpha", "a@x.test");
    let hubs = site.run(crate::platform::hubs, get(&apex("/api/platform/hubs")).bearer(&site.admin_token()), &[]);
    assert_eq!(hubs.status_code(), 200, "{}", hubs.body_str());
    assert!(hubs.body_str().contains("alpha"), "{}", hubs.body_str());
    let errs = site.run(crate::platform::errors, get(&apex("/api/platform/errors")).bearer(&site.admin_token()), &[]);
    assert_eq!(errs.status_code(), 200, "{}", errs.body_str());
    for r in [
        site.run(crate::platform::hubs, get(&apex("/api/platform/hubs")).bearer(&owner), &[]),
        site.run(crate::platform::errors, get(&apex("/api/platform/errors")).bearer(&owner), &[]),
        site.run(crate::platform::hubs, get(&apex("/api/platform/hubs")), &[]),
    ] {
        assert!(r.status_code() >= 400, "{}", r.body_str());
    }
}

#[test]
fn the_privacy_notice_names_the_venue_and_the_owner_accepts_the_dpa() {
    let site = Site::new();
    let owner = site.venue("alpha", "a@x.test");
    let n = site.run(crate::privacy::notice::serve, get(&at("/privacy")).on("alpha"), &[]);
    assert_eq!(n.status_code(), 200, "{}", n.body_str());
    assert!(n.body_str().contains("Venue alpha"), "the notice names its controller");
    let d = site.run(crate::privacy::dpa::page, get(&apex("/dpa")), &[]);
    assert_eq!(d.status_code(), 200);
    let read = site.run(crate::privacy::dpa::read, get(&at("/api/owner/dpa")).bearer(&owner).on("alpha"), &[]);
    assert_eq!(read.status_code(), 200, "{}", read.body_str());
    let stale = site.run(crate::privacy::dpa::accept, post(&at("/api/owner/dpa/accept"), &json!({"version": "dpa v0"})).bearer(&owner).on("alpha"), &[]);
    assert_eq!(stale.status_code(), 400, "{}", stale.body_str());
    let ok = site.run(
        crate::privacy::dpa::accept,
        post(&at("/api/owner/dpa/accept"), &json!({"version": crate::privacy::dpa::VERSION})).bearer(&owner).on("alpha"),
        &[],
    );
    assert_eq!(ok.status_code(), 200, "{}", ok.body_str());
}

#[test]
fn an_estimate_is_quoted_for_a_basket() {
    let site = Site::new();
    let (_owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let r = site.run(
        crate::eta::quote,
        post(&at("/api/public/locations/alpha/eta"), &json!({"items": [{"productId": dish, "quantity": 2, "cookingMin": 10}], "distanceM": 2500})).on("alpha"),
        &[("slug", "alpha")],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let empty = site.run(crate::eta::quote, post(&at("/api/public/locations/alpha/eta"), &json!({"items": []})).on("alpha"), &[("slug", "alpha")]);
    assert!(empty.status_code() < 500, "{}", empty.body_str());
}

#[test]
fn a_top_up_is_recorded_by_the_venue_only_and_read_back_on_the_balance() {
    let site = Site::new();
    let owner = site.venue("alpha", "a@x.test");
    let top = |tok: Option<&str>, body: serde_json::Value| {
        let c = post(&at("/api/public/locations/alpha/wallet/topup"), &body).on("alpha");
        site.run(crate::wallet::top_up, match tok { Some(t) => c.bearer(t), None => c }, &[("slug", "alpha")])
    };
    let body = json!({"user": "u1", "amountMinor": 1000, "currency": "ALL", "providerRef": "cash-1", "requestId": "r1"});
    assert_eq!(top(None, body.clone()).status_code(), 401);
    let no_ref = top(Some(&owner), json!({"user": "u1", "amountMinor": 1000, "currency": "ALL", "providerRef": " ", "requestId": "r2"}));
    assert_eq!(no_ref.status_code(), 422, "{}", no_ref.body_str());
    let r = top(Some(&owner), body);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let bal = site.run(crate::wallet::balance, get(&at("/api/public/locations/alpha/wallet?user=u1")).bearer(&owner).on("alpha"), &[("slug", "alpha")]);
    assert_eq!(bal.status_code(), 200, "{}", bal.body_str());
    assert!(bal.body_str().contains("1000"), "{}", bal.body_str());
    let st = site.run(crate::wallet::statement, get(&at("/api/public/locations/alpha/wallet/statement?user=u1")).bearer(&owner).on("alpha"), &[("slug", "alpha")]);
    assert_eq!(st.status_code(), 200, "{}", st.body_str());
    let b = site.venue("beta", "b@x.test");
    let foreign = site.run(crate::wallet::balance, get(&at("/api/public/locations/alpha/wallet?user=u1")).bearer(&b).on("alpha"), &[("slug", "alpha")]);
    assert!(foreign.status_code() >= 400 && !foreign.body_str().contains("1000"), "{}", foreign.body_str());
}

#[test]
fn rates_answer_without_a_provider_and_the_bootstrap_door_is_shut_without_its_secret() {
    let site = Site::new();
    let r = site.run(crate::services::ordering::rates::rates, get(&apex("/api/public/rates?base=ALL")), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let bundle = json!({"location": gamma("Gamma")});
    let none = site.run(crate::bootstrap::seed, post(&apex("/api/bootstrap"), &bundle), &[]);
    assert_eq!(none.status_code(), 404, "no secret header: {}", none.body_str());
    let wrong = site.run(crate::bootstrap::seed, post(&apex("/api/bootstrap"), &bundle).with_header("x-dowiz-bootstrap", "nope"), &[]);
    assert_eq!(wrong.status_code(), 404);
    let ok = site.run(
        crate::bootstrap::seed,
        post(&apex("/api/bootstrap"), &json!({"location": gamma("Gamma"),
            "categories": [{"id": "c1", "name": "Mains"}], "products": [{"id": "p1", "name": "Soup", "price": 500, "category_id": "c1"}],
            "owner": {"email": "g@x.test", "password": "owner-password-1"}}))
            .with_header("x-dowiz-bootstrap", "bootstrap-secret-bootstrap-secret-0"),
        &[],
    );
    assert_eq!(ok.status_code(), 200, "{}", ok.body_str());
    let reg = crate::edge::mem::block_on(crate::identity_store::registry(&site.env())).unwrap();
    assert!(reg.lookup(&crate::identity_store::loc_by_slug("gamma")).is_some(), "the venue is registered");
}

#[test]
fn a_bootstrap_seeds_couriers_who_can_sign_in_and_a_replacing_bundle_removes_what_it_leaves_out() {
    let site = Site::new();
    let seed = |body: serde_json::Value| {
        site.run(crate::bootstrap::seed, post(&apex("/api/bootstrap"), &body).with_header("x-dowiz-bootstrap", "bootstrap-secret-bootstrap-secret-0"), &[])
    };
    // A new venue's record must be one the storefront can read: a partial one is refused by name.
    let partial = seed(json!({"location": {"id": "gamma", "slug": "gamma", "name": "Gamma"}}));
    assert_eq!(partial.status_code(), 400, "{}", partial.body_str());
    assert!(partial.body_str().contains("phone"), "the missing field is named: {}", partial.body_str());
    let loc = gamma("Gamma");
    let r = seed(json!({"location": loc,
        "categories": [{"id": "c1", "name": "Mains"}, {"id": "c2", "name": "Drinks"}],
        "products": [{"id": "p1", "name": "Soup", "price": 500, "categoryId": "c1", "available": true, "allergens": []},
                     {"id": "p2", "name": "Tea", "price": 200, "categoryId": "c2", "available": true, "allergens": []}],
        "owner": {"email": "g@x.test", "password": "owner-password-1"},
        "couriers": [{"phone": "+355690001111", "password": "rider-password-1", "name": "Rider", "email": "r@x.test"}]}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    // The seeded courier signs in at that venue, by phone or by email.
    let login = |body: serde_json::Value| {
        site.run(crate::accounts::courier_login, post(&format!("https://gamma.{PLATFORM_HOST}/api/courier/auth/login"), &body).on("gamma"), &[])
    };
    let by_phone = login(json!({"phone": "+355690001111", "password": "rider-password-1"}));
    assert_eq!(by_phone.status_code(), 200, "{}", by_phone.body_str());
    assert_eq!(login(json!({"email": "R@x.test", "password": "rider-password-1"})).status_code(), 200, "email is case-blind");
    assert_eq!(login(json!({"phone": "+355690001111", "password": "wrong"})).status_code(), 401);
    assert_eq!(login(json!({"password": "rider-password-1"})).status_code(), 400, "who is required");

    // The same bundle again: the courier is not made twice, the menu not duplicated.
    assert_eq!(seed(json!({"location": loc, "couriers": [{"phone": "+355690001111", "password": "rider-password-1"}]})).status_code(), 200);
    let crew = crate::edge::mem::block_on(crate::identity_store::couriers(&site.env())).unwrap();
    assert_eq!(crew.all(crate::identity_store::K_COURIER).len(), 1, "one courier, not two");

    // A replacing bundle is the whole catalogue: Tea and Drinks go.
    // Over an existing venue a partial record merges: only the name changes.
    let r = seed(json!({"location": {"id": "gamma", "slug": "gamma", "name": "Gamma Grill"}, "replace": true,
        "categories": [{"id": "c1", "name": "Mains"}], "products": [{"id": "p1", "name": "Soup", "price": 550, "categoryId": "c1", "available": true, "allergens": []}]}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let menu = site.run(crate::storefront::menu, get(&format!("https://gamma.{PLATFORM_HOST}/api/public/locations/gamma/menu?fresh=1")).on("gamma"), &[("slug", "gamma")]).body_str();
    assert!(menu.contains("Soup") && !menu.contains("Tea") && !menu.contains("Drinks"), "{menu}");
    assert!(menu.contains("Gamma Grill"), "the venue record was merged: {menu}");
    // A bundle that is not one is refused, by name.
    let bad = seed(json!({"categories": []}));
    assert_eq!(bad.status_code(), 400, "{}", bad.body_str());
}

/// A venue record the storefront reads (`storefront::LocRow`): every required field.
fn gamma(name: &str) -> serde_json::Value {
    json!({"id": "gamma", "slug": "gamma", "name": name, "phone": "+355690000003", "status": "open",
        "delivery_eta": "30-45", "delivery_fee": 0, "min_order": 0, "currency_code": "ALL", "menu_version": 1,
        "supported_locales": "sq,en", "default_locale": "sq", "delivery_paused": 0})
}
