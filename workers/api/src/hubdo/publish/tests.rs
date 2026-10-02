//! BN2: the object publishes what changed, and only that, through the real
//! `HubImages` over memory (`hubdo/host/mem.rs`) into a recording bucket.

use super::mem::{bucket, no_bucket, MemBucket};
use super::{manifest_of, media_names, plan, Object, Published, ROOT_CACHE};
use crate::hubdo::host::mem::Harness;
use crate::hubdo::menu::tests::{catalog, words};
use serde_json::Value;
use std::rc::Rc;

fn i18n_image() -> Vec<u8> {
    let mut t = dowiz_hub::table::Table::create(crate::hubstore::I18N_BYTES).unwrap();
    for (k, v) in words() {
        t.put(crate::hubstore::I18N_KIND, &k, &v, &[], &[]).unwrap();
    }
    t.to_bytes().unwrap()
}

/// A venue whose catalogue, translations and two of three photos exist.
fn published_venue() -> (Harness, Rc<MemBucket>) {
    let b = bucket();
    b.photos.borrow_mut().insert("aaa.jpg".into(), (vec![0xff, 0xd8, 1, 2, 3], "image/jpeg".into()));
    b.photos.borrow_mut().insert("aaas.jpg".into(), (vec![0xff, 0xd8, 9], "image/jpeg".into()));
    b.photos.borrow_mut().insert("logo.png".into(), (vec![0x89, b'P', b'N', b'G'], "image/png".into()));
    let h = Harness::new();
    // No catalogue yet: a translation write has nothing to publish.
    assert_eq!(h.put("i18n", 0, &i18n_image()).status_code(), 200);
    assert_eq!(b.drain(), Vec::<String>::new(), "no venue record, no objects");
    assert_eq!(h.put("catalog", 0, &catalog(900).to_bytes().unwrap()).status_code(), 200);
    (h, b)
}

fn manifest(b: &MemBucket) -> Value {
    let puts = b.puts.borrow();
    let m = puts.iter().rev().find(|p| p.key.ends_with("/manifest.json")).expect("a manifest was written");
    serde_json::from_slice(&m.bytes).unwrap()
}

/// THE ACCEPTANCE NUMBER. A one-dish price edit writes the fragment, the
/// prices block and the manifest -- three objects -- and never a photo, a
/// names block or another language's words.
#[test]
fn only_changed_blocks_are_written() {
    let (h, b) = published_venue();
    let before = manifest(&b);
    let first = b.drain();
    assert!(first.iter().any(|k| k == "v/dubin/manifest.json"), "the first publish writes the root: {first:?}");

    assert_eq!(h.put("catalog", 1, &catalog(950).to_bytes().unwrap()).status_code(), 200);
    let after = manifest(&b);
    let edit = b.drain();
    assert_eq!(edit.len(), 3, "a price edit is three writes, not {}: {edit:?}", edit.len());
    assert!(edit.iter().filter(|k| k.ends_with(".dwb")).count() == 1, "the prices block moved: {edit:?}");
    assert!(edit.iter().filter(|k| k.ends_with(".json") && !k.ends_with("manifest.json")).count() == 1, "the fragment moved: {edit:?}");
    assert_eq!(edit.last().map(String::as_str), Some("v/dubin/manifest.json"), "the root is written last");
    assert!(!edit.iter().any(|k| k.contains("/m/")), "a price edit never rewrites a photograph: {edit:?}");

    assert_ne!(after["fragment"], before["fragment"], "the fragment's address moved with the price");
    assert_ne!(after["blocks"]["menu_prices"], before["blocks"]["menu_prices"]);
    assert_eq!(after["blocks"]["names"], before["blocks"]["names"], "no name changed: the names block is the same object");
    assert_eq!(after["words"]["en"], before["words"]["en"], "no word changed: the English words are the same object");
    assert_eq!(after["media"], before["media"], "the photo list is the same object");
    assert_eq!(after["gens"]["catalog"], 2);
    assert!(edit.contains(&format!("v/dubin/{}", after["fragment"].as_str().unwrap())));
    assert!(edit.contains(&format!("v/dubin/{}", after["blocks"]["menu_prices"].as_str().unwrap())));

    // The same bytes again (a write that changed nothing visible): no object,
    // only the root, because the generation it names moved.
    assert_eq!(h.put("catalog", 2, &catalog(950).to_bytes().unwrap()).status_code(), 200);
    assert_eq!(b.drain(), vec!["v/dubin/manifest.json".to_string()]);
}

