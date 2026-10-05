//! PURE. The forecast of one venue-day (`dowiz_hub::forecast`), composed:
//! the venue's orders and portions, its bands, each dish with its own
//! measured error; tonight's bookings on top; the whole expanded through
//! the recipes (`expand`) into the prep list. The JSON is
//! `kitchen.prep_forecast.v1` (`tools/live-proof/contracts/feature-prep-forecast.json`).

use serde_json::{json, Value};
use std::collections::BTreeMap;

use super::expand;
use dowiz_hub::forecast::{self as fc, Estimate, Error, History};
use dowiz_hub::stock::meta::{day_of_number, show_day};

/// One dish's forecast and the error it made over the last four weeks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DishCast {
    pub id: String,
    pub est: Estimate,
    pub err: Error,
}

/// One venue-day's forecast. `first` is the venue's first day of history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cast {
    pub first: Option<i64>,
    pub orders: Estimate,
    pub portions: Estimate,
    pub err: Error,
    pub bands: [Estimate; 3],
    pub dishes: Vec<DishCast>,
}

fn of_dish(id: &str) -> impl Fn(&fc::Day) -> i64 + '_ {
    move |d: &fc::Day| d.dishes.get(id).copied().unwrap_or(0)
}

/// The forecast for `target` as of `today`; `measure` also backtests each
/// series (the prep list does; the expiry check does not need to).
pub fn cast(h: &History, today: i64, target: i64, measure: bool) -> Cast {
    let none = Estimate::Learning { weeks: 0 };
    let Some(first) = fc::first_day(h, today) else {
        return Cast { first: None, orders: none, portions: none, err: Error::default(), bands: [none; 3], dishes: Vec::new() };
    };
    let est = |f: &dyn Fn(&fc::Day) -> i64| fc::estimate_of(h, first, today, target, f);
    let portions = |d: &fc::Day| d.portions();
    let sampled = fc::samples(first, today, target);
    let mut ids: Vec<String> = sampled.iter().filter_map(|n| h.days.get(n)).flat_map(|d| d.dishes.keys().cloned()).collect();
    ids.sort();
    ids.dedup();
    let dishes = ids
        .into_iter()
        .map(|id| {
            let from = fc::first_sale(h, &id, today).unwrap_or(first);
            let f = of_dish(&id);
            let est = fc::estimate_of(h, from, today, target, &f);
            let err = if measure { fc::backtest(h, from, today, &f) } else { Error::default() };
            drop(f);
            DishCast { id, est, err }
        })
        .collect();
    Cast {
        first: Some(first),
        orders: est(&|d: &fc::Day| d.orders),
        portions: est(&portions),
        err: if measure { fc::backtest(h, first, today, &portions) } else { Error::default() },
        bands: [0usize, 1, 2].map(|b| est(&move |d: &fc::Day| d.bands[b])),
        dishes,
    }
}

/// The dishes with a number, and more than nothing.
pub fn portions_of(c: &Cast) -> BTreeMap<String, i64> {
    c.dishes.iter().filter_map(|d| d.est.value().filter(|v| *v > 0).map(|v| (d.id.clone(), v))).collect()
}

/// TONIGHT'S BOOKINGS × THE USUAL BASKET: each booked guest is counted as
/// one order of the sample days' average, per dish (the dish's portions
/// over the orders, rounded half up). Only dishes that have a number.
pub fn bookings(h: &History, c: &Cast, today: i64, target: i64, covers: i64) -> BTreeMap<String, i64> {
    let Some(first) = c.first.filter(|_| covers > 0) else { return BTreeMap::new() };
    let days: Vec<&fc::Day> = fc::samples(first, today, target).iter().filter_map(|n| h.days.get(n)).collect();
    let orders: i64 = days.iter().map(|d| d.orders).sum();
    if orders <= 0 {
        return BTreeMap::new();
    }
    c.dishes
        .iter()
        .filter(|d| d.est.value().is_some())
        .map(|d| (d.id.clone(), (covers * days.iter().map(|x| x.dishes.get(&d.id).copied().unwrap_or(0)).sum::<i64>() * 2 + orders).div_euclid(orders * 2)))
        .filter(|(_, v)| *v > 0)
        .collect()
}

/// One weekday's opening windows from the venue's record; `None` when the
/// venue has no hours set (the bands then fall back to the clock's).
pub fn windows(venue: Option<&Value>, weekday: usize) -> Option<Vec<(i64, i64)>> {
    let sched = dowiz_hub::hours::from_json(&venue?.get("hours")?.to_string());
    if sched.is_empty() {
        return None;
    }
    Some(sched.days[weekday % 7].iter().map(|w| (w.open, w.close)).collect())
}

