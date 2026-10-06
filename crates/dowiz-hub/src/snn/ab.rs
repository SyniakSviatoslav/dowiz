//! THE A/B (W-SNN row 2), named and reproducible:
//!   cargo test --release --lib snn::ab::snn_ab_harness -- --ignored --nocapture
//! SNN vs the current integer rankers (the phone's strip, `rank::strip`; the server's cosine,
//! `block::taste::top_k_json`) vs R-S7 E3's sheaf diffusion (Q16) vs plain PPR (E3's port), on
//! W-SENSE's 300 guests, noisy variants, cold guests and a venue-shaped menu -- every guest drawn
//! from a seed the trainer never saw. A score is the held-out new dish's place among the dishes
//! the guest has not ordered. Reported as measured, a loss included.

use std::collections::BTreeMap;
use std::time::Instant;

use serde_json::{json, Value};

use super::synth::{self, Case, DishRow, Lcg, Shape};
use super::{blob, rank, Guest, Menu, Model, AXES};
use crate::rank::strip;

pub const DAY: i64 = 20_000;
const NDCG_PM: [i64; 5] = [1000, 631, 500, 431, 387];

/// The world one guest sees: the products (menu + the new dish last) and the ordered dishes.
pub struct World {
    pub products: Vec<Value>,
    pub pairs: Vec<(String, String)>,
    pub rows: Vec<[i64; AXES]>,
    pub ordered: Vec<(usize, i64)>,
    pub new: usize,
}

fn id(i: usize, n: usize) -> String {
    if i == n { "new".into() } else { format!("d{i:03}") }
}

pub fn world(menu: &[DishRow], c: Case) -> World {
    let n = menu.len();
    let all: Vec<&DishRow> = menu.iter().chain(std::iter::once(&c.new)).collect();
    let products: Vec<Value> = all.iter().enumerate().map(|(i, d)| synth::product(&id(i, n), d)).collect();
    let pairs = products.iter().map(|p| (p["id"].as_str().unwrap_or("").to_string(), p.to_string())).collect();
    World { products, pairs, rows: all.iter().map(|d| d.v).collect(), ordered: c.ordered, new: n }
}

/// The device profile the phone would hold for these orders (all on day `DAY`).
pub fn profile(w: &World) -> Value {
    let dishes: serde_json::Map<String, Value> = w.ordered.iter().map(|(d, q)| (id(*d, w.new), json!({ "order": [[DAY, q]] }))).collect();
    json!({ "v": 1, "dishes": dishes, "cats": {} })
}

/// The phone's strip score of every dish (`strip::strip`'s `total`, prior/ctx/mood absent).
pub fn strip_scores(w: &World) -> Vec<(String, i64)> {
    let prof = profile(w);
    let on: Vec<&Value> = w.products.iter().collect();
    let wt = strip::weights(&prof, &on, DAY);
    let guest = crate::rank::per_mille(&strip::sense_vec(&prof, &on, DAY, None));
    let none = BTreeMap::new();
    w.products
        .iter()
        .map(|p| {
            let pid = p["id"].as_str().unwrap_or("").to_string();
            let mut s = wt.dish.get(&pid).copied().unwrap_or(0);
            for t in p.get("tags").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str) {
                s += wt.tag.get(t).copied().unwrap_or(0);
            }
            s += p.get("categoryId").and_then(Value::as_str).and_then(|c| wt.cat.get(c)).copied().unwrap_or(0) / 2;
            s += strip::sense_score(&strip::vector_of(p), &guest, &none, &none);
            (pid, s)
        })
        .collect()
}

/// The server profile's maps for these orders: sense = sum qty x vector, cats and tags = sum qty.
pub fn server_maps(w: &World) -> (BTreeMap<String, i64>, BTreeMap<String, i64>, BTreeMap<String, i64>, BTreeMap<String, i64>) {
    let (mut sense, mut cats, mut tags, mut dishes) = (BTreeMap::new(), BTreeMap::new(), BTreeMap::new(), BTreeMap::new());
    for &(d, q) in &w.ordered {
        let p = &w.products[d];
        for (k, x) in strip::vector_of(p) {
            *sense.entry(k).or_insert(0) += q * x;
        }
        if let Some(c) = p.get("categoryId").and_then(Value::as_str) {
            *cats.entry(c.to_string()).or_insert(0) += q * 1000;
        }
        for t in p.get("tags").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str) {
            *tags.entry(t.to_string()).or_insert(0) += q * 1000;
        }
        dishes.insert(id(d, w.new), q * 1000);
    }
    (sense, cats, tags, dishes)
}

