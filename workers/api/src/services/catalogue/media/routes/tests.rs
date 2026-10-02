//! Photographs and the logo through their routes (W-COV C2): bytes go to the MEDIA namespace
//! under a content address, the catalogue names them, `/media/:name` serves them back with the
//! headers that make an upload safe to show -- and what is not an image is refused before any
//! byte is stored. Another venue's owner changes nothing here.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::open_venue;
use crate::wire::Call;
use base64::Engine;

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}

/// A complete 1x1 PNG: it begins with the magic AND ends with IEND (`media::prepare` reads both).
fn png() -> Vec<u8> {
    base64::engine::general_purpose::STANDARD
        .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==")
        .unwrap()
}

fn menu(site: &Site) -> serde_json::Value {
    site.run(crate::storefront::menu, get(&at("/api/public/locations/alpha/menu?fresh=1")).on("alpha"), &[("slug", "alpha")]).body_value()
}

fn product_field(site: &Site, id: &str, field: &str) -> serde_json::Value {
    let m = menu(site).to_string();
    let v: serde_json::Value = serde_json::from_str(&m).unwrap();
    fn find(v: &serde_json::Value, id: &str) -> Option<serde_json::Value> {
        match v {
            serde_json::Value::Object(o) if o.get("id").and_then(|x| x.as_str()) == Some(id) && o.contains_key("name") => Some(v.clone()),
            serde_json::Value::Object(o) => o.values().find_map(|x| find(x, id)),
            serde_json::Value::Array(a) => a.iter().find_map(|x| find(x, id)),
            _ => None,
        }
    }
    find(&v, id).unwrap_or_else(|| panic!("no {id} in {m}"))[field].clone()
}

#[test]
fn a_dish_photo_is_stored_named_served_and_cleared() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let set = |tok: &str, id: &str, query: &str, bytes: Vec<u8>| {
        site.run(
            crate::services::catalogue::media::set_product_image,
            Call::new(&at(&format!("/api/owner/products/{id}/image{query}")), worker::Method::Post).unwrap().with_body(bytes).bearer(tok).on("alpha"),
            &[("id", id)],
        )
    };
    // Not an image, a truncated one, and a dish that does not exist: refused, nothing stored.
    assert_eq!(set(&t, &dish, "", b"hello, world".to_vec()).status_code(), 400);
    let mut cut = png();
    cut.truncate(cut.len() - 12);
    assert_eq!(set(&t, &dish, "", cut).status_code(), 400, "a PNG with no end");
    assert_eq!(set(&t, "ghost", "", png()).status_code(), 404);
    assert!(site.world.kv.borrow().is_empty(), "a refusal stored bytes");

    let r = set(&t, &dish, "", png());
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let url = r.body_value()["imageUrl"].as_str().unwrap().to_string();
    assert!(url.starts_with("/media/") && url.ends_with(".png"), "{url}");
    assert_eq!(r.body_value()["type"], "image/png");
    assert_eq!(product_field(&site, &dish, "imageUrl"), url.as_str());

    // Served back: the same bytes, its own type, and the headers that keep an upload inert.
    let name = url.trim_start_matches("/media/").to_string();
    let served = site.run(crate::services::catalogue::media::media, get(&at(&url)).on("alpha"), &[("name", &name)]);
    assert_eq!(served.status_code(), 200);
    assert_eq!(served.body(), png().as_slice());
    let h = served.headers();
    assert_eq!(h.get("content-type").unwrap().as_deref(), Some("image/png"));
    assert_eq!(h.get("x-content-type-options").unwrap().as_deref(), Some("nosniff"));
    assert!(h.get("content-security-policy").unwrap().unwrap_or_default().contains("sandbox"));
    for bad in ["../etc", "x".repeat(81).as_str(), "nope.png"] {
        let r = site.run(crate::services::catalogue::media::media, get(&at("/media/x")).on("alpha"), &[("name", bad)]);
        assert_eq!(r.status_code(), 404, "{bad}");
    }

    // The card's small rendering is a second field; a new full photo drops the stale small one.
    let r = set(&t, &dish, "?variant=small", png());
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(product_field(&site, &dish, "imageUrlSmall"), url.as_str());
    assert_eq!(set(&t, &dish, "", png()).status_code(), 200);
    assert!(product_field(&site, &dish, "imageUrlSmall").is_null());

    // Another venue's owner neither sets nor clears alpha's photo.
    let (beta, _) = open_venue(&site, "beta", "b@x.test");
    let r = site.run(
        crate::services::catalogue::media::set_product_image,
        Call::new(&at(&format!("/api/owner/products/{dish}/image?location_id=alpha")), worker::Method::Post).unwrap().with_body(png()).bearer(&beta).on("alpha"),
        &[("id", &dish)],
    );
    assert!(r.status_code() >= 400, "{}", r.body_str());
    let clear = |tok: &str, id: &str| {
        site.run(
            crate::services::catalogue::media::clear_product_image,
            post(&at(&format!("/api/owner/products/{id}/image/clear?location_id=alpha")), &serde_json::json!({})).bearer(tok).on("alpha"),
            &[("id", id)],
        )
    };
    assert!(clear(&beta, &dish).status_code() >= 400);
    assert_eq!(product_field(&site, &dish, "imageUrl"), url.as_str(), "beta's refusal cleared nothing");
    assert_eq!(clear(&t, "ghost").status_code(), 404);
    assert_eq!(clear(&t, &dish).status_code(), 200);
    assert!(product_field(&site, &dish, "imageUrl").is_null());
}

#[test]
fn the_logo_is_stored_and_becomes_the_storefronts_icon() {
    let site = Site::new();
    let (t, _) = open_venue(&site, "alpha", "a@x.test");
    let logo = |bytes: Vec<u8>| {
        site.run(
            crate::services::catalogue::media::set_venue_logo,
            Call::new(&at("/api/owner/logo"), worker::Method::Post).unwrap().with_body(bytes).bearer(&t).on("alpha"),
            &[],
        )
    };
    assert_eq!(logo(b"GIF89a-not-really".to_vec()).status_code(), 400);
    let r = logo(png());
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let url = r.body_value()["logoUrl"].as_str().unwrap().to_string();
    let manifest = || site.run(crate::storefront::manifest, get(&at("/manifest.webmanifest")).on("alpha"), &[]).body_value();
    let m = manifest();
    let icon = m["icons"].as_array().unwrap().iter().find(|i| i["src"] == url.as_str()).cloned().unwrap_or_else(|| panic!("{m}"));
    assert_eq!(icon["sizes"], "1x1", "the size is read from the stored PNG: {icon}");
    let r = site.run(crate::services::catalogue::media::clear_venue_logo, post(&at("/api/owner/logo/clear"), &serde_json::json!({})).bearer(&t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200);
    assert!(!manifest().to_string().contains(&url), "the cleared logo is no icon");
    let anon = site.run(crate::services::catalogue::media::clear_venue_logo, post(&at("/api/owner/logo/clear"), &serde_json::json!({})).on("alpha"), &[]);
    assert_eq!(anon.status_code(), 401);
}
