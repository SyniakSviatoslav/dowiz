//! The menu's edit journal through the real routes (W-PITR): an owner's edit is listed with its
//! editor, a restore puts the price back, replay to every step is byte-equal to the catalogue
//! of that step (incl. remove + re-add), and every door that writes the catalogue journals.

use crate::edge::mem::block_on;
use crate::edge::site::{post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::open_venue;
use crate::wire::Call;
use dowiz_hub::catalog::edits::{self, Cut, State};
use serde_json::{json, Value};

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}

fn place(site: &Site) -> crate::hubstore::Place {
    crate::hubstore::Place::of_authorised(&site.ctx(&[]), "alpha").unwrap()
}

fn journal(site: &Site) -> dowiz_hub::logimage::LogImage {
    block_on(crate::hubstore::load_log(&place(site), super::IMAGE)).unwrap().log
}

/// The catalogue now: its keys, and its compacted (canonical) bytes.
fn catalogue(site: &Site) -> (State, Vec<u8>) {
    let mut c = block_on(crate::hubstore::load_catalog(&place(site))).unwrap().catalog;
    (edits::state_of(&c), c.compact().unwrap())
}

fn price(site: &Site, t: &str, dish: &str, p: i64) {
    let r = site.run(crate::owner::update_product, post(&at(&format!("/api/owner/products/{dish}")), &json!({"location_id": "alpha", "price": p})).bearer(t).on("alpha"), &[("id", dish)]);
    assert_eq!(r.status_code(), 200, "price: {}", r.body_str());
}

fn history(site: &Site, t: &str) -> Value {
    let r = site.run(super::list, Call::new(&at("/api/owner/menu/history?location_id=alpha&limit=50"), worker::Method::Get).unwrap().bearer(t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "history: {}", r.body_str());
    r.body_value()
}

#[test]
fn an_owner_edits_a_price_reads_the_history_and_restores_the_dish() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    price(&site, &t, &dish, 1200);
    let h = history(&site, &t);
    let newest = &h["edits"][0];
    assert_eq!((newest["key"].as_str(), newest["price"].as_i64()), (Some(&*format!("product:{dish}")), Some(1200)), "{h}");
    assert!(!newest["by"].as_str().unwrap_or("").is_empty() && newest["by"] != "?", "the owner is the editor: {newest}");
    assert_eq!(newest["restorable"], true);

    let body = json!({"location_id": "alpha", "seq": newest["seq"], "key": newest["key"]});
    let r = site.run(super::restore, post(&at("/api/owner/menu/history/restore"), &body).bearer(&t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "restore: {}", r.body_str());
    let c = block_on(crate::hubstore::load_catalog(&place(&site))).unwrap().catalog;
    let p: Value = serde_json::from_str(&c.product(&dish).unwrap()).unwrap();
    assert_eq!(p["price"], 900, "the price is back to before the edit");
    // The restore is an edit too, and the journal still replays to the catalogue.
    assert_eq!(history(&site, &t)["edits"][0]["price"], 900);
    assert_eq!(edits::replay(&journal(&site), Cut::All).unwrap(), catalogue(&site).0);

    // A stale or wrong key is refused, never applied to another record.
    let stale = json!({"location_id": "alpha", "seq": 0, "key": "product:nope"});
    let r = site.run(super::restore, post(&at("/api/owner/menu/history/restore"), &stale).bearer(&t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 409, "{}", r.body_str());

    // Restoring the edit that CREATED a dish removes it (its positive twin is the price above).
    let cat_id = block_on(crate::hubstore::load_catalog(&place(&site))).unwrap().catalog.categories()[0].0.clone();
    let r = site.run(crate::catalog_edit::create_product, post(&at("/api/owner/products"), &json!({"location_id": "alpha", "category_id": cat_id, "name": "Gyoza", "price": 700})).bearer(&t).on("alpha"), &[]);
    let gyoza = r.body_value()["id"].as_str().unwrap().to_string();
    let made = history(&site, &t)["edits"].as_array().unwrap().iter().find(|e| e["key"] == format!("product:{gyoza}")).cloned().unwrap();
    assert_eq!((made["added"].clone(), made["restorable"].clone()), (json!(true), json!(true)), "{made}");
    let body = json!({"location_id": "alpha", "seq": made["seq"], "key": made["key"]});
    let r = site.run(super::restore, post(&at("/api/owner/menu/history/restore"), &body).bearer(&t).on("alpha"), &[]);
    assert_eq!((r.status_code(), r.body_value()["removed"].clone()), (200, json!(true)), "{}", r.body_str());
    assert!(block_on(crate::hubstore::load_catalog(&place(&site))).unwrap().catalog.product(&gyoza).is_none());
}

#[test]
fn replay_to_every_step_is_byte_equal_to_that_steps_catalogue_with_remove_and_readd() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let cat_id = block_on(crate::hubstore::load_catalog(&place(&site))).unwrap().catalog.categories()[0].0.clone();
    let mut snaps: Vec<(u64, Vec<u8>)> = vec![(journal(&site).len() as u64, catalogue(&site).1)];
    let snap = |s: &mut Vec<(u64, Vec<u8>)>| s.push((journal(&site).len() as u64, catalogue(&site).1));
    price(&site, &t, &dish, 950);
    snap(&mut snaps);
    let r = site.run(crate::catalog_edit::delete_product, post(&at(&format!("/api/owner/products/{dish}/delete")), &json!({"location_id": "alpha"})).bearer(&t).on("alpha"), &[("id", &dish)]);
    assert_eq!(r.status_code(), 200, "delete: {}", r.body_str());
    snap(&mut snaps);
    let r = site.run(crate::catalog_edit::create_product, post(&at("/api/owner/products"), &json!({"location_id": "alpha", "category_id": cat_id, "name": "Futomaki", "price": 990})).bearer(&t).on("alpha"), &[]);
    assert_eq!(r.body_value()["id"].as_str(), Some(dish.as_str()), "re-added under the same id: {}", r.body_str());
    snap(&mut snaps);
    price(&site, &t, &dish, 1010);
    snap(&mut snaps);
    let log = journal(&site);
    assert!(log.quarantined().is_empty(), "the decoder reads every record the writer wrote");
    for (n, want) in &snaps {
        let mut got = edits::rebuild(&edits::replay(&log, Cut::Seq(*n)).unwrap()).unwrap();
        assert_eq!(&got.compact().unwrap(), want, "replay to record {n} is not the catalogue of that step");
    }
    assert!(edits::recent(&log, 50).unwrap().iter().all(|e| e.by != edits::UNSEEN || e.seq < snaps[0].0), "no write after the first went unjournaled");
}

#[test]
fn the_bulk_import_door_journals_in_the_objects_own_turn() {
    let site = Site::new();
    let (t, _) = open_venue(&site, "alpha", "a@x.test");
    let csv = "id,name,unit\nrice,Sushi rice,g\nsalmon,Salmon,g\n";
    let call = Call::new(&at("/api/owner/supplies/import?apply=1"), worker::Method::Post).unwrap().with_body(csv.as_bytes().to_vec()).bearer(&t).on("alpha");
    let r = site.run(crate::services::catalogue::import::bulk::import_supplies, call, &[]);
    assert_eq!(r.status_code(), 200, "import: {}", r.body_str());
    let log = journal(&site);
    let newest = edits::recent(&log, 2).unwrap();
    assert!(newest.iter().all(|e| e.key.starts_with("supply:") && !e.by.is_empty() && e.by != edits::UNSEEN), "{newest:?}");
    assert_eq!(edits::replay(&log, Cut::All).unwrap(), catalogue(&site).0);
}

/// The text of the function around byte `at` of `src`: from its `fn` to its matching brace.
fn body_around(src: &str, at: usize) -> &str {
    let start = src[..at].rfind("fn ").expect("a fn");
    let open = start + src[start..].find('{').expect("a body");
    let mut depth = 0;
    for (i, ch) in src[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &src[start..open + i + 1];
                }
            }
            _ => {}
        }
    }
    &src[start..]
}