/// The first publish writes every object the manifest names, the photos KV
/// has, and the manifest LAST; the one KV does not have is left to the Worker.
#[test]
fn the_first_publish_writes_the_set_and_the_root_last() {
    let (_h, b) = published_venue();
    let puts = b.puts.borrow();
    let keys: Vec<&str> = puts.iter().map(|p| p.key.as_str()).collect();
    assert_eq!(keys.last().copied(), Some("v/dubin/manifest.json"));
    let m: Value = serde_json::from_slice(&puts.last().unwrap().bytes).unwrap();
    assert_eq!(m["v"], 1);
    assert_eq!(m["slug"], "dubin");
    assert_eq!(m["default"], "sq");
    assert_eq!(m["locales"], serde_json::json!(["sq", "en"]));
    assert_eq!(m["gens"], serde_json::json!({"catalog": 1, "i18n": 1, "settings": 0}));
    for named in [&m["fragment"], &m["words"]["en"], &m["blocks"]["menu_prices"], &m["blocks"]["names"], &m["media"]] {
        let key = format!("v/dubin/{}", named.as_str().expect("every name in the manifest is a key"));
        assert!(keys.contains(&key.as_str()), "{key} named but not written: {keys:?}");
    }
    // Photos: the two KV holds, under their own names; the missing one is not listed.
    assert!(keys.contains(&"v/dubin/m/aaa.jpg"));
    assert!(keys.contains(&"v/dubin/m/aaas.jpg"));
    assert!(keys.contains(&"v/dubin/m/logo.png"));
    assert!(!keys.iter().any(|k| k.contains("missing")), "a photo KV does not have is not published: {keys:?}");
    let list = puts.iter().find(|p| p.key == format!("v/dubin/{}", m["media"].as_str().unwrap())).unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&list.bytes).unwrap(), serde_json::json!(["aaa.jpg", "aaas.jpg", "logo.png"]));
    // Headers: immutable everywhere but the root; types by object.
    for p in puts.iter() {
        if p.key.ends_with("/manifest.json") {
            assert_eq!(p.cache_control, ROOT_CACHE);
            assert_eq!(p.content_type, "application/json");
        } else {
            assert_eq!(p.cache_control, "public, max-age=31536000, immutable", "{}", p.key);
        }
    }
    let photo = puts.iter().find(|p| p.key == "v/dubin/m/aaa.jpg").unwrap();
    assert_eq!(photo.content_type, "image/jpeg");
    assert_eq!(puts.iter().find(|p| p.key.ends_with(".dwb")).unwrap().content_type, "application/vnd.dowiz.block");
    // The fragment really is the storefront's body without the clock.
    let frag = puts.iter().find(|p| p.key == format!("v/dubin/{}", m["fragment"].as_str().unwrap())).unwrap();
    let f: Value = serde_json::from_slice(&frag.bytes).unwrap();
    assert_eq!(f["location"]["slug"], "dubin");
    assert!(f["location"].get("status").is_none());
    assert_eq!(f["categories"][1]["products"][1]["price"], 900);
}

#[test]
fn a_translation_edit_writes_the_words_the_names_block_and_the_root() {
    let (h, b) = published_venue();
    b.drain();
    let mut t = dowiz_hub::table::Table::create(crate::hubstore::I18N_BYTES).unwrap();
    for (k, v) in words() {
        t.put(crate::hubstore::I18N_KIND, &k, &v, &[], &[]).unwrap();
    }
    t.put(crate::hubstore::I18N_KIND, "en/product/p_s/name", "Miso soup", &[], &[]).unwrap();
    assert_eq!(h.put("i18n", 1, &t.to_bytes().unwrap()).status_code(), 200);
    let puts: Vec<super::mem::Put> = b.puts.borrow_mut().drain(..).collect();
    let edit: Vec<&str> = puts.iter().map(|p| p.key.as_str()).collect();
    // The names block is folded from the CATALOGUE, so it does not move; the words do.
    assert_eq!(edit.len(), 2, "words + root: {edit:?}");
    assert!(edit[0].ends_with(".json") && !edit[0].ends_with("manifest.json"));
    assert_eq!(edit[1], "v/dubin/manifest.json");
    let w: Value = serde_json::from_slice(&puts[0].bytes).unwrap();
    assert_eq!(w["words"]["p_s"]["name"], "Miso soup");
}

/// Until the operator step, there is no binding: nothing is written and the
/// write that would have published succeeds anyway.
#[test]
fn without_a_binding_nothing_is_published_and_the_write_still_lands() {
    no_bucket();
    let h = Harness::new();
    assert_eq!(h.put("catalog", 0, &catalog(900).to_bytes().unwrap()).status_code(), 200);
    let status = h.get("/fold/publish");
    assert_eq!(status.status_code(), 200);
    let v = status.body_value();
    assert_eq!(v["enabled"], false);
    assert_eq!(v["manifest"], "v/dubin/manifest.json");
    assert_eq!(v["published"]["objects"], serde_json::json!({}));
    let now = h.post("/fold/publish", &serde_json::json!({}));
    assert_eq!(now.status_code(), 503, "publish now, with nothing to publish to, says so");
}

