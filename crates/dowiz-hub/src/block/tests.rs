//! Row DG7's named tests (BLUEPRINT §DG7 "RED first") and the fixtures.
//!
//! The catalogue is SYNTHETIC and seeded (`catalogue(SEED)`): 165 dishes, 120
//! supplies, prices in minor units, `vat_ppm` on most, modifier deltas (some
//! negative), recipes of 1-6 lines. The fixtures under `fixtures/blocks/` are
//! its encoding; `fixtures_are_the_encoder_output` holds them byte-equal.
//! Regenerate ONLY on a deliberate format change: `DWB_WRITE_FIXTURES=1`.

use super::decode::{check, decode};
use super::encode::{encode, project, stock_levels};
use super::view::{short_mask, short_scalar, Catalogue};
use super::*;

const SEED: u64 = 0xD6_7000_0165;
const DISHES: usize = 165;
const SUPPLIES: usize = 120;

fn lcg(s: &mut u64) -> u64 {
    *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *s >> 33
}

/// `(id, product JSON)` in the hub's stored shape (`Catalog::products`).
pub(super) fn catalogue(seed: u64) -> Vec<(String, String)> {
    let mut s = seed;
    (0..DISHES)
        .map(|d| {
            let id = format!("dish-{d:03}");
            let price = 300 + 50 * (lcg(&mut s) % 55) as i64;
            let vat = if d % 7 == 3 { String::new() } else { format!(",\"vat_ppm\":{}", [200_000, 60_000, 0][d % 3]) };
            let nopt = (lcg(&mut s) % 4) as usize;
            let opts: Vec<String> = (0..nopt)
                .map(|o| format!("{{\"id\":\"opt-{}\",\"name\":\"O\",\"priceDelta\":{}}}", lcg(&mut s) % 40, 50 * o as i64 - 50))
                .collect();
            let lines = if d == 5 { 5 } else { 1 + (lcg(&mut s) % 6) as usize };
            let bom: Vec<String> = (0..lines)
                .map(|l| format!("{{\"supply\":\"supply-{:03}\",\"qty\":{}}}", (d * 7 + l * 13) % SUPPLIES, 1 + lcg(&mut s) % 400))
                .collect();
            let json = format!(
                "{{\"id\":\"{id}\",\"name\":\"Dish {d}\",\"price\":{price}{vat},\"modifierGroups\":[{{\"id\":\"g\",\"name\":\"G\",\"min\":0,\"max\":3,\"options\":[{}]}}],\"bom\":[{}]}}",
                opts.join(","),
                bom.join(",")
            );
            (id, json)
        })
        .collect()
}

