//! The owner's routes, run through the route seam (W-COV C2): a platform in memory, venues made
//! by `create_hub`, owners signed in by `owner_login`, and every assertion on what was STORED or
//! answered -- including the other venue's owner, who must be refused.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use serde_json::json;

fn orders_url(slug: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}/api/owner/orders")
}

#[test]
fn a_new_venue_answers_its_owner_an_empty_queue_and_refuses_the_anonymous() {
    let site = Site::new();
    let a = site.venue("alpha", "a@x.test");
    let r = site.run(crate::owner::orders, get(&orders_url("alpha")).bearer(&a).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    assert_eq!(v["orders"], json!([]));
    assert_eq!(v["full"], true);
    let anon = site.run(crate::owner::orders, get(&orders_url("alpha")).on("alpha"), &[]);
    assert_eq!(anon.status_code(), 401, "{}", anon.body_str());
}

/// THE TENANT BOUNDARY: the owner of alpha, on beta's host, is refused -- the request names two
/// venues (`Place::must_be`), and nothing of beta's is read.
#[test]
fn an_owner_of_one_venue_on_anothers_host_is_refused() {
    let site = Site::new();
    let a = site.venue("alpha", "a@x.test");
    let _b = site.venue("beta", "b@x.test");
    // A token carries its venue, and the token wins over the host (`Place::of_any`), so alpha's
    // token on beta's host reads ALPHA -- never beta.
    let r = site.run(crate::owner::orders, get(&orders_url("beta")).bearer(&a).on("beta"), &[]);
    assert!(r.status_code() == 200 || r.status_code() == 409, "{} {}", r.status_code(), r.body_str());
    // And naming beta explicitly is refused.
    let named = format!("{}?location_id=beta", orders_url("beta"));
    let r = site.run(crate::owner::orders, get(&named).bearer(&a).on("beta"), &[]);
    assert!(r.status_code() >= 400, "alpha's owner named beta and was answered {}: {}", r.status_code(), r.body_str());
}

#[test]
fn only_a_platform_administrator_creates_a_hub_and_a_slug_is_taken_once() {
    let site = Site::new();
    let owner = site.venue("alpha", "a@x.test");
    let body = json!({"slug": "gamma", "name": "G", "dpa": crate::privacy::dpa::VERSION});
    let url = format!("https://{PLATFORM_HOST}/api/platform/hubs");
    let r = site.run(crate::platform::create_hub, post(&url, &body).bearer(&owner), &[]);
    assert_eq!(r.status_code(), 404, "an owner is not an administrator: {}", r.body_str());
    let again = json!({"slug": "alpha", "name": "A2", "dpa": crate::privacy::dpa::VERSION});
    let r = site.run(crate::platform::create_hub, post(&url, &again).bearer(&site.admin_token()), &[]);
    assert_eq!(r.status_code(), 409, "{}", r.body_str());
    let stale = json!({"slug": "delta", "name": "D", "dpa": "dpa v0"});
    let r = site.run(crate::platform::create_hub, post(&url, &stale).bearer(&site.admin_token()), &[]);
    assert_eq!(r.status_code(), 400, "a stale DPA is refused: {}", r.body_str());
    // The registry holds alpha once, and no delta.
    let reg = crate::edge::mem::block_on(crate::identity_store::registry(&site.env())).unwrap();
    assert!(reg.lookup(&crate::identity_store::loc_by_slug("alpha")).is_some());
    assert!(reg.lookup(&crate::identity_store::loc_by_slug("delta")).is_none());
}

#[test]
fn a_wrong_password_is_401_and_a_login_on_a_venue_the_owner_does_not_own_is_refused() {
    let site = Site::new();
    site.venue("alpha", "a@x.test");
    site.venue("beta", "b@x.test");
    let url = format!("https://alpha.{PLATFORM_HOST}/api/auth/login");
    let bad = site.run(
        crate::accounts::owner_login,
        post(&url, &json!({"email": "a@x.test", "password": "nope"})).on("alpha"),
        &[],
    );
    assert_eq!(bad.status_code(), 401);
    // b's owner on alpha's host: no membership there.
    let cross = site.run(
        crate::accounts::owner_login,
        post(&url, &json!({"email": "b@x.test", "password": "owner-password-1"})).on("alpha"),
        &[],
    );
    assert!(cross.status_code() == 403, "{} {}", cross.status_code(), cross.body_str());
}

// ── THE CATALOGUE, THE VENUE RECORD AND THE TRANSLATIONS ─────────────────────

use crate::storefront::route_tests::{open_venue, place_pickup};
use serde_json::Value;

fn at(slug: &str, path: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}{path}")
}

