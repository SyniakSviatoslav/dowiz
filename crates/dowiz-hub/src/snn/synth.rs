//! TEST ONLY. The A/B's guests, by W-SENSE's method (`store/sense.test.mjs:125-153`, ported in
//! R-S7 `s7-lab/lab/src/e3.rs`) plus the venue shape the trainer also uses (categories with
//! signature axes, tags and ingredients that follow the category). Seeded LCG (the E3 one); the
//! trainer draws from numpy's generator, so no guest here was ever trained on (the held-out split).

use serde_json::{json, Value};

use super::AXES;
use crate::sense;

pub struct Lcg(pub u64);
impl Lcg {
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    pub fn range(&mut self, n: u64) -> i64 {
        (self.next() % n) as i64
    }
    pub fn pm(&mut self) -> i64 {
        self.range(1000)
    }
}

/// Two taste axes 1..5 and three texture/aroma axes 1..3, per mille (E3 `random_sense`).
pub fn random_sense(r: &mut Lcg) -> [i64; AXES] {
    let mut v = [0i64; AXES];
    let a = r.range(6) as usize;
    let mut b = r.range(6) as usize;
    while b == a {
        b = r.range(6) as usize;
    }
    v[a] = (1 + r.range(5)) * 1000 / 5;
    v[b] = (1 + r.range(5)) * 1000 / 5;
    let mut tags: Vec<usize> = Vec::new();
    while tags.len() < 3 {
        let t = 6 + r.range(21) as usize;
        if !tags.contains(&t) {
            tags.push(t);
        }
    }
    for t in tags {
        v[t] = (1 + r.range(3)) * 1000 / 3;
    }
    v
}

/// Float-free cosine for the generator's "nearest" (an integer ratio compare is exact).
fn near_key(a: &[i64; AXES], b: &[i64; AXES]) -> (i128, i128) {
    let (mut dot, mut nb) = (0i128, 0i128);
    for k in 0..AXES {
        dot += i128::from(a[k]) * i128::from(b[k]);
        nb += i128::from(b[k]) * i128::from(b[k]);
    }
    (dot, nb)
}

/// Is cos(a, x) > cos(a, y)? (|a| cancels; compares dot_x/|x| with dot_y/|y| exactly, by squares.)
fn closer(a: &[i64; AXES], x: &[i64; AXES], y: &[i64; AXES]) -> std::cmp::Ordering {
    let ((dx, nx), (dy, ny)) = (near_key(a, x), near_key(a, y));
    let lhs = dx.signum() * dx * dx * ny;
    let rhs = dy.signum() * dy * dy * nx;
    rhs.cmp(&lhs)
}

pub struct Shape {
    pub proto: Vec<Vec<(usize, i64)>>,
    pub ctags: Vec<[usize; 2]>,
    pub cing: Vec<Vec<usize>>,
}

pub struct DishRow {
    pub v: [i64; AXES],
    pub cat: Option<usize>,
    pub tags: Vec<usize>,
    pub ingr: Vec<usize>,
}

fn distinct(r: &mut Lcg, n: u64, k: usize) -> Vec<usize> {
    let mut out = Vec::new();
    while out.len() < k {
        let x = r.range(n) as usize;
        if !out.contains(&x) {
            out.push(x);
        }
    }
    out
}

pub fn shape(r: &mut Lcg) -> Shape {
    let mut s = Shape { proto: Vec::new(), ctags: Vec::new(), cing: Vec::new() };
    for _ in 0..16 {
        let ta = r.range(6) as usize;
        let tg = distinct(r, 21, 2);
        s.proto.push(vec![(ta, (1 + r.range(5)) * 1000 / 5), (6 + tg[0], (1 + r.range(3)) * 1000 / 3), (6 + tg[1], (1 + r.range(3)) * 1000 / 3)]);
        let t = distinct(r, 12, 2);
        s.ctags.push([t[0], t[1]]);
        s.cing.push(distinct(r, 40, 6));
    }
    s
}

pub fn shaped_dish(r: &mut Lcg, s: &Shape, mut v: [i64; AXES], c: usize) -> DishRow {
    for &(ax, lvl) in &s.proto[c] {
        if r.pm() < 700 {
            v[ax] = lvl;
        }
    }
    let mut tags: Vec<usize> = s.ctags[c].iter().copied().filter(|_| r.pm() < 700).collect();
    if r.pm() < 300 {
        tags.push(r.range(12) as usize);
    }
    let mut ingr: Vec<usize> = (0..1 + r.range(3)).map(|_| s.cing[c][r.range(6) as usize]).collect();
    if r.pm() < 500 {
        ingr.push(r.range(40) as usize);
    }
    tags.sort_unstable();
    tags.dedup();
    ingr.sort_unstable();
    ingr.dedup();
    DishRow { v, cat: Some(c), tags, ingr }
}

