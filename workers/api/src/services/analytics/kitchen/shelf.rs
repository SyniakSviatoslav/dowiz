//! THE RECORDED SIDE, pure: what the stock log says happened in the window --
//! deliveries and what they cost, what the orders drew, what was binned and
//! why, what a count found missing, what prep weighed -- each record dated by
//! its own `at`, else by its order's placement (a reservation belongs to the
//! day its order was placed), and valued at the average of its moment.
//!
//! A record with neither date is not guessed into a day: it is counted as
//! `undated` and left out, so an owner can see how much of the log predates
//! dates (research R2) rather than having it silently land on "today".

use super::sales::{Supply, Window};
use dowiz_hub::stock::journal::Entry;
use dowiz_hub::stock::StockEvent;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};

/// One ingredient's recorded movements in the window, base units and money.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Moved {
    pub received: i64,
    pub received_value: i64,
    /// Drawn by orders: consumed + served - unserved.
    pub drawn: i64,
    pub drawn_value: i64,
    pub wasted: i64,
    pub wasted_value: i64,
    /// Count drift: observed - expected, summed (negative = missing).
    pub drift: i64,
    pub drift_value: i64,
    /// Prep: onto the board and off it.
    pub prep_in: i64,
    pub prep_out: i64,
    pub by_day_drawn: Vec<i64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Shelf {
    pub moved: BTreeMap<String, Moved>,
    /// reason -> (qty rows, value)
    pub waste: BTreeMap<String, (i64, i64)>,
    pub waste_by_day: Vec<i64>,
    pub received_by_day: Vec<i64>,
    pub yields: Vec<Value>,
    pub prices: BTreeMap<String, Vec<Value>>,
    pub undated: i64,
}

/// The instant a record happened: its own clock, else its order's placement.
pub fn when(e: &Entry, placed_at: &HashMap<String, i64>) -> Option<i64> {
    e.meta.at.or_else(|| e.ev.order_id().and_then(|o| placed_at.get(o).copied()))
}

/// Fold the journal over the window.
pub fn fold(entries: &[Entry], supplies: &HashMap<String, Supply>, placed_at: &HashMap<String, i64>, w: &Window) -> Shelf {
    let n = w.starts.len();
    let mut s = Shelf { waste_by_day: vec![0; n], received_by_day: vec![0; n], ..Shelf::default() };
    for e in entries {
        if matches!(e.ev, StockEvent::Reserved { .. } | StockEvent::Released { .. }) {
            continue; // a hold is not a movement of food
        }
        let Some(at) = when(e, placed_at) else {
            s.undated += 1;
            continue;
        };
        let Some(d) = w.bucket(at) else { continue };
        let item = e.ev.item().to_string();
        let v = e.value.unwrap_or(0);
        let m = s.moved.entry(item.clone()).or_insert_with(|| Moved { by_day_drawn: vec![0; n], ..Moved::default() });
        match &e.ev {
            StockEvent::Received { qty, .. } => {
                m.received += qty;
                m.received_value += v;
                s.received_by_day[d] += v;
                if let (Some(c), Some(per)) = (e.meta.unit_cost, e.meta.per) {
                    let basis = supplies.get(&item).map_or(100, |x| x.basis);
                    s.prices.entry(item.clone()).or_default().push(json!({
                        "at": at, "day": dowiz_hub::stock::meta::show_day(w.days[d]), "perBasis": dowiz_hub::stock::journal::priced(basis, c, per),
                        "qty": qty, "supplier": e.meta.supplier, "doc": e.meta.doc,
                    }));
                }
            }
            StockEvent::Consumed { qty, .. } | StockEvent::Served { qty, .. } => {
                m.drawn += qty;
                m.drawn_value += v;
                m.by_day_drawn[d] += qty;
            }
            StockEvent::Unserved { qty, .. } => {
                m.drawn -= qty;
                m.by_day_drawn[d] -= qty;
            }
            StockEvent::Wasted { qty, reason, .. } => {
                m.wasted += qty;
                m.wasted_value += v;
                s.waste_by_day[d] += v;
                let r = s.waste.entry(reason.as_str().to_string()).or_default();
                r.0 += 1;
                r.1 += v;
            }
            StockEvent::Returned { resell: false, .. } => {
                let r = s.waste.entry("returned".into()).or_default();
                r.0 += 1;
                r.1 += v;
                s.waste_by_day[d] += v;
            }
            StockEvent::Stocktake { observed, .. } => {
                m.drift += observed - e.expected();
                m.drift_value += v;
            }
            StockEvent::Produced { qty, out, stage, into, .. } => {
                m.prep_in += qty;
                m.prep_out += out;
                let sup = supplies.get(&item);
                let expected = sup.map(|x| if stage.as_str() == "clean" { x.clean_pm } else { x.cook_pm });
                let measured = if *qty > 0 { out * 1000 / qty } else { 0 };
                s.yields.push(json!({
                    "item": item, "name": sup.map(|x| x.name.clone()), "stage": stage.as_str(), "at": at, "day": dowiz_hub::stock::meta::show_day(w.days[d]),
                    "qty": qty, "out": out, "into": into, "measuredPm": measured, "expectedPm": expected,
                    "diffPm": expected.map(|x| measured - x), "lossValue": e.value.map(|val| val * (qty - out).max(0) / (*qty).max(1)),
                }));
            }
            _ => {}
        }
    }
    s
}

#[cfg(test)]
#[path = "shelf/tests.rs"]
mod tests;
