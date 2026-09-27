//! A 165-dish venue, the size of the live catalogue (sushi-durres, 165 dishes),
//! with every field the storefront passes through and two translated
//! languages; and R2's before/after measurement over it.

use super::*;
use dowiz_hub::catalog::Catalog;
use dowiz_hub::settings::Settings;
use dowiz_hub::table::Table;
use serde_json::json;

pub(super) const DISHES: usize = 165;
const CATEGORIES: usize = 12;

/// The venue record: hours, theme, map pin, wallets -- everything `base` reads.
pub(super) fn record(extra: Value) -> Value {
    let day = json!([{"open": 660, "close": 1380}]);
    let mut v = json!({
        "id": "loc_sushi", "name": "Sushi Durrës", "slug": "sushi-durres", "phone": "+355 69 000 0000",
        "address": "Rruga Taulantia, Durrës", "status": "open", "closes_at": null, "delivery_eta": "35-45",
        "delivery_fee": 200, "free_delivery_threshold": 3000, "min_order": 1000, "currency_code": "ALL",
        "menu_version": 41, "supported_locales": "[\"sq\",\"en\",\"uk\"]", "default_locale": "sq",
        "delivery_paused": 0, "tz": "Europe/Tirane", "lat": 41.3131, "lng": 19.4453, "pickup": true,
        "hours": [day, day, day, day, day, day, day], "logo_url": "/media/sushi/logo.png",
        "theme": {"paper": "#f6f1e7", "ink": "#1d1a16", "accent": "#b3261e"},
        "google": {"rating": 4.6, "reviews": 812, "placeId": "fixture-place"},
        "payments": {"crypto": [{"network": "tron", "symbol": "USDT", "address": "TQ1234567890"}]},
        "delivery_zones": [{"name": "Qendra", "fee": 200, "polygon": [[41.31, 19.44], [41.32, 19.45], [41.30, 19.46]]}],
    });
    for (k, x) in extra.as_object().unwrap() {
        v[k] = x.clone();
    }
    v
}

fn product(n: usize) -> Value {
    json!({
        "name": format!("Roll {n} me salmon dhe avokado"),
        "description": format!("Oriz sushi, salmon i freskët, avokado, krem djathi, susam i pjekur dhe salcë e shtëpisë. Pjata {n}, e përgatitur me dorë çdo ditë."),
        "categoryId": format!("c_{}", n % CATEGORIES), "price": 700 + 10 * n as i64, "sortOrder": n as i64,
        "available": n % 17 != 0, "unavailableNote": if n % 17 == 0 { json!("Mbaroi sot") } else { Value::Null },
        "imageUrl": format!("/media/sushi/p{n}.jpg"), "imageUrlSmall": format!("/media/sushi/p{n}-s.jpg"),
        "allergens": if n % 5 == 0 { Value::Null } else { json!(["fish", "sesame", "milk"]) },
        "modifierGroups": [{"id": "g_size", "name": "Madhësia", "min": 1, "max": 1,
            "options": [{"id": "o_8", "name": "8 copë", "price": 0}, {"id": "o_12", "name": "12 copë", "price": 350}]}],
        "sizeCm": 22, "cookingMin": 8 + (n % 9) as i64, "tags": ["salmon", "cold"],
        "ingredients": ["oriz", "salmon", "avokado", "krem djathi", "susam"], "weightG": 240 + (n % 60) as i64,
        "nutrition": {"kcal": 420, "protein_g": 18, "fat_g": 14, "carbs_g": 52},
        "nutritionDerived": {"kcal": 415}, "taste": {"salty": 2, "sweet": 1, "spicy": 0}, "station": "sushi",
        "calories": 420,
    })
}

