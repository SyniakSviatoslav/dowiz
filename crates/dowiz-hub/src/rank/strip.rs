//! PURE. THE PHONE'S "FOR YOU" STRIP, IN RUST (W-TASTE row 1).
//!
//! The twin of `workers/api/public/store/taste.js` `scored()`/`strip()`: the same profile record
//! the phone keeps (`{v:1, dishes:{id:{order|open|dwell|add|drop:[[day,n]]}}, cats:{c:{seen}}, ctx}`),
//! the same menu, the same answer to the integer. It exists so the device's ranking can be
//! checked against a second implementation (`rank/tests.rs`, 1000/1000 on the shared fixture), and
//! so a hub-side caller can rank with the device's rule without inventing a third one.
//!
//! The arithmetic is `super` (fade, per mille, cosine); the dish's vector is `crate::sense`'s, the
//! one the server folds orders with. Weights are the phone's old ones x `UNIT`: an order 1000 per
//! portion, an open 200, a second of dwell 5, an add 500, a remove -400, a category seen 50.

use std::collections::BTreeMap;

use serde_json::Value;

use super::{cos_pm, fade, per_mille, UNIT};

pub const W_ORDER: i64 = UNIT;
pub const W_OPEN: i64 = 200;
pub const W_DWELL_SEC: i64 = 5;
pub const W_ADD: i64 = 500;
pub const W_DROP: i64 = -400;
pub const W_SEEN: i64 = 50;
/// The strip: at most this many dishes, of which at most `AGAIN_MAX` are "again".
pub const STRIP_MAX: usize = 6;
pub const AGAIN_MAX: usize = 2;

/// The session's moods (`store/sense.js` MOODS): axis weights, + wanted, - avoided.
pub fn mood(name: &str) -> BTreeMap<String, i64> {
    let rows: &[(&str, i64)] = match name {
        "quick" => &[("t:spicy", 600), ("t:sour", 500), ("a:citrus", 800), ("x:crispy", 700), ("x:crunchy", 600), ("x:juicy", 500)],
        "cosy" => &[("t:umami", 900), ("x:creamy", 700), ("x:tender", 700), ("a:toasty", 700), ("a:buttery", 600), ("a:spice-warm", 800), ("a:smoky", 500)],
        "light" => &[
            ("t:sour", 600),
            ("a:citrus", 800),
            ("a:herbal", 800),
            ("a:marine", 600),
            ("x:crunchy", 600),
            ("x:juicy", 600),
            ("x:creamy", -700),
            ("a:buttery", -700),
        ],
        "treat" => &[("t:sweet", 1000), ("x:creamy", 800), ("a:buttery", 700), ("a:nutty", 600), ("x:crispy", 500), ("a:fruity", 500)],
        _ => &[],
    };
    rows.iter().map(|(k, w)| (k.to_string(), *w)).collect()
}

/// What the strip is asked with besides the profile and the menu.
#[derive(Debug, Clone, Default)]
pub struct Opts {
    /// INFERRED allergens: they only move a dish to the end, never hide it.
    pub avoid_guess: Vec<String>,
    /// The venue's view of this guest (`GET /api/order/:id/taste` -> `taste`).
    pub prior: Option<Value>,
    /// The venue's moment (`band:evening`, `wx:rain`).
    pub ctx: Option<Vec<String>>,
    pub mood: Option<String>,
}

/// One dish of the strip: `why` is "again" or "taste"; `s` the integer it was ranked by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: String,
    pub why: &'static str,
    pub guessed: bool,
    pub s: i64,
}

/// Sum of a `[[day, n], ...]` list, each entry `n * w` faded to `day`. Non-integer entries add nothing.
pub fn sum(list: Option<&Value>, day: i64, w: i64) -> i64 {
    let Some(rows) = list.and_then(Value::as_array) else { return 0 };
    rows.iter()
        .filter_map(|e| Some((e.get(0)?.as_i64()?, e.get(1)?.as_i64()?)))
        .map(|(d, n)| fade(n.saturating_mul(w), day - d))
        .sum()
}

fn id_of(p: &Value) -> Option<&str> {
    p.get("id").and_then(Value::as_str)
}

fn strings(v: Option<&Value>) -> Vec<&str> {
    v.and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default()
}

/// The dish's vector (`sense::vector` of `sense::of_product`), empty when it declares nothing.
pub fn vector_of(p: &Value) -> BTreeMap<String, i64> {
    crate::sense::of_product(p).map(|s| crate::sense::vector(&s)).unwrap_or_default()
}

/// Weights per dish, per tag and per category (`taste.js weights`).
#[derive(Debug, Default)]
pub struct Weights {
    pub dish: BTreeMap<String, i64>,
    pub tag: BTreeMap<String, i64>,
    pub cat: BTreeMap<String, i64>,
}

fn add(m: &mut BTreeMap<String, i64>, k: &str, w: i64) {
    *m.entry(k.to_string()).or_default() += w;
}

pub fn weights(profile: &Value, on: &[&Value], day: i64) -> Weights {
    let mut w = Weights::default();
    if profile.get("v").and_then(Value::as_i64) != Some(1) {
        return w;
    }
    for (id, e) in profile.get("dishes").and_then(Value::as_object).into_iter().flatten() {
        let x = sum(e.get("order"), day, W_ORDER)
            + sum(e.get("open"), day, W_OPEN)
            + sum(e.get("dwell"), day, W_DWELL_SEC)
            + sum(e.get("add"), day, W_ADD)
            + sum(e.get("drop"), day, W_DROP);
        if x != 0 {
            w.dish.insert(id.clone(), x);
        }
    }
    for p in on {
        let x = id_of(p).and_then(|id| w.dish.get(id).copied()).unwrap_or(0);
        if x == 0 {
            continue;
        }
        for t in strings(p.get("tags")) {
            add(&mut w.tag, t, x);
        }
        if let Some(c) = p.get("categoryId").and_then(Value::as_str).filter(|c| !c.is_empty()) {
            add(&mut w.cat, c, x);
        }
    }
    for (c, e) in profile.get("cats").and_then(Value::as_object).into_iter().flatten() {
        add(&mut w.cat, c, sum(e.get("seen"), day, W_SEEN));
    }
    w
}

