//! R2's tests: the catalogue projection, through the real `Memo` and `read`.

use super::*;
use dowiz_hub::catalog::Catalog;
use dowiz_hub::settings::Settings;
use serde_json::json;

/// 2026-07-01 12:00 in Tirane (10:00 UTC), and 03:00 the same night.
const NOON: i64 = 1_782_900_000_000;
const NIGHT: i64 = 1_782_867_600_000;

fn venue(extra: Value) -> Value {
    let mut v = json!({
        "id": "loc_1", "name": "Dubin & Sushi", "slug": "dubin", "phone": "+355", "address": null,
        "status": "open", "closes_at": null, "delivery_eta": "30-40", "delivery_fee": 200,
        "free_delivery_threshold": null, "min_order": 1000, "currency_code": "ALL", "menu_version": 7,
        "supported_locales": "[\"sq\",\"en\"]", "default_locale": "sq", "delivery_paused": 0,
        "hours": [[{"open": 660, "close": 1380}], [{"open": 660, "close": 1380}], [{"open": 660, "close": 1380}],
            [{"open": 660, "close": 1380}], [{"open": 660, "close": 1380}], [{"open": 660, "close": 1380}],
            [{"open": 660, "close": 1380}]],
    });
    for (k, x) in extra.as_object().unwrap() {
        v[k] = x.clone();
    }
    v
}

fn catalog(rec: Option<&str>) -> Catalog {
    let mut c = Catalog::create().unwrap();
    if let Some(r) = rec {
        c.set_location(r);
    }
    c.set_category("c_rolls", &json!({"name": "Rolls", "sortOrder": 2}).to_string());
    c.set_category("c_soup", &json!({"name": "Supa", "sortOrder": 1}).to_string());
    c.set_category("c_empty", &json!({"name": "Bosh", "sortOrder": 3}).to_string());
    c.set_product("p_b", &json!({"name": "Maki", "categoryId": "c_rolls", "price": 900, "sortOrder": 2,
        "ingredients": ["oriz"], "allergens": [], "cookingMin": 12}).to_string());
    c.set_product("p_a", &json!({"name": "Nigiri", "categoryId": "c_rolls", "price": 700, "sortOrder": 1,
        "available": false}).to_string());
    c.set_product("p_s", &json!({"name": "Miso", "categoryId": "c_soup", "price": 400}).to_string());
    c
}

fn words() -> Vec<(String, String)> {
    vec![
        ("en/product/p_b/name".into(), "Maki roll".into()),
        ("en/product/p_b/ingredients".into(), "[\"rice\"]".into()),
        ("en/product/p_a/ingredients".into(), "not a list".into()),
        ("en/category/c_rolls/name".into(), "Rolls EN".into()),
        ("en/product/p_s/price".into(), "1".into()),
        ("uk/product/p_b/name".into(), "Макі".into()),
    ]
}

fn memo_with(rec: Option<Value>, i18n: Result<Vec<(String, String)>, String>, rails: Rails) -> Memo {
    let text = rec.map(|r| r.to_string());
    Memo::build(Gens { catalog: 3, i18n: 2, settings: 1 }, &catalog(text.as_deref()), i18n, &Settings::create().unwrap(), rails)
}

fn memo() -> Memo {
    memo_with(Some(venue(json!({}))), Ok(words()), Rails::default())
}

fn body(a: Answer) -> Value {
    match a {
        Answer::Body(b) => serde_json::from_str(&b).expect("the menu is JSON"),
        other => panic!("expected a menu, got {other:?}"),
    }
}

#[test]
fn a_memo_is_current_only_at_the_generations_it_was_folded_from() {
    let at = Gens { catalog: 3, i18n: 2, settings: 1 };
    let m = Some(memo());
    assert!(is_current(&m, at), "the same three generations: a hit");
    for bumped in [Gens { catalog: 4, ..at }, Gens { i18n: 3, ..at }, Gens { settings: 2, ..at }, Gens { catalog: 0, ..at }] {
        assert!(!is_current(&m, bumped), "a catalogue, translation or settings write refolds: {bumped:?}");
    }
    assert!(!is_current(&None, at), "no memo is never current");
    assert!(!is_current(&None, Gens::default()), "not even for a venue with no images");
}

