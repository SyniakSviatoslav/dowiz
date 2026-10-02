//! The published shape (BN2): the fragment's bytes are pinned, the words carry
//! only what differs, the content address is what it says.

use super::{fragment_of, k64, words_of, CLOCK_FIELDS};
use crate::fold::menu::{Answer, Gens, Memo, Rails};
use dowiz_hub::catalog::Catalog;
use dowiz_hub::settings::Settings;
use serde_json::{json, Value};

/// 2026-07-01 12:00 in Tirane (10:00 UTC), and 03:00 the same night.
const NOON: i64 = 1_782_900_000_000;
const NIGHT: i64 = 1_782_867_600_000;

const GOLDEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/hubdo/menu/fragment.golden.json");

fn venue() -> Value {
    json!({
        "id": "loc_1", "name": "Dubin & Sushi", "slug": "dubin", "phone": "+355", "address": null,
        "status": "open", "closes_at": null, "delivery_eta": "30-40", "delivery_fee": 200,
        "free_delivery_threshold": null, "min_order": 1000, "currency_code": "ALL", "menu_version": 7,
        "supported_locales": "[\"sq\",\"en\"]", "default_locale": "sq", "delivery_paused": 0,
        "logo_url": "/media/logo.png", "tz": "Europe/Tirane",
        "hours": [[{"open": 660, "close": 1380}], [{"open": 660, "close": 1380}], [{"open": 660, "close": 1380}],
            [{"open": 660, "close": 1380}], [{"open": 660, "close": 1380}], [{"open": 660, "close": 1380}],
            [{"open": 660, "close": 1380}]],
    })
}

pub(crate) fn catalog(price_of_maki: i64) -> Catalog {
    let mut c = Catalog::create().unwrap();
    c.set_location(&venue().to_string());
    c.set_category("c_rolls", &json!({"name": "Rolls", "sortOrder": 2}).to_string());
    c.set_category("c_soup", &json!({"name": "Supa", "sortOrder": 1}).to_string());
    c.set_product("p_b", &json!({"name": "Maki", "categoryId": "c_rolls", "price": price_of_maki, "sortOrder": 2,
        "ingredients": ["oriz"], "allergens": [], "cookingMin": 12, "imageUrl": "/media/aaa.jpg", "imageUrlSmall": "/media/aaas.jpg"}).to_string());
    c.set_product("p_a", &json!({"name": "Nigiri", "categoryId": "c_rolls", "price": 700, "sortOrder": 1,
        "available": false, "imageUrl": "/media/missing.jpg"}).to_string());
    c.set_product("p_s", &json!({"name": "Miso", "categoryId": "c_soup", "price": 400}).to_string());
    c
}

pub(crate) fn words() -> Vec<(String, String)> {
    vec![
        ("en/product/p_b/name".into(), "Maki roll".into()),
        ("en/product/p_b/ingredients".into(), "[\"rice\"]".into()),
        ("en/product/p_a/ingredients".into(), "not a list".into()),
        ("en/category/c_rolls/name".into(), "Rolls EN".into()),
        ("en/product/p_s/price".into(), "1".into()),
    ]
}

fn memo() -> Memo {
    Memo::build(Gens { catalog: 3, i18n: 2, settings: 1 }, &catalog(900), Ok(words()), &Settings::create().unwrap(), Rails::default())
}

fn body(m: &mut Memo, locale: Option<&str>, now: i64) -> String {
    match m.menu_as("dubin", locale, false, now) {
        Answer::Body(b) => b,
        other => panic!("expected a menu, got {other:?}"),
    }
}

