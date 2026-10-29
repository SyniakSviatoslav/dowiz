//! THE RESTORE DRILL over a copy produced by the REAL nightly path, in process (W-PITR row 1):
//! a venue with orders and menu edits, a bucket, `site.night` -> `cloud::push_place` -> the PUT
//! bodies the bucket received. The drill passes on the copy as it was uploaded and its log tip
//! IS the witness's; one changed cell (catalogue value, with its sha256 fixed so only the crc
//! can see it; an order record) is REFUSED; and the copy restores into an empty venue.

use crate::edge::mem::{answer_outbound, block_on, sent};
use crate::edge::site::{post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::{open_venue, place_pickup};
use crate::wire::Reply;
use base64::Engine;
use dowiz_hub::drill::{self, Tip};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// `(bundle bytes, witness bytes)` exactly as the bucket received them.
fn nightly_copy() -> (Site, Vec<u8>, Vec<u8>) {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    for _ in 0..3 {
        assert_eq!(place_pickup(&site, "alpha", &dish, 1).status_code(), 200);
    }
    let edit = json!({"location_id": "alpha", "price": 1100});
    let r = site.run(crate::owner::update_product, post(&format!("https://alpha.{PLATFORM_HOST}/api/owner/products/{dish}"), &edit).bearer(&t).on("alpha"), &[("id", &dish)]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    for (k, v) in [("cloud.s3.endpoint", "https://s3.example"), ("cloud.s3.bucket", "alpha-copies"), ("cloud.s3.key", "AKIA"), ("cloud.s3.secret", "shh")] {
        let r = site.run(crate::services::venue::settings::set_setting, post(&format!("https://alpha.{PLATFORM_HOST}/api/owner/settings"), &json!({"key": k, "value": v})).bearer(&t).on("alpha"), &[]);
        assert_eq!(r.status_code(), 200, "{k}: {}", r.body_str());
    }
    answer_outbound(|_| {
        let mut r = Reply::empty().unwrap();
        r.headers_mut().set("etag", "\"e1\"").unwrap();
        Ok(r)
    });
    site.night(site.now_ms + 3_600_000);
    let puts: Vec<(String, Vec<u8>)> = sent().iter().filter(|c| c.method() == worker::Method::Put)
        .map(|c| (c.url().unwrap().to_string(), c.body_bytes())).collect();
    let bundle = puts.iter().find(|(u, _)| u.ends_with(".json") && !u.ends_with(".witness.json")).map(|(_, b)| b.clone());
    let witness = puts.iter().find(|(u, _)| u.ends_with(".witness.json")).map(|(_, b)| b.clone());
    let (Some(bundle), Some(witness)) = (bundle, witness) else { panic!("the night uploaded no bundle and witness: {:?}", puts.iter().map(|p| &p.0).collect::<Vec<_>>()) };
    // `PITR_FIXTURE_OUT=<dir>` keeps the copy, for the CLI (`restore_drill`) to be run on it.
    if let Ok(dir) = std::env::var("PITR_FIXTURE_OUT") {
        std::fs::write(format!("{dir}/alpha.json"), &bundle).unwrap();
        std::fs::write(format!("{dir}/alpha.witness.json"), &witness).unwrap();
    }
    (site, bundle, witness)
}

/// The copy with one image's bytes changed by `f`, and its sha256 and length fixed to match.
fn with_image(bundle: &[u8], id: &str, f: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
    let mut v: Value = serde_json::from_slice(bundle).unwrap();
    let e = &mut v["images"][id];
    // The copy is PACKED (W-SPACE): the cell is changed in the raw image, then packed again.
    let mut bytes = dowiz_hub::pack::unpack(&drill::b64(e["image"].as_str().unwrap()).unwrap()).unwrap().into_owned();
    f(&mut bytes);
    e["sha256"] = json!(Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect::<String>());
    e["bytes"] = json!(bytes.len());
    e["image"] = json!(base64::engine::general_purpose::STANDARD.encode(dowiz_hub::pack::pack(&bytes)));
    serde_json::to_vec(&v).unwrap()
}

fn flip_in(bytes: &mut [u8], needle: &[u8], at: usize) {
    let i = bytes.windows(needle.len()).position(|w| w == needle).unwrap_or_else(|| panic!("{:?} is not in the image", String::from_utf8_lossy(needle)));
    bytes[i + at] ^= 0x01;
}

#[test]
fn the_nightly_copy_passes_the_drill_and_its_tip_is_the_published_tip() {
    let (site, bundle, witness) = nightly_copy();
    let d = drill::run(&bundle, Some(&witness));
    assert!(d.passed(), "{:?}", d.refused);
    assert_eq!(d.tip, Tip::Equal, "log tip {:?} vs witness {:?}", d.log_tip, d.witness_tip);
    let ids: Vec<&str> = d.loaded.iter().map(|l| l.id.as_str()).collect();
    for want in ["log", "catalog", "settings", "catalog.edits"] {
        assert!(ids.contains(&want), "{want} not in the copy: {ids:?}");
    }
    assert!(d.chain.is_some_and(|c| c.records >= 3 && c.intact()), "{:?}", d.chain);
    assert!(d.notes.iter().any(|n| n.contains("replays to exactly this catalogue")), "{:?}", d.notes);

    // THE RESTORE DIRECTION (`hubstore::import`, `POST /api/owner/restore`): the same bytes
    // into a venue that has nothing, and the menu and its history come back whole.
    let v: Value = serde_json::from_slice(&bundle).unwrap();
    let gamma = crate::hubstore::Place::of_authorised(&site.ctx(&[]), "gamma").unwrap();
    let written = block_on(crate::hubstore::import(&gamma, &v)).unwrap();
    assert!(written.iter().any(|w| w == "catalog.edits"), "{written:?}");
    let alpha = crate::hubstore::Place::of_authorised(&site.ctx(&[]), "alpha").unwrap();
    let root = |p| block_on(crate::hubstore::load_catalog(p)).unwrap().catalog.root();
    assert_eq!(root(&gamma), root(&alpha));
    let log = block_on(crate::hubstore::load_log(&gamma, crate::catalog_history::IMAGE)).unwrap().log;
    let c = block_on(crate::hubstore::load_catalog(&gamma)).unwrap().catalog;
    assert_eq!(dowiz_hub::catalog::edits::replay(&log, dowiz_hub::catalog::edits::Cut::All).unwrap(), dowiz_hub::catalog::edits::state_of(&c));
}

/// RED-able: ONE named cell, the first digit of the dish's `"price":1100` inside the catalogue
/// image, with the manifest's sha256 recomputed -- so only the loader's crc stands between this
/// copy and a restore that serves a different price.
#[test]
fn a_copy_with_one_changed_catalogue_cell_is_refused_even_with_its_digest_fixed() {
    let (_, bundle, witness) = nightly_copy();
    let bad = with_image(&bundle, "catalog", |b| flip_in(b, b"\"price\":1100", 8));
    let d = drill::run(&bad, Some(&witness));
    assert!(d.refused.iter().any(|r| r.starts_with("catalog: refused by its loader: BadCrc")), "{:?}", d.refused);
}

#[test]
fn a_copy_with_one_changed_order_record_or_a_foreign_witness_is_refused() {
    let (_, bundle, witness) = nightly_copy();
    let bad = with_image(&bundle, "log", |b| flip_in(b, b"Futomaki", 0));
    let d = drill::run(&bad, Some(&witness));
    assert!(d.refused.iter().any(|r| r.starts_with("log: ")), "{:?}", d.refused);
    let mut w: Value = serde_json::from_slice(&witness).unwrap();
    w["tip"] = json!("ab".repeat(32));
    let d = drill::run(&bundle, Some(&serde_json::to_vec(&w).unwrap()));
    assert_eq!((d.tip.clone(), d.passed()), (Tip::Missing, false));
}

/// W-SPACE: the night's copy carries its small images PACKED (`dowiz_hub::pack`, within
/// `PACK_BUDGET`), smaller than the raw image and with the RAW image's length in its manifest; and a copy written BEFORE the
/// envelope (raw images, no `codec`) still passes the drill and restores. RED-able: drop the
/// `pack::unpack` in `hubstore::import` and the packed copy stops restoring (its sha256 is the
/// raw image's) -- `the_nightly_copy_passes_the_drill...` goes red; make `unpack` refuse what
/// lacks the magic and this one does.
#[test]
fn the_copy_is_packed_and_a_copy_from_before_the_envelope_still_restores() {
    let (site, bundle, _) = nightly_copy();
    let mut v: Value = serde_json::from_slice(&bundle).unwrap();
    let (mut raw_total, mut packed_total, mut budget_spent) = (0, 0, 0);
    for (id, e) in v["images"].as_object_mut().unwrap() {
        let packed = drill::b64(e["image"].as_str().unwrap()).unwrap();
        // The pack budget (`dowiz_hub::pack::PACK_BUDGET`): `codec` names exactly the packed ones.
        assert_eq!(dowiz_hub::pack::is_packed(&packed), e.get("codec").is_some(), "{id}: codec vs bytes");
        let raw = dowiz_hub::pack::unpack(&packed).unwrap().into_owned();
        assert_eq!(Some(raw.len() as u64), e["bytes"].as_u64(), "{id}: the manifest names the raw length");
        if dowiz_hub::pack::is_packed(&packed) {
            budget_spent += raw.len();
        }
        if ["catalog", "catalog.edits", "settings"].contains(&id.as_str()) {
            assert!(dowiz_hub::pack::is_packed(&packed), "{id}: a small image is packed");
        }
        (raw_total, packed_total) = (raw_total + raw.len(), packed_total + packed.len());
        // The shape every bundle had before W-SPACE.
        e["image"] = json!(base64::engine::general_purpose::STANDARD.encode(&raw));
        e.as_object_mut().unwrap().remove("codec");
    }
    assert!(packed_total < raw_total, "packed {packed_total} vs raw {raw_total}");
    assert!(budget_spent <= dowiz_hub::pack::PACK_BUDGET, "{budget_spent} raw bytes packed in one export");
    let old = serde_json::to_vec(&v).unwrap();
    let d = drill::run(&old, None);
    assert!(d.passed(), "{:?}", d.refused);
    let gamma = crate::hubstore::Place::of_authorised(&site.ctx(&[]), "gamma").unwrap();
    let written = block_on(crate::hubstore::import(&gamma, &v)).unwrap();
    for want in ["log", "catalog", "catalog.edits"] {
        assert!(written.iter().any(|w| w == want), "{want} not restored: {written:?}");
    }
}
