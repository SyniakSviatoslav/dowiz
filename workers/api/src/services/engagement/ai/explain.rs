//! PURE. THE "EXPLAIN" CARD ON THE TWO NUMBERS SCREENS (W-AI row 3).
//!
//! Templates first: each card is a sentence from `words.rs` filled with cells
//! of the screen's own answer -- the trend against the period before, the
//! busiest and quietest hours, the menu matrix's quadrants, food cost, waste
//! and what runs low. A model only rewords a card when the owner asks and AI
//! is on, and `words::keeps_numbers` decides whether the rewording is shown.
//!
//! WHAT A SCREEN HAS DECIDES WHAT IS SAID. The richer answers (`compare`,
//! `hours`, `menu`: `analytics.owner.v2`, `analytics.kitchen.v2`) are read
//! when present; without them the trend is the window's second half against
//! its first, read from `byDay`, and the matrix card is not drawn.

use super::answer::{self, Fold, Num};
use super::query::{Kind, Query};
use super::words::{self as w, fill};
use serde_json::{json, Value};

#[derive(Clone, Debug)]
pub struct Card {
    pub kind: &'static str,
    pub text: String,
    pub numbers: Vec<Num>,
    pub source: String,
}

impl Card {
    pub fn json(&self) -> Value {
        json!({ "kind": self.kind, "text": self.text, "source": self.source,
                "numbers": self.numbers.iter().map(|n| n.json(&self.source)).collect::<Vec<_>>() })
    }
}

fn int(v: &Value, p: &str) -> Option<i64> {
    v.pointer(p).and_then(Value::as_i64)
}

fn n(value: i64, unit: &'static str, p: &str) -> Num {
    Num { value, unit, pointers: vec![p.to_string()], days: vec![] }
}

/// The analytics screen's cards. `days` is the window it was asked for.
pub fn analytics(a: &Value, lang: &str, days: i64) -> Vec<Card> {
    let src = answer::source_of(Fold::Analytics, days);
    let cur = a.get("currency").and_then(Value::as_str).unwrap_or("ALL").to_string();
    let money = |v: i64| crate::notify::money_text(v, &cur);
    let mut out = Vec::new();
    // THE TREND.
    if let (Some(now), Some(before)) = (int(a, "/revenue"), int(a, "/compare/prev/revenue")) {
        let pct = int(a, "/compare/prev/deltaPm/revenue").map_or("—".to_string(), w::signed_pct);
        out.push(Card { kind: "trend", text: fill(&w::TREND_PREV, lang, &[money(now), money(before), pct]), source: src.clone(),
                        numbers: vec![n(now, "money", "/revenue"), n(before, "money", "/compare/prev/revenue")] });
    } else {
        let by: Vec<i64> = a.pointer("/byDay").and_then(Value::as_array).map_or(vec![], |d| d.iter().map(|x| x["revenue"].as_i64().unwrap_or(0)).collect());
        let h = by.len() / 2;
        if h > 0 {
            let first: i64 = by[..h].iter().sum();
            let second: i64 = by[by.len() - h..].iter().sum();
            let pct = if first > 0 { w::signed_pct((second - first) * 1000 / first) } else { "—".into() };
            let ptrs = |r: std::ops::Range<usize>| r.map(|i| format!("/byDay/{i}/revenue")).collect::<Vec<_>>();
            out.push(Card { kind: "trend", text: fill(&w::TREND_HALF, lang, &[money(second), money(first), pct]), source: src.clone(),
                            numbers: vec![Num { value: second, unit: "money", pointers: ptrs(by.len() - h..by.len()), days: vec![] },
                                          Num { value: first, unit: "money", pointers: ptrs(0..h), days: vec![] }] });
        }
    }
    // THE HOURS: the busiest and the quietest that had any order at all.
    let hours: Vec<i64> = a.pointer("/byHour").and_then(Value::as_array).map_or(vec![], |d| d.iter().map(|x| x.as_i64().unwrap_or(0)).collect());
    let open: Vec<usize> = (0..hours.len()).filter(|h| hours[*h] > 0).collect();
    let busy = open.iter().copied().max_by(|x, y| hours[*x].cmp(&hours[*y]).then(y.cmp(x)));
    let quiet = open.iter().copied().min_by(|x, y| hours[*x].cmp(&hours[*y]).then(x.cmp(y)));
    if let (Some(b), Some(q)) = (busy, quiet) {
        out.push(Card { kind: "hours", source: src.clone(),
            text: fill(&w::HOURS, lang, &[format!("{b:02}"), hours[b].to_string(), format!("{q:02}"), hours[q].to_string()]),
            numbers: vec![n(hours[b], "orders", &format!("/byHour/{b}")), n(hours[q], "orders", &format!("/byHour/{q}"))] });
    }
    out
}

/// The kitchen screen's cards: food cost, waste and what runs low (the same
/// answers the questions give), then the menu matrix when the screen has one.
pub fn kitchen(k: &Value, lang: &str, days: i64) -> Vec<Card> {
    let mut out: Vec<Card> = [(Kind::FoodCost, "food_cost"), (Kind::Waste, "waste"), (Kind::LowStock, "low_stock")]
        .into_iter()
        .map(|(kind, name)| {
            let a = answer::answer(&Query { kind, days, weekday: None, dish: None }, k, lang);
            Card { kind: name, text: a.text, numbers: a.numbers, source: a.source }
        })
        .collect();
    let src = answer::source_of(Fold::Kitchen, days);
    let Some(rows) = k.pointer("/menu/dishes").and_then(Value::as_array) else { return out };
    let count = |q: &str| rows.iter().filter(|d| d["quadrant"].as_str() == Some(q)).count() as i64;
    let quads = ["star", "plowhorse", "puzzle", "dog"];
    let counts: Vec<i64> = quads.iter().map(|q| count(q)).collect();
    if counts.iter().sum::<i64>() == 0 {
        return out;
    }
    out.push(Card { kind: "menu", source: src.clone(), text: fill(&w::MENU, lang, &counts.iter().map(i64::to_string).collect::<Vec<_>>()),
                    numbers: counts.iter().map(|c| Num { value: *c, unit: "dishes", pointers: vec![], days: vec![] }).collect() });
    // One dish per quadrant, the matrix's own order (most sold first).
    let cur = k.get("currency").and_then(Value::as_str).unwrap_or("ALL").to_string();
    for q in quads {
        let Some((i, d)) = rows.iter().enumerate().find(|(_, d)| d["quadrant"].as_str() == Some(q)) else { continue };
        let name = d["name"].as_str().unwrap_or("").to_string();
        let (tpl, args, nums) = match q {
            "star" => (&w::STAR, vec![name], vec![]),
            "puzzle" => (&w::PUZZLE, vec![name], vec![]),
            "dog" => (&w::DOG, vec![name], vec![]),
            _ => {
                let raise = d["raiseBy"].as_i64().unwrap_or(0);
                (&w::PLOWHORSE, vec![name, crate::notify::money_text(raise, &cur)], vec![n(raise, "money", &format!("/menu/dishes/{i}/raiseBy"))])
            }
        };
        out.push(Card { kind: q, text: fill(tpl, lang, &args), numbers: nums, source: src.clone() });
    }
    out
}
