//! The journal in the object's write (W-PITR2): a CUT between the catalogue and its journal
//! leaves both or neither (MemHost, one named cut), and the MEASURE of one price edit on a
//! dubin-size catalogue (165 dishes, ~538 KB) with a ~600-record journal: bytes across the
//! hop and chunks written, before (W-PITR's Worker-side `with_log`) and after.

use super::super::host::mem::{Harness, MemHost, T0};
use crate::wire::Call;
use dowiz_hub::catalog::edits::{self, Cut, State};
use dowiz_hub::catalog::Catalog;
use dowiz_hub::logimage::LogImage;

const JOURNAL: &str = crate::catalog_history::IMAGE;

fn put(h: &Harness, id: &str, gen: i64, bytes: &[u8], by: Option<&str>) -> worker::Result<crate::wire::Reply> {
    let mut c = Call::new(&format!("https://hub/img/{id}"), worker::Method::Put).unwrap().with_header("x-generation", &gen.to_string()).with_body(bytes.to_vec());
    if let Some(by) = by {
        c = c.with_header("x-edit-by", by).with_header("x-edit-at", &T0.to_string());
    }
    h.try_call(c)
}

fn bytes_of(h: &Harness, id: &str) -> Vec<u8> {
    h.get(&format!("/img/{id}")).body().to_vec()
}

fn state(h: &Harness) -> State {
    edits::state_of(&Catalog::load(&bytes_of(h, "catalog")).unwrap())
}

fn replayed(h: &Harness) -> State {
    edits::replay(&LogImage::load(&bytes_of(h, JOURNAL)).unwrap(), Cut::All).unwrap()
}

