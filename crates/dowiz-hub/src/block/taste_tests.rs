//! W-TASTE row 2's named tests: the `taste` block round-trips, ranks byte-equal with the JSON path
//! on every fixture, never returns an avoided allergen, and leaves every OTHER reader's golden alone.
//!
//! The catalogue is SYNTHETIC and seeded (`menu(SEED)`): 165 dishes, new `sense` and old `taste`
//! fields and none, declared / empty / undeclared allergens, some off sale, ids that differ in case.
//! Its block is pinned at `fixtures/taste/taste.dwb` -- NOT `fixtures/blocks/`, which every
//! reader of the shared table (oracle.py, block.bp, bebop-wasm) walks. Regenerate ONLY on a
//! deliberate format change: `DWB_WRITE_FIXTURES=1`.

use std::collections::BTreeMap;

use super::super::decode::{check, decode};
use super::super::encode::encode;
use super::super::schema::{self, HUB_ONLY, TABLE};
use super::super::view::View;
use super::*;

const SEED: u64 = 0x7A57_E000_0165;
const DISHES: usize = 165;
const EU14: [&str; 14] = crate::allergens::EU14;

fn lcg(s: &mut u64) -> u64 {
    *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *s >> 33
}

/// `lo + lcg % span` distinct words.
fn some<'a>(s: &mut u64, words: &[&'a str], lo: u64, span: u64) -> Vec<&'a str> {
    let n = lo + lcg(s) % span;
    let mut left: Vec<&str> = words.to_vec();
    (0..n.min(words.len() as u64)).map(|_| left.remove((lcg(s) % left.len() as u64) as usize)).collect()
}

/// `(id, product JSON)` as `Catalog::products` gives them.
pub(crate) fn menu(seed: u64) -> Vec<(String, String)> {
    let mut s = seed;
    (0..DISHES)
        .map(|d| {
            let id = format!("{}-{d:03}", if d % 11 == 4 { "Dish" } else { "dish" });
            let mut p = serde_json::json!({ "id": id, "name": format!("Dish {d}"), "price": 300 + 50 * (lcg(&mut s) % 40) });
            match lcg(&mut s) % 10 {
                0 | 1 => {}
                2 => {
                    let axes = some(&mut s, &["spicy", "sweet", "salty", "sour"], 1, 3);
                    let mut t: serde_json::Map<String, Value> = axes.iter().map(|a| (a.to_string(), Value::from(1 + lcg(&mut s) % 3))).collect();
                    t.insert("richness".into(), Value::from(2));
                    p["taste"] = Value::Object(t);
                }
                _ => {
                    let mut sense = serde_json::json!({ "v": 1, "taste": {}, "texture": {}, "aroma": {} });
                    for a in some(&mut s, &sense::TASTE, 0, 4) {
                        sense["taste"][a] = Value::from(lcg(&mut s) % 6);
                    }
                    for t in some(&mut s, &sense::TEXTURE, 0, 4) {
                        sense["texture"][t] = Value::from(1 + lcg(&mut s) % 3);
                    }
                    for a in some(&mut s, &sense::AROMA, 0, 4) {
                        sense["aroma"][a] = Value::from(1 + lcg(&mut s) % 3);
                    }
                    p["sense"] = sense;
                }
            }
            if lcg(&mut s) % 4 != 0 {
                p["allergens"] = Value::from(some(&mut s, &EU14, 0, 3));
            }
            if lcg(&mut s) % 10 == 0 {
                p["available"] = Value::Bool(false);
            }
            (id, p.to_string())
        })
        .collect()
}

fn block_bytes() -> Vec<u8> {
    let (b, skipped) = project(&menu(SEED)).expect("the synthetic catalogue projects");
    assert!(skipped.is_empty(), "{skipped:?}");
    encode(&b).expect("encodes")
}

