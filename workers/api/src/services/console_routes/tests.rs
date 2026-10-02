//! The owner console's settings and operations routes, through the route seam (W-COV C2).
//! Every write is read back through the matching read route (or the venue's object), every
//! read is also asked by the OTHER venue's owner naming this venue -- and refused.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use serde_json::{json, Value};

fn at(slug: &str, path: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}{path}")
}

/// GET `path` on alpha as alpha's owner (must be 200 JSON) and as beta's owner naming alpha
/// (must be refused). Returns alpha's answer.
macro_rules! read_both {
    ($site:expr, $a:expr, $b:expr, $h:path, $path:expr) => {{
        let own = $site.run($h, get(&at("alpha", $path)).bearer($a).on("alpha"), &[]);
        assert_eq!(own.status_code(), 200, "{}: {}", $path, own.body_str());
        let sep = if $path.contains('?') { '&' } else { '?' };
        let foreign = $site.run($h, get(&at("alpha", &format!("{}{}location_id=alpha", $path, sep))).bearer($b).on("alpha"), &[]);
        assert!(foreign.status_code() >= 400, "beta's owner read alpha's {}: {} {}", $path, foreign.status_code(), foreign.body_str());
        own.body_value()
    }};
}

fn two() -> (Site, String, String) {
    let site = Site::new();
    let a = site.venue("alpha", "a@x.test");
    let b = site.venue("beta", "b@x.test");
    (site, a, b)
}

