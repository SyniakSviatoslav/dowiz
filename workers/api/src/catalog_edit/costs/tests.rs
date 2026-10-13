//! WHAT ONE OWNER CATALOGUE EDIT COSTS ON THE HOP (W-D9, R-GRAPH D9).
//!
//! Every billed Durable Object request is counted by the test edge with the bytes it
//! carried each way (`edge::mem::object_traffic`): not estimated. Each edit the owner's
//! console makes is run twice over the same fat menu (165 dishes, the live venue's size):
//! `Via::Worker` runs the handler's body (`*_at`) from the Worker, as every catalogue edit
//! ran before W-D9 -- `hubstore::with_catalog` read the whole catalogue image across the hop
//! and wrote the whole image back; `Via::Object` runs the route as the router does now
//! (`hubdo/command.rs`): the Worker picks the object, the object runs the same body in its
//! own turn, and the image never leaves it. Both walks must end in the same menu.

use crate::edge::mem::{block_on, object_traffic};
use crate::edge::site::{post, As, Site, PLATFORM_HOST};
use crate::storefront::route_costs::is_venue;
use crate::wire::{Call, Reply};
use serde_json::{json, Value};

const DISHES: usize = 165;

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Via {
    Worker,
    Object,
}

/// One edit: its name, the venue-object requests it made (path, bytes sent, bytes back), its status.
struct Step {
    name: &'static str,
    venue: Vec<(String, usize, usize)>,
    status: u16,
}

impl Step {
    fn bytes(&self) -> usize {
        self.venue.iter().map(|(_, s, r)| s + r).sum()
    }
}

/// A dish as the live venue writes one: every field the storefront passes through.
fn fat(n: usize, cat: &str) -> String {
    json!({
        "id": format!("p_{n}"), "categoryId": cat, "name": format!("Roll {n} me salmon dhe avokado"),
        "description": format!("Oriz sushi, salmon i freskët, avokado, krem djathi, susam i pjekur dhe salcë e shtëpisë. Pjata {n}."),
        "price": 700 + 10 * n as i64, "sortOrder": n as i64, "available": true, "allergens": ["fish", "sesame"],
        "imageUrl": format!("/media/p{n}.jpg"), "imageUrlSmall": format!("/media/p{n}-s.jpg"),
        "modifierGroups": [{"id": "g_size", "name": "Madhësia", "min": 1, "max": 1,
            "options": [{"id": "o_8", "name": "8 copë", "price": 0}, {"id": "o_12", "name": "12 copë", "price": 350}]}],
        "sizeCm": 22, "cookingMin": 9, "tags": ["salmon", "cold"], "ingredients": ["oriz", "salmon", "avokado"],
        "weightG": 260, "nutrition": {"kcal": 420, "protein_g": 18, "fat_g": 14, "carbs_g": 52}, "station": "sushi",
    })
    .to_string()
}

/// The venue `alpha` with an owner, one category and a menu of the live venue's size.
/// The owner's token, the dish to edit, the category, and the catalogue image's length.
fn fat_venue(site: &Site) -> (String, String, String, usize) {
    let (t, dish) = crate::storefront::route_tests::open_venue(site, "alpha", "alpha@example.com");
    let place = crate::hubstore::Place::of_authorised(&site.ctx(&[]), "alpha").expect("place");
    let cat = block_on(crate::hubstore::with_catalog(&place, |c| {
        let cat = c.categories()[0].0.clone();
        for n in 0..DISHES {
            c.set_product(&format!("p_{n}"), &fat(n, &cat));
        }
        Ok(cat)
    }))
    .expect("fatten");
    let len = block_on(crate::hubstore::load_catalog(&place)).expect("load").catalog.to_bytes().expect("bytes").len();
    (t, dish, cat, len)
}

/// The venue-object requests `f` made, with their bytes, and what it answered.
fn paid(f: impl FnOnce() -> Reply) -> (Vec<(String, usize, usize)>, Reply) {
    let from = object_traffic().len();
    let r = f();
    (object_traffic()[from..].iter().filter(|(p, _, _)| is_venue(p)).cloned().collect(), r)
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

fn png() -> Vec<u8> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==")
        .unwrap()
}