#[test]
fn from_images_reads_absent_as_empty_and_corrupt_as_an_error() {
    let g = Gens { catalog: 1, i18n: 0, settings: 0 };
    let mut c = catalog(Some(&venue(json!({})).to_string()));
    let cat = c.to_bytes().unwrap();
    let mut m = from_images_ok(g, Some(&cat), None, None);
    assert_eq!(body(m.menu("dubin", Some("en"), NOON))["warnings"], json!([]), "no translation image: no words, no warning");
    let mut t = dowiz_hub::table::Table::create(crate::hubstore::I18N_BYTES).unwrap();
    t.put(crate::hubstore::I18N_KIND, "en/product/p_b/name", "Maki roll", &[], &[]).unwrap();
    let words = t.to_bytes().unwrap();
    let mut s = Settings::create().unwrap();
    let set = s.to_bytes().unwrap();
    let mut m = from_images_ok(g, Some(&cat), Some(&words), Some(&set));
    assert_eq!(body(m.menu("dubin", Some("en"), NOON))["categories"][1]["products"][1]["name"], "Maki roll");
    let mut broken_words = from_images_ok(g, Some(&cat), Some(b"not a table"), Some(&set));
    assert_eq!(body(broken_words.menu("dubin", Some("en"), NOON))["warnings"], json!(["translations unavailable: image i18n is unreadable"]));
    assert_eq!(Memo::from_images(g, Some(b"junk"), None, None, Rails::default()).err().as_deref(), Some("catalogue image is unreadable"));
    assert_eq!(Memo::from_images(g, Some(&cat), None, Some(b"junk"), Rails::default()).err().as_deref(), Some("settings image is unreadable"));
    let mut empty = from_images_ok(Gens::default(), None, None, None);
    assert_eq!(empty.menu("dubin", None, NOON), Answer::NotFound, "a venue with no catalogue has no menu");
    assert_eq!(empty.gens(), Gens::default());
}

fn from_images_ok(g: Gens, c: Option<&[u8]>, i: Option<&[u8]>, s: Option<&[u8]>) -> Memo {
    Memo::from_images(g, c, i, s, Rails::default()).expect("readable images")
}