/// THE FRAGMENT IS THE CONTRACT the shell reads: its bytes for a fixed menu are
/// pinned to a golden, and the golden has no clock in it. A change to the
/// published shape must re-derive the golden AND `store/shell.js` in the same
/// commit (the written-format law).
#[test]
fn fragment_bytes_pinned() {
    let mut m = memo();
    let fragment = fragment_of(&body(&mut m, None, NOON)).expect("the own-language body renders");
    let golden = std::fs::read_to_string(GOLDEN).unwrap_or_else(|e| panic!("no golden at {GOLDEN}: {e}\nactual fragment:\n{fragment}"));
    assert_eq!(fragment, golden.trim_end(), "the fragment's bytes moved; re-derive the golden and store/shell.js together");
    assert_eq!(k64(fragment.as_bytes()), "2e227551754217ff", "the content address of the golden");
    let v: Value = serde_json::from_str(&fragment).unwrap();
    for f in CLOCK_FIELDS {
        assert!(v["location"].get(f).is_none(), "{f} is the clock's and is not in the fragment");
    }
    // The clock is out: the same menu at night is the same bytes.
    assert_eq!(fragment_of(&body(&mut m, None, NIGHT)).unwrap(), fragment, "an immutable object cannot depend on the hour");
    // What the shell needs to put the clock back is in it.
    assert_eq!(v["location"]["ownerStatus"], "open");
    assert_eq!(v["location"]["deliveryPaused"], false);
    assert_eq!(v["location"]["tz"], "Europe/Tirane");
    assert_eq!(v["location"]["hours"].as_array().map(Vec::len), Some(7));
}

#[test]
fn the_body_itself_still_carries_the_clock() {
    let mut m = memo();
    let v: Value = serde_json::from_str(&body(&mut m, None, NIGHT)).unwrap();
    assert_eq!(v["location"]["status"], "closed", "03:00 is outside 11:00-23:00");
    assert_eq!(v["location"]["closedReason"], "hours");
}

#[test]
fn fragment_of_refuses_a_body_without_a_location() {
    assert_eq!(fragment_of("{\"categories\":[]}").unwrap_err(), "menu body has no location");
    assert!(fragment_of("not json").unwrap_err().starts_with("menu body is not JSON"));
}

/// The words of a locale are the DIFFERENCE from the fragment: a translated
/// name is in, an untranslated one is not, a price never is.
#[test]
fn words_carry_only_what_differs() {
    let mut m = memo();
    let fragment = fragment_of(&body(&mut m, None, NOON)).unwrap();
    let en = body(&mut m, Some("en"), NOON);
    let w: Value = serde_json::from_str(&words_of(&fragment, &en).unwrap()).unwrap();
    assert_eq!(w["words"]["p_b"]["name"], "Maki roll");
    assert_eq!(w["words"]["p_b"]["ingredients"], json!(["rice"]));
    assert!(w["words"]["p_b"].get("description").is_none(), "an untranslated field is not a word");
    assert!(w["words"]["p_b"].get("price").is_none(), "a price is never a word");
    assert_eq!(w["words"]["c_rolls"]["name"], "Rolls EN");
    assert!(w["words"].get("p_a").is_none(), "`not a list` fell back to the venue's own: nothing differs");
    assert!(w["words"].get("p_s").is_none(), "a `price` translation key is not a word (`menu_venue::render` ignores it)");
    assert_eq!(w["warnings"], json!([]));
    // The venue's own language against itself: no words at all.
    let own: Value = serde_json::from_str(&words_of(&fragment, &body(&mut m, Some("sq"), NOON)).unwrap()).unwrap();
    assert_eq!(own["words"], json!({}));
}

#[test]
fn words_carry_the_locale_warnings() {
    let mut broken = Memo::build(Gens::default(), &catalog(900), Err("image i18n is unreadable".into()), &Settings::create().unwrap(), Rails::default());
    let fragment = fragment_of(&body(&mut broken, None, NOON)).unwrap();
    let w: Value = serde_json::from_str(&words_of(&fragment, &body(&mut broken, Some("en"), NOON)).unwrap()).unwrap();
    assert_eq!(w["warnings"], json!(["translations unavailable: image i18n is unreadable"]));
}

#[test]
fn k64_is_the_first_sixteen_hex_of_sha256() {
    assert_eq!(k64(b""), "e3b0c44298fc1c14");
    assert_eq!(k64(b"abc"), "ba7816bf8f01cfea");
    assert_ne!(k64(b"price 900"), k64(b"price 950"));
}