pub fn hhmm(m: i64) -> String {
    let m = m.rem_euclid(24 * 60);
    format!("{:02}:{:02}", m / 60, m % 60)
}

pub fn est_json(e: &Estimate) -> Value {
    match e {
        Estimate::Number { value, method, weeks } => json!({ "value": value, "learning": false, "weeks": weeks, "method": method.as_str() }),
        Estimate::Learning { weeks } => json!({ "value": null, "learning": true, "weeks": weeks, "method": null }),
    }
}

pub fn err_json(mut v: Value, e: &Error) -> Value {
    v["offBy"] = json!(e.off_by());
    v["naiveOffBy"] = json!(e.naive_off_by());
    v["masePm"] = json!(e.mase_pm());
    v["checkedDays"] = json!(e.days);
    v
}

/// A dish's word from the catalogue (its `name`, as the menu stores it).
fn dish_name(cat: &dowiz_hub::catalog::Catalog, id: &str) -> Value {
    cat.product(id).and_then(|j| serde_json::from_str::<Value>(&j).ok()).and_then(|p| p.get("name").cloned()).unwrap_or(json!(id))
}

/// The whole answer but `history` and `today` (the caller adds them).
pub fn answer(h: &History, today: i64, target: i64, cat: &dowiz_hub::catalog::Catalog, led: &dowiz_hub::stock::StockLedger, covers: i64, windows: Option<Vec<(i64, i64)>>) -> Value {
    let c = cast(h, today, target, true);
    let extra = bookings(h, &c, today, target, covers);
    let mut eat = portions_of(&c);
    for (k, v) in &extra {
        *eat.entry(k.clone()).or_default() += v;
    }
    let needs = expand::needs(&eat, cat);
    let known = windows.is_some();
    let night = fc::NIGHT_ENDS as i64 * 60;
    let spans = fc::bands_for(&windows.unwrap_or_else(|| vec![(night, night)]));
    let bands: Vec<Value> = (0..3)
        .filter_map(|b| spans[b].map(|(f, t)| (b, f, t)))
        .map(|(b, f, t)| {
            let mut v = est_json(&c.bands[b]);
            v["band"] = json!(b);
            v["from"] = json!(hhmm(f));
            v["to"] = json!(hhmm(t));
            v
        })
        .collect();
    let mut dishes: Vec<&DishCast> = c.dishes.iter().collect();
    dishes.sort_by(|a, b| b.est.value().unwrap_or(-1).cmp(&a.est.value().unwrap_or(-1)).then(a.id.cmp(&b.id)));
    let dish_rows: Vec<Value> = dishes
        .iter()
        .map(|d| {
            let mut v = err_json(est_json(&d.est), &d.err);
            v["id"] = json!(d.id);
            v["name"] = dish_name(cat, &d.id);
            v["fromBookings"] = json!(extra.get(&d.id).copied().unwrap_or(0));
            v
        })
        .collect();
    let first = c.first.unwrap_or(today);
    let samples: Vec<Value> = fc::samples(first, today, target)
        .into_iter()
        .map(|n| {
            let d = h.days.get(&n).cloned().unwrap_or_default();
            json!({ "day": show_day(day_of_number(n)), "placed": d.bands.iter().sum::<i64>(), "orders": d.orders, "portions": d.portions() })
        })
        .collect();
    json!({
        "contract": super::CONTRACT,
        "day": show_day(day_of_number(target)),
        "weekday": super::super::history::weekday(target),
        "hours": { "known": known, "open": spans.iter().any(Option::is_some) },
        "orders": est_json(&c.orders),
        "portions": err_json(est_json(&c.portions), &c.err),
        "bands": bands,
        "dishes": dish_rows,
        "bookings": { "covers": covers, "portions": extra.values().sum::<i64>() },
        "preps": expand::prep_rows(&needs, cat, led),
        "raw": needs.raw.iter().filter(|(_, q)| **q > 0).map(|(id, q)| expand::item_json(cat, id, *q)).collect::<Vec<_>>(),
        "unmodelled": needs.unmodelled,
        "refused": needs.refused.iter().map(|(d, why)| json!({ "dish": d, "why": why })).collect::<Vec<_>>(),
        "samples": samples,
    })
}