/// A random vector over the vocabulary (and, now and then, a key outside it, or a negative weight).
fn want(s: &mut u64) -> BTreeMap<String, i64> {
    let keys = sense::all_keys();
    let mut out: BTreeMap<String, i64> = (0..1 + lcg(s) % 8).map(|_| (keys[(lcg(s) % keys.len() as u64) as usize].clone(), 1 + (lcg(s) % 1000) as i64)).collect();
    if lcg(s) % 9 == 0 {
        out.insert("t:richness".into(), 400);
    }
    if lcg(s) % 7 == 0 {
        out.insert(keys[(lcg(s) % keys.len() as u64) as usize].clone(), -700);
    }
    out
}

#[test]
fn the_taste_block_round_trips_and_is_pinned() {
    let b = block_bytes();
    let back = decode(&b).unwrap();
    assert_eq!(back.schema, &schema::TASTE);
    assert_eq!(encode(&back).unwrap(), b, "encode(decode(b)) == b");
    assert_eq!(back.n as usize, DISHES);
    assert!(b.len() <= super::super::MAX_BLOCK, "one Durable Object chunk");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/taste/taste.dwb");
    if std::env::var("DWB_WRITE_FIXTURES").is_ok() {
        std::fs::write(&path, &b).unwrap();
    }
    let pinned = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e} (DWB_WRITE_FIXTURES=1 writes it)", path.display()));
    assert_eq!(pinned, b, "the taste block's bytes moved: a format change needs its fixture regenerated on purpose");
    println!("MEASURE taste.dwb: {} dishes, {} bytes", DISHES, b.len());
}

#[test]
fn every_dish_reads_back_as_its_json_says() {
    let b = block_bytes();
    let v = View::new(&b).unwrap();
    for (row, (id, json)) in menu(SEED).iter().enumerate() {
        let p: Value = serde_json::from_str(json).unwrap();
        assert_eq!(v.str_at(row), Some(id.as_str()));
        assert_eq!(v.i64_at(DISH, row), Some(schema::k64(id.as_bytes()) as i64));
        assert_eq!(vector_at(&v, row), sense::of_product(&p).map(|s| sense::vector(&s)).unwrap_or_default(), "{id}");
        assert_eq!(v.i64_at(ALLERGENS, row), Some(allergen_mask(json)), "{id}");
        assert_eq!(v.i64_at(FLAGS, row).unwrap() & ON_SALE == ON_SALE, p.get("available") != Some(&Value::Bool(false)), "{id}");
    }
}

#[test]
fn top_k_from_the_block_equals_the_json_path_on_every_fixture() {
    let b = block_bytes();
    let v = View::new(&b).unwrap();
    let products = menu(SEED);
    let mut s = SEED ^ 0xA11;
    let (mut cases, mut nonempty) = (0, 0);
    for i in 0..1000 {
        let w = want(&mut s);
        let avoid = if i % 3 == 0 { avoid_mask(&some(&mut s, &EU14, 1, 2)) } else { 0 };
        for k in [5, DISHES] {
            let (got, oracle) = (top_k(&v, &w, avoid, k).unwrap(), top_k_json(&products, &w, avoid, k));
            assert_eq!(got, oracle, "case {i}, k {k}, want {w:?}, avoid {avoid:#x}");
            cases += 1;
            nonempty += usize::from(!got.is_empty());
        }
    }
    println!("MEASURED taste block top-k vs JSON path: {cases}/{cases} byte-equal rankings ({nonempty} non-empty)");
    assert!(nonempty * 10 >= cases * 8, "the fixture is not a file of empty answers: {nonempty}/{cases}");
}

