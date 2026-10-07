//! The object's in-place catalogue reads (W-ZC): the same answers as `Catalog::load`,
//! one crc per generation, and a re-read after an eviction checked again.
use crate::hubdo::host::mem::Harness;
use dowiz_hub::catalog::Catalog;
use serde_json::{json, Value};

fn catalog() -> Catalog {
    let mut c = Catalog::create().unwrap();
    c.set_location(r#"{"id":"v1","currency_code":"ALL","timezone":"Europe/Tirane"}"#);
    for i in 0..40 {
        c.set_product(&format!("p{i:02}"), &json!({"id": format!("p{i:02}"), "name": format!("Maki {i}"), "price": 500 + i}).to_string());
    }
    c.set_supply("rice", r#"{"id":"rice","name":"Rice","unit":"g","active":true}"#);
    c.set_promo("VERE10", r#"{"code":"VERE10","pct":10}"#);
    c
}

fn product(h: &Harness, id: &str) -> Value {
    h.get(&format!("/fold/catalogue?q=product&id={id}")).body_value()["product"].clone()
}

/// THE SAME ANSWERS: one dish (first, last, absent), the fold, the supplies, the venue
/// and a basket, against what `Catalog::load` gives on the same bytes.
#[test]
fn the_object_answers_in_place_what_catalog_load_answers() {
    let h = Harness::new();
    let mut cat = catalog();
    h.put("catalog", 0, &cat.to_bytes().unwrap());
    for id in ["p00", "p39", "p17", "nope", ""] {
        let want = cat.product(id).map_or(Value::Null, Value::String);
        assert_eq!(product(&h, id), want, "product {id:?}");
    }
    assert_eq!(h.get("/fold/catalogue?q=root").body_value()["root"], cat.root());
    let supplies = crate::services::engagement::voice::kitchen::supplies(&cat.supplies());
    assert_eq!(h.get("/fold/catalogue?q=supplies").body_value()["supplies"], json!(supplies));
    assert_eq!(h.get("/fold/venue").body_value()["timezone"], "Europe/Tirane");
    let ids: Vec<String> = ["p01", "p39", "gone", "p01"].map(String::from).to_vec();
    let want = crate::services::ordering::basket::answer(&cat, &ids, Some("VERE10"));
    assert_eq!(h.get("/fold/basket?ids=p01&ids=p39&ids=gone&ids=p01&promo=VERE10").body_value(), want);
    let gen = h.obj.mem.borrow().get("catalog").map(|(m, _)| m.generation).unwrap();
    assert_eq!(h.obj.cat_checked.get().map(|(g, _)| g), Some(gen), "the check is kept for this generation");
}

/// Flip one byte of the stored image, inside a dish's value.
fn corrupt_storage(h: &Harness, needle: &[u8]) {
    let mut kv = h.host.kv.borrow_mut();
    let crate::hubdo::host::mem::Stored::Bytes(b) = kv.get_mut("c:catalog:0").expect("chunk 0") else { panic!("chunk is not bytes") };
    let at = b.windows(needle.len()).position(|w| w == needle).expect("needle in chunk 0");
    b[at] ^= 0x01;
}

/// A RE-READ AFTER AN EVICTION IS CHECKED AGAIN, at the same generation: the chunk on
/// storage was changed after the first check, `mem` lost its copy, and the next read
/// must refuse -- not `reopen` the changed bytes on the old `Checked`.
#[test]
fn a_reread_after_eviction_is_crc_checked_again() {
    let h = Harness::new();
    h.put("catalog", 0, &catalog().to_bytes().unwrap());
    assert!(product(&h, "p05").as_str().unwrap().contains("Maki 5"));
    corrupt_storage(&h, b"Maki 5\"");
    h.obj.mem.borrow_mut().remove("catalog");
    let r = h.try_call(crate::wire::Call::new("https://hub/fold/catalogue?q=product&id=p05", worker::Method::Get).unwrap());
    match r {
        Err(e) => assert!(e.to_string().contains("unreadable"), "{e}"),
        Ok(r) => panic!("a changed byte was read as data: {} {}", r.status_code(), r.body_str()),
    }
    // A cold object over the same storage refuses too.
    assert!(h.cold().try_call(crate::wire::Call::new("https://hub/fold/catalogue?q=product&id=p05", worker::Method::Get).unwrap()).is_err());
}

/// W-DELTA THROUGH THE OBJECT: a 165 x 2.5 KB catalogue, then ONE price edit and one removal
/// written as a delta (v3) by `Catalog::to_bytes`. The next in-place read answers the edit
/// (the previous generation's `Checked` never serves it), a cold object over the same storage
/// answers the same, and the write sent the chunk holding the superblocks and the tail chunk
/// only -- counted on the storage the object wrote to, not computed.
#[test]
fn a_delta_write_is_read_next_and_writes_two_chunks() {
    let h = Harness::new();
    let mut big = Catalog::create().unwrap();
    for i in 0..165 {
        big.set_product(&format!("p{i:03}"), &json!({"id": format!("p{i:03}"), "price": 500 + i, "d": "x".repeat(2400)}).to_string());
    }
    let base = big.to_bytes().unwrap();
    h.put("catalog", 0, &base);
    assert!(product(&h, "p005").as_str().unwrap().contains("\"price\":505"));
    let mut c = Catalog::load(&base).unwrap();
    c.set_product("p005", &json!({"id": "p005", "price": 777}).to_string());
    c.remove_product("p006");
    let after = c.to_bytes().unwrap();
    assert!(after.len() > base.len() && after.len() - base.len() < 1024, "{} -> {} B: not an append", base.len(), after.len());
    h.host.writes.borrow_mut().clear();
    h.put("catalog", 1, &after);
    let chunks: Vec<String> = h.host.writes.borrow().iter().filter(|k| k.starts_with("c:catalog:")).cloned().collect();
    println!("delta write through the object: {} B image, chunks written {chunks:?}", after.len());
    assert!(chunks.len() <= 2 && chunks.contains(&"c:catalog:0".to_string()), "{chunks:?}");
    assert!(product(&h, "p005").as_str().unwrap().contains("\"price\":777"), "the edit was not read");
    assert_eq!(product(&h, "p006"), Value::Null, "the removal was not read");
    assert_eq!(h.get("/fold/catalogue?q=root").body_value()["root"], c.root());
    let cold = h.cold();
    assert!(cold.get("/fold/catalogue?q=product&id=p005").body_value()["product"].as_str().unwrap().contains("777"), "a cold object");
}