/// The category whose prototype is nearest the vector (ties: the lower index).
pub fn cat_of(s: &Shape, v: &[i64; AXES]) -> usize {
    let protos: Vec<[i64; AXES]> = s.proto.iter().map(|p| {
        let mut x = [0i64; AXES];
        for &(a, l) in p {
            x[a] = l;
        }
        x
    }).collect();
    (0..protos.len()).min_by(|&a, &b| closer(v, &protos[a], &protos[b]).then(a.cmp(&b))).unwrap_or(0)
}

/// One guest: the dishes ordered (index, qty) and the held-out new dish.
pub struct Case {
    pub ordered: Vec<(usize, i64)>,
    pub new: DishRow,
}

/// W-SENSE's guest: a latent, the 4 nearest dishes ordered (qty 2,1,1,1), the latent itself is
/// the new dish; `noise` 1 = every axis of the new dish one level off and orders 3 and 4 random
/// (E3's hard variant); `m` keeps the first `m` orders (1 = a cold guest).
pub fn case(r: &mut Lcg, menu: &[DishRow], shape: Option<&Shape>, noise: u8, m: usize) -> Case {
    let n = menu.len();
    let lat = random_sense(r);
    let mut near: Vec<usize> = (0..n).collect();
    near.sort_by(|&a, &b| closer(&lat, &menu[a].v, &menu[b].v).then(a.cmp(&b)));
    let mut ordered: Vec<(usize, i64)> = near.iter().take(4).enumerate().map(|(i, &d)| (d, if i == 0 { 2 } else { 1 })).collect();
    let mut nv = lat;
    if noise == 1 {
        for (k, x) in nv.iter_mut().enumerate() {
            if *x > 0 {
                let step = if k < 6 { 200 } else { 333 };
                *x = (*x + if r.range(2) == 0 { -step } else { step }).clamp(step, 1000);
            }
        }
        for slot in [2usize, 3] {
            let mut d = r.range(n as u64) as usize;
            while ordered.iter().any(|o| o.0 == d) {
                d = r.range(n as u64) as usize;
            }
            ordered[slot] = (d, 1);
        }
    }
    ordered.truncate(m);
    let new = match shape {
        Some(s) => {
            let c = cat_of(s, &nv);
            shaped_dish(r, s, nv, c)
        }
        None => DishRow { v: nv, cat: None, tags: Vec::new(), ingr: Vec::new() },
    };
    Case { ordered, new }
}

pub fn menu(r: &mut Lcg, n: usize, shape: Option<&Shape>) -> Vec<DishRow> {
    (0..n)
        .map(|_| {
            let v = random_sense(r);
            match shape {
                Some(s) => {
                    let c = r.range(16) as usize;
                    shaped_dish(r, s, v, c)
                }
                None => DishRow { v, cat: None, tags: Vec::new(), ingr: Vec::new() },
            }
        })
        .collect()
}

/// The dish as a product the real readers take (`sense::of_product`, `strip`, `top_k_json`).
pub fn product(id: &str, d: &DishRow) -> Value {
    let mut s = json!({ "v": 1, "taste": {}, "texture": {}, "aroma": {} });
    for (k, x) in d.v.iter().enumerate().filter(|(_, x)| **x > 0) {
        let (dim, word, lvl) = if k < 6 {
            ("taste", sense::TASTE[k], (x * 5 + 500) / 1000)
        } else if k < 15 {
            ("texture", sense::TEXTURE[k - 6], (x * 3 + 500) / 1000)
        } else {
            ("aroma", sense::AROMA[k - 15], (x * 3 + 500) / 1000)
        };
        s[dim][word] = json!(lvl);
    }
    let mut p = json!({ "id": id, "name": id, "price": 500, "sense": s });
    if let Some(c) = d.cat {
        p["categoryId"] = json!(format!("c{c}"));
    }
    if !d.tags.is_empty() {
        p["tags"] = json!(d.tags.iter().map(|t| format!("t{t}")).collect::<Vec<_>>());
    }
    if !d.ingr.is_empty() {
        p["ingredients"] = json!(d.ingr.iter().map(|t| format!("i{t}")).collect::<Vec<_>>());
    }
    p
}