fn dish(i: usize, price: i64, pad: usize) -> String {
    format!(r#"{{"id":"d{i:03}","name":"Dish {i}","price":{price},"available":true,"description":"{}"}}"#, "x".repeat(pad))
}

fn small() -> Vec<u8> {
    let mut c = Catalog::create().unwrap();
    c.set_location(r#"{"name":"Cut","currency":"ALL"}"#);
    for i in 0..3 {
        c.set_product(&format!("d{i:03}"), &dish(i, 500, 10));
    }
    c.to_bytes().unwrap()
}

/// RED-able: the catalogue's chunk + meta are 2 keys; the cut lets exactly those 2 through.
/// One storage write for both images = nothing lands; a journal written after the catalogue
/// would leave a catalogue whose price edit the journal does not have.
#[test]
fn a_cut_between_the_catalogue_and_its_journal_leaves_both_or_neither() {
    let h = Harness::new();
    assert_eq!(put(&h, "catalog", 0, &small(), Some("owner-1")).unwrap().status_code(), 200);
    assert_eq!(replayed(&h), state(&h), "the first write is journaled (its baseline)");
    let mut c = Catalog::load(&bytes_of(&h, "catalog")).unwrap();
    c.set_product("d001", &dish(1, 777, 10));
    let next = c.to_bytes().unwrap();
    h.host.puts_left.set(Some(2));
    assert!(put(&h, "catalog", 1, &next, Some("owner-1")).map(|r| r.status_code() != 200).unwrap_or(true), "the cut write must fail");
    h.host.puts_left.set(None);
    let cold = h.cold();
    assert_eq!(replayed(&cold), state(&cold), "a cut left a catalogue and a journal that disagree");
    // Positive twin: the same write uncut lands both, the journal naming the signer.
    assert_eq!(put(&cold, "catalog", 1, &next, Some("owner-1")).unwrap().status_code(), 200);
    let cold = cold.cold();
    assert_eq!(replayed(&cold), state(&cold));
    let newest = edits::recent(&LogImage::load(&bytes_of(&cold, JOURNAL)).unwrap(), 1).unwrap();
    assert_eq!((newest[0].key.as_str(), newest[0].by.as_str()), ("product:d001", "owner-1"));
}

/// Chunk keys of `id` written since `from` (`MemHost::writes`), and their stored bytes.
fn written(host: &MemHost, from: usize, id: &str) -> (usize, usize) {
    let pre = format!("c:{id}:");
    let keys: Vec<String> = host.writes.borrow()[from..].iter().filter(|k| k.starts_with(&pre)).cloned().collect();
    let kv = host.kv.borrow();
    let bytes = keys.iter().map(|k| match kv.get(k) {
        Some(super::super::host::mem::Stored::Bytes(b)) => b.len(),
        _ => 0,
    }).sum();
    (keys.len(), bytes)
}

/// THE MEASURE (pass mark: <= 2 journal chunks + <= 2 catalogue chunks per edit, 0 journal
/// bytes across the hop). Printed with `--nocapture` as `PITR2-MEASURE`.
#[test]
fn one_price_edit_on_a_dubin_size_menu_writes_two_chunks_per_image_and_no_journal_crosses() {
    let h = Harness::new();
    let mut c = Catalog::create().unwrap();
    c.set_location(r#"{"name":"Dubin-size","currency":"ALL","menu_version":1}"#);
    c.set_category("rolls", r#"{"id":"rolls","name":"Rolls"}"#);
    for i in 0..165 {
        c.set_product(&format!("d{i:03}"), &dish(i, 900, 3150));
    }
    let base = c.to_bytes().unwrap();
    assert_eq!(put(&h, "catalog", 0, &base, Some("owner-1")).unwrap().status_code(), 200);
    // A 599-record journal that replays to this catalogue and is in step with its generation (1):
    // the baseline (167), then 432 price toggles ending where they began.
    let s0 = edits::state_of(&Catalog::load(&base).unwrap());
    let mut log = LogImage::create().unwrap();
    let (mut cur, toggles) = (s0.clone(), 432i64);
    for i in 0..toggles {
        let mut next = cur.clone();
        let d = (i / 2) as usize % 165;
        next.insert(format!("product:d{d:03}"), dish(d, if i % 2 == 0 { 901 } else { 900 }, 3150));
        edits::journal(&mut log, &cur, &next, T0 + i, "owner-1", (i - toggles + 1, i - toggles + 2)).unwrap();
        cur = next;
    }
    assert_eq!((log.len(), &cur), (599, &s0));
    let jgen = h.get(&format!("/img/{JOURNAL}")).headers().get("x-generation").ok().flatten().unwrap().parse::<i64>().unwrap();
    assert_eq!(h.put(JOURNAL, jgen, &log.to_bytes()).status_code(), 200);
    let journal_bytes = log.to_bytes().len();

    // THE EDIT, as `with_catalog` makes it: GET the catalogue, one price, PUT it back.
    let from = h.host.writes.borrow().len();
    let got = bytes_of(&h, "catalog");
    let mut c = Catalog::load(&got).unwrap();
    c.set_product("d007", &dish(7, 1250, 3150));
    let next = c.to_bytes().unwrap();
    assert_eq!(put(&h, "catalog", 1, &next, Some("owner-1")).unwrap().status_code(), 200);
    let (cat_chunks, cat_bytes) = written(&h.host, from, "catalog");
    let (j_chunks, j_bytes) = written(&h.host, from, JOURNAL);
    let after = bytes_of(&h, JOURNAL);
    let after_len = LogImage::load(&after).unwrap().len();
    println!(
        "PITR2-MEASURE catalogue {} B, journal {} B ({} -> {} records) | hop: catalogue GET {} + PUT {} = {} B, journal 0 B (before: GET {} + PUT {} = {} B more) | DO writes: catalogue {} chunk(s) {} B, journal {} chunk(s) {} B",
        base.len(), journal_bytes, 599, after_len, got.len(), next.len(), got.len() + next.len(),
        journal_bytes, after.len(), journal_bytes + after.len(), cat_chunks, cat_bytes, j_chunks, j_bytes
    );
    assert_eq!(after_len, 600, "one record for one price");
    assert!(cat_chunks <= 2 && j_chunks <= 2, "catalogue {cat_chunks} / journal {j_chunks} chunks for one edit");
    assert_eq!(replayed(&h.cold()), state(&h.cold()));
}