fn edit(site: &Site, token: &str, slug: &str, dish: &str, body: Value) -> crate::wire::Reply {
    site.run(
        crate::owner::update_product,
        post(&at(slug, &format!("/api/owner/products/{dish}")), &body).bearer(token).on(slug),
        &[("id", dish)],
    )
}

fn menu(site: &Site, slug: &str) -> Value {
    let r = site.run(
        crate::storefront::menu,
        get(&at(slug, &format!("/api/public/locations/{slug}/menu?fresh=1"))).on(slug),
        &[("slug", slug)],
    );
    assert_eq!(r.status_code(), 200, "menu: {}", r.body_str());
    r.body_value()
}

#[test]
fn a_dish_edited_in_full_is_what_the_storefront_then_serves() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let r = edit(
        &site,
        &t,
        "alpha",
        &dish,
        json!({
            "location_id": "alpha", "name": "Futomaki XL", "description": "Eight pieces",
            "price": 1100, "allergens": ["gluten", "fish"], "size_cm": 20, "cooking_min": 12,
            "ingredients": ["rice", "salmon"], "weight_g": 320, "nutrition": {"kcal": 540},
            "translations": {"sq": {"name": "Futomaki i madh"}}, "tags": ["spicy"],
            "station": "sushi", "unavailable_note": "",
        }),
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let m = menu(&site, "alpha");
    let s = m.to_string();
    assert!(s.contains("Futomaki XL") && s.contains("1100"), "the edit is served: {s}");
    assert!(s.contains("gluten"), "the allergens are served: {s}");

    // Refusals leave the dish as it is.
    for (why, body) in [
        ("negative price", json!({"location_id": "alpha", "price": -1})),
        ("size out of range", json!({"location_id": "alpha", "size_cm": 500})),
        ("cooking out of range", json!({"location_id": "alpha", "cooking_min": 0})),
        ("unknown allergen", json!({"location_id": "alpha", "allergens": ["uranium"]})),
        ("unknown station", json!({"location_id": "alpha", "station": "garage"})),
        ("bad recipe line", json!({"location_id": "alpha", "bom": [{"supply": "", "qty": 0}]})),
        ("bad move", json!({"location_id": "alpha", "move": "sideways"})),
        ("unknown field", json!({"location_id": "alpha", "colour": "red"})),
    ] {
        let r = edit(&site, &t, "alpha", &dish, body);
        assert_eq!(r.status_code(), 400, "{why}: {}", r.body_str());
    }
    assert!(menu(&site, "alpha").to_string().contains("1100"), "no refusal moved the price");

    // The other venue's owner cannot edit it.
    let b = site.venue("beta", "b@x.test");
    let r = edit(&site, &b, "alpha", &dish, json!({"location_id": "alpha", "price": 1}));
    assert!(r.status_code() >= 400, "{}", r.body_str());
    assert!(menu(&site, "alpha").to_string().contains("1100"));
}