/// EVERY CATALOGUE WRITE IS JOURNALED IN THE OBJECT'S OWN STORAGE WRITE (W-PITR2): the one
/// storage writer of an image, `write_chunks_then_meta`, is called only from `hubdo/journal.rs`
/// (`write_image`), `put_image_as` stores through `write_image`, and the Worker's catalogue PUT
/// carries the signer. A door that rewrites the same content says `W-PITR: content unchanged`.
#[test]
fn every_catalogue_write_is_journaled_in_the_objects_write() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    let mut files = vec![std::path::PathBuf::from(root)];
    let mut callers = vec![];
    while let Some(p) = files.pop() {
        if p.is_dir() {
            files.extend(std::fs::read_dir(&p).unwrap().map(|e| e.unwrap().path()));
            continue;
        }
        if p.extension().and_then(|e| e.to_str()) != Some("rs") || p.file_name().and_then(|n| n.to_str()) == Some("tests.rs") {
            continue;
        }
        let src = std::fs::read_to_string(&p).unwrap();
        for (i, _) in src.match_indices("write_chunks_then_meta(") {
            let body = body_around(&src, i);
            if !body.starts_with("fn write_chunks_then_meta(") {
                let name: String = body[3..].chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                callers.push((p.file_name().unwrap().to_string_lossy().into_owned(), name));
            }
        }
    }
    callers.sort();
    callers.dedup();
    assert_eq!(callers, vec![("journal.rs".to_string(), "write_image".to_string())], "an image write that bypasses the journal's door");
    let hubdo = include_str!("../hubdo.rs");
    let at = hubdo.find("async fn put_image_as(").expect("put_image_as");
    assert!(body_around(hubdo, at + 12).contains("self.write_image("), "put_image_as does not store through write_image");
    let hubstore = include_str!("../hubstore.rs");
    let at = hubstore.find("async fn save_image(").expect("save_image");
    assert!(body_around(hubstore, at + 12).contains("x-edit-by"), "the catalogue PUT does not say who signed");
    let compact = include_str!("../hubdo/compact.rs");
    let at = compact.find("async fn compact_one(").expect("compact_one");
    assert!(body_around(compact, at + 12).contains("W-PITR: content unchanged"));
}
