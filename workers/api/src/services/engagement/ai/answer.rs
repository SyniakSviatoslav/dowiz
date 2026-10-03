//! PURE. A [`Query`] ANSWERED FROM THE VENUE'S OWN FOLDS, EVERY NUMBER WITH ITS SOURCE (W-AI row 2).
//!
//! The numbers are READ, never recomputed: `a` is the answer of
//! `GET /api/owner/analytics?days=` and `k` of `/api/owner/analytics/kitchen?days=`
//! (the same folds, asked in the venue's object). A total over several days is
//! the sum of cells the answer names, so each [`Num`] carries the JSON
//! pointers it was read from and the days it covers: the console opens the
//! source, and each day opens its records (`/api/owner/analytics?trace=`).
//!
//! AGGREGATES ONLY. A fold answer holds dishes, hours, days, channels and
//! ingredients; nothing here reads a person, and nothing else is passed on.

use super::query::{Kind, Query};
use super::words::{self as w, fill, W};
use dowiz_hub::stock::meta::{day_number, day_of_local_ms, parse_day, show_day};
use serde_json::{json, Value};

/// Which fold a kind reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fold {
    Analytics,
    Kitchen,
}

pub fn fold_of(kind: Kind) -> Fold {
    match kind {
        Kind::BestDish | Kind::WorstDish | Kind::DishSold | Kind::FoodCost | Kind::Waste | Kind::LowStock | Kind::WorstMargin => Fold::Kitchen,
        _ => Fold::Analytics,
    }
}

/// The window the analytics fold is asked for: it answers 7 or 30 days.
pub fn analytics_days(days: i64) -> i64 {
    if days >= 30 { 30 } else { 7 }
}

/// The window a kind is answered over. The analytics fold has no one-day
/// answer for these, so "today" is answered over 7 days and the sentence says so.
pub fn effective_days(q: &Query) -> i64 {
    match q.kind {
        Kind::AverageOrder | Kind::Rejected | Kind::BusiestHour | Kind::QuietestHour | Kind::Channels | Kind::BestDay | Kind::WorstDay => analytics_days(q.days),
        _ => q.days,
    }
}

/// The public route a number was read from: the same fold, asked the same way
/// (`v=2`, so the cells of the richer answer -- `compare`, `menu` -- are there
/// too once the route carries them; before that the route ignores it).
pub fn source_of(fold: Fold, days: i64) -> String {
    match fold {
        Fold::Analytics => format!("/api/owner/analytics?days={}&v=2", analytics_days(days)),
        Fold::Kitchen => format!("/api/owner/analytics/kitchen?days={days}&v=2"),
    }
}

/// One number of an answer and where it came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Num {
    pub value: i64,
    pub unit: &'static str,
    pub pointers: Vec<String>,
    pub days: Vec<String>,
}

