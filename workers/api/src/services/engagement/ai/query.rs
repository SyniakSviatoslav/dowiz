//! PURE. THE CLOSED LIST OF QUESTIONS THE NUMBERS ANSWER (W-AI row 2, research U6).
//!
//! A question becomes ONE [`Query`]: a kind from [`KINDS`] and at most three
//! parameters, each from a closed set. It comes from the 4-language lexicon
//! (`lexicon.rs`) or, when that does not recognise it, from a model asked to
//! pick from this list and nothing else -- and [`validate`] refuses whatever
//! the model wrote that is not on it: an unknown kind, an extra field, a window
//! that is not one, a dish the menu does not have, a weekday on a kind that has
//! none. The model never computes: `answer.rs` takes the numbers from the
//! venue's own folds.
//!
//! NO KIND IS ABOUT A PERSON. Dishes, hours, days, channels, the shelf: the
//! no-scoring invariant (`tools/gates/no-scoring.sh`) holds by construction,
//! because a question about a courier, a customer or a member of staff has no
//! kind to become.

use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Revenue,
    Orders,
    AverageOrder,
    Rejected,
    BestDish,
    WorstDish,
    DishSold,
    BusiestHour,
    QuietestHour,
    BestDay,
    WorstDay,
    FoodCost,
    Waste,
    LowStock,
    WorstMargin,
    Channels,
}

/// Every kind with its wire name: the list the model is shown and held to.
pub const KINDS: [(&str, Kind); 16] = [
    ("revenue", Kind::Revenue),
    ("orders", Kind::Orders),
    ("average_order", Kind::AverageOrder),
    ("rejected", Kind::Rejected),
    ("best_dish", Kind::BestDish),
    ("worst_dish", Kind::WorstDish),
    ("dish_sold", Kind::DishSold),
    ("busiest_hour", Kind::BusiestHour),
    ("quietest_hour", Kind::QuietestHour),
    ("best_day", Kind::BestDay),
    ("worst_day", Kind::WorstDay),
    ("food_cost", Kind::FoodCost),
    ("waste", Kind::Waste),
    ("low_stock", Kind::LowStock),
    ("worst_margin", Kind::WorstMargin),
    ("channels", Kind::Channels),
];

/// Weekdays, Monday first.
pub const WEEKDAYS: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];

/// The windows a question may ask over, in days.
pub const WINDOWS: [i64; 3] = [1, 7, 30];

impl Kind {
    pub fn name(self) -> &'static str {
        KINDS.iter().find(|(_, k)| *k == self).map_or("", |(n, _)| n)
    }
    pub fn of(name: &str) -> Option<Kind> {
        KINDS.iter().find(|(n, _)| *n == name).map(|(_, k)| *k)
    }
    /// The kinds a weekday narrows: the ones summed over days.
    pub fn takes_weekday(self) -> bool {
        matches!(self, Kind::Revenue | Kind::Orders | Kind::BestDish | Kind::WorstDish | Kind::DishSold)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Query {
    pub kind: Kind,
    pub days: i64,
    /// 0 = Monday.
    pub weekday: Option<usize>,
    /// A dish id of this venue's menu.
    pub dish: Option<String>,
}

impl Query {
    pub fn json(&self) -> Value {
        serde_json::json!({
            "query": self.kind.name(), "days": self.days,
            "weekday": self.weekday.map(|w| WEEKDAYS[w]), "dish": self.dish,
        })
    }
}

/// A dish as the picker may name it: (id, name).
pub type Dish = (String, String);

/// The model's pick, or the reason it is not one of ours.
pub fn validate(v: &Value, dishes: &[Dish]) -> Result<Query, String> {
    let o = v.as_object().ok_or("not an object")?;
    if let Some(k) = o.keys().find(|k| !["query", "days", "weekday", "dish"].contains(&k.as_str())) {
        return Err(format!("{k:?} is not a field of a query"));
    }
    let name = o.get("query").and_then(Value::as_str).ok_or("no query")?;
    let kind = Kind::of(name).ok_or_else(|| format!("{name:?} is not on the list"))?;
    let weekday = match o.get("weekday") {
        None | Some(Value::Null) => None,
        Some(Value::String(w)) => Some(WEEKDAYS.iter().position(|d| d == w).ok_or_else(|| format!("{w:?} is not a weekday"))?),
        Some(other) => return Err(format!("weekday {other} is not a weekday")),
    };
    if weekday.is_some() && !kind.takes_weekday() {
        return Err(format!("{name} has no weekday"));
    }
    let days = match o.get("days") {
        None | Some(Value::Null) => if weekday.is_some() { 30 } else { 7 },
        Some(d) => d.as_i64().filter(|d| WINDOWS.contains(d)).ok_or_else(|| format!("days {d} is not 1, 7 or 30"))?,
    };
    if weekday.is_some() && days < 7 {
        return Err("a weekday needs a window of 7 or 30 days".into());
    }
    let dish = match o.get("dish") {
        None | Some(Value::Null) => None,
        Some(Value::String(d)) => Some(dish_id(d, dishes).ok_or_else(|| format!("{d:?} is not on the menu"))?),
        Some(other) => return Err(format!("dish {other} is not a dish")),
    };
    if (kind == Kind::DishSold) != dish.is_some() {
        return Err(if dish.is_some() { format!("{name} names no dish") } else { "dish_sold needs a dish".into() });
    }
    Ok(Query { kind, days, weekday, dish })
}

/// A dish named by id or by its exact name, any case.
pub fn dish_id(said: &str, dishes: &[Dish]) -> Option<String> {
    let s = said.trim().to_lowercase();
    dishes.iter().find(|(id, name)| id.to_lowercase() == s || name.to_lowercase() == s).map(|(id, _)| id.clone())
}

/// The first `{...}` object in a model's text, parsed: models wrap JSON in
/// prose or a code fence, and only the object is read.
pub fn object_in(text: &str) -> Option<Value> {
    let a = text.find('{')?;
    let b = text.rfind('}')?;
    (b > a).then(|| serde_json::from_str(&text[a..=b]).ok()).flatten()
}

/// What the model is told when it picks: the list, the fields, and that
/// anything else is refused. The menu is named so a dish can be chosen; no
/// person is ever in it.
pub fn picker_prompt(dishes: &[Dish]) -> String {
    let kinds: Vec<&str> = KINDS.iter().map(|(n, _)| *n).collect();
    let menu: Vec<&str> = dishes.iter().take(80).map(|(_, n)| n.as_str()).collect();
    format!(
        "You map a restaurant owner's question to ONE query from a closed list. \
         Answer with ONE JSON object and nothing else: \
         {{\"query\": <one of {kinds:?}>, \"days\": <1, 7 or 30>, \"weekday\": <one of {WEEKDAYS:?} or null>, \"dish\": <a dish name from the menu or null>}}. \
         weekday only for revenue, orders, best_dish, worst_dish, dish_sold. dish only for dish_sold. \
         If the question is not one of these, answer {{\"query\": \"none\"}}. Never compute a number. \
         Menu: {menu:?}"
    )
}