fn ppr_lift(w: &World) -> Vec<(String, i64)> {
    const SCALE: i64 = 1_000_000;
    let n = w.rows.len();
    let nn = n + AXES;
    let edges: Vec<(usize, usize)> = (0..n).flat_map(|d| (0..AXES).filter(move |&k| w.rows[d][k] > 0).map(move |k| (d, n + k))).collect();
    let mut deg = vec![0i64; nn];
    for &(a, b) in &edges {
        deg[a] += 1;
        deg[b] += 1;
    }
    let mut restart = vec![0i64; nn];
    for &(s, _) in &w.ordered {
        restart[s] += SCALE / w.ordered.len() as i64;
    }
    let mut rank_v = restart.clone();
    for _ in 0..12 {
        let each: Vec<i64> = (0..nn).map(|i| if deg[i] == 0 { 0 } else { rank_v[i] * 85 / 100 / deg[i] }).collect();
        let mut next = vec![0i64; nn];
        for &(a, b) in &edges {
            next[b] += each[a];
            next[a] += each[b];
        }
        for j in 0..nn {
            next[j] += (SCALE - SCALE * 85 / 100) * restart[j] / SCALE;
        }
        rank_v = next;
    }
    (0..n).map(|i| (id(i, w.new), rank_v[i] / deg[i].max(1))).collect()
}

fn sheaf_diffusion(w: &World) -> Vec<(String, i64)> {
    let n = w.rows.len();
    let mut edges: Vec<(usize, usize, [bool; AXES])> = Vec::new();
    for a in 0..n {
        for b in a + 1..n {
            let m: [bool; AXES] = std::array::from_fn(|k| w.rows[a][k] > 0 && w.rows[b][k] > 0);
            if m.iter().any(|x| *x) {
                edges.push((a, b, m));
            }
        }
    }
    let mut deg = vec![0i64; n];
    for &(a, b, _) in &edges {
        deg[a] += 1;
        deg[b] += 1;
    }
    let maxdeg = i128::from(*deg.iter().max().unwrap_or(&1).max(&1));
    let mut x = vec![[0i64; AXES]; n];
    for &(d, q) in &w.ordered {
        for k in 0..AXES {
            x[d][k] = q * w.rows[d][k] << 16;
        }
    }
    for _ in 0..6 {
        let mut lap = vec![[0i128; AXES]; n];
        for &(a, b, m) in &edges {
            for k in (0..AXES).filter(|&k| m[k]) {
                let d = i128::from(x[a][k]) - i128::from(x[b][k]);
                lap[a][k] += d;
                lap[b][k] -= d;
            }
        }
        for i in 0..n {
            for k in 0..AXES {
                x[i][k] = (i128::from(x[i][k]) - lap[i][k] / (2 * maxdeg)) as i64;
            }
        }
    }
    (0..n)
        .map(|c| {
            let (mut dot, mut nn) = (0i128, 0i128);
            for k in 0..AXES {
                dot += i128::from(x[c][k] >> 16) * i128::from(w.rows[c][k]);
                nn += i128::from(w.rows[c][k]) * i128::from(w.rows[c][k]);
            }
            (id(c, w.new), if nn > 0 { (dot * 1000 / super::infer::isqrt(nn)) as i64 } else { 0 })
        })
        .collect()
}

/// The 0-based place of `target` among the guest's candidates (score > 0, best first, ties by id).
pub fn place(mut s: Vec<(String, i64)>, w: &World, target: &str) -> Option<usize> {
    let ordered: Vec<String> = w.ordered.iter().map(|o| id(o.0, w.new)).collect();
    s.retain(|x| x.1 > 0 && !ordered.contains(&x.0));
    s.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.as_bytes().cmp(b.0.as_bytes())));
    s.iter().position(|x| x.0 == target)
}

#[derive(Default, Clone)]
pub struct Score {
    pub top3: i64,
    pub hit5: i64,
    pub ndcg_pm: i64,
    pub ns: u128,
}

pub const METHODS: [&str; 7] = ["strip(phone,current)", "cosine(server,current)", "snn(phone form)", "snn(server form)", "snn_pure(no residual)", "sheaf_diffusion_q16", "ppr"];

fn snn_list(m: &Model, w: &World, device: bool) -> Vec<(String, i64)> {
    let menu = Menu::from_products(&w.pairs, 0);
    let (sense, cats, tags, dishes) = server_maps(w);
    let none = BTreeMap::new();
    let g = Guest::from_maps(&menu, &sense, &cats, &tags, if device { &dishes } else { &none });
    rank(m, &menu, &g, menu.dishes.len(), &[])
}

