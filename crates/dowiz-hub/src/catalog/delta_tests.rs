//! The catalogue's delta writes (W-DELTA, R-BEBOPDB D.1 #3), through the real `Catalog` API.
//!
//! THE PASS MARK: a read after N deltas equals `Catalog::load` of the COMPACTED image, on 1000
//! fixed seeded edit sequences -- a deterministic, named invariant over typed edits, not a
//! fuzzer over image bytes (operator rule, memory no-fuzzers-in-this-repo).
use super::view::tests::big_165;
use super::*;
use crate::catalog::view::{CatalogRead, CatalogView};

/// Everything a catalogue answers, as one comparable value.
fn answers(c: &dyn CatalogRead) -> (Option<String>, Vec<(String, String)>, Vec<(String, String)>, Vec<(String, String)>, Vec<(String, String)>, String) {
    (c.location(), c.products(), c.categories(), c.promos(), c.supplies(), c.root())
}

/// `Catalog::load` of the compacted image of `bytes`.
fn compacted(bytes: &[u8]) -> Vec<u8> {
    Catalog::load(bytes).unwrap().compact().unwrap()
}

fn base() -> Vec<u8> {
    let mut c = Catalog::create().unwrap();
    c.set_location(r#"{"name":"Delta","currency":"ALL"}"#);
    for i in 0..24 {
        c.set_product(&format!("d{i:02}"), &format!(r#"{{"id":"d{i:02}","price":{},"pad":"{}"}}"#, 300 + i * 10, "x".repeat(150)));
    }
    c.set_category("hot", r#"{"id":"hot"}"#);
    c.to_bytes().unwrap()
}

#[test]
fn a_read_after_deltas_equals_the_compacted_image_on_1000_seeded_sequences() {
    let (mut deltas, mut compactions) = (0usize, 0usize);
    let start = base();
    for seed in 0..1000u64 {
        let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        let mut r = |n: u64| {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (x >> 33) % n
        };
        let mut bytes = start.clone();
        for batch in 0..(1 + r(6)) {
            let mut c = Catalog::load(&bytes).unwrap();
            for _ in 0..(1 + r(3)) {
                let id = format!("d{:02}", r(30));
                match r(6) {
                    0 => {
                        c.remove_product(&id);
                    }
                    1 => c.set_promo(&format!("P{}", r(4)), &format!(r#"{{"pct":{}}}"#, r(50))),
                    2 => {
                        c.remove_promo(&format!("P{}", r(4)));
                    }
                    3 => c.set_supply(&format!("s{}", r(5)), &format!(r#"{{"unit":"g","n":{}}}"#, r(9))),
                    _ => c.set_product(&id, &format!(r#"{{"id":"{id}","price":{},"s":{seed},"b":{batch}}}"#, 100 + r(900) * 10)),
                }
            }
            let want = answers(&c);
            let next = c.to_bytes().unwrap();
            let v = Store::from_bytes(&next);
            if bebop_store::kv::Kv::version(&v) == 3 { deltas += 1 } else { compactions += 1 }
            let read = Catalog::load(&next).unwrap();
            let comp = Catalog::load(&compacted(&next)).unwrap();
            let what = format!("seed {seed} batch {batch}");
            assert_eq!(answers(&read), want, "{what}: read after deltas != the edits");
            assert_eq!(answers(&read), answers(&comp), "{what}: read after deltas != Catalog::load of the compacted image");
            assert_eq!(answers(&CatalogView::open(&next).unwrap()), want, "{what}: CatalogView");
            bytes = next;
        }
    }
    println!("catalog delta sequences: 1000 seeds, {deltas} delta writes, {compactions} compacted writes");
    assert!(deltas > 1000 && compactions > 0, "both paths must be exercised: {deltas} / {compactions}");
}

/// A SINGLE PRICE EDIT on 165 x 2.5 KB: one appended record, and the bytes differ only in
/// the superblock pages (cells < 1024) and past the old end -- so a DO writes the chunk
/// holding the superblocks and the tail chunk(s), never the middle.
#[test]
fn one_price_edit_changes_the_superblock_page_and_the_tail_only() {
    let before = big_165().to_bytes().unwrap();
    let mut c = Catalog::load(&before).unwrap();
    let dish = c.product("item-042").unwrap().replace("\"price\":542", "\"price\":600");
    c.set_product("item-042", &dish);
    let after = c.to_bytes().unwrap();
    assert_eq!(bebop_store::kv::Kv::version(&Store::from_bytes(&after)), 3, "the edit was not a delta");
    let grew = after.len() - before.len();
    assert!(grew < 4 * 1024, "the image grew by {grew} B for one dish");
    let first_mid = (bebop_store::ARENA * 8..before.len()).find(|&i| before[i] != after[i]);
    assert_eq!(first_mid, None, "a byte between the superblock pages and the old end changed");
    assert!(Catalog::load(&after).unwrap().product("item-042").unwrap().contains("\"price\":600"));
    // fold byte-equal after compaction
    let comp = Catalog::load(&compacted(&after)).unwrap();
    assert_eq!(comp.root(), Catalog::load(&after).unwrap().root());
}

/// No edit: the bytes come back as they were (no new generation, no compaction).
#[test]
fn to_bytes_without_an_edit_is_the_image_unchanged() {
    let b = base();
    assert_eq!(Catalog::load(&b).unwrap().to_bytes().unwrap(), b);
    // and removing an absent dish is no edit either
    let mut c = Catalog::load(&b).unwrap();
    assert!(!c.remove_product("nope"));
    assert_eq!(c.to_bytes().unwrap(), b);
}

/// Thirty-three single edits in a row: the chain stops at `MAX_DELTAS` and the next write
/// compacts to v2 -- whose bytes are what compacting the same entries always wrote.
#[test]
fn the_chain_is_compacted_at_its_limit() {
    let mut bytes = big_165().to_bytes().unwrap();
    let mut versions = Vec::new();
    for i in 0..=delta::MAX_DELTAS {
        let mut c = Catalog::load(&bytes).unwrap();
        c.set_product("item-001", &format!(r#"{{"id":"item-001","price":{}}}"#, 1000 + i));
        bytes = c.to_bytes().unwrap();
        versions.push(bebop_store::kv::Kv::version(&Store::from_bytes(&bytes)));
    }
    assert!(versions[..delta::MAX_DELTAS].iter().all(|&v| v == 3), "{versions:?}");
    assert_eq!(versions[delta::MAX_DELTAS], 2, "the 33rd write did not compact");
    let entries = Catalog::load(&bytes).unwrap();
    assert_eq!(bytes, compacted(&bytes), "a compacted write is not what compaction writes");
    assert!(entries.product("item-001").unwrap().contains("1032"));
}

/// THE DO MEMO (workers/api/src/hubdo/catview.rs): `put_image` forgets the `Checked` before
/// new bytes enter memory, so the next read is a full `open`. And even a `Checked` taken on
/// the PREVIOUS bytes cannot give a stale answer on v3 bytes: it carries only the key order,
/// and `reopen` walks the chain again -- shown here, with the old memo held across a delta.
#[test]
fn a_checked_from_before_the_delta_still_reads_the_delta() {
    let before = big_165().to_bytes().unwrap();
    let ck = CatalogView::open(&before).unwrap().checked();
    let mut c = Catalog::load(&before).unwrap();
    c.set_product("item-007", r#"{"id":"item-007","price":1}"#);
    c.remove_product("item-008");
    let after = c.to_bytes().unwrap();
    let view = CatalogView::reopen(&after, ck).unwrap();
    assert_eq!(answers(&view), answers(&Catalog::load(&after).unwrap()));
    assert_eq!(view.product("item-007").as_deref(), Some(r#"{"id":"item-007","price":1}"#));
    assert_eq!(view.product("item-008"), None);
}
