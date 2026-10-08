//! B2: the catalogue edit journal's per-write cost in the object (workers/api/src/hubdo/journal.rs
//! `journal_append` + `write_image`), on a 165-dish catalogue with a ~590-record journal.
//!
//! CURRENT (what the object does per catalogue write, CPU phases only):
//!   p1 `image(img)`            clone of the journal bytes out of `mem` (hubdo.rs:237)
//!   p2 `Catalog::load`+`state_of` on the held bytes (`before`)       [decode = W-KVDEC's, context only]
//!   p3 `Catalog::load`+`state_of` on the PUT bytes (`after`)         [decode = W-KVDEC's, context only]
//!   p4 `LogImage::load(&bytes)` = `Store::from_bytes` + `chain_scan` (crc of every record)
//!   p5 `edits::journal`      diff(before, after) + one `append`
//!   p6 `log.to_bytes()`      `to_bytes_trimmed` copy
//!   p7 `changed_chunks(old, new, 96 KiB)`
//! REPLACEMENT A (class C): the `LogImage` stays resident per generation, as `cat_checked` keeps the
//!   catalogue's crc: p5 + p6 + p7 only (no clone, no from_bytes, no chain_scan).
//! REPLACEMENT B (class C): A + the previous write's `after` State kept as this write's `before`:
//!   drops p2 (one decode + one state_of).
//! Equivalence: the journal bytes and the changed-chunk list after the write are byte-identical
//! between CURRENT and REPLACEMENT A/B (asserted below before any timing).
use dowiz_hub::catalog::edits::{self, State};
use dowiz_hub::catalog::Catalog;
use dowiz_hub::logimage::LogImage;
use serde_json::{json, Value};

const DISHES: usize = 165;
const CATEGORIES: usize = 12;
const CHUNK: usize = 96 * 1024;

fn product(n: usize, price: i64) -> Value {
    json!({
        "name": format!("Roll {n} me salmon dhe avokado"),
        "description": format!("Oriz sushi, salmon i freskët, avokado, krem djathi, susam i pjekur dhe salcë e shtëpisë. Pjata {n}, e përgatitur me dorë çdo ditë."),
        "categoryId": format!("c_{}", n % CATEGORIES), "price": price, "sortOrder": n as i64,
        "available": n % 17 != 0, "unavailableNote": if n % 17 == 0 { json!("Mbaroi sot") } else { Value::Null },
        "imageUrl": format!("/media/sushi/p{n}.jpg"), "imageUrlSmall": format!("/media/sushi/p{n}-s.jpg"),
        "allergens": if n % 5 == 0 { Value::Null } else { json!(["fish", "sesame", "milk"]) },
        "modifierGroups": [{"id": "g_size", "name": "Madhësia", "min": 1, "max": 1,
            "options": [{"id": "o_8", "name": "8 copë", "price": 0}, {"id": "o_12", "name": "12 copë", "price": 350}]}],
        "sizeCm": 22, "cookingMin": 8 + (n % 9) as i64, "tags": ["salmon", "cold"],
        "ingredients": ["oriz", "salmon", "avokado", "krem djathi", "susam"], "weightG": 240 + (n % 60) as i64,
        "nutrition": {"kcal": 420, "protein_g": 18, "fat_g": 14, "carbs_g": 52},
        "nutritionDerived": {"kcal": 415}, "taste": {"salty": 2, "sweet": 1, "spicy": 0}, "station": "sushi",
        "calories": 420,
    })
}

fn born() -> Vec<u8> {
    let mut c = Catalog::create().unwrap();
    c.set_location(&json!({"id": "loc_sushi", "name": "Sushi Durrës", "slug": "sushi-durres", "tz": "Europe/Tirane", "menu_version": 41}).to_string());
    for k in 0..CATEGORIES {
        c.set_category(&format!("c_{k}"), &json!({"name": format!("Kategoria {k}"), "sortOrder": k}).to_string());
    }
    for n in 0..DISHES {
        c.set_product(&format!("p_{n}"), &product(n, 700 + 10 * n as i64).to_string());
    }
    c.to_bytes().unwrap()
}

/// The Worker's edit: load, change one dish's price, `to_bytes` (W-DELTA append path).
fn edit(bytes: &[u8], n: usize, price: i64) -> Vec<u8> {
    let mut c = Catalog::load(bytes).unwrap();
    c.set_product(&format!("p_{n}"), &product(n, price).to_string());
    c.to_bytes().unwrap()
}

pub(crate) fn changed_chunks(old: Option<&[u8]>, new: &[u8], chunk: usize) -> Vec<usize> {
    let chunks = new.len().div_ceil(chunk).max(1);
    (0..chunks)
        .filter(|&n| {
            let at = n * chunk;
            let end = (at + chunk).min(new.len());
            match old {
                Some(o) if (at + chunk).min(o.len()) == end => o[at..end] != new[at..end],
                _ => true,
            }
        })
        .collect()
}

