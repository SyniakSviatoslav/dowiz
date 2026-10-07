//! `CatalogView` against `Catalog::load` on every catalogue shape in the tree (W-ZC).
use super::*;
use crate::catalog::tests::{dubin, live_dish, live_supply, recipe_keys};

/// The live 165 dishes with recipes, supplies, categories, promos and a non-ASCII id.
fn dubin_full() -> Catalog {
    let mut c = dubin();
    for i in 1..=73 {
        c.set_product(&format!("item-{i:03}"), &live_dish(i, Some(recipe_keys(i))));
    }
    for i in 0..72 {
        c.set_supply(&format!("supply-{i:02}x"), &live_supply(i));
    }
    for k in ["futomaki", "nigiri", "drinks"] {
        c.set_category(k, &format!(r#"{{"id":"{k}"}}"#));
    }
    c.set_promo("VERE10", r#"{"code":"VERE10","pct":10}"#);
    c.set_product("ujë", r#"{"id":"ujë","price":100}"#);
    c
}

/// 165 dishes of ~2.5 KB each: the report's run2 shape (F.3).
pub(crate) fn big_165() -> Catalog {
    let mut c = Catalog::create().unwrap();
    c.set_location(r#"{"id":"v1","currency_code":"ALL"}"#);
    for i in 0..165 {
        let pad = "x".repeat(2500 - 120);
        c.set_product(&format!("item-{i:03}"), &format!(r#"{{"id":"item-{i:03}","name":"Dish {i}","price":{},"description":"{pad}"}}"#, 500 + i));
    }
    c
}

/// Every image the walk covers.
fn images() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("empty Catalog::create", Catalog::create().unwrap().to_bytes().unwrap()),
        ("dubin 165 dishes", dubin().to_bytes().unwrap()),
        ("dubin + recipes/supplies/promos", dubin_full().to_bytes().unwrap()),
        ("165 x 2.5 KB", big_165().to_bytes().unwrap()),
        ("kv.store v1 fixture", include_bytes!("../../../../bebop-wasm/fixtures/kv.store").to_vec()),
        ("kv2.store v2 fixture", include_bytes!("../../../../bebop-wasm/fixtures/kv2.store").to_vec()),
        ("165 x 2.5 KB + 3 delta writes (v3)", with_deltas()),
    ]
}

/// W-DELTA: a v3 image -- an edit, a new dish (sorts last), a removal (the first dish).
fn with_deltas() -> Vec<u8> {
    let mut b = big_165().to_bytes().unwrap();
    for step in 0..3 {
        let mut c = Catalog::load(&b).unwrap();
        match step {
            0 => c.set_product("item-042", r#"{"id":"item-042","price":600}"#),
            1 => c.set_product("item-999", r#"{"id":"item-999","price":1}"#),
            _ => {
                c.remove_product("item-000");
            }
        }
        b = c.to_bytes().unwrap();
    }
    b
}

/// THE EQUALITY WALK: every read `Catalog` offers, on every image, byte for byte --
/// every product id (first and last included), ids that are not there, every list
/// (a reader that drops the last entry fails the list equality), and the fold.
#[test]
fn the_view_answers_what_catalog_load_answers() {
    for (name, bytes) in images() {
        let cat = Catalog::load(&bytes).unwrap_or_else(|e| panic!("{name}: load {e:?}"));
        let view = CatalogView::open(&bytes).unwrap_or_else(|e| panic!("{name}: view {e:?}"));
        let again = CatalogView::reopen(&bytes, view.checked()).unwrap();
        for v in [&view as &dyn CatalogRead, &again] {
            assert_eq!(v.location(), cat.location(), "{name}: location");
            assert_eq!(v.products(), cat.products(), "{name}: products");
            assert_eq!(v.supplies(), cat.supplies(), "{name}: supplies");
            assert_eq!(v.promos(), cat.promos(), "{name}: promos");
            assert_eq!(v.categories(), cat.categories(), "{name}: categories");
            assert_eq!(v.root(), cat.root(), "{name}: root");
            let mut ids: Vec<String> = cat.products().into_iter().map(|(id, _)| id).collect();
            ids.extend(["", "item-000", "item-999", "item-16", "zzz", "item-1655"].map(String::from));
            for id in &ids {
                assert_eq!(v.product(id), cat.product(id), "{name}: product {id:?}");
                assert_eq!(v.supply(id), cat.supply(id), "{name}: supply {id:?}");
            }
            for (s, _) in cat.supplies() {
                assert_eq!(v.supply(&s), cat.supply(&s), "{name}: supply {s}");
            }
            assert_eq!(v.promo("VERE10"), cat.promo("VERE10"), "{name}: promo");
            assert_eq!(v.promo("NONE"), cat.promo("NONE"), "{name}: no promo");
        }
    }
}

/// THE SAME REFUSALS: a non-image is `NotAHub` for both, a changed byte in the value
/// blob is `BadCrc` naming the same object for both.
#[test]
fn the_view_refuses_what_catalog_load_refuses() {
    assert!(matches!(CatalogView::open(b"not an image"), Err(HubError::NotAHub)));
    assert!(matches!(Catalog::load(b"not an image"), Err(HubError::NotAHub)));
    let mut bytes = dubin().to_bytes().unwrap();
    let st = bebop_store::Store::from_bytes(&bytes);
    let vblob = st.follow(st.root().unwrap(), 4).unwrap();
    bytes[(vblob + 2 + 100) * 8 + 3] ^= 0x20;
    match (Catalog::load(&bytes), CatalogView::open(&bytes)) {
        (Err(HubError::BadCrc(a)), Err(HubError::BadCrc(b))) => assert_eq!((a.obj, b.obj), (vblob, vblob)),
        (a, b) => panic!("load {:?} / view {:?}", a.err(), b.err()),
    }
}
