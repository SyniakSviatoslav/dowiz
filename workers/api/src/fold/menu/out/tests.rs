//! AX3's cutoff on the memo itself (`fold/menu/out.rs`): same output keeps the
//! generation, a changed output bumps it, and an unpublished predecessor is
//! never cut off (a failed publish is retried by the next write).

use crate::fold::menu::{Gens, Memo, Rails};
use dowiz_hub::catalog::Catalog;
use dowiz_hub::settings::Settings;

fn cat(price: i64) -> Vec<u8> {
    let mut c = Catalog::create().unwrap();
    c.set_location(&serde_json::json!({
        "id": "loc_1", "name": "Dubin", "slug": "dubin", "phone": "+355", "address": null, "status": "open",
        "closes_at": null, "delivery_eta": "30-40", "delivery_fee": 200, "free_delivery_threshold": null,
        "min_order": 1000, "currency_code": "ALL", "menu_version": 7, "supported_locales": "[\"sq\"]",
        "default_locale": "sq", "delivery_paused": 0, "hours": [],
    }).to_string());
    c.set_category("c", r#"{"name":"Rolls","sortOrder":1}"#);
    c.set_product("p", &format!(r#"{{"name":"Maki","categoryId":"c","price":{price}}}"#));
    c.to_bytes().unwrap()
}

fn settings(pairs: &[(&str, &str)]) -> Vec<u8> {
    let mut s = Settings::create().unwrap();
    for (k, v) in pairs {
        s.set(k, v);
    }
    s.to_bytes().unwrap()
}

fn memo(gen: i64, catalog: &[u8], s: &[u8]) -> Memo {
    let g = Gens { catalog: 1, i18n: 0, settings: gen };
    Memo::from_images(g, Some(catalog), None, Some(s), Rails::default()).unwrap()
}

#[test]
fn the_same_output_keeps_its_generation_and_is_cut_off() {
    let mut prev = memo(1, &cat(900), &settings(&[]));
    prev.mark_published(true);
    let mut next = memo(2, &cat(900), &settings(&[("notify.telegram.chat", "-100")]));
    assert_eq!(next.out_key(), prev.out_key());
    assert!(next.succeed(&prev), "a notification setting is not a menu byte");
    assert_eq!(next.out_generation(), prev.out_generation());
    assert!(next.published());
}

#[test]
fn a_changed_output_bumps_and_is_not_cut_off() {
    let mut prev = memo(1, &cat(900), &settings(&[]));
    prev.mark_published(true);
    let mut price = memo(1, &cat(950), &settings(&[]));
    assert_ne!(price.out_key(), prev.out_key());
    assert!(!price.succeed(&prev));
    assert_eq!(price.out_generation(), prev.out_generation() + 1);
    assert!(!price.published());
    assert!(prev.out_bytes().windows(4).any(|w| w == b"tips"), "the venue record parsed: its features are in the output");
    let mut feature = memo(2, &cat(900), &settings(&[("feature.tips", "0")]));
    assert!(!feature.succeed(&prev), "a storefront feature IS a menu byte");
}

/// The memory is load-bearing: a predecessor whose publish failed is not cut
/// off, even with the same bytes, so the next write retries the publish.
#[test]
fn an_unpublished_predecessor_is_never_cut_off() {
    let prev = memo(1, &cat(900), &settings(&[]));
    assert!(!prev.published());
    let mut next = memo(2, &cat(900), &settings(&[]));
    assert!(!next.succeed(&prev));
    assert_eq!(next.out_generation(), prev.out_generation(), "same bytes: same generation, publish still owed");
}