/// The three images' bytes, as the object holds them.
pub(super) fn images(rec: &Value) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let mut c = Catalog::create().unwrap();
    c.set_location(&rec.to_string());
    for k in 0..CATEGORIES {
        c.set_category(&format!("c_{k}"), &json!({"name": format!("Kategoria {k}"), "sortOrder": k}).to_string());
    }
    let mut t = Table::create(crate::hubstore::I18N_BYTES).unwrap();
    let kind = crate::hubstore::I18N_KIND;
    for n in 0..DISHES {
        let id = format!("p_{n}");
        c.set_product(&id, &product(n).to_string());
        // Every language but the venue's own (sq), as a live venue translates.
        for l in dowiz_hub::lang::LANGS.iter().copied().filter(|l| *l != "sq") {
            let key = |f: &str| crate::hubstore::i18n_key(l, "product", &id, f);
            t.put(kind, &key("name"), &format!("Roll {n} with salmon and avocado ({l})"), &[], &[]).unwrap();
            t.put(kind, &key("description"), &format!("Sushi rice, fresh salmon, avocado, cream cheese ({l} {n})"), &[], &[]).unwrap();
            t.put(kind, &key("ingredients"), "[\"rice\",\"salmon\",\"avocado\"]", &[], &[]).unwrap();
        }
    }
    for k in 0..CATEGORIES {
        for l in dowiz_hub::lang::LANGS.iter().copied().filter(|l| *l != "sq") {
            t.put(kind, &crate::hubstore::i18n_key(l, "category", &format!("c_{k}"), "name"), &format!("Category {k} ({l})"), &[], &[]).unwrap();
        }
    }
    let mut s = Settings::create().unwrap();
    (c.to_bytes().unwrap(), t.to_bytes().unwrap(), s.to_bytes().unwrap())
}

/// MEASURED, not asserted (`cargo test --release --lib measure_menu -- --ignored
/// --nocapture`). BEFORE: what the Worker did per menu request -- read the
/// catalogue (and settings, and on another language the translations) and
/// render. AFTER: what the object does per request on a hit (the clock part
/// and one concatenation) and per catalogue write (the fold); the Worker now
/// copies the body.
#[test]
#[ignore]
fn measure_menu_before_and_after() {
    let (c, t, s) = images(&record(json!({})));
    let g = Gens { catalog: 1, i18n: 1, settings: 1 };
    let noon = 1_782_900_000_000;
    let us = |t: std::time::Instant, n: u128| t.elapsed().as_nanos() / n / 1000;
    for locale in [None, Some("en")] {
        const N: u128 = 50;
        let t0 = std::time::Instant::now();
        let mut bytes = 0;
        for _ in 0..N {
            // The old path's reads: the i18n image only on another language.
            let words = locale.map(|_| t.as_slice());
            let mut m = Memo::from_images(g, Some(&c), words, Some(&s), Rails::default()).unwrap();
            let Answer::Body(b) = m.menu("sushi-durres", locale, noon) else { panic!() };
            bytes = b.len();
        }
        let before = us(t0, N);
        let t0 = std::time::Instant::now();
        let mut m = Memo::from_images(g, Some(&c), Some(&t), Some(&s), Rails::default()).unwrap();
        let _ = m.menu("sushi-durres", locale, noon);
        let first = us(t0, 1);
        const H: u128 = 2000;
        let t0 = std::time::Instant::now();
        for _ in 0..H {
            std::hint::black_box(m.menu("sushi-durres", locale, noon));
        }
        let hit = t0.elapsed().as_nanos() / H;
        let t0 = std::time::Instant::now();
        for _ in 0..H {
            std::hint::black_box(m.products(&["p_3".into(), "p_77".into(), "p_140".into()]));
        }
        let products = t0.elapsed().as_nanos() / H;
        println!(
            "dishes={DISHES} locale={locale:?} catalog_image={}B i18n_image={}B settings_image={}B menu_body={bytes}B | \
             BEFORE worker per request={before}us | AFTER object hit={hit}ns first render after a write={first}us \
             products(3 ids)={products}ns body={}B",
            c.len(), t.len(), s.len(), m.products(&["p_3".into(), "p_77".into(), "p_140".into()]).len()
        );
    }
}
