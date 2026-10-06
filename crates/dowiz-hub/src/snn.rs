//! PURE. A SHEAF NEURAL NETWORK THAT RANKS DISHES FOR A GUEST, IN INTEGERS, IN SHADOW (W-SNN).
//!
//! OPERATOR 2026-10-06: "потрібні нейромережі пучків уже зараз" -- although R-S7 E3 measured no gain
//! for sheaf diffusion on synthetic data. So it is built to be able ONLY to help: trained offline
//! (floats live in the trainer, never here), inferred in Q16 integers, checked against a second
//! implementation to the integer, and wired in SHADOW -- computed beside the current ranker, its
//! agreement counted per venue, never shown -- until the A/B (`snn/ab.rs`) says it wins.
//!
//! THE GRAPH (a cellular sheaf with LEARNED restriction maps, Neural Sheaf Diffusion 2202.04579 /
//! Sheaf4Rec 2304.09097 shape, kept small): dishes; their 27 sense axes (`sense::all_keys` order);
//! three kinds of group a dish belongs to -- its category, its tags, its ingredients (the product's
//! `ingredients` list). Stalks are D = 8 integers. Restriction maps are DIAGONAL and learned per
//! group kind (`rho`); the axis -> stalk map (`Win`) and two diffusion layers are learned. A guest is
//! not a node anyone stores: their stalk is the transport of what the profile ALREADY holds (the
//! taste vector, category and tag weights, and on the device the dish weights) -- no new
//! per-person feature exists because of this module.
//!
//! THE SCORE keeps a residual: a per-axis-weighted cosine of guest and dish (the current ranker's
//! own signal, re-weighted by `lam`) blended by a learned `beta` with the cosine of the diffused
//! stalks. The A/B reports the model WITHOUT that residual too (`fixtures/snn/pure.snn`): alone, the
//! 8-dim stalk loses badly to the 27-dim cosine, and that is printed, not hidden.
//!
//! NEVER PRICE, NEVER ELIGIBILITY: the answer is an ORDER of dishes that are on sale and that the
//! guest's avoided allergens do not meet -- the same set the current ranker draws from
//! (`block::taste::top_k_json`). An objection is honoured by the caller before anything here runs.

use std::collections::BTreeMap;

use serde_json::Value;

pub mod blob;
pub mod infer;
pub mod quality;
pub mod shadow;

pub use blob::{Model, Refusal};

/// Q16: 1.0.
pub const Q: i64 = 1 << 16;
/// Sense axes: 6 taste, 9 texture, 12 aroma (`sense::all_keys`).
pub const AXES: usize = 27;
/// Group kinds: category, tag, ingredient.
pub const GROUP_TYPES: usize = 3;
pub const CATEGORY: u8 = 0;
pub const TAG: u8 = 1;
pub const INGREDIENT: u8 = 2;

/// The shipped weights (trained 2026-10-06, `snn-train/train.py`, 3000 steps; model id 20261006).
pub static WEIGHTS: &[u8] = include_bytes!("snn/weights.snn");

/// The shipped model, checked whole, or why not.
pub fn shipped() -> Result<Model, Refusal> {
    blob::decode(WEIGHTS)
}

/// One dish as the network reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dish {
    pub id: String,
    /// Per mille, `sense::vector` in `sense::all_keys` order.
    pub v: [i64; AXES],
    /// (kind, group index), each group at most once.
    pub groups: Vec<(u8, u32)>,
}

/// A menu: the dishes and how many groups of each kind; `names[t]` maps a group's name to its index.
#[derive(Debug, Clone, Default)]
pub struct Menu {
    pub dishes: Vec<Dish>,
    pub n_groups: [usize; GROUP_TYPES],
    pub names: [BTreeMap<String, u32>; GROUP_TYPES],
}

/// What the guest's profile holds, as indices into the menu. Weights are positive integers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Guest {
    /// Per mille of the strongest axis (`rank::per_mille`), `sense::all_keys` order.
    pub u: [i64; AXES],
    /// (dish index, weight): the device's dish weights; empty on the server.
    pub w: Vec<(usize, i64)>,
    /// Per group kind: (group index, weight).
    pub c: [Vec<(usize, i64)>; GROUP_TYPES],
}

fn strs(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).collect()).unwrap_or_default()
}