#[test]
fn an_avoided_allergen_is_never_returned_and_an_undeclared_dish_goes_with_it() {
    let products: Vec<(String, String)> = [
        ("fishy", r#"{"sense":{"v":1,"aroma":{"smoky":3}},"allergens":["fish"]}"#),
        ("plain", r#"{"sense":{"v":1,"aroma":{"smoky":2}},"allergens":[]}"#),
        ("nobody-said", r#"{"sense":{"v":1,"aroma":{"smoky":3}}}"#),
        ("off-sale", r#"{"sense":{"v":1,"aroma":{"smoky":3}},"allergens":[],"available":false}"#),
    ]
    .iter()
    .map(|(a, b)| (a.to_string(), b.to_string()))
    .collect();
    let b = encode(&project(&products).unwrap().0).unwrap();
    let v = View::new(&b).unwrap();
    let w: BTreeMap<String, i64> = [("a:smoky".to_string(), 1000)].into_iter().collect();
    let ids = |r: Vec<(String, i64)>| r.into_iter().map(|(id, _)| id).collect::<Vec<_>>();
    assert_eq!(ids(top_k(&v, &w, 0, 5).unwrap()), ["fishy", "nobody-said", "plain"], "nothing avoided: all on sale, ties by id");
    assert_eq!(ids(top_k(&v, &w, avoid_mask(&["fish"]), 5).unwrap()), ["plain"], "fish avoided: the fish dish AND the undeclared one go");
    assert_eq!(ids(top_k(&v, &w, avoid_mask(&["milk"]), 5).unwrap()), ["fishy", "plain"]);
    assert_eq!(avoid_mask(&[]), 0);
    assert_eq!(avoid_mask(&["not-a-code"]), 0, "an unknown code avoids nothing (the edit path refuses it)");
    assert!(top_k(&View::new(&super::super::encode::encode(&super::super::encode::stock_levels(&[])).unwrap()).unwrap(), &w, 0, 5).is_err(), "only a taste block");
}

#[test]
fn the_shared_table_and_its_goldens_do_not_move() {
    // Every implementation of the shared table carries exactly these four (bebop-wasm gate B-2).
    let names: Vec<&str> = TABLE.iter().map(|s| s.name).collect();
    assert_eq!(names, ["menu_prices", "bom", "stock_levels", "names"]);
    assert!(HUB_ONLY.iter().all(|h| !TABLE.iter().any(|t| t.string == h.string)), "a hub-only schema is not in the shared table");
    // A reader that knows only the shared table refuses the taste block by name, never misreads it.
    let b = block_bytes();
    let key = u64::from_le_bytes(b[16..24].try_into().unwrap());
    assert!(!TABLE.iter().any(|t| schema::k64(t.string.as_bytes()) == key), "an old reader meets an unknown schema");
    // And the shared fixtures directory, which every other reader walks, holds no block it cannot read.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/blocks");
    for e in std::fs::read_dir(&dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "dwb") {
            let l = check(&std::fs::read(&p).unwrap()).unwrap_or_else(|r| panic!("{}: {r}", p.display()));
            assert!(TABLE.contains(&l.known.schema), "{} is a {} block in the shared fixtures", p.display(), l.known.schema.name);
        }
    }
}

#[test]
#[cfg_attr(debug_assertions, ignore = "release only: a debug build measures nothing about the row")]
fn top5_on_165_dishes_ns() {
    use std::hint::black_box;
    let b = block_bytes();
    let v = View::new(&b).unwrap();
    let products = menu(SEED);
    let mut s = SEED ^ 0xBE;
    let wants: Vec<BTreeMap<String, i64>> = (0..64).map(|_| want(&mut s)).collect();
    let ns = |iters: u32, f: &mut dyn FnMut(usize)| {
        let mut t: Vec<u128> = (0..21)
            .map(|_| {
                let at = std::time::Instant::now();
                for i in 0..iters {
                    f(i as usize);
                }
                at.elapsed().as_nanos() / u128::from(iters)
            })
            .collect();
        t.sort_unstable();
        t[10]
    };
    let block = ns(2000, &mut |i| {
        black_box(top_k(black_box(&v), &wants[i % 64], 0, 5).unwrap());
    });
    let view_too = ns(2000, &mut |i| {
        let v = View::new(black_box(&b)).unwrap();
        black_box(top_k(&v, &wants[i % 64], 0, 5).unwrap());
    });
    let json = ns(20, &mut |i| {
        black_box(top_k_json(black_box(&products), &wants[i % 64], 0, 5));
    });
    println!("MEASURE taste top-5 over {DISHES} dishes: block {block} ns, View::new + top-5 {view_too} ns, JSON path {json} ns; row bound 10000 ns");
    assert!(block <= 10_000, "top-5 from the block {block} ns > 10 us");
}