impl Num {
    pub fn json(&self, source: &str) -> Value {
        json!({
            "value": self.value, "unit": self.unit, "source": source, "pointers": self.pointers,
            "trace": self.days.iter().map(|d| format!("/api/owner/analytics?trace={d}")).collect::<Vec<_>>(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Answer {
    pub text: String,
    pub numbers: Vec<Num>,
    pub source: String,
}

impl Answer {
    pub fn json(&self) -> Value {
        json!({ "text": self.text, "source": self.source, "numbers": self.numbers.iter().map(|n| n.json(&self.source)).collect::<Vec<_>>() })
    }
}

/// 0 = Monday, from `yyyy-mm-dd`.
pub fn weekday_of(day: &str) -> Option<usize> {
    parse_day(day).map(|d| (day_number(d) + 3).rem_euclid(7) as usize)
}

/// The analytics fold's days as `yyyy-mm-dd`: its own `day` when it has one,
/// else the local date of its `at` (a local midnight as a UTC instant: twelve
/// hours later is noon of the same local day in every zone within +-12 h).
pub fn analytics_days_of(a: &Value) -> Vec<String> {
    arr(a, "/byDay")
        .iter()
        .map(|d| match d.get("day").and_then(Value::as_str) {
            Some(s) => s.to_string(),
            None => show_day(day_of_local_ms(d["at"].as_i64().unwrap_or(0) + 43_200_000)),
        })
        .collect()
}

fn arr<'a>(v: &'a Value, p: &str) -> &'a [Value] {
    v.pointer(p).and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

fn int(v: &Value, p: &str) -> i64 {
    v.pointer(p).and_then(Value::as_i64).unwrap_or(0)
}

fn num(value: i64, unit: &'static str, pointers: Vec<String>, days: Vec<String>) -> Num {
    Num { value, unit, pointers, days }
}

/// Which of `days` a query covers: all, the last one (today), or one weekday.
fn picked(days: &[String], q: &Query, eff: i64) -> Vec<usize> {
    let n = days.len();
    let from = n.saturating_sub(eff.max(1) as usize);
    (from..n).filter(|i| q.weekday.map_or(true, |wd| weekday_of(&days[*i]) == Some(wd))).collect()
}

/// The answer. `fold` is the answer of the fold `fold_of(q.kind)` names.
pub fn answer(q: &Query, fold: &Value, lang: &str) -> Answer {
    let eff = effective_days(q);
    let src = source_of(fold_of(q.kind), eff);
    let cur = fold.get("currency").and_then(Value::as_str).unwrap_or("ALL").to_string();
    let money = |n: i64| crate::notify::money_text(n, &cur);
    let win = w::window(lang, eff, q.weekday);
    let say = |t: &W, args: Vec<String>, numbers: Vec<Num>| Answer { text: fill(t, lang, &args), numbers, source: src.clone() };
    match fold_of(q.kind) {
        Fold::Analytics => {
            let days = analytics_days_of(fold);
            let idx = picked(&days, q, eff);
            let ptr = |i: usize, f: &str| format!("/byDay/{i}/{f}");
            let span = |ix: &[usize]| ix.iter().map(|i| days[*i].clone()).collect::<Vec<_>>();
            // The fold's own total when the query is its whole window; else the named days' cells.
            let whole = q.weekday.is_none() && q.days != 1;
            match q.kind {
                Kind::Revenue | Kind::Orders => {
                    let (rev, ord, rp, op) = if whole {
                        (int(fold, "/revenue"), int(fold, "/orders"), vec!["/revenue".into()], vec!["/orders".into()])
                    } else {
                        let r = idx.iter().map(|i| int(fold, &ptr(*i, "revenue"))).sum();
                        let o = idx.iter().map(|i| int(fold, &ptr(*i, "orders"))).sum();
                        (r, o, idx.iter().map(|i| ptr(*i, "revenue")).collect(), idx.iter().map(|i| ptr(*i, "orders")).collect())
                    };
                    let (r, o) = (num(rev, "money", rp, span(&idx)), num(ord, "orders", op, span(&idx)));
                    if q.kind == Kind::Revenue {
                        say(&w::REVENUE, vec![win, money(rev), ord.to_string()], vec![r, o])
                    } else {
                        say(&w::ORDERS, vec![win, ord.to_string()], vec![o])
                    }
                }
                Kind::AverageOrder => {
                    let v = int(fold, "/averageOrder");
                    say(&w::AVERAGE, vec![win, money(v)], vec![num(v, "money", vec!["/averageOrder".into()], span(&idx))])
                }
                Kind::Rejected => {
                    let (x, o) = (int(fold, "/rejected"), int(fold, "/orders"));
                    say(&w::REJECTED, vec![win, x.to_string(), o.to_string()], vec![num(x, "orders", vec!["/rejected".into()], span(&idx)), num(o, "orders", vec!["/orders".into()], span(&idx))])
                }
                Kind::BusiestHour | Kind::QuietestHour => {
                    let hours: Vec<i64> = arr(fold, "/byHour").iter().map(|v| v.as_i64().unwrap_or(0)).collect();
                    let open: Vec<usize> = (0..hours.len()).filter(|h| hours[*h] > 0).collect();
                    let pick = if q.kind == Kind::BusiestHour {
                        open.iter().copied().max_by(|a, b| hours[*a].cmp(&hours[*b]).then(b.cmp(a)))
                    } else {
                        open.iter().copied().min_by(|a, b| hours[*a].cmp(&hours[*b]).then(a.cmp(b)))
                    };
                    let Some(h) = pick else { return say(&w::NO_SALES, vec![win], vec![]) };
                    let t = if q.kind == Kind::BusiestHour { &w::BUSIEST_HOUR } else { &w::QUIET_HOUR };
                    say(t, vec![win, format!("{h:02}"), hours[h].to_string()], vec![num(hours[h], "orders", vec![format!("/byHour/{h}")], span(&idx))])
                }
                Kind::BestDay | Kind::WorstDay => {
                    let rev = |i: &usize| int(fold, &ptr(*i, "revenue"));
                    let pick = if q.kind == Kind::BestDay {
                        idx.iter().copied().max_by(|a, b| rev(a).cmp(&rev(b)).then(b.cmp(a)))
                    } else {
                        idx.iter().copied().min_by(|a, b| rev(a).cmp(&rev(b)).then(a.cmp(b)))
                    };
                    let Some(i) = pick else { return say(&w::NO_SALES, vec![win], vec![]) };
                    let t = if q.kind == Kind::BestDay { &w::BEST_DAY } else { &w::WORST_DAY };
                    say(t, vec![win, days[i].clone(), money(rev(&i))], vec![num(rev(&i), "money", vec![ptr(i, "revenue")], vec![days[i].clone()])])
                }
                Kind::Channels => {
                    let by = fold.get("byChannel").and_then(Value::as_object);
                    let best = by.and_then(|m| m.iter().filter_map(|(c, n)| n.as_i64().map(|n| (c, n))).max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(a.0))));
                    let Some((c, n)) = best.filter(|(_, n)| *n > 0) else { return say(&w::NO_SALES, vec![win], vec![]) };
                    let o = int(fold, "/orders");
                    say(&w::CHANNELS, vec![win, c.clone(), n.to_string(), o.to_string()],
                        vec![num(n, "orders", vec![format!("/byChannel/{c}")], span(&idx)), num(o, "orders", vec!["/orders".into()], span(&idx))])
                }
                _ => say(&w::UNKNOWN, vec![], vec![]),
            }
        }
        Fold::Kitchen => kitchen(q, fold, lang, &src, &money, win),
    }
}

fn kitchen(q: &Query, k: &Value, lang: &str, src: &str, money: &dyn Fn(i64) -> String, win: String) -> Answer {
    let days: Vec<String> = arr(k, "/days").iter().filter_map(|d| d.as_str().map(str::to_string)).collect();
    let idx = picked(&days, q, q.days);
    let span = || idx.iter().map(|i| days[*i].clone()).collect::<Vec<_>>();
    let say = |t: &W, args: Vec<String>, numbers: Vec<Num>| Answer { text: fill(t, lang, &args), numbers, source: src.to_string() };
    let dishes = arr(k, "/dishes");
    // A dish's portions over the query's days: the row's total, or the sum of its weekday cells.
    let sold = |j: usize| -> (i64, Vec<String>) {
        match q.weekday {
            None => (int(&dishes[j], "/sold"), vec![format!("/dishes/{j}/sold")]),
            Some(_) => (idx.iter().map(|i| int(&dishes[j], &format!("/byDay/{i}"))).sum(), idx.iter().map(|i| format!("/dishes/{j}/byDay/{i}")).collect()),
        }
    };
    let name = |j: usize| dishes[j].get("name").and_then(Value::as_str).unwrap_or("").to_string();
    // The money is a dish's revenue over the whole window; a weekday has no money cell, so none is said.
    let takings = |j: usize| if q.weekday.is_none() { format!(", {}", money(int(&dishes[j], "/revenue"))) } else { String::new() };
    match q.kind {
        Kind::BestDish | Kind::WorstDish => {
            let ord = |a: &usize, b: &usize| sold(*a).0.cmp(&sold(*b).0).then(name(*b).cmp(&name(*a)));
            let pick = if q.kind == Kind::BestDish { (0..dishes.len()).max_by(ord) } else { (0..dishes.len()).min_by(|a, b| sold(*a).0.cmp(&sold(*b).0).then(name(*a).cmp(&name(*b)))) };
            let Some(j) = pick else { return say(&w::NO_SALES, vec![win], vec![]) };
            let (n, ptrs) = sold(j);
            let t = if q.kind == Kind::BestDish { &w::BEST_DISH } else { &w::WORST_DISH };
            say(t, vec![win, name(j), n.to_string(), takings(j)], vec![num(n, "portions", ptrs, span())])
        }
        Kind::DishSold => {
            let id = q.dish.clone().unwrap_or_default();
            match (0..dishes.len()).find(|j| dishes[*j].get("id").and_then(Value::as_str) == Some(id.as_str())) {
                Some(j) => {
                    let (n, ptrs) = sold(j);
                    say(&w::DISH_SOLD, vec![win, name(j), n.to_string(), takings(j)], vec![num(n, "portions", ptrs, span())])
                }
                // Not a row of the fold: it sold nothing in the window.
                None => say(&w::DISH_SOLD, vec![win, id, "0".into(), String::new()], vec![num(0, "portions", vec![], span())]),
            }
        }
        Kind::FoodCost => match k.pointer("/totals/foodCostPm").and_then(Value::as_i64) {
            Some(pm) => {
                let (c, r) = (int(k, "/totals/cogs"), int(k, "/totals/revenue"));
                say(&w::FOOD_COST, vec![win, w::pct(pm), money(c), money(r)], vec![
                    num(pm, "permille", vec!["/totals/foodCostPm".into()], span()),
                    num(c, "money", vec!["/totals/cogs".into()], span()),
                    num(r, "money", vec!["/totals/revenue".into()], span()),
                ])
            }
            None => say(&w::FOOD_COST_NONE, vec![win], vec![]),
        },
        Kind::Waste => {
            let v = int(k, "/totals/wasteValue");
            let reasons = arr(k, "/waste");
            let top = (0..reasons.len()).max_by(|a, b| int(&reasons[*a], "/value").cmp(&int(&reasons[*b], "/value")).then(b.cmp(a)));
            match top.filter(|_| v > 0) {
                Some(r) => say(&w::WASTE, vec![win, money(v), reasons[r]["reason"].as_str().unwrap_or("").to_string()],
                    vec![num(v, "money", vec!["/totals/wasteValue".into()], span())]),
                None => say(&w::WASTE_NONE, vec![win], vec![num(0, "money", vec!["/totals/wasteValue".into()], span())]),
            }
        }
        Kind::LowStock => {
            let ing = arr(k, "/ingredients");
            let low: Vec<usize> = (0..ing.len()).filter(|i| ing[*i].get("reorder").is_some_and(|r| !r.is_null())).collect();
            if low.is_empty() {
                return say(&w::LOW_NONE, vec![], vec![]);
            }
            let names: Vec<String> = low.iter().take(6).map(|i| ing[*i]["name"].as_str().unwrap_or("").to_string()).collect();
            // One number per ingredient: what the fold suggests ordering, from its own cell.
            say(&w::LOW, vec![win, low.len().to_string(), names.join(", ")],
                low.iter().map(|i| num(int(&ing[*i], "/reorder"), "reorder", vec![format!("/ingredients/{i}/reorder")], span())).collect())
        }
        Kind::WorstMargin => {
            let m = |j: &usize| dishes[*j].get("marginPortion").and_then(Value::as_i64);
            match (0..dishes.len()).filter(|j| m(j).is_some()).min_by(|a, b| m(a).cmp(&m(b)).then(name(*a).cmp(&name(*b)))) {
                Some(j) => {
                    let v = m(&j).unwrap_or(0);
                    say(&w::MARGIN, vec![win, name(j), money(v)], vec![num(v, "money", vec![format!("/dishes/{j}/marginPortion")], span())])
                }
                None => say(&w::MARGIN_NONE, vec![win], vec![]),
            }
        }
        _ => say(&w::UNKNOWN, vec![], vec![]),
    }
}