/// Every catalogue edit the console makes, in order, and the menu as it ended.
fn the_edits(site: &Site, via: Via) -> (Vec<Step>, usize, Value) {
    let (t, dish, cat, image) = fat_venue(site);
    let mut steps = Vec::new();
    let mut step = |name: &'static str, f: &dyn Fn() -> Reply| {
        let (venue, r) = paid(f);
        steps.push(Step { name, venue, status: r.status_code() });
        r
    };
    let p = |path: &str, body: Value| post(&at(path), &body).bearer(&t).on("alpha");
    let d = format!("/api/owner/products/{dish}");
    use crate::catalog_edit as ce;
    let made = step("dish create", &|| by!(via, site, ce::create_product_at, ce::create_product, p("/api/owner/products", json!({"location_id": "alpha", "category_id": cat, "name": "Gyoza", "price": 600})), &[]));
    let gyoza = made.body_value()["id"].as_str().unwrap_or_default().to_string();
    step("dish edit (name)", &|| by!(via, site, crate::owner::update_product_at, crate::owner::update_product, p(&d, json!({"location_id": "alpha", "name": "Futomaki XL"})), &[("id", &dish)]));
    step("dish price", &|| by!(via, site, crate::owner::update_product_at, crate::owner::update_product, p(&d, json!({"location_id": "alpha", "price": 950})), &[("id", &dish)]));
    step("dish availability", &|| by!(via, site, crate::owner::update_product_at, crate::owner::update_product, p(&d, json!({"location_id": "alpha", "available": false, "unavailable_note": "sold out"})), &[("id", &dish)]));
    step("dish photo", &|| {
        let call = Call::new(&at(&format!("{d}/image")), worker::Method::Post).unwrap().with_body(png()).bearer(&t).on("alpha");
        site.run(crate::services::catalogue::media::set_product_image, call, &[("id", &dish)])
    });
    step("dish photo clear", &|| by!(via, site, crate::services::catalogue::media::clear_product_image_at, crate::services::catalogue::media::clear_product_image, p(&format!("{d}/image/clear"), json!({})), &[("id", &dish)]));
    step("category create", &|| by!(via, site, ce::set_category_at, ce::set_category, p("/api/owner/categories", json!({"location_id": "alpha", "name": "Soups"})), &[]));
    step("category rename", &|| by!(via, site, ce::set_category_at, ce::set_category, p("/api/owner/categories", json!({"location_id": "alpha", "id": cat, "name": "Rolls & more"})), &[]));
    step("translation", &|| {
        let e = json!([{"entity": "product", "id": dish, "locale": "en", "field": "name", "value": "Futomaki"}]);
        by!(via, site, crate::owner::write_translations_at, crate::owner::write_translations, p("/api/owner/i18n", json!({"location_id": "alpha", "entries": e})), &[])
    });
    step("dish delete", &|| by!(via, site, ce::delete_product_at, ce::delete_product, p(&format!("/api/owner/products/{gyoza}/delete"), json!({"location_id": "alpha"})), &[("id", &gyoza)]));
    step("dishes delete (many)", &|| by!(via, site, ce::delete_products_at, ce::delete_products, p("/api/owner/products/delete?location_id=alpha", json!({"ids": ["p_1", "p_2"]})), &[]));
    let h = site.run(crate::catalog_history::list, Call::new(&at("/api/owner/menu/history?location_id=alpha&limit=50"), worker::Method::Get).unwrap().bearer(&t).on("alpha"), &[]).body_value();
    let newest = h["edits"].as_array().and_then(|l| l.iter().find(|e| e["restorable"] == true)).cloned().unwrap_or(Value::Null);
    step("history restore", &|| by!(via, site, crate::catalog_history::restore_at, crate::catalog_history::restore, p("/api/owner/menu/history/restore", json!({"location_id": "alpha", "seq": newest["seq"], "key": newest["key"]})), &[]));
    step("menu import (csv)", &|| {
        let call = Call::new(&at("/api/owner/menu/import?apply=true"), worker::Method::Post).unwrap().with_body(b"name,price\nSake Nigiri,900\nMiso,400\n".to_vec()).bearer(&t).on("alpha");
        by!(via, site, crate::services::catalogue::import::import_menu_at, crate::services::catalogue::import::import_menu, call, &[])
    });
    step("venue status", &|| by!(via, site, crate::owner::update_location_at, crate::owner::update_location, p("/api/owner/location", json!({"location_id": "alpha", "status": "busy"})), &[]));
    let place = crate::hubstore::Place::of_authorised(&site.ctx(&[]), "alpha").expect("place");
    let c = block_on(crate::hubstore::load_catalog(&place)).expect("load").catalog;
    let mut loc: Value = serde_json::from_str(&c.location().unwrap_or_default()).unwrap_or(Value::Null);
    loc["menu_version"] = Value::Null; // the same count of bumps either way; checked below by equality anyway
    let menu = json!({ "products": c.products(), "categories": c.categories(), "status": loc["status"] });
    (steps, image, menu)
}

