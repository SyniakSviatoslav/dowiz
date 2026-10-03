//! UNEXPLAINED LOSS BETWEEN TWO COUNTS (W-STOCK P4), pure: the honest
//! actual-versus-theoretical, per supply, valued in the venue's money.
//!
//! Between two counts of one supply:
//!
//!   actual       = opening count + what came onto the shelf - closing count
//!   theoretical  = what the sales drew by their recipes (an order's draw at
//!                  PREPARING, a till sale net of its voids)
//!   explained    = write-offs + prep: cooked into a batch, moved into another supply
//!   unexplained  = actual - theoretical - explained
//!
//! "Came onto the shelf" is a delivery, food resold from a door, a batch made,
//! prep that came out as this supply. Every term is a record in the log, and
//! the sum closes: `unexplained` IS minus the closing count's drift (a test
//! holds it), so this never invents a loss the ledger did not see -- it says
//! where the rest went.
//!
//! A SUPPLY NO RECIPE NAMES IS NOT TRACKED, NEVER A LOSS: nothing sold could
//! have drawn it, so its whole use would read as "unexplained". It is listed
//! apart. A supply counted once has no window yet: "count twice to see this",
//! never a zero.

use std::collections::BTreeMap;

use dowiz_hub::stock::journal::Entry;
use dowiz_hub::stock::{moved_into, StockEvent};
use serde_json::{json, Value};

/// A window whose unexplained loss is above this per mille of the window's
/// food sales is flagged.
pub const FLAG_PM: i64 = 30;
/// How many losses the screen and the digest lead with.
pub const TOP: usize = 5;
/// Past windows needed before "unusual for this supply" (2 sigma) is said.
pub const HISTORY_MIN: usize = 3;
/// Records kept per window for the screen.
pub const RECORDS: usize = 20;

/// One supply between two of its counts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Window {
    pub opening: i64,
    pub opened_at: Option<i64>,
    pub closing: i64,
    pub closed_at: Option<i64>,
    /// Onto the shelf: deliveries, resold returns, batches made, prep in.
    pub received: i64,
    /// Theoretical: recipe draws of the sales.
    pub sold: i64,
    pub wasted: i64,
    /// Cooked into a batch, or moved into another supply by prep.
    pub prep: i64,
    /// The closing count's drift at the average then (signed), if priced.
    pub drift_value: Option<i64>,
    /// Indices into the entries: the window's records, the closing count last.
    pub records: Vec<usize>,
}

impl Window {
    pub fn actual(&self) -> i64 {
        self.opening + self.received - self.closing
    }
    pub fn unexplained(&self) -> i64 {
        self.actual() - self.sold - self.wasted - self.prep
    }
}

/// Every supply's windows between consecutive counts, oldest first. One pass.
/// A deletion (`Removed`) ends the supply's open window without closing it.
pub fn windows(entries: &[Entry]) -> BTreeMap<String, Vec<Window>> {
    let mut open: BTreeMap<String, Window> = BTreeMap::new();
    let mut done: BTreeMap<String, Vec<Window>> = BTreeMap::new();
    for (i, e) in entries.iter().enumerate() {
        let item = e.ev.item().to_string();
        let mut add = |id: &str, f: &dyn Fn(&mut Window)| {
            if let Some(w) = open.get_mut(id) {
                f(w);
                if w.records.len() < RECORDS {
                    w.records.push(i);
                }
            }
        };
        match &e.ev {
            StockEvent::Stocktake { observed, .. } => {
                if let Some(mut w) = open.remove(&item) {
                    w.closing = *observed;
                    w.closed_at = e.meta.at;
                    w.drift_value = e.value;
                    w.records.push(i);
                    done.entry(item.clone()).or_default().push(w);
                }
                open.insert(item, Window { opening: *observed, opened_at: e.meta.at, ..Window::default() });
            }
            StockEvent::Removed { .. } => {
                open.remove(&item);
            }
            StockEvent::Received { qty, .. } | StockEvent::Made { qty, .. } | StockEvent::Returned { qty, resell: true, .. } => {
                add(&item, &|w: &mut Window| w.received += *qty)
            }
            StockEvent::Consumed { qty, .. } | StockEvent::Served { qty, .. } => add(&item, &|w: &mut Window| w.sold += *qty),
            StockEvent::Unserved { qty, .. } => add(&item, &|w: &mut Window| w.sold -= *qty),
            StockEvent::Wasted { qty, .. } => add(&item, &|w: &mut Window| w.wasted += *qty),
            StockEvent::Cooked { qty, .. } => add(&item, &|w: &mut Window| w.prep += *qty),
            StockEvent::Produced { qty, out, into, .. } => {
                if let Some(to) = moved_into(&item, into).map(str::to_string) {
                    add(&item, &|w: &mut Window| w.prep += *qty);
                    add(&to, &|w: &mut Window| w.received += *out);
                }
            }
            StockEvent::Reserved { .. } | StockEvent::Released { .. } | StockEvent::Returned { .. } => {}
        }
    }
    done
}