struct Phases {
    p1_clone: u128,
    p2_before: u128,
    p3_after: u128,
    p4_load: u128,
    p5_journal: u128,
    p6_to_bytes: u128,
    p7_chunks: u128,
}

/// CURRENT: `journal_append` + the CPU part of `write_image`, phase by phase. Returns the new
/// journal bytes, the changed chunks and the phase times.
fn current(held_cat: &[u8], put_cat: &[u8], journal_bytes: &[u8], gens: (i64, i64), at: i64) -> (Vec<u8>, Vec<usize>, Phases) {
    let t = std::time::Instant::now();
    let jb: Vec<u8> = journal_bytes.to_vec(); // p1: `image()` -> `hit.clone()`
    let p1_clone = t.elapsed().as_nanos();
    let t = std::time::Instant::now();
    let before: State = edits::state_of(&Catalog::load(held_cat).unwrap());
    let p2_before = t.elapsed().as_nanos();
    let t = std::time::Instant::now();
    let after: State = edits::state_of(&Catalog::load(put_cat).unwrap());
    let p3_after = t.elapsed().as_nanos();
    let t = std::time::Instant::now();
    let mut log = LogImage::load(&jb).unwrap();
    let p4_load = t.elapsed().as_nanos();
    let t = std::time::Instant::now();
    let j = edits::journal(&mut log, &before, &after, at, "staff-1", gens).unwrap();
    assert_eq!((j.edits, j.unseen, j.baseline, j.compacted), (1, 0, 0, false), "one in-step edit");
    let p5_journal = t.elapsed().as_nanos();
    let t = std::time::Instant::now();
    let new = log.to_bytes();
    let p6_to_bytes = t.elapsed().as_nanos();
    let t = std::time::Instant::now();
    let changed = changed_chunks(Some(journal_bytes), &new, CHUNK);
    let p7_chunks = t.elapsed().as_nanos();
    (new, changed, Phases { p1_clone, p2_before, p3_after, p4_load, p5_journal, p6_to_bytes, p7_chunks })
}

/// REPLACEMENT A: the resident `LogImage` (loaded once per generation) takes the append.
fn replacement_a(log: &mut LogImage, before: &State, put_cat: &[u8], journal_bytes: &[u8], gens: (i64, i64), at: i64) -> (Vec<u8>, Vec<usize>, State, (u128, u128, u128, u128)) {
    let t = std::time::Instant::now();
    let after: State = edits::state_of(&Catalog::load(put_cat).unwrap());
    let p3 = t.elapsed().as_nanos();
    let t = std::time::Instant::now();
    let j = edits::journal(log, before, &after, at, "staff-1", gens).unwrap();
    assert_eq!((j.edits, j.unseen, j.baseline, j.compacted), (1, 0, 0, false));
    let p5 = t.elapsed().as_nanos();
    let t = std::time::Instant::now();
    let new = log.to_bytes();
    let p6 = t.elapsed().as_nanos();
    let t = std::time::Instant::now();
    let changed = changed_chunks(Some(journal_bytes), &new, CHUNK);
    let p7 = t.elapsed().as_nanos();
    (new, changed, after, (p3, p5, p6, p7))
}

fn us(n: u128) -> f64 {
    n as f64 / 1e3
}