fn table(via: Via, steps: &[Step], image: usize) {
    println!("{via:?} (catalogue image {image} B):");
    for s in steps {
        let paths: Vec<&str> = s.venue.iter().map(|(p, _, _)| p.as_str()).collect();
        println!("  {:<22} {} venue req  {:>8} B  status {}  {:?}", s.name, s.venue.len(), s.bytes(), s.status, paths);
    }
}

/// THE RED (W-D9): a catalogue edit is ONE venue-object request and moves no catalogue
/// image across the hop. A photo is two: the dish must exist before its blob is written
/// (`media::set_product_image`), then the edit. Both walks end in the same menu.
#[test]
fn a_catalogue_edit_is_one_object_request_and_moves_no_image() {
    let (before, image, menu_before) = the_edits(&Site::new(), Via::Worker);
    let (after, _, menu_after) = the_edits(&Site::new(), Via::Object);
    table(Via::Worker, &before, image);
    table(Via::Object, &after, image);
    assert!(image > 100_000, "the fixture is a real menu: {image} B");
    for (b, a) in before.iter().zip(&after) {
        assert_eq!((b.name, b.status), (a.name, a.status), "the same answer either way");
        assert!(a.status < 300, "{}: {}", a.name, a.status);
        // The measurement measures: from the Worker, each of these moved a whole image (photo aside: both walks route).
        assert!(a.name == "dish photo" || b.venue.iter().any(|(p, _, _)| p.starts_with("/img/")), "{}: the Worker walk {:?}", b.name, b.venue);
        let most = if a.name == "dish photo" { 2 } else { 1 };
        assert!(a.venue.len() <= most, "{}: {} venue-object requests {:?}", a.name, a.venue.len(), a.venue);
        assert!(!a.venue.iter().any(|(p, _, _)| p.starts_with("/img/")), "{}: an image crossed the hop {:?}", a.name, a.venue);
        // The import answers with its per-row report (the owner reads it either way), so it is
        // held to a twentieth of what its Worker walk moved rather than of the image.
        let bound = if a.name == "menu import (csv)" { b.bytes() } else { image };
        assert!(a.bytes() * 20 < bound, "{}: {} B crossed for a {bound} B bound", a.name, a.bytes());
    }
    assert_eq!(menu_before, menu_after, "the move changed the bill and nothing else");
}

/// MEASURED, not asserted (`cargo test --release --lib measure_catalogue_edit_wall -- --ignored
/// --nocapture`): the wall of one price edit over the fat menu, from the Worker (`*_at`: the
/// image read and written over the test edge) and as routed (the object's own turn). The test
/// edge's hop is a function call, so this is the CPU of an edit on each side, NOT the platform's
/// transfer of the image twice -- which is what the bytes above count and this cannot.
#[test]
#[ignore]
fn measure_catalogue_edit_wall() {
    const N: u32 = 30;
    for via in [Via::Worker, Via::Object] {
        let site = Site::new();
        let (t, dish, _, image) = fat_venue(&site);
        let d = format!("/api/owner/products/{dish}");
        let t0 = std::time::Instant::now();
        for k in 0..N {
            let call = post(&at(&d), &json!({"location_id": "alpha", "price": 1000 + k})).bearer(&t).on("alpha");
            let r = by!(via, site, crate::owner::update_product_at, crate::owner::update_product, call, &[("id", &dish)]);
            assert_eq!(r.status_code(), 200, "{}", r.body_str());
        }
        println!("{via:?}: one price edit = {} us (catalogue image {image} B, N={N})", t0.elapsed().as_micros() / N as u128);
    }
}
