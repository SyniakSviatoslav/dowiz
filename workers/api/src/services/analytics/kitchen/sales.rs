//! THE THEORETICAL SIDE, pure: what the recipes say the sales used. Each sold
//! portion of a dish draws its recipe's lines -- gross, net and out -- on the
//! day the order was placed (the venue's day), and costs what those grams cost
//! at the average a priced delivery set (else the owner's list price).
//!
//! Every number is a sum of order lines x recipe lines; nothing is stored.

use dowiz_hub::stock::cost::CostBook;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

/// A supply as the report needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Supply {
    pub id: String,
    pub name: String,
    pub unit: String,
    /// 100 for g/ml, 1 for pieces: what the list price is per.
    pub basis: i64,
    /// The owner's typed list price per basis, minor units.
    pub list_cost: Option<i64>,
    pub clean_pm: i64,
    pub cook_pm: i64,
}

/// One recipe line: gross in the supply's base unit, and the three weights.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DishLine {
    pub supply: String,
    pub qty: i64,
    pub gross_g: Option<i64>,
    pub net_g: Option<i64>,
    pub out_g: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dish {
    pub id: String,
    pub name: String,
    pub lines: Vec<DishLine>,
}

/// The days of the report, local midnights oldest first, and where it ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    pub starts: Vec<i64>,
    pub end: i64,
    /// The same days as `yyyymmdd`, for the screen.
    pub days: Vec<i64>,
}

impl Window {
    /// Which day `at` belongs to, or `None` outside the window.
    pub fn bucket(&self, at: i64) -> Option<usize> {
        if self.starts.is_empty() || at < self.starts[0] || at >= self.end {
            return None;
        }
        self.starts.iter().rposition(|s| *s <= at)
    }
}

/// What `qty` base units of `s` cost: the average now, else the list price.
pub fn cost_of(s: &Supply, book: &CostBook, qty: i64) -> Option<i64> {
    book.value_of(&s.id, qty).or_else(|| s.list_cost.map(|c| dowiz_hub::stock::journal::priced(qty, c, s.basis).unwrap_or(0)))
}

/// One dish's row.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DishRow {
    pub sold: i64,
    pub revenue: i64,
    pub by_day: Vec<i64>,
}

/// One ingredient's theoretical use, grams by stage and base units by day.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Use {
    /// Base units (what the ledger counts), per day.
    pub by_day: Vec<i64>,
    pub qty: i64,
    pub gross_g: i64,
    pub net_g: i64,
    pub out_g: i64,
}

/// One day's sales.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DayRow {
    pub orders: i64,
    /// Food sales: Σ unit price x quantity of accepted orders.
    pub revenue: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sales {
    pub days: Vec<DayRow>,
    pub dishes: BTreeMap<String, DishRow>,
    pub uses: BTreeMap<String, Use>,
    /// Every order's placement instant, for dating the stock log's draws.
    pub placed_at: HashMap<String, i64>,
    /// Sold lines whose product has no recipe (so no ingredient is known).
    pub unmodelled: i64,
}

/// Fold the venue's orders over the window.
pub fn fold(orders: &[Value], dishes: &HashMap<String, Dish>, w: &Window) -> Sales {
    let n = w.starts.len();
    let mut s = Sales { days: vec![DayRow::default(); n], ..Sales::default() };
    for o in orders {
        let at = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        if let Some(id) = o.get("id").and_then(Value::as_str) {
            s.placed_at.insert(id.to_string(), at);
        }
        let Some(d) = w.bucket(at) else { continue };
        let st = o.get("status").and_then(Value::as_str).unwrap_or("");
        if !crate::services::orders::status::took_money(st) {
            continue;
        }
        s.days[d].orders += 1;
        for it in o.get("items").and_then(Value::as_array).into_iter().flatten() {
            let Some(pid) = it.get("product_id").and_then(Value::as_str).filter(|p| !p.is_empty()) else { continue };
            let q = it.get("quantity").and_then(Value::as_i64).unwrap_or(0).max(0);
            let money = it.get("unit_price").and_then(Value::as_i64).unwrap_or(0) * q;
            s.days[d].revenue += money;
            let row = s.dishes.entry(pid.to_string()).or_insert_with(|| DishRow { by_day: vec![0; n], ..DishRow::default() });
            row.sold += q;
            row.revenue += money;
            row.by_day[d] += q;
            let Some(dish) = dishes.get(pid).filter(|x| !x.lines.is_empty()) else {
                s.unmodelled += q;
                continue;
            };
            for l in &dish.lines {
                let u = s.uses.entry(l.supply.clone()).or_insert_with(|| Use { by_day: vec![0; n], ..Use::default() });
                u.by_day[d] += l.qty * q;
                u.qty += l.qty * q;
                u.gross_g += l.gross_g.unwrap_or(0) * q;
                u.net_g += l.net_g.unwrap_or(0) * q;
                u.out_g += l.out_g.unwrap_or(0) * q;
            }
        }
    }
    s
}

/// One portion's cost: every line's gross at its cost; `None` unless every
/// line has one (a partial sum is not a cost).
pub fn portion_cost(dish: &Dish, supplies: &HashMap<String, Supply>, book: &CostBook) -> Option<i64> {
    if dish.lines.is_empty() {
        return None;
    }
    dish.lines.iter().map(|l| supplies.get(&l.supply).and_then(|s| cost_of(s, book, l.qty))).sum()
}

#[cfg(test)]
#[path = "sales/tests.rs"]
mod tests;