/// A positive integer weight of a prior row, or nothing.
fn prior_w(r: &Value) -> Option<(&str, i64)> {
    Some((r.get("key")?.as_str()?, r.get("w")?.as_i64().filter(|x| *x > 0)?))
}

fn prior_rows<'a>(prior: Option<&'a Value>, k: &str) -> impl Iterator<Item = (&'a str, i64)> {
    prior.and_then(|p| p.get(k)).and_then(Value::as_array).into_iter().flatten().filter_map(prior_w)
}

/// The guest's vector over the dishes' axes: each dish's weight x its vector / 1000. With `ctx`,
/// the weights are the orders filed under those contexts instead.
pub fn sense_vec(profile: &Value, on: &[&Value], day: i64, ctx: Option<&[String]>) -> BTreeMap<String, i64> {
    let mut w = weights(profile, on, day).dish;
    if let Some(ctx) = ctx {
        w.clear();
        for c in ctx {
            let filed = profile.get("ctx").and_then(|x| x.get(c.as_str())).and_then(Value::as_object);
            for (id, list) in filed.into_iter().flatten() {
                add(&mut w, id, sum(Some(list), day, W_ORDER));
            }
        }
    }
    let mut vec = BTreeMap::new();
    for p in on {
        let x = id_of(p).and_then(|id| w.get(id).copied()).unwrap_or(0);
        if x <= 0 {
            continue;
        }
        for (k, v) in vector_of(p) {
            add(&mut vec, &k, x * v / 1000);
        }
    }
    vec
}

/// What the axes add: the match with the guest (x2), with the moment (x1), with the mood (x3/2).
pub fn sense_score(v: &BTreeMap<String, i64>, guest: &BTreeMap<String, i64>, moment: &BTreeMap<String, i64>, mood: &BTreeMap<String, i64>) -> i64 {
    if v.is_empty() {
        return 0;
    }
    2 * cos_pm(guest, v).max(0) + cos_pm(moment, v).max(0) + 3 * cos_pm(mood, v) / 2
}

/// Every dish on sale with the integer it is ranked by, then the strip (`taste.js strip`).
pub fn strip(products: &[Value], profile: &Value, day: i64, o: &Opts) -> Vec<Item> {
    let on: Vec<&Value> = products.iter().filter(|p| p.is_object() && p.get("available") != Some(&Value::Bool(false)) && id_of(p).is_some()).collect();
    let mut w = weights(profile, &on, day);
    let prior = o.prior.as_ref();
    for (k, x) in prior_rows(prior, "tags") {
        add(&mut w.tag, k, x);
    }
    for (k, x) in prior_rows(prior, "cats") {
        add(&mut w.cat, k, x);
    }
    let mut guest = sense_vec(profile, &on, day, None);
    for (k, x) in prior_rows(prior, "sense") {
        add(&mut guest, k, x);
    }
    let guest = per_mille(&guest);
    let moment = o.ctx.as_deref().map(|c| per_mille(&sense_vec(profile, &on, day, Some(c)))).unwrap_or_default();
    let mood = o.mood.as_deref().map(mood).unwrap_or_default();
    let ordered = |id: &str| sum(profile.get("dishes").and_then(|d| d.get(id)).and_then(|e| e.get("order")), day, W_ORDER);
    let total = |p: &Value| {
        let id = id_of(p).unwrap_or("");
        let mut s = w.dish.get(id).copied().unwrap_or(0);
        for t in strings(p.get("tags")) {
            s += w.tag.get(t).copied().unwrap_or(0);
        }
        let cat = p.get("categoryId").and_then(Value::as_str).and_then(|c| w.cat.get(c)).copied().unwrap_or(0);
        s + cat / 2 + sense_score(&vector_of(p), &guest, &moment, &mood)
    };
    let guessed = |p: &Value| strings(p.get("allergens")).iter().any(|c| o.avoid_guess.iter().any(|g| g == c));
    let mut again: Vec<(&Value, i64)> = on.iter().map(|p| (*p, ordered(id_of(p).unwrap_or("")))).filter(|(_, n)| *n > 0).collect();
    again.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| id_of(a.0).cmp(&id_of(b.0))));
    again.truncate(AGAIN_MAX);
    let taken: Vec<&str> = again.iter().filter_map(|(p, _)| id_of(p)).collect();
    let mut rest: Vec<(&Value, i64)> = on.iter().filter(|p| !taken.contains(&id_of(p).unwrap_or(""))).map(|p| (*p, total(p))).filter(|(_, s)| *s > 0).collect();
    rest.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| id_of(a.0).cmp(&id_of(b.0))));
    let item = |p: &Value, why, guessed, s| Item { id: id_of(p).unwrap_or("").to_string(), why, guessed, s };
    let mut out: Vec<Item> = again.iter().map(|(p, s)| item(p, "again", false, *s)).collect();
    out.extend(rest.iter().filter(|(p, _)| !guessed(p)).map(|(p, s)| item(p, "taste", false, *s)));
    out.extend(rest.iter().filter(|(p, _)| guessed(p)).map(|(p, s)| item(p, "taste", true, *s)));
    out.truncate(STRIP_MAX);
    out
}
