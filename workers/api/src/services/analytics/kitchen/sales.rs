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
    /// THE RAW ITEMS ONE PORTION TAKES, in millionths of each one's base
    /// unit, when a line names a semi-finished product (W-PF2 R3,
    /// `dowiz_hub::prep::expand`): the ingredient use is counted from these,
    /// not from the ПФ line. Empty for a recipe of raw lines only.
    pub leaves: Vec<(String, i64)>,
}

const MICRO: i64 = dowiz_hub::prep::MICRO;

/// Millionths to whole base units, half up.
fn whole(micro: i64) -> i64 {
    (micro + MICRO / 2).div_euclid(MICRO)
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
    /// Portions whose line carries the cost stamped at placement (R4), and
    /// what they cost then, in total and per day. The rest fall back to
    /// today's average -- every order placed before stamps existed.
    pub stamped: i64,
    pub stamped_cogs: i64,
    pub stamped_by_day: Vec<i64>,
    pub stamped_cogs_by_day: Vec<i64>,
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
    /// Leaf draws not yet rounded (millionths), whole and per day: a ПФ's
    /// fraction of a gram is summed over the window and rounded ONCE.
    pub micro: i64,
    pub micro_by_day: Vec<i64>,
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
            let row = s.dishes.entry(pid.to_string()).or_insert_with(|| DishRow {
                by_day: vec![0; n], stamped_by_day: vec![0; n], stamped_cogs_by_day: vec![0; n], ..DishRow::default()
            });
            row.sold += q;
            row.revenue += money;
            row.by_day[d] += q;
            if let Some(c) = crate::command::place::cost::line_cost(it, q) {
                row.stamped += q;
                row.stamped_cogs += c;
                row.stamped_by_day[d] += q;
                row.stamped_cogs_by_day[d] += c;
            }
            let Some(dish) = dishes.get(pid).filter(|x| !x.lines.is_empty()) else {
                s.unmodelled += q;
                continue;
            };
            for (item, uq) in &dish.leaves {
                let u = s.uses.entry(item.clone()).or_insert_with(|| Use { by_day: vec![0; n], micro_by_day: vec![0; n], ..Use::default() });
                u.micro += uq * q;
                u.micro_by_day[d] += uq * q;
            }
            for l in dish.lines.iter().filter(|_| dish.leaves.is_empty()) {
                let u = s.uses.entry(l.supply.clone()).or_insert_with(|| Use { by_day: vec![0; n], micro_by_day: vec![0; n], ..Use::default() });
                u.by_day[d] += l.qty * q;
                u.qty += l.qty * q;
                u.gross_g += l.gross_g.unwrap_or(0) * q;
                u.net_g += l.net_g.unwrap_or(0) * q;
                u.out_g += l.out_g.unwrap_or(0) * q;
            }
        }
    }
    // The leaves' millionths, rounded once over the window. Grams through a
    // ПФ are its raw grams: no cleaning or cooking loss at the dish (the
    // batch's loss is the production act's, `shelf::fold` "yields").
    for u in s.uses.values_mut() {
        let g = whole(u.micro);
        u.qty += g;
        u.gross_g += g;
        u.net_g += g;
        u.out_g += g;
        for (d, m) in u.micro_by_day.iter().enumerate() {
            u.by_day[d] += whole(*m);
        }
    }
    s
}

/// A dish's cost of goods over the window, whole and per day: the stamped
/// portions at what they cost when sold, the rest at `today` (a portion's
/// cost now). `None` for the whole when some portions have no cost either way.
pub fn cogs_of(row: &DishRow, today: Option<i64>) -> (Option<i64>, Vec<i64>) {
    let rest = row.sold - row.stamped;
    let by_day = (0..row.by_day.len())
        .map(|d| {
            let at = |v: &Vec<i64>| v.get(d).copied().unwrap_or(0);
            at(&row.stamped_cogs_by_day) + today.map_or(0, |c| c * (row.by_day[d] - at(&row.stamped_by_day)))
        })
        .collect();
    let whole = match today {
        Some(c) => Some(row.stamped_cogs + c * rest),
        None => (rest == 0).then_some(row.stamped_cogs),
    };
    (whole, by_day)
}

/// One portion's cost: every line's gross at its cost; `None` unless every
/// line has one (a partial sum is not a cost).
pub fn portion_cost(dish: &Dish, supplies: &HashMap<String, Supply>, book: &CostBook) -> Option<i64> {
    if dish.lines.is_empty() {
        return None;
    }
    if !dish.leaves.is_empty() {
        // Exact through the tree: Σ millionths × the average, rounded once.
        let mut sum: i128 = 0;
        for (item, uq) in &dish.leaves {
            let s = supplies.get(item)?;
            let per = match book.avg_micro(item) {
                Some(a) => a,
                None => i128::from(s.list_cost?) * i128::from(MICRO) / i128::from(s.basis.max(1)),
            };
            sum += per * i128::from(*uq);
        }
        let m2 = i128::from(MICRO) * i128::from(MICRO);
        return i64::try_from((sum + m2 / 2) / m2).ok();
    }
    dish.lines.iter().map(|l| supplies.get(&l.supply).and_then(|s| cost_of(s, book, l.qty))).sum()
}

#[cfg(test)]
#[path = "sales/tests.rs"]
mod tests;