/// `DWB_FIXTURE_DIR` points the tests at a SCRATCH copy (the mutation proof corrupts that, never the tree).
fn fixture_path(name: &str) -> std::path::PathBuf {
    let dir = std::env::var("DWB_FIXTURE_DIR").map(std::path::PathBuf::from);
    let dir = dir.unwrap_or_else(|_| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/blocks"));
    dir.join(format!("{name}.dwb"))
}

/// The four fixture blocks as the encoder writes them today.
fn encoded() -> Vec<(&'static str, Vec<u8>)> {
    let p = project(&catalogue(SEED)).expect("the synthetic catalogue projects");
    assert!(p.skipped.is_empty(), "skipped {:?}", p.skipped);
    let levels: Vec<(i64, i64, i64)> = (0..SUPPLIES as i64).map(|i| (schema::k64(format!("supply-{i:03}").as_bytes()) as i64, 1000 + i * 37, 7)).collect();
    vec![
        ("menu_prices", encode(&p.menu_prices).unwrap()),
        ("bom", encode(&p.bom).unwrap()),
        ("names", encode(&p.names).unwrap()),
        ("stock_levels", encode(&stock_levels(&levels)).unwrap()),
    ]
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(fixture_path(name)).unwrap_or_else(|e| panic!("fixture {name}: {e} (DWB_WRITE_FIXTURES=1 writes it)"))
}

/// Overwrite a column VALUE in a copy and re-seal the crc, so the check under test is reached.
fn patch_u32(b: &[u8], col: usize, idx: usize, v: u32) -> Vec<u8> {
    let l = check(b).unwrap();
    let mut out = b.to_vec();
    let at = l.cols[col].0 + 4 * idx;
    out[at..at + 4].copy_from_slice(&v.to_le_bytes());
    reseal(out)
}

fn reseal(mut b: Vec<u8>) -> Vec<u8> {
    let end = b.len() - CRC;
    let c = crc32(&b[..end]);
    b[end..].copy_from_slice(&c.to_le_bytes());
    b
}

#[test]
fn fixtures_are_the_encoder_output() {
    let write = std::env::var("DWB_WRITE_FIXTURES").is_ok_and(|v| v == "1");
    for (name, bytes) in encoded() {
        if write {
            std::fs::create_dir_all(fixture_path(name).parent().unwrap()).unwrap();
            std::fs::write(fixture_path(name), &bytes).unwrap();
        }
        assert_eq!(fixture(name), bytes, "fixture {name} is not what the encoder writes");
        println!("fixture {name}: {} bytes", bytes.len());
    }
}

#[test]
fn roundtrip_menu_prices() {
    let b = fixture("menu_prices");
    let block = decode(&b).expect("the fixture decodes");
    assert_eq!((block.n, block.cols.len()), (DISHES as u32, 6));
    assert_eq!(encode(&block).unwrap(), b, "encode(decode(b)) == b");
    let p = project(&catalogue(SEED)).unwrap();
    assert_eq!(decode(&encode(&p.menu_prices).unwrap()).unwrap(), p.menu_prices, "decode(encode(x)) == x");
    // The column holds what the JSON says, row for row.
    let Col::I64(prices) = &block.cols[1] else { panic!("price is i64") };
    for (row, (_, json)) in catalogue(SEED).iter().enumerate() {
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(prices[row], v["price"].as_i64().unwrap());
    }
    assert!(b.len() <= 10 * 1024, "menu_prices is {} bytes", b.len());
}

#[test]
fn roundtrip_bom() {
    for name in ["bom", "names", "stock_levels"] {
        let b = fixture(name);
        let block = decode(&b).unwrap_or_else(|r| panic!("{name}: {r}"));
        assert_eq!(encode(&block).unwrap(), b, "{name}: encode(decode(b)) == b");
    }
    // The empty block (n = 0) round-trips too (B-6).
    let empty = project(&[]).unwrap();
    for x in [empty.menu_prices, empty.bom, empty.names] {
        let b = encode(&x).unwrap();
        assert_eq!(decode(&b).unwrap(), x);
        assert_eq!(encode(&decode(&b).unwrap()).unwrap(), b);
    }
}

#[test]
fn refuse_bad_magic() {
    let b = fixture("menu_prices");
    assert!(decode(&b).is_ok(), "the twin: the untouched fixture decodes");
    let mut bad = b.clone();
    bad[0] ^= 0x01;
    assert_eq!(decode(&bad).unwrap_err(), Refusal::BadMagic);
    let mut v = b.clone();
    v[4] = 2;
    assert_eq!(decode(&reseal(v)).unwrap_err(), Refusal::BadVersion(2));
    let mut s = b;
    s[16] ^= 0xff;
    assert_eq!(decode(&reseal(s)).unwrap_err().code(), "unknown_schema");
}

#[test]
fn refuse_bad_crc() {
    let b = fixture("bom");
    assert!(decode(&b).is_ok());
    let mut bad = b.clone();
    let mid = b.len() / 2;
    bad[mid] ^= 0x10;
    assert_eq!(decode(&bad).unwrap_err().code(), "bad_crc");
    let mut tail = b;
    let last = tail.len() - 1;
    tail[last] ^= 0x80;
    assert_eq!(decode(&tail).unwrap_err().code(), "bad_crc");
}

#[test]
fn refuse_bad_offsets() {
    let b = fixture("menu_prices");
    // A row_ptr VALUE past nnz, a row_ptr that goes backwards, and a last entry short of nnz.
    let nnz = check(&b).unwrap().nnz;
    assert_eq!(decode(&patch_u32(&b, 3, 1, nnz + 1)).unwrap_err(), Refusal::BadOffsets { col: "mods_ptr" });
    assert_eq!(decode(&patch_u32(&b, 3, DISHES, nnz - 1)).unwrap_err(), Refusal::BadOffsets { col: "mods_ptr" });
    let bom = fixture("bom");
    assert_eq!(decode(&patch_u32(&bom, 0, 0, 1)).unwrap_err(), Refusal::BadOffsets { col: "dish_ptr" });
    let names = fixture("names");
    assert_eq!(decode(&patch_u32(&names, 2, 3, 0)).unwrap_err(), Refusal::BadOffsets { col: "off" });
    // A column DESCRIPTOR: offset moved by 8 (aligned, wrong place) and by 4 (misaligned).
    let at = header_size(0) + COLDESC * 2 + 8;
    let off = u32::from_le_bytes(b[at..at + 4].try_into().unwrap());
    let mut moved = b.clone();
    moved[at..at + 4].copy_from_slice(&(off + 8).to_le_bytes());
    assert_eq!(decode(&reseal(moved)).unwrap_err(), Refusal::BadOffsets { col: "tax_ppm" });
    let mut odd = b.clone();
    odd[at..at + 4].copy_from_slice(&(off + 4).to_le_bytes());
    assert_eq!(decode(&reseal(odd)).unwrap_err(), Refusal::BadAlignment { col: "tax_ppm" });
    let mut ty = b.clone();
    ty[header_size(0) + COLDESC] = super::ty::I32;
    assert_eq!(decode(&reseal(ty)).unwrap_err(), Refusal::BadType { col: "price" });
    assert!(decode(&b).is_ok(), "the twin");
}

#[test]
fn price_overflow_refused() {
    let big = format!("{{\"id\":\"x\",\"price\":{},\"vat_ppm\":200000,\"bom\":[]}}", i64::MAX - 10);
    let ok = r#"{"id":"y","price":1500,"vat_ppm":200000,"bom":[]}"#.to_string();
    let p = project(&[("x".into(), big), ("y".into(), ok)]).unwrap();
    let (m, bm, nm) = (encode(&p.menu_prices).unwrap(), encode(&p.bom).unwrap(), encode(&p.names).unwrap());
    let cat = Catalogue::new(&m, &bm, &nm).unwrap();
    let x = cat.row_of("x").unwrap();
    assert_eq!(cat.unit_gross(x, 0, false).unwrap_err(), Refusal::Overflow { col: "price" });
    assert_eq!(cat.unit_gross(x, 0, true).unwrap(), i64::MAX - 10, "inclusive: the price IS the gross");
    let y = cat.row_of("y").unwrap();
    let want = 1500 + dowiz_core::tax::tax_of(1500, dowiz_core::tax::RatePpm(200_000), false).unwrap();
    assert_eq!(cat.unit_gross(y, 0, false).unwrap(), want, "the twin, against the money law");
    assert_eq!(cat.unit_gross(y, 0, false).unwrap(), 1800);
}

#[test]
fn mask_vec_equals_scalar() {
    let b = decode(&fixture("bom")).unwrap();
    let (Col::U32(rp), Col::U32(col), Col::I64(val)) = (&b.cols[0], &b.cols[1], &b.cols[2]) else { panic!("bom columns") };
    let rows = check(&fixture("names")).unwrap().n as usize;
    let mut s = SEED;
    for portions in [0i64, 1, 2, 3, 10] {
        let stock: Vec<i64> = (0..rows).map(|_| (lcg(&mut s) % 900) as i64).collect();
        let a = short_scalar(rp, col, val, &stock, portions);
        let v = short_mask(rp, col, val, &stock, portions);
        assert_eq!(a.len(), DISHES);
        assert_eq!(a, v, "portions {portions}");
        if portions == 10 {
            assert!(a.iter().any(|x| *x), "ten portions run something short");
        }
        if portions == 0 {
            assert!(a.iter().all(|x| !*x), "zero portions need nothing");
        }
    }
}

#[test]
fn schema_table_agrees() {
    let mut keys = Vec::new();
    for k in schema::known() {
        let s = k.schema.string.as_bytes();
        // The second implementation: bebop_store's bitwise crc (the `crc32x` twin).
        let twin = (u64::from(bebop_store::crc32(s)) << 32) | s.len() as u64;
        assert_eq!(k.k64, twin, "{}", k.schema.name);
        assert!(k.schema.string.starts_with(&format!("{}:v1(", k.schema.name)));
        keys.push(k.k64);
    }
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), schema::TABLE.len() + schema::HUB_ONLY.len(), "every schema has its own K64");
    let names: Vec<&str> = schema::of(&schema::MENU_PRICES).unwrap().cols.iter().map(|c| c.name).collect();
    assert_eq!(names, ["dish", "price", "tax_ppm", "mods_ptr", "mods_col", "mods_val"]);
    // The slice-by-8 crc IS bebop_store::crc32, on every fixture and on seeded noise of every length.
    for (_, b) in encoded() {
        assert_eq!(crc32(&b), bebop_store::crc32(&b));
    }
    let mut s = SEED;
    let noise: Vec<u8> = (0..700).map(|_| lcg(&mut s) as u8).collect();
    for n in 0..noise.len() {
        assert_eq!(crc32(&noise[..n]), bebop_store::crc32(&noise[..n]), "len {n}");
    }
}

#[test]
fn bom_of_block_equals_json() {
    let (m, b, n) = (fixture("menu_prices"), fixture("bom"), fixture("names"));
    let cat = Catalogue::new(&m, &b, &n).unwrap();
    for (id, json) in catalogue(SEED) {
        let from_json = crate::stock::bom_of(&json);
        assert!(!from_json.is_empty());
        assert_eq!(cat.bom_of(&id), Some(from_json.clone()), "{id}");
        assert_eq!(crate::stock::bom_of_product(Some(&cat), &id, &json), from_json, "{id} through stock");
        assert_eq!(crate::stock::bom_of_product(None, &id, &json), from_json, "{id} with no block");
    }
    assert_eq!(cat.bom_of("not-a-dish"), None, "an unknown id is not an empty recipe");
    let late = r#"{"id":"late","bom":[{"supply":"rice","qty":9}]}"#;
    assert_eq!(crate::stock::bom_of_product(Some(&cat), "late", late), crate::stock::bom_of(late), "not in the block -> JSON");
}

mod measure;