pub fn method(i: usize, w: &World, full: &Model, pure: &Model) -> Vec<(String, i64)> {
    match i {
        0 => strip_scores(w),
        1 => crate::block::taste::top_k_json(&w.pairs, &server_maps(w).0, 0, w.pairs.len()),
        2 => snn_list(full, w, true),
        3 => snn_list(full, w, false),
        4 => snn_list(pure, w, true),
        5 => sheaf_diffusion(w),
        _ => ppr_lift(w),
    }
}

/// One dataset: (name, menu size, venue-shaped, noise, orders per guest).
pub const SETS: [(&str, usize, bool, u8, usize); 8] = [
    ("wsense-40", 40, false, 0, 4),
    ("wsense-165", 165, false, 0, 4),
    ("noisy-40", 40, false, 1, 4),
    ("noisy-165", 165, false, 1, 4),
    ("cold-165 (1 order)", 165, false, 0, 1),
    ("venue-165", 165, true, 0, 4),
    ("venue-165-noisy", 165, true, 1, 4),
    ("venue-165-cold", 165, true, 0, 1),
];

pub fn run_set(set: usize, guests: usize, full: &Model, pure: &Model) -> Vec<Score> {
    let (_, n, shaped, noise, m) = SETS[set];
    let mut r = Lcg(20_261_006 + set as u64 * 7919);
    let shape: Option<Shape> = shaped.then(|| synth::shape(&mut r));
    let menu = synth::menu(&mut r, n, shape.as_ref());
    let mut out = vec![Score::default(); METHODS.len()];
    for _ in 0..guests {
        let c = synth::case(&mut r, &menu, shape.as_ref(), noise, m);
        let w = world(&menu, c);
        for (i, o) in out.iter_mut().enumerate() {
            let t = Instant::now();
            let list = method(i, &w, full, pure);
            o.ns += t.elapsed().as_nanos();
            if let Some(p) = place(list, &w, "new") {
                o.top3 += i64::from(p < 3);
                o.hit5 += i64::from(p < 5);
                o.ndcg_pm += NDCG_PM.get(p).copied().unwrap_or(0);
            }
        }
    }
    out
}

pub fn models() -> (Model, Model) {
    let pure = std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/snn/pure.snn")).expect("fixtures/snn/pure.snn");
    (super::shipped().expect("the shipped weights decode"), blob::decode(&pure).expect("pure.snn decodes"))
}

#[test]
#[ignore]
fn snn_ab_harness() {
    let (full, pure) = models();
    println!("AB | set | method | top3/300 | hit5/300 | NDCG@5 (sum, x1000) | us/guest");
    for set in 0..SETS.len() {
        for (i, s) in run_set(set, 300, &full, &pure).iter().enumerate() {
            println!("AB | {} | {} | {} | {} | {} | {}", SETS[set].0, METHODS[i], s.top3, s.hit5, s.ndcg_pm, s.ns / 300 / 1000);
        }
    }
}

/// The harness's strip column IS the phone's strip: on 20 guests per set, `strip::strip`'s "taste"
/// items come out in the order `strip_scores` ranks them (the A/B does not re-implement the rule).
#[test]
fn the_harness_strip_column_is_the_phones_strip() {
    let mut checked = 0;
    for set in [0usize, 5] {
        let (_, n, shaped, noise, m) = SETS[set];
        let mut r = Lcg(99 + set as u64);
        let shape = shaped.then(|| synth::shape(&mut r));
        let menu = synth::menu(&mut r, n, shape.as_ref());
        for _ in 0..20 {
            let w = world(&menu, synth::case(&mut r, &menu, shape.as_ref(), noise, m));
            let got: Vec<String> = strip::strip(&w.products, &profile(&w), DAY, &strip::Opts::default()).into_iter().filter(|x| x.why == "taste").map(|x| x.id).collect();
            let mut mine = strip_scores(&w);
            let again: Vec<String> = strip::strip(&w.products, &profile(&w), DAY, &strip::Opts::default()).into_iter().filter(|x| x.why == "again").map(|x| x.id).collect();
            mine.retain(|x| x.1 > 0 && !again.contains(&x.0));
            mine.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.as_bytes().cmp(b.0.as_bytes())));
            let mine: Vec<String> = mine.into_iter().map(|x| x.0).take(got.len()).collect();
            assert_eq!(got, mine, "set {set}");
            checked += 1;
        }
    }
    assert_eq!(checked, 40);
}