pub fn run(reps: usize) {
    for &records in &[100usize, 590] {
        // ── the fixture: a 165-dish catalogue and a journal of `records` price edits ──
        let mut cat = born();
        let mut log = LogImage::create().unwrap();
        let mut gen: i64 = 1;
        for i in 0..records {
            let n = i % DISHES;
            let next = edit(&cat, n, 700 + 10 * n as i64 + 5 * (i as i64 / DISHES as i64 + 1));
            let before = edits::state_of(&Catalog::load(&cat).unwrap());
            let after = edits::state_of(&Catalog::load(&next).unwrap());
            edits::journal(&mut log, &before, &after, 1_000 * (i as i64 + 1), "staff-1", (gen, gen + 1)).unwrap();
            gen += 1;
            cat = next;
        }
        let jbytes = log.to_bytes();
        let chunks = jbytes.len().div_ceil(CHUNK);
        println!("B2 fixture: {} dishes, catalogue {} B, journal {} records = {} B = {chunks} chunks of 96 KiB", DISHES, cat.len(), log.len(), jbytes.len());
        let put = edit(&cat, 7, 9_999);
        let at = 1_000 * (records as i64 + 1);

        // ── equivalence ──
        let (cur_bytes, cur_changed, _) = current(&cat, &put, &jbytes, (gen, gen + 1), at);
        let mut resident = LogImage::load(&jbytes).unwrap();
        let before = edits::state_of(&Catalog::load(&cat).unwrap());
        let (a_bytes, a_changed, _, _) = replacement_a(&mut resident, &before, &put, &jbytes, (gen, gen + 1), at);
        assert_eq!(cur_bytes, a_bytes, "REPLACEMENT A must produce the same journal bytes");
        assert_eq!(cur_changed, a_changed, "REPLACEMENT A must store the same chunks");
        // B: the `before` kept from the previous write equals the `before` decoded now.
        let kept_before: State = {
            // the previous write's `after` is the state of `cat` (the bytes it produced)
            let prev_after = edits::state_of(&Catalog::load(&cat).unwrap());
            prev_after
        };
        assert_eq!(kept_before, before, "the kept `after` equals the decoded `before`");
        println!("B2 EQUIV ok (n={records}): bytes {} B identical, changed chunks {:?} identical", cur_bytes.len(), cur_changed);

        // ── timing: CURRENT, phase medians ──
        let mut ph: Vec<Phases> = Vec::new();
        for _ in 0..reps {
            let (_, _, p) = current(&cat, &put, &jbytes, (gen, gen + 1), at);
            ph.push(p);
        }
        let med = |f: &dyn Fn(&Phases) -> u128| {
            let mut v: Vec<u128> = ph.iter().map(f).collect();
            v.sort();
            v[v.len() / 2]
        };
        let (p1, p2, p3, p4, p5, p6, p7) = (
            med(&|p| p.p1_clone), med(&|p| p.p2_before), med(&|p| p.p3_after), med(&|p| p.p4_load),
            med(&|p| p.p5_journal), med(&|p| p.p6_to_bytes), med(&|p| p.p7_chunks),
        );
        let total = p1 + p2 + p3 + p4 + p5 + p6 + p7;
        println!("B2 CURRENT n={records} medians us: p1 clone {:.0} | p2 before(decode+state_of) {:.0} | p3 after(decode+state_of) {:.0} | p4 LogImage::load {:.0} | p5 journal {:.0} | p6 to_bytes {:.0} | p7 changed_chunks {:.0} | TOTAL {:.0}",
            us(p1), us(p2), us(p3), us(p4), us(p5), us(p6), us(p7), us(total));
        let own = p1 + p4 + p5 + p6 + p7; // without the two decodes (W-KVDEC's)
        println!("B2 CURRENT n={records} journal-only (p1+p4+p5+p6+p7) = {:.0} us; with decodes = {:.0} us", us(own), us(total));

        // ── timing: REPLACEMENT A (resident log; each rep reloads a fresh resident copy OUTSIDE the timed region) ──
        let mut ta: Vec<(u128, u128, u128, u128)> = Vec::new();
        for _ in 0..reps {
            let mut resident = LogImage::load(&jbytes).unwrap();
            let (_, _, _, t) = replacement_a(&mut resident, &before, &put, &jbytes, (gen, gen + 1), at);
            ta.push(t);
        }
        let medt = |f: &dyn Fn(&(u128, u128, u128, u128)) -> u128| {
            let mut v: Vec<u128> = ta.iter().map(f).collect();
            v.sort();
            v[v.len() / 2]
        };
        let (a3, a5, a6, a7) = (medt(&|t| t.0), medt(&|t| t.1), medt(&|t| t.2), medt(&|t| t.3));
        println!("B2 REPLACE-A n={records} medians us: p3 after {:.0} | p5 journal {:.0} | p6 to_bytes {:.0} | p7 changed_chunks {:.0} | journal-only (p5+p6+p7) {:.0} | with one decode {:.0}",
            us(a3), us(a5), us(a6), us(a7), us(a5 + a6 + a7), us(a3 + a5 + a6 + a7));
        println!("B2 REPLACE-B n={records} (A + kept `before`, drops p2): with one decode = {:.0} us vs CURRENT {:.0} us", us(a3 + a5 + a6 + a7), us(total));
        println!("B2 RATIO n={records}: journal-only {:.1}x ({:.0} -> {:.0} us); whole write path {:.1}x ({:.0} -> {:.0} us); share of a 10 ms cap: {:.1}% -> {:.1}% (object-side, 30 s DO cap applies)",
            us(own) / us(a5 + a6 + a7).max(0.001), us(own), us(a5 + a6 + a7),
            us(total) / us(a3 + a5 + a6 + a7).max(0.001), us(total), us(a3 + a5 + a6 + a7),
            us(total) / 100.0, us(a3 + a5 + a6 + a7) / 100.0);
    }
}
