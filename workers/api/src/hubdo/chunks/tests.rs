//! W-DELTA's chunk pass mark, measured with THE function the object writes with.
//!
//! One price edit on a 165 x 2.5 KB catalogue: before W-DELTA `Catalog::to_bytes` compacted
//! the whole image (the "before" column here is `Catalog::compact`, the same code); now it
//! appends one record and a new root, so only the chunk holding the superblock pages and the
//! tail chunk(s) differ. Printed per edit for the verdict; asserted <= 2 for one edit.
use super::super::CHUNK;
use super::changed_chunks;
use dowiz_hub::catalog::Catalog;

fn catalogue() -> Vec<u8> {
    let mut c = Catalog::create().unwrap();
    c.set_location(r#"{"id":"v1","currency_code":"ALL"}"#);
    for i in 0..165 {
        let pad = "x".repeat(2500 - 120);
        c.set_product(&format!("item-{i:03}"), &format!(r#"{{"id":"item-{i:03}","name":"Dish {i}","price":{},"description":"{pad}"}}"#, 500 + i));
    }
    c.to_bytes().unwrap()
}

fn edit(bytes: &[u8], i: usize, price: usize) -> Catalog {
    let mut c = Catalog::load(bytes).unwrap();
    let id = format!("item-{i:03}");
    let dish = c.product(&id).unwrap();
    let at = dish.find("\"price\":").unwrap() + 8;
    let end = at + dish[at..].find(',').unwrap();
    c.set_product(&id, &format!("{}{price}{}", &dish[..at], &dish[end..]));
    c
}

#[test]
fn one_price_edit_writes_at_most_two_chunks() {
    let base = catalogue();
    let total = base.len().div_ceil(CHUNK);
    // 542 -> 600 keeps the value's length (the old path's offsets do not move); 542 -> 1600
    // grows it by one byte, and every offset after dish 42 moves in a compacted image.
    for price in [600, 1600] {
        let after = edit(&base, 42, price).to_bytes().unwrap();
        let old_path = edit(&base, 42, price).compact().unwrap();
        let new = changed_chunks(Some(&base), &after, CHUNK);
        let old = changed_chunks(Some(&base), &old_path, CHUNK);
        println!("chunks: image {} B = {total} chunks; price 542 -> {price}: delta writes {new:?}, compacted (the old path) {old:?}", base.len());
        assert!(new.len() <= 2, "one price edit wrote chunks {new:?}");
        assert_eq!(new[0], 0, "the superblock chunk");
    }
}

/// A run of single edits, each against the image the previous one wrote: the distribution,
/// including the edit whose record crosses a chunk boundary (3: superblock, old tail, new).
#[test]
fn a_run_of_price_edits_writes_two_chunks_except_at_a_boundary() {
    let mut bytes = catalogue();
    let mut hist = [0usize; 8];
    for k in 0..32 {
        let next = edit(&bytes, (k * 37) % 165, 700 + k).to_bytes().unwrap();
        let n = changed_chunks(Some(&bytes), &next, CHUNK).len();
        hist[n.min(7)] += 1;
        assert!(n <= 3, "edit {k} wrote {n} chunks");
        bytes = next;
    }
    println!("chunks per edit over 32 successive edits: {:?} (index = chunks written)", &hist[..5]);
    assert!(hist[1] + hist[2] >= 30, "{hist:?}");
}