#[test]
fn the_venues_own_language_orders_groups_and_passes_fields_through() {
    let out = body(memo().menu("dubin", None, NOON));
    let cats = out["categories"].as_array().unwrap();
    let names: Vec<&str> = cats.iter().map(|c| c["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["Supa", "Rolls"], "by sortOrder, and an empty category is left out");
    let rolls: Vec<&str> = cats[1]["products"].as_array().unwrap().iter().map(|p| p["name"].as_str().unwrap()).collect();
    assert_eq!(rolls, ["Nigiri", "Maki"]);
    let maki = &cats[1]["products"][1];
    assert_eq!(maki["allergens"], json!([]), "[] is a claim and stays one");
    assert_eq!(maki["cookingMin"], 12);
    assert_eq!(cats[0]["products"][0]["allergens"], Value::Null, "absent stays absent");
    assert_eq!(cats[1]["products"][0]["available"], false);
    assert_eq!(cats[0]["products"][0]["available"], true);
    assert_eq!(out["warnings"], json!([]));
    assert_eq!(out["stripePublishableKey"], Value::Null);
    assert_eq!(out["location"]["menuVersion"], 7);
    assert_eq!(out["location"]["tz"], "Europe/Tirane");
    assert_eq!(out["location"]["supportedLocales"], json!(["sq", "en"]));
}

#[test]
fn another_language_is_translated_and_falls_back_to_the_venues_words() {
    let out = body(memo().menu("dubin", Some("en"), NOON));
    let rolls = &out["categories"][1];
    assert_eq!(rolls["name"], "Rolls EN");
    assert_eq!(rolls["products"][1]["name"], "Maki roll");
    assert_eq!(rolls["products"][1]["ingredients"], json!(["rice"]));
    assert_eq!(rolls["products"][0]["name"], "Nigiri", "no translation: the venue's own name");
    assert_eq!(rolls["products"][0]["ingredients"], Value::Null, "a stored list that is not a list falls back");
    assert_eq!(out["categories"][0]["products"][0]["price"], 400, "only name, description and ingredients translate");
    assert_eq!(out["categories"][0]["name"], "Supa");
}

#[test]
fn a_broken_translation_table_warns_on_another_language_only() {
    let mut m = memo_with(Some(venue(json!({}))), Err("image i18n is unreadable".into()), Rails::default());
    let en = body(m.menu("dubin", Some("en"), NOON));
    assert_eq!(en["warnings"], json!(["translations unavailable: image i18n is unreadable"]));
    assert_eq!(en["categories"][1]["products"][1]["name"], "Maki");
    let own = body(m.menu("dubin", Some("sq"), NOON));
    assert_eq!(own["warnings"], json!([]));
    let blank = body(m.menu("dubin", Some(""), NOON));
    assert_eq!(blank["warnings"], json!([]), "an empty locale is the venue's own");
}

#[test]
fn the_status_is_decided_per_request_from_the_clock() {
    let mut m = memo();
    let day = body(m.menu("dubin", None, NOON));
    assert_eq!(day["location"]["status"], "open");
    assert_eq!(day["location"]["closedReason"], Value::Null);
    let night = body(m.menu("dubin", None, NIGHT));
    assert_eq!(night["location"]["status"], "closed");
    assert_eq!(night["location"]["closedReason"], "hours");
    assert_eq!(night["location"]["nextOpen"]["minute"], 660);
    assert_eq!(night["location"]["ownerStatus"], "open");
    let mut paused = memo_with(Some(venue(json!({"delivery_paused": 1}))), Ok(vec![]), Rails::default());
    let p = body(paused.menu("dubin", None, NOON));
    assert_eq!((p["location"]["status"].as_str(), p["location"]["closedReason"].as_str()), (Some("closed"), Some("paused")));
    assert_eq!(p["location"]["deliveryPaused"], true);
    let mut shut = memo_with(Some(venue(json!({"status": "closed", "hours": null}))), Ok(vec![]), Rails::default());
    let s = body(shut.menu("dubin", None, NOON));
    assert_eq!((s["location"]["status"].as_str(), s["location"]["closedReason"].as_str()), (Some("closed"), Some("manual")));
    let mut always = memo_with(Some(venue(json!({"hours": null}))), Ok(vec![]), Rails::default());
    assert_eq!(body(always.menu("dubin", None, NIGHT))["location"]["status"], "open", "no schedule never closes");
}

#[test]
fn the_slug_must_match_and_a_missing_or_broken_record_says_which() {
    assert_eq!(memo().menu("other", None, NOON), Answer::NotFound);
    let mut none = memo_with(None, Ok(vec![]), Rails::default());
    assert_eq!(none.menu("dubin", None, NOON), Answer::NotFound);
    assert_eq!(none.venue(), "null");
    let mut broken = memo_with(Some(json!({"id": "loc_1", "tz": "Europe/Tirane"})), Ok(vec![]), Rails::default());
    assert!(matches!(broken.menu("dubin", None, NOON), Answer::Broken(e) if e.starts_with("catalogue location unreadable")));
    assert_eq!(serde_json::from_str::<Value>(broken.venue()).unwrap()["tz"], "Europe/Tirane", "a record that is not a LocRow is still the record");
    assert_eq!(serde_json::from_str::<Value>(memo().venue()).unwrap()["slug"], "dubin");
}

#[test]
fn the_rails_and_the_venues_material_reach_the_location() {
    let rails = Rails { stripe_key: Some("pk_test_1".into()), telegram_bot: Some("dubin_bot".into()) };
    let rec = venue(json!({"theme": {"paper": "#fff"}, "logo_url": "/media/l.png", "pickup": true,
        "payments": {"crypto": [{"network": "tron", "symbol": "USDT", "address": "T1"}, {"network": "x", "symbol": "", "address": "a"}]}}));
    let out = body(memo_with(Some(rec), Ok(vec![]), rails).menu("dubin", None, NOON));
    let loc = &out["location"];
    assert_eq!(out["stripePublishableKey"], "pk_test_1");
    assert_eq!(loc["payments"]["card"], true);
    assert_eq!(loc["payments"]["crypto"].as_array().unwrap().len(), 1);
    assert_eq!(loc["telegramBot"], "dubin_bot");
    assert_eq!(loc["theme"]["paper"], "#fff");
    assert_eq!(loc["logoUrl"], "/media/l.png");
    assert_eq!(loc["pickup"], true);
    assert_eq!(loc["features"]["tips"], json!(dowiz_hub::features::all(&Settings::create().unwrap()).iter().any(|(f, on)| f.key == "feature.tips" && *on)));
    let bare = body(memo().menu("dubin", None, NOON));
    assert_eq!((bare["location"]["payments"]["card"].clone(), bare["location"]["telegramBot"].clone()), (json!(false), Value::Null));
}

#[test]
fn a_locale_is_rendered_once_and_strangers_are_not_kept() {
    let mut m = memo();
    let first = m.menu("dubin", Some("en"), NOON);
    m.rendered.get_mut("en").unwrap().0 = "[\"kept\"]".into();
    assert_eq!(body(m.menu("dubin", Some("en"), NOON))["categories"], json!(["kept"]), "the second read is the memo");
    assert_ne!(first, m.menu("dubin", Some("en"), NOON));
    for n in 0..20 {
        let _ = m.menu("dubin", Some(&format!("x{n}")), NOON);
    }
    assert_eq!(m.rendered.len(), LOCALES_KEPT);
    let unkept = body(m.menu("dubin", Some("zz"), NOON));
    assert_eq!(unkept["categories"][0]["name"], "Supa", "past the cap it is still answered");
}

#[test]
fn products_answers_only_what_was_asked_as_stored() {
    let m = memo();
    let out: Value = serde_json::from_str(&m.products(&["p_b".into(), "gone".into(), "p_s".into()])).unwrap();
    assert_eq!(out["venue"]["slug"], "dubin");
    let got = out["products"].as_object().unwrap();
    assert_eq!(got.len(), 2);
    assert_eq!(got["p_b"]["cookingMin"], 12);
    let none: Value = serde_json::from_str(&memo_with(None, Ok(vec![]), Rails::default()).products(&["a\"b".into()])).unwrap();
    assert_eq!(none, json!({"venue": null, "products": {}}));
}

#[test]
fn a_customer_missing_a_name_reads_the_english_one_and_the_console_does_not() {
    let mut w = words();
    w.push(("ru/product/p_b/name".into(), "Маки".into()));
    let mut m = memo_with(Some(venue(json!({}))), Ok(w), Rails::default());
    let ru = body(m.menu("dubin", Some("ru"), NOON));
    assert_eq!(ru["categories"][1]["products"][1]["name"], "Маки", "the language asked for wins over the fallback");
    assert_eq!(ru["categories"][1]["name"], "Rolls EN", "no Russian name: the English one");
    assert_eq!(ru["categories"][1]["products"][1]["ingredients"], json!(["rice"]), "lists fall back too");
    assert_eq!(ru["categories"][0]["products"][0]["name"], "Miso", "no Russian, no English: the venue's own");
    let console = body(m.menu_as("dubin", Some("ru"), true, NOON));
    assert_eq!(console["categories"][1]["name"], "Rolls", "the console's fresh read never shows English in the Russian field");
    assert_eq!(console["categories"][1]["products"][1]["name"], "Маки");
    assert_eq!(body(m.menu("dubin", Some("ru"), NOON))["categories"][1]["name"], "Rolls EN", "the two renderings are kept apart");
    let uk = body(m.menu("dubin", Some("uk"), NOON));
    assert_eq!((uk["categories"][1]["products"][1]["name"].clone(), uk["categories"][1]["name"].clone()), (json!("Макі"), json!("Rolls EN")));
}

#[test]
fn the_memo_answers_the_catalogue_blocks_beside_the_json() {
    use dowiz_hub::block::{decode::decode, view::Catalogue, Col};
    let m = memo();
    let (prices, skipped) = m.block("menu_prices").unwrap().expect("menu_prices is a block");
    assert_eq!(skipped, 0);
    let block = decode(prices).expect("the block the route answers decodes");
    assert_eq!(block.n, 3, "the three products");
    let Col::I64(p) = &block.cols[1] else { panic!("price is i64") };
    let mut got = p.clone();
    got.sort_unstable();
    assert_eq!(got, vec![400, 700, 900], "the prices the JSON holds");
    let (bom, _) = m.block("bom").unwrap().unwrap();
    let (names, _) = m.block("names").unwrap().unwrap();
    let cat = Catalogue::new(prices, bom, names).expect("the three blocks read together");
    assert_eq!(cat.bom_of("p_b"), Some(vec![]), "a product with no recipe has an empty one");
    assert_eq!(m.block("json").unwrap(), None, "an unknown block is not an empty one");
}
