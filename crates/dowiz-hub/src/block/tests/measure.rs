//! Row DG7's acceptance numbers, MEASURED (BLUEPRINT §DG7 "Acceptance").
//!
//! RELEASE ONLY: a debug build times the debug build (the first lane's 28,303 ns
//! "decode" was one), so these are `ignore`d under `debug_assertions` and run by
//!   cargo test --release --lib block::tests::measure -- --nocapture
//! Each prints its number and asserts the row's bound. Integer nanoseconds.

use super::super::decode::{check, decode};
use super::super::encode::encode;
use super::super::view::Catalogue;
use super::{catalogue, fixture, SEED};
use std::hint::black_box;
use std::time::Instant;

/// Median of `rounds` timings of `iters` calls, in ns per call.
fn ns_per<F: FnMut()>(rounds: usize, iters: u32, mut f: F) -> u128 {
    let mut t: Vec<u128> = (0..rounds)
        .map(|_| {
            let s = Instant::now();
            for _ in 0..iters {
                f();
            }
            s.elapsed().as_nanos() / u128::from(iters)
        })
        .collect();
    t.sort_unstable();
    t[rounds / 2]
}

#[test]
#[cfg_attr(debug_assertions, ignore = "release only: a debug build measures nothing about the row")]
fn decode_menu_prices_ns() {
    let b = fixture("menu_prices");
    let crc = ns_per(21, 2000, || {
        black_box(super::super::crc32(black_box(&b)));
    });
    let chk = ns_per(21, 2000, || {
        black_box(check(black_box(&b)).unwrap());
    });
    let dec = ns_per(21, 2000, || {
        black_box(decode(black_box(&b)).unwrap());
    });
    println!("MEASURE menu_prices {} bytes: crc32 {crc} ns, check {chk} ns, decode (check + copy out) {dec} ns; row bound 10000 ns", b.len());
    assert!(dec <= 10_000, "decode {dec} ns > 10 us");
}

#[test]
#[cfg_attr(debug_assertions, ignore = "release only")]
fn bom_of_five_lines_ns() {
    let (m, bm, n) = (fixture("menu_prices"), fixture("bom"), fixture("names"));
    let cat = Catalogue::new(&m, &bm, &n).unwrap();
    let (id, json) = catalogue(SEED).swap_remove(5);
    assert_eq!(cat.bom_of(&id).unwrap().len(), 5, "dish-005 is the five-line placement");
    let block = ns_per(21, 20_000, || {
        black_box(cat.bom_of(black_box(&id)));
    });
    let from_json = ns_per(21, 20_000, || {
        black_box(crate::stock::bom_of(black_box(&json)));
    });
    let build = ns_per(11, 200, || {
        black_box(Catalogue::new(&m, &bm, &n).unwrap());
    });
    println!("MEASURE bom_of 5 lines: block {block} ns, JSON (minijson) {from_json} ns; Catalogue::new {build} ns once per generation; row bound 1000 ns");
    assert!(block <= 1_000, "bom_of from the block {block} ns > 1 us");
}

#[test]
#[cfg_attr(debug_assertions, ignore = "release only")]
fn bytes_and_json_parse() {
    let sizes: Vec<(&str, usize)> = ["menu_prices", "bom", "names", "stock_levels"].iter().map(|n| (*n, fixture(n).len())).collect();
    let prices_and_bom = sizes[0].1 + sizes[1].1;
    let products = catalogue(SEED);
    let json_bytes: usize = products.iter().map(|(_, j)| j.len()).sum();
    let parse = ns_per(11, 20, || {
        for (_, j) in &products {
            black_box(serde_json::from_str::<serde_json::Value>(black_box(j)).unwrap());
        }
    });
    // What a memo miss now pays on top of its JSON fold (`fold::menu::blocks_of`).
    let fold = ns_per(11, 20, || {
        let p = super::super::encode::project(black_box(&products)).unwrap();
        black_box((encode(&p.menu_prices).unwrap(), encode(&p.bom).unwrap(), encode(&p.names).unwrap()));
    });
    println!("MEASURE bytes {sizes:?}; menu_prices + bom = {prices_and_bom} B; the same 165 products as JSON {json_bytes} B, serde parse {parse} ns; project + encode 3 blocks {fold} ns");
    // The row's bound (10 KB for 165 dishes) is held PER BLOCK: each is one Durable Object
    // chunk's worth of one relation. The sum is printed, not asserted (see the verdict).
    for (name, len) in &sizes {
        assert!(*len <= 10 * 1024, "{name} is {len} B");
    }
}