/// Is `x` more than two standard deviations from `past`? Exact integers:
/// |x - mean| > 2 sigma  <=>  (n x - sum)^2 > 4 (n sumsq - sum^2).
pub fn unusual(x: i64, past: &[i64]) -> bool {
    if past.len() < HISTORY_MIN {
        return false;
    }
    let n = past.len() as i128;
    let sum: i128 = past.iter().map(|v| i128::from(*v)).sum();
    let sumsq: i128 = past.iter().map(|v| i128::from(*v) * i128::from(*v)).sum();
    let d = n * i128::from(x) - sum;
    d * d > 4 * (n * sumsq - sum * sum)
}

/// What the report needs to know beside the log.
pub struct Ctx<'a> {
    /// A recipe (directly, or through a semi-finished card) names this supply.
    pub linked: &'a dyn Fn(&str) -> bool,
    pub name: &'a dyn Fn(&str) -> Option<(String, String)>,
    /// `qty` of a supply at its list price, for a count no priced delivery valued.
    pub list_value: &'a dyn Fn(&str, i64) -> Option<i64>,
    /// The window's food sales, minor units (`None`: not known here).
    pub revenue: Option<i64>,
    /// Is this instant inside the report's window?
    pub in_window: &'a dyn Fn(i64) -> bool,
    /// One record as the screen shows it.
    pub record: &'a dyn Fn(&Entry) -> Value,
}

/// The value of a window's unexplained quantity: minus the closing count's
/// drift value (the average at that count), else the list price.
pub fn value_of(w: &Window, id: &str, list_value: &dyn Fn(&str, i64) -> Option<i64>) -> Option<i64> {
    let u = w.unexplained();
    w.drift_value.map(|v| -v).or_else(|| list_value(id, u.abs()).map(|v| if u < 0 { -v } else { v }))
}

/// The `avt` block: the latest window of each tracked supply that CLOSED in
/// the report's window, biggest loss in money first; who must count twice;
/// what no recipe tracks.
pub fn report(entries: &[Entry], cx: &Ctx) -> Value {
    let all = windows(entries);
    let mut counted: Vec<String> = Vec::new();
    for e in entries {
        if let StockEvent::Stocktake { item, .. } = &e.ev {
            if !counted.contains(item) {
                counted.push(item.clone());
            }
        }
    }
    counted.sort();
    let named = |id: &str| (cx.name)(id).unwrap_or_else(|| (id.to_string(), String::new()));
    let (mut rows, mut twice, mut untracked) = (Vec::new(), Vec::new(), Vec::new());
    for id in &counted {
        let (name, unit) = named(id);
        if !(cx.linked)(id) {
            untracked.push(json!({ "id": id, "name": name }));
            continue;
        }
        let Some(ws) = all.get(id).filter(|ws| !ws.is_empty()) else {
            twice.push(json!({ "id": id, "name": name }));
            continue;
        };
        let Some(k) = ws.iter().rposition(|w| w.closed_at.is_some_and(|a| (cx.in_window)(a))) else { continue };
        let w = &ws[k];
        let past: Vec<i64> = ws[..k].iter().map(Window::unexplained).collect();
        let value = value_of(w, id, cx.list_value);
        let pm = match (value, cx.revenue) {
            (Some(v), Some(r)) if r > 0 => Some(v * 1000 / r),
            _ => None,
        };
        let mut flags: Vec<&str> = Vec::new();
        if pm.is_some_and(|p| p > FLAG_PM) {
            flags.push("over30pm");
        }
        if unusual(w.unexplained(), &past) {
            flags.push("unusual");
        }
        rows.push(json!({
            "id": id, "name": name, "unit": unit, "from": w.opened_at, "to": w.closed_at,
            "opening": w.opening, "received": w.received, "sold": w.sold, "wasted": w.wasted, "prep": w.prep,
            "closing": w.closing, "actual": w.actual(), "unexplained": w.unexplained(), "value": value,
            "revenuePm": pm, "flags": flags, "windowsBefore": past.len(),
            "records": w.records.iter().map(|i| (cx.record)(&entries[*i])).collect::<Vec<_>>(),
        }));
    }
    rows.sort_by(|a, b| {
        let v = |r: &Value| r["value"].as_i64().unwrap_or(i64::MIN);
        v(b).cmp(&v(a)).then(b["unexplained"].as_i64().cmp(&a["unexplained"].as_i64())).then(a["id"].as_str().cmp(&b["id"].as_str()))
    });
    let top: Vec<Value> = rows.iter().filter(|r| r["unexplained"].as_i64().unwrap_or(0) > 0).take(TOP).map(|r| r["id"].clone()).collect();
    json!({ "rows": rows, "top": top, "countTwice": twice, "notTracked": untracked, "revenue": cx.revenue, "thresholdPm": FLAG_PM })
}

#[cfg(test)]
#[path = "avt/tests.rs"]
mod tests;