#[test]
fn a_setting_is_written_read_back_and_a_secret_is_never_shown() {
    let (site, a, b) = two();
    let r = site.run(
        crate::services::venue::settings::set_setting,
        post(&at("alpha", "/api/owner/settings"), &json!({"key": "notify.telegram.chat", "value": "-100777"})).bearer(&a).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = read_both!(site, &a, &b, crate::services::venue::settings::settings, "/api/owner/settings");
    assert!(v.to_string().contains("-100777"), "{v}");
    // Beta's settings do not carry alpha's value (the tenant boundary, on the settings image).
    let bv = site.run(crate::services::venue::settings::settings, get(&at("beta", "/api/owner/settings")).bearer(&b).on("beta"), &[]);
    assert!(!bv.body_str().contains("-100777"), "{}", bv.body_str());
    // Beta's owner writing to alpha is refused and changes nothing.
    let r = site.run(
        crate::services::venue::settings::set_setting,
        post(&at("alpha", "/api/owner/settings?location_id=alpha"), &json!({"key": "notify.telegram.chat", "value": "-1"})).bearer(&b).on("alpha"),
        &[],
    );
    assert!(r.status_code() >= 400, "{}", r.body_str());
    let v = site.run(crate::services::venue::settings::settings, get(&at("alpha", "/api/owner/settings")).bearer(&a).on("alpha"), &[]);
    assert!(v.body_str().contains("-100777"));
}

#[test]
fn a_feature_is_switched_and_read_back() {
    let (site, a, b) = two();
    let before = read_both!(site, &a, &b, crate::services::venue::settings::features, "/api/owner/features");
    let on_of = |v: &Value, k: &str| v["features"].as_array().and_then(|a| a.iter().find(|f| f["key"] == k)).map(|f| f["on"].clone());
    assert_eq!(on_of(&before, "feature.tips"), Some(json!(true)), "tips default on: {before}");
    let r = site.run(
        crate::services::venue::settings::set_feature,
        post(&at("alpha", "/api/owner/features"), &json!({"key": "feature.tips", "on": false})).bearer(&a).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let after = site.run(crate::services::venue::settings::features, get(&at("alpha", "/api/owner/features")).bearer(&a).on("alpha"), &[]);
    assert_eq!(on_of(&after.body_value(), "feature.tips"), Some(json!(false)));
    let beta = site.run(crate::services::venue::settings::features, get(&at("beta", "/api/owner/features")).bearer(&b).on("beta"), &[]);
    assert_eq!(on_of(&beta.body_value(), "feature.tips"), Some(json!(true)), "beta's switch did not move");
    let unknown = site.run(
        crate::services::venue::settings::set_feature,
        post(&at("alpha", "/api/owner/features"), &json!({"key": "feature.nope", "on": true})).bearer(&a).on("alpha"),
        &[],
    );
    assert_eq!(unknown.status_code(), 400, "{}", unknown.body_str());
}

#[test]
fn branding_is_set_by_colour_and_by_preset_and_a_bad_colour_is_refused() {
    let (site, a, b) = two();
    let bad = site.run(
        crate::services::venue::brand::set_branding,
        post(&at("alpha", "/api/owner/branding"), &json!({"primary": "not-a-colour"})).bearer(&a).on("alpha"),
        &[],
    );
    assert_eq!(bad.status_code(), 400, "{}", bad.body_str());
    let r = site.run(
        crate::services::venue::brand::set_branding,
        post(&at("alpha", "/api/owner/branding"), &json!({"primary": "#aa3311"})).bearer(&a).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = read_both!(site, &a, &b, crate::services::venue::brand::branding, "/api/owner/branding");
    assert!(v.to_string().to_lowercase().contains("aa3311"), "{v}");
    let r = site.run(
        crate::services::venue::brand::set_preset,
        post(&at("alpha", "/api/owner/branding/preset"), &json!({"preset": "no-such-preset"})).bearer(&a).on("alpha"),
        &[],
    );
    assert!(r.status_code() >= 400, "{}", r.body_str());
}

#[test]
fn delivery_zones_are_stored_and_answered_to_the_public() {
    let (site, a, _b) = two();
    let bad = site.run(
        crate::services::venue::zones::set_zones,
        post(&at("alpha", "/api/owner/zones"), &json!({"location_id": "alpha", "zones": [{"nonsense": true}]})).bearer(&a).on("alpha"),
        &[],
    );
    assert!(bad.status_code() >= 400, "{}", bad.body_str());
    let reach = site.run(crate::services::venue::zones::reach, get(&at("alpha", "/api/public/reach?slug=alpha")).on("alpha"), &[]);
    assert!(reach.status_code() < 500, "{}", reach.body_str());
}

#[test]
fn a_supply_and_a_prep_are_made_listed_and_retired() {
    let (site, a, b) = two();
    let r = site.run(
        crate::services::operations::supplies::set_supply,
        post(&at("alpha", "/api/owner/supplies"), &json!({"id": "rice", "name": "Rice", "unit": "g", "lowAt": 1000})).bearer(&a).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let stock = read_both!(site, &a, &b, crate::services::operations::stock::stock, "/api/owner/stock");
    assert!(stock.to_string().contains("rice"), "{stock}");
    let r = site.run(
        crate::services::operations::preps::set_prep,
        post(&at("alpha", "/api/owner/preps"), &json!({"id": "sushi-rice", "name": "Sushi rice", "unit": "g", "lines": [{"item": "rice", "qty": 900}], "yield": 1000})).bearer(&a).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let preps = read_both!(site, &a, &b, crate::services::operations::preps::list_preps, "/api/owner/preps");
    assert!(preps.to_string().contains("sushi-rice"), "{preps}");
    let uses = site.run(crate::services::operations::preps::uses, get(&at("alpha", "/api/owner/supplies/rice/uses")).bearer(&a).on("alpha"), &[("id", "rice")]);
    assert_eq!(uses.status_code(), 200, "{}", uses.body_str());
    assert!(uses.body_str().contains("sushi-rice"), "the prep uses rice: {}", uses.body_str());
    let r = site.run(
        crate::services::operations::supplies::retire_supply,
        post(&at("alpha", "/api/owner/supplies/rice/retire"), &json!({"location_id": "alpha"})).bearer(&a).on("alpha"),
        &[("id", "rice")],
    );
    assert!(r.status_code() == 200 || r.status_code() == 409, "{}", r.body_str());
}

#[test]
fn a_backup_restores_into_the_same_venue_and_is_refused_by_another_owner() {
    let (site, a, b) = two();
    site.run(
        crate::services::venue::settings::set_setting,
        post(&at("alpha", "/api/owner/settings"), &json!({"key": "notify.telegram.chat", "value": "-100888"})).bearer(&a).on("alpha"),
        &[],
    );
    let bundle = site.run(crate::services::operations::backup, get(&at("alpha", "/api/owner/backup")).bearer(&a).on("alpha"), &[]);
    assert_eq!(bundle.status_code(), 200, "{}", bundle.body_str());
    assert!(bundle.headers().get("content-disposition").unwrap().unwrap_or_default().contains("dowiz-backup-"));
    let foreign = site.run(crate::services::operations::backup, get(&at("alpha", "/api/owner/backup?location_id=alpha")).bearer(&b).on("alpha"), &[]);
    assert!(foreign.status_code() >= 400, "{}", foreign.body_str());
    let body: Value = bundle.body_value();
    let r = site.run(crate::services::operations::restore, post(&at("alpha", "/api/owner/restore"), &body).bearer(&a).on("alpha"), &[]);
    assert!(r.status_code() == 200 || r.status_code() == 409, "{}", r.body_str());
}

#[test]
fn health_history_and_a_rotation_answer_the_owner() {
    let (site, a, b) = two();
    let h = read_both!(site, &a, &b, crate::services::operations::health, "/api/owner/health");
    assert!(h.is_object(), "{h}");
    let _ = read_both!(site, &a, &b, crate::services::operations::history, "/api/owner/history");
    let r = site.run(crate::services::operations::rotate_now, post(&at("alpha", "/api/owner/hub/rotate"), &json!({})).bearer(&a).on("alpha"), &[]);
    assert!(r.status_code() == 200 || r.status_code() == 409, "{}", r.body_str());
    let r = site.run(crate::services::operations::rotate_now, post(&at("alpha", "/api/owner/hub/rotate?location_id=alpha"), &json!({})).bearer(&b).on("alpha"), &[]);
    assert!(r.status_code() >= 400, "{}", r.body_str());
}

/// EVERY OWNER READ, on a venue with an order in it: 200 and JSON for the venue's own owner,
/// refused for the other venue's owner naming it, 401 for nobody. Failures are collected so one
/// run names every route that does not hold, not just the first.
#[test]
fn every_owner_read_answers_its_owner_and_refuses_the_rest() {
    let site = Site::new();
    let (a, dish) = crate::storefront::route_tests::open_venue(&site, "alpha", "a@x.test");
    let b = site.venue("beta", "b@x.test");
    crate::storefront::route_tests::place_pickup(&site, "alpha", &dish, 1);
    let mut bad: Vec<String> = Vec::new();
    macro_rules! probe {
        ($h:path, $path:expr) => {{
            let own = site.try_run($h, get(&at("alpha", $path)).bearer(&a).on("alpha"), &[]);
            match own {
                Ok(r) if r.status_code() == 200 => {
                    if serde_json::from_slice::<Value>(r.body()).is_err() && !r.body().is_empty() {
                        bad.push(format!("{} own: 200 but not JSON: {}", $path, r.body_str().chars().take(80).collect::<String>()));
                    }
                }
                Ok(r) => bad.push(format!("{} own: {} {}", $path, r.status_code(), r.body_str().chars().take(160).collect::<String>())),
                Err(e) => bad.push(format!("{} own: Err {e}", $path)),
            }
            let sep = if $path.contains('?') { '&' } else { '?' };
            let foreign = site.try_run($h, get(&at("alpha", &format!("{}{}location_id=alpha", $path, sep))).bearer(&b).on("alpha"), &[]);
            match foreign {
                Ok(r) if r.status_code() >= 400 => {}
                Ok(r) => bad.push(format!("{} FOREIGN owner answered {}: {}", $path, r.status_code(), r.body_str().chars().take(160).collect::<String>())),
                Err(e) => bad.push(format!("{} foreign: Err {e}", $path)),
            }
            match site.try_run($h, get(&at("alpha", $path)).on("alpha"), &[]) {
                Ok(r) if r.status_code() == 401 || r.status_code() == 403 || r.status_code() == 404 => {}
                Ok(r) => bad.push(format!("{} ANONYMOUS answered {}", $path, r.status_code())),
                Err(e) => bad.push(format!("{} anon: Err {e}", $path)),
            }
        }};
    }
    probe!(crate::owner::dashboard, "/api/owner/dashboard");
    probe!(crate::catalog_edit::list_categories, "/api/owner/categories");
    probe!(crate::services::analytics::analytics, "/api/owner/analytics");
    probe!(crate::services::analytics::kitchen::kitchen, "/api/owner/analytics/kitchen");
    probe!(crate::exceptions::exceptions, "/api/owner/exceptions");
    probe!(crate::services::ordering::promotions::promotions, "/api/owner/promotions");
    probe!(crate::services::venue::activation::activation, "/api/owner/activation");
    probe!(crate::services::campaigns::handlers::list, "/api/owner/campaigns");
    probe!(crate::services::customers::handlers::customers, "/api/owner/customers");
    probe!(crate::services::customers::handlers::reveals, "/api/owner/customers/reveals");
    probe!(crate::services::operations::waste::waste_report, "/api/owner/stock/waste");
    probe!(crate::services::catalogue::import::bulk::owner_products, "/api/owner/products");
    probe!(crate::notify::hook::owner::state, "/api/owner/telegram");
    probe!(crate::channels::inbox, "/api/owner/inbox");
    probe!(crate::social::inbox::list, "/api/owner/threads");
    probe!(crate::cloud::status, "/api/owner/backup/cloud");
    probe!(crate::integrations::status, "/api/owner/integrations");
    probe!(crate::privacy::dpa::read, "/api/owner/dpa");
    probe!(crate::ebills::routes::status, "/api/owner/ebills");
    probe!(crate::fiscal::routes::status, "/api/owner/fiscal");
    probe!(crate::mcp::owner_list, "/api/owner/mcp/keys");
    probe!(crate::services::courier::console::couriers, "/api/owner/couriers");
    probe!(crate::services::engagement::posts::posts, "/api/owner/posts");
    probe!(crate::services::engagement::assist::graph, "/api/owner/graph");
    probe!(crate::services::identity::keys::list_api_keys, "/api/owner/apikeys");
    probe!(crate::services::identity::staff_admin::list_staff, "/api/owner/staff");
    probe!(crate::services::orders::print::jobs, "/api/owner/print/jobs");
    probe!(crate::services::orders::legs::audit, "/api/owner/wallet/legs");
    probe!(crate::services::orders::room::table_qr::list, "/api/owner/tables/qr");
    probe!(crate::services::venue::settings::settings, "/api/owner/settings");
    probe!(crate::services::operations::stock::stock, "/api/owner/stock");
    probe!(crate::services::operations::preps::list_preps, "/api/owner/preps");
    probe!(crate::booking::get_plan, "/api/owner/floorplan");
    assert!(bad.is_empty(), "{} route(s) did not hold:\n{}", bad.len(), bad.join("\n"));
}

fn supply(site: &Site, a: &str, id: &str) {
    let r = site.run(
        crate::services::operations::supplies::set_supply,
        post(&at("alpha", "/api/owner/supplies"), &json!({"id": id, "name": id, "unit": "g"})).bearer(a).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
}

fn stock_move(site: &Site, a: &str, kind: &str, body: Value) -> crate::wire::Reply {
    site.run(
        crate::services::operations::stock::stock_move,
        post(&at("alpha", &format!("/api/owner/stock/{kind}")), &body).bearer(a).on("alpha"),
        &[("kind", kind)],
    )
}

#[test]
fn stock_is_received_wasted_counted_and_reported() {
    let (site, a, b) = two();
    supply(&site, &a, "salmon");
    let r = stock_move(&site, &a, "received", json!({"item": "salmon", "qty": 5000}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(stock_move(&site, &a, "wasted", json!({"item": "salmon", "qty": 100})).status_code(), 400, "a write-off says why");
    assert_eq!(stock_move(&site, &a, "wasted", json!({"item": "salmon", "qty": 100, "reason": "bored"})).status_code(), 400);
    let r = stock_move(&site, &a, "wasted", json!({"item": "salmon", "qty": 100, "reason": "spoiled"}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let r = stock_move(&site, &a, "stocktake", json!({"item": "salmon", "observed": 4800}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(stock_move(&site, &a, "received", json!({"item": "", "qty": 1})).status_code(), 400);
    assert_eq!(stock_move(&site, &a, "removed", json!({"item": "salmon"})).status_code(), 400);
    assert_eq!(stock_move(&site, &a, "received", json!({"item": "salmon", "qty": 1, "by": "someone-else"})).status_code(), 400, "no body names its signer");
    let stock = site.run(crate::services::operations::stock::stock, get(&at("alpha", "/api/owner/stock")).bearer(&a).on("alpha"), &[]);
    assert!(stock.body_str().contains("4800"), "the count is the balance: {}", stock.body_str());
    let w = site.run(crate::services::operations::waste::waste_report, get(&at("alpha", "/api/owner/stock/waste")).bearer(&a).on("alpha"), &[]);
    assert!(w.body_str().contains("spoiled"), "{}", w.body_str());
    let foreign = site.run(
        crate::services::operations::stock::stock_move,
        post(&at("alpha", "/api/owner/stock/received?location_id=alpha"), &json!({"item": "salmon", "qty": 1})).bearer(&b).on("alpha"),
        &[("kind", "received")],
    );
    assert!(foreign.status_code() >= 400, "{}", foreign.body_str());
}

#[test]
fn supplies_are_added_in_bulk_deleted_and_the_ingredients_reset_needs_its_word() {
    let (site, a, _b) = two();
    let r = site.run(
        crate::services::operations::supplies::quick::add_supplies,
        post(&at("alpha", "/api/owner/supplies/bulk"), &json!({"items": [{"id": "nori", "name": "Nori", "unit": "unit"}, {"id": "wasabi", "name": "Wasabi", "unit": "g"}]}))
            .bearer(&a)
            .on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let s = site.run(crate::services::operations::stock::stock, get(&at("alpha", "/api/owner/stock")).bearer(&a).on("alpha"), &[]);
    assert!(s.body_str().contains("nori") && s.body_str().contains("wasabi"), "{}", s.body_str());
    let r = site.run(
        crate::services::operations::supplies::delete::delete_supplies,
        post(&at("alpha", "/api/owner/supplies/delete"), &json!({"ids": ["wasabi"]})).bearer(&a).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let s = site.run(crate::services::operations::stock::stock, get(&at("alpha", "/api/owner/stock")).bearer(&a).on("alpha"), &[]);
    assert!(!s.body_str().contains("wasabi"), "{}", s.body_str());
    let no_word = site.run(
        crate::services::operations::ingredients_reset::reset_ingredients,
        post(&at("alpha", "/api/owner/ingredients/reset"), &json!({"confirm": "maybe"})).bearer(&a).on("alpha"),
        &[],
    );
    assert!(no_word.status_code() >= 400, "{}", no_word.body_str());
}

#[test]
fn a_menu_csv_is_previewed_then_applied_and_a_bad_header_is_refused() {
    let (site, a, _b) = two();
    let csv = "name,price,category\nMiso soup,400,Soups\nRamen,1200,Soups\n";
    let call = |q: &str, body: &str| {
        crate::wire::Call::new(&at("alpha", &format!("/api/owner/menu/import{q}")), worker::Method::Post)
            .unwrap()
            .with_body(body.as_bytes().to_vec())
            .bearer(&a)
            .on("alpha")
    };
    let dry = site.run(crate::services::catalogue::import::import_menu, call("", csv), &[]);
    assert_eq!(dry.status_code(), 200, "{}", dry.body_str());
    let menu_before = site.run(crate::services::catalogue::import::bulk::owner_products, get(&at("alpha", "/api/owner/products")).bearer(&a).on("alpha"), &[]);
    assert!(!menu_before.body_str().contains("Ramen"), "a preview writes nothing");
    let applied = site.run(crate::services::catalogue::import::import_menu, call("?apply=1", csv), &[]);
    assert_eq!(applied.status_code(), 200, "{}", applied.body_str());
    let after = site.run(crate::services::catalogue::import::bulk::owner_products, get(&at("alpha", "/api/owner/products")).bearer(&a).on("alpha"), &[]);
    assert!(after.body_str().contains("Ramen"), "{}", after.body_str());
    let bad = site.run(crate::services::catalogue::import::import_menu, call("?apply=1", "dish;cost\nx;1\n"), &[]);
    assert!(bad.status_code() == 400 || bad.body_str().contains("header"), "{}", bad.body_str());
}

#[test]
fn the_venues_place_is_written_field_by_field_and_a_bad_coordinate_is_refused() {
    let (site, a, b) = two();
    let set = |tok: &str, body: serde_json::Value| {
        site.run(crate::services::venue::place::set_place, post(&at("alpha", "/api/owner/place"), &body).bearer(tok).on("alpha"), &[])
    };
    let menu = || {
        site.run(crate::storefront::menu, get(&at("alpha", "/api/public/locations/alpha/menu?fresh=1")).on("alpha"), &[("slug", "alpha")]).body_str()
    };
    let week = |open: i64, close: i64| serde_json::json!((0..7).map(|_| vec![serde_json::json!({"open": open, "close": close})]).collect::<Vec<_>>());
    let r = set(&a, serde_json::json!({"address": "Rruga e Durrësit 5", "lat": 41.3231, "lng": 19.4414, "hours": week(660, 1380)}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let m = menu();
    assert!(m.contains("Rruga e Durrësit 5") && m.contains("41.3231"), "{m}");
    // A field not sent is not erased.
    assert_eq!(set(&a, serde_json::json!({"google": {"rating": 4.6, "url": "https://maps.example/x"}})).status_code(), 200);
    let m = menu();
    assert!(m.contains("Rruga e Durrësit 5") && m.contains("maps.example"), "{m}");
    // Refused, not clamped: a latitude of 91, a week of six days, an unknown field.
    assert_eq!(set(&a, serde_json::json!({"lat": 91.0, "lng": 19.0})).status_code(), 400);
    let six = serde_json::json!((0..6).map(|_| Vec::<serde_json::Value>::new()).collect::<Vec<_>>());
    assert_eq!(set(&a, serde_json::json!({"hours": six})).status_code(), 400);
    assert_eq!(set(&a, serde_json::json!({"phone": "1"})).status_code(), 400);
    assert!(!menu().contains("91.0"), "nothing refused was written");
    // An empty address clears it.
    assert_eq!(set(&a, serde_json::json!({"address": "  "})).status_code(), 200);
    assert!(!menu().contains("Rruga e Durrësit 5"));
    // Beta's owner does not move alpha.
    let r = set(&b, serde_json::json!({"address": "Somewhere else", "location_id": "alpha"}));
    assert!(r.status_code() >= 400, "{}", r.body_str());
    let r = site.run(
        crate::services::venue::place::set_place,
        post(&at("alpha", "/api/owner/place?location_id=alpha"), &serde_json::json!({"address": "Somewhere else"})).bearer(&b).on("alpha"),
        &[],
    );
    assert!(r.status_code() >= 400, "{}", r.body_str());
    assert!(!menu().contains("Somewhere else"));
}

#[test]
fn a_logo_samples_dominant_colours_each_with_its_contrast_verdict() {
    let (site, a, _) = two();
    let ex = |tok: Option<&str>, pixels: &str| {
        let c = post(&at("alpha", "/api/owner/branding/extract"), &json!({"pixels": pixels})).on("alpha");
        let c = match tok {
            Some(t) => c.bearer(t),
            None => c,
        };
        site.run(crate::services::venue::brand_extract::extract_branding, c, &[])
    };
    // Mostly a deep red, some white: red first, by share.
    let pixels = "b01e2e".repeat(70) + &"ffffff".repeat(30);
    let r = ex(Some(&a), &pixels);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    let s = v["swatches"].as_array().unwrap();
    assert!(!s.is_empty(), "{v}");
    assert!(s[0]["sharePct"].as_f64().unwrap() >= s.last().unwrap()["sharePct"].as_f64().unwrap(), "{v}");
    assert!(s.iter().all(|w| w["hex"].as_str().unwrap().starts_with('#') && w["passes"].is_boolean()), "{v}");
    // Refused, each by name: not triples, not hex, too many, no owner.
    assert_eq!(ex(Some(&a), "ff00").status_code(), 400);
    assert_eq!(ex(Some(&a), "zzzzzz").status_code(), 400);
    assert_eq!(ex(Some(&a), "").status_code(), 400);
    assert_eq!(ex(Some(&a), &"000000".repeat(65_537)).status_code(), 400);
    assert_eq!(ex(None, "ffffff").status_code(), 401);
}