impl Menu {
    /// The dishes on sale whose allergens do not meet `avoid` (`block::taste::avoid_mask`), from
    /// `(id, product JSON)` -- the set `block::taste::top_k_json` ranks. Unparseable rows are left out.
    pub fn from_products(products: &[(String, String)], avoid: i64) -> Menu {
        let mut rows: Vec<(String, [i64; AXES], [Vec<String>; GROUP_TYPES])> = Vec::new();
        for (id, json) in products {
            let Ok(p) = serde_json::from_str::<Value>(json) else { continue };
            if p.get("available") == Some(&Value::Bool(false)) || crate::block::taste::allergen_mask(json) & avoid != 0 {
                continue;
            }
            let v = crate::block::taste::dense(&crate::rank::strip::vector_of(&p));
            let cat: Vec<String> = p.get("categoryId").and_then(Value::as_str).filter(|c| !c.is_empty()).map(|c| vec![c.trim().to_lowercase()]).unwrap_or_default();
            rows.push((id.clone(), v, [cat, strs(p.get("tags")), strs(p.get("ingredients"))]));
        }
        Menu::from_rows(rows)
    }

    /// The menu from already-read rows (id, vector, [categories, tags, ingredients]).
    pub fn from_rows(rows: Vec<(String, [i64; AXES], [Vec<String>; GROUP_TYPES])>) -> Menu {
        let mut m = Menu::default();
        for (_, _, gs) in &rows {
            for (t, names) in gs.iter().enumerate() {
                for n in names {
                    m.names[t].entry(n.clone()).or_insert(0);
                }
            }
        }
        for t in 0..GROUP_TYPES {
            for (i, (_, idx)) in m.names[t].iter_mut().enumerate() {
                *idx = i as u32;
            }
            m.n_groups[t] = m.names[t].len();
        }
        for (id, v, gs) in rows {
            let mut groups: Vec<(u8, u32)> = Vec::new();
            for (t, names) in gs.iter().enumerate() {
                for n in names {
                    let g = (t as u8, m.names[t][n]);
                    if !groups.contains(&g) {
                        groups.push(g);
                    }
                }
            }
            groups.sort_unstable();
            m.dishes.push(Dish { id, v, groups });
        }
        m
    }

    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.dishes.iter().position(|d| d.id == id)
    }
}

impl Guest {
    /// From what a profile holds: the taste vector (any scale; per-milled here), category and tag
    /// weights by name, and dish weights by id. Unknown names and weights <= 0 add nothing.
    pub fn from_maps(menu: &Menu, sense: &BTreeMap<String, i64>, cats: &BTreeMap<String, i64>, tags: &BTreeMap<String, i64>, dishes: &BTreeMap<String, i64>) -> Guest {
        let mut g = Guest { u: crate::block::taste::dense(&crate::rank::per_mille(sense)), ..Guest::default() };
        g.w = dishes.iter().filter(|(_, w)| **w > 0).filter_map(|(id, w)| Some((menu.index_of(id)?, *w))).collect();
        for (t, m) in [(CATEGORY, cats), (TAG, tags)] {
            g.c[t as usize] = m.iter().filter(|(_, w)| **w > 0).filter_map(|(k, w)| Some((*menu.names[t as usize].get(&k.trim().to_lowercase())? as usize, *w))).collect();
        }
        g
    }

    pub fn is_empty(&self) -> bool {
        self.u.iter().all(|x| *x == 0) && self.w.is_empty() && self.c.iter().all(Vec::is_empty)
    }
}

/// The top `k` dishes for the guest: score above 0, best first, ties by the id's bytes; dishes at
/// the indices in `skip` (already ordered, on the device) are left out.
pub fn rank(m: &Model, menu: &Menu, g: &Guest, k: usize, skip: &[usize]) -> Vec<(String, i64)> {
    if g.is_empty() || k == 0 || menu.dishes.is_empty() {
        return Vec::new();
    }
    let s = infer::stalks(m, menu);
    let mut out: Vec<(String, i64)> = infer::scores(m, menu, &s, g)
        .into_iter()
        .enumerate()
        .filter(|(i, sc)| *sc > 0 && !skip.contains(i))
        .map(|(i, sc)| (menu.dishes[i].id.clone(), sc))
        .collect();
    out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.as_bytes().cmp(b.0.as_bytes())));
    out.truncate(k);
    out
}

#[cfg(test)]
#[path = "snn/tests.rs"]
mod tests;
#[cfg(test)]
#[path = "snn/ab.rs"]
mod ab;
#[cfg(test)]
#[path = "snn/synth.rs"]
mod synth;