#[test]
fn the_route_reports_the_record_and_republishes_on_demand() {
    let (h, b) = published_venue();
    b.drain();
    let v = h.get("/fold/publish").body_value();
    assert_eq!(v["enabled"], true);
    assert_eq!(v["published"]["generation"], serde_json::json!([1, 1, 0]));
    assert_eq!(v["published"]["objects"].as_object().unwrap().len(), 5, "fragment, words/en, two blocks, media");
    assert_eq!(v["published"]["media"], serde_json::json!(["aaa.jpg", "aaas.jpg", "logo.png"]));
    // Nothing moved: publish now writes nothing.
    assert_eq!(h.post("/fold/publish", &serde_json::json!({})).body_value()["written"], 0);
    assert_eq!(b.drain(), Vec::<String>::new());
    // `all` rewrites every object and the root (photos included: the record is ignored).
    let all = h.call(crate::wire::Call::new("https://hub/fold/publish?all=1", worker::Method::Post).unwrap());
    assert_eq!(all.body_value()["written"], 5 + 3 + 1);
    assert_eq!(b.drain().len(), 9);
}

/// A cold object publishes against the record on disk, not against nothing.
#[test]
fn a_cold_object_reads_the_record_and_writes_only_what_moved() {
    let (h, b) = published_venue();
    b.drain();
    let cold = h.cold();
    assert_eq!(cold.put("catalog", 1, &catalog(1000).to_bytes().unwrap()).status_code(), 200);
    let edit = b.drain();
    assert_eq!(edit.len(), 3, "{edit:?}");
    assert!(!edit.iter().any(|k| k.contains("/m/")));
}

#[test]
fn plan_names_what_the_record_does_not_have() {
    let prev = Published { objects: [("fragment".to_string(), "aa.json".to_string())].into_iter().collect(), ..Default::default() };
    let next = vec![
        Object { name: "fragment".into(), key: "aa.json".into(), bytes: vec![], content_type: "application/json" },
        Object { name: "block/names".into(), key: "bb.dwb".into(), bytes: vec![], content_type: "x" },
    ];
    let todo = plan(&prev, &next);
    assert_eq!(todo.len(), 1);
    assert_eq!(todo[0].name, "block/names");
    let moved = vec![Object { name: "fragment".into(), key: "cc.json".into(), bytes: vec![], content_type: "application/json" }];
    assert_eq!(plan(&prev, &moved).len(), 1, "a different key is a write");
    assert_eq!(plan(&Published::default(), &next).len(), 2, "no record: everything");
}

#[test]
fn media_names_are_the_hashes_the_menu_shows_once_each() {
    let fragment = serde_json::json!({
        "location": {"logoUrl": "/media/logo.png"},
        "categories": [{"products": [
            {"imageUrl": "/media/a.jpg", "imageUrlSmall": "/media/as.jpg"},
            {"imageUrl": "/media/a.jpg"},
            {"imageUrl": "https://elsewhere.example/x.jpg"},
            {"imageUrl": "/media/../etc/passwd"},
            {"imageUrl": null}
        ]}]
    })
    .to_string();
    let names = media_names(&fragment, &serde_json::json!({"logo_url": "/media/logo.png"}));
    assert_eq!(names, vec!["a.jpg", "as.jpg", "logo.png"]);
    assert_eq!(media_names("junk", &serde_json::json!({})), Vec::<String>::new());
}

#[test]
fn the_manifest_names_every_object_by_kind() {
    let set = vec![
        Object { name: "fragment".into(), key: "f.json".into(), bytes: vec![], content_type: "" },
        Object { name: "words/en".into(), key: "w.json".into(), bytes: vec![], content_type: "" },
        Object { name: "block/names".into(), key: "n.dwb".into(), bytes: vec![], content_type: "" },
        Object { name: "media".into(), key: "m.json".into(), bytes: vec![], content_type: "" },
    ];
    let m: Value = serde_json::from_str(&manifest_of("dubin", [3, 2, 1], "sq", &["sq".into(), "en".into()], &set)).unwrap();
    assert_eq!(m["fragment"], "f.json");
    assert_eq!(m["words"], serde_json::json!({"en": "w.json"}));
    assert_eq!(m["blocks"], serde_json::json!({"names": "n.dwb"}));
    assert_eq!(m["media"], "m.json");
    assert!(m.get("at").is_none(), "no clock in the root: the generations are its version");
    assert_eq!(m["gens"]["settings"], 1);
}