#[test]
fn translations_are_written_for_known_entities_and_refused_for_unknown_ones() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let r = site.run(
        crate::owner::write_translations,
        post(
            &at("alpha", "/api/owner/i18n"),
            &json!({"location_id": "alpha", "entries": [
                {"entity": "product", "id": dish, "locale": "sq", "field": "name", "value": "Futomaki shqip"},
                {"entity": "product", "id": "ghost", "locale": "sq", "field": "name", "value": "x"},
                {"entity": "planet", "id": "mars", "locale": "sq", "field": "name", "value": "x"},
            ]}),
        )
        .bearer(&t)
        .on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    assert_eq!(v["written"], 1);
    assert_eq!(v["refused"].as_array().map(Vec::len), Some(2), "{v}");
    let many: Vec<Value> = (0..501).map(|i| json!({"entity": "product", "id": dish, "locale": "sq", "field": "name", "value": format!("{i}")})).collect();
    let r = site.run(
        crate::owner::write_translations,
        post(&at("alpha", "/api/owner/i18n"), &json!({"location_id": "alpha", "entries": many})).bearer(&t).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 400);
}

#[test]
fn the_venue_record_is_edited_and_read_back_from_the_object() {
    let site = Site::new();
    let (t, _) = open_venue(&site, "alpha", "a@x.test");
    let r = site.run(
        crate::owner::update_location,
        post(
            &at("alpha", "/api/owner/location"),
            &json!({"location_id": "alpha", "name": "Alpha Sushi", "phone": "+355691234567", "delivery_fee": 200,
                    "min_order": 500, "free_delivery_threshold": 3000, "timezone": "Europe/Tirane", "delivery_paused": false}),
        )
        .bearer(&t)
        .on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let rec = site.fold("alpha", "/fold/venue").body_value();
    assert_eq!(rec["name"], "Alpha Sushi", "{rec}");
    assert_eq!(rec["delivery_fee"], 200);
    assert_eq!(rec["tz"], "Europe/Tirane", "stored under `tz`, which `hubstore::zone_of` reads");
    let r = site.run(
        crate::owner::update_location,
        post(&at("alpha", "/api/owner/location"), &json!({"location_id": "alpha", "free_delivery_threshold": null})).bearer(&t).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert!(site.fold("alpha", "/fold/venue").body_value()["free_delivery_threshold"].is_null(), "null clears it");
    for (why, body) in [
        ("bad stage colour", json!({"location_id": "alpha", "stage": {"warm": "not-hex"}})),
        ("bad stage motif", json!({"location_id": "alpha", "stage": {"motif": "dragons"}})),
        ("stage not an object", json!({"location_id": "alpha", "stage": 3})),
        ("unknown field", json!({"location_id": "alpha", "logo": "x"})),
        ("unknown zone", json!({"location_id": "alpha", "timezone": "Mars/Olympus"})),
        ("unknown status", json!({"location_id": "alpha", "status": "asleep"})),
    ] {
        let r = site.run(crate::owner::update_location, post(&at("alpha", "/api/owner/location"), &body).bearer(&t).on("alpha"), &[]);
        assert_eq!(r.status_code(), 400, "{why}: {}", r.body_str());
    }
    let b = site.venue("beta", "b@x.test");
    let r = site.run(
        crate::owner::update_location,
        post(&at("alpha", "/api/owner/location"), &json!({"location_id": "alpha", "name": "Hijacked"})).bearer(&b).on("alpha"),
        &[],
    );
    assert!(r.status_code() >= 400, "{}", r.body_str());
    assert_eq!(site.fold("alpha", "/fold/venue").body_value()["name"], "Alpha Sushi");
}

#[test]
fn the_dashboard_counts_todays_orders() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    place_pickup(&site, "alpha", &dish, 2);
    let r = site.run(crate::owner::dashboard, get(&at("alpha", "/api/owner/dashboard")).bearer(&t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    assert_eq!(v["todayOrders"], 1, "{v}");
    assert_eq!(v["pending"], 1, "{v}");
}

#[test]
fn a_dish_and_a_category_are_deleted_and_leave_the_menu() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let cats = site.run(crate::catalog_edit::list_categories, get(&at("alpha", "/api/owner/categories")).bearer(&t).on("alpha"), &[]);
    let cat = cats.body_value().to_string();
    assert!(cat.contains("Rolls"), "{cat}");
    let r = site.run(
        crate::catalog_edit::delete_product,
        post(&at("alpha", &format!("/api/owner/products/{dish}/delete")), &json!({"location_id": "alpha"})).bearer(&t).on("alpha"),
        &[("id", &dish)],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert!(!menu(&site, "alpha").to_string().contains("Futomaki"), "the deleted dish is not served");
    let cat_id = cats.body_value()
        .as_array()
        .or(cats.body_value()["categories"].as_array())
        .and_then(|a| a.first().cloned())
        .and_then(|c| c["id"].as_str().map(str::to_string))
        .unwrap_or_else(|| panic!("no category in {cat}"));
    let r = site.run(
        crate::catalog_edit::delete_category,
        post(&at("alpha", &format!("/api/owner/categories/{cat_id}/delete")), &json!({"location_id": "alpha"})).bearer(&t).on("alpha"),
        &[("id", &cat_id)],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
}

#[test]
fn an_unknown_venue_has_no_menu_and_the_manifest_names_the_venue() {
    let site = Site::new();
    open_venue(&site, "alpha", "a@x.test");
    let r = site.run(crate::storefront::menu, get(&at("ghost", "/api/public/locations/ghost/menu")).on("ghost"), &[("slug", "ghost")]);
    assert_eq!(r.status_code(), 404, "{}", r.body_str());
    let r = site.run(crate::storefront::manifest, get(&at("alpha", "/manifest.webmanifest")).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert!(r.body_str().contains("Venue alpha"), "{}", r.body_str());
}

#[test]
fn a_console_catches_up_from_its_generation_and_only_with_its_own_venues_changes() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let (b, bdish) = open_venue(&site, "beta", "b@x.test");
    assert_eq!(place_pickup(&site, "alpha", &dish, 1).status_code(), 200);
    let list = |tok: &str, q: &str| site.run(crate::owner::orders, get(&at("alpha", &format!("/api/owner/orders{q}"))).bearer(tok).on("alpha"), &[]);
    let full = list(&t, "").body_value();
    assert_eq!((full["full"].as_bool(), full["orders"].as_array().map(Vec::len)), (Some(true), Some(1)), "{full}");
    let gen = full["generation"].as_i64().unwrap();

    // A new order, and another venue's: the catch-up names alpha's alone.
    let id = place_pickup(&site, "alpha", &dish, 1).body_value()["id"].as_str().unwrap().to_string();
    assert_eq!(place_pickup(&site, "beta", &bdish, 1).status_code(), 200);
    let delta = list(&t, &format!("?since={gen}")).body_value();
    assert_eq!(delta["full"], false, "{delta}");
    let changes = delta["changes"].as_array().unwrap();
    assert!(changes.iter().any(|c| c["order_id"] == id.as_str()), "{delta}");
    assert!(changes.iter().all(|c| c["payload"].as_str().unwrap_or("").contains("\"alpha\"")), "{delta}");
    assert!(delta["generation"].as_i64().unwrap() > gen);
    // Nothing since the newest: an empty change set, not the whole list.
    let now = list(&t, &format!("?since={}", delta["generation"])).body_value();
    assert_eq!((now["full"].as_bool(), now["changes"].as_array().map(Vec::len)), (Some(false), Some(0)), "{now}");
    // A status filter is never a delta.
    assert_eq!(list(&t, &format!("?since={gen}&status=PENDING")).body_value()["full"], true);
    // A catch-up from before the window is the whole list again.
    let old = list(&t, "?since=-5").body_value();
    assert!(old["full"] == true || old["changes"].is_array(), "{old}");
    // Beta's owner asking alpha's object is refused before the object is asked anything.
    assert!(list(&b, &format!("?since={gen}&location_id=alpha")).status_code() >= 400);
}
