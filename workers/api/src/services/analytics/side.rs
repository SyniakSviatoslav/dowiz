//! KITCHEN OR DRINKS (W-DRINKS-KITCHEN, operator 2026-11-04: "розділи
//! аналітику на напої та кухню"). PURE: which SIDE of the house a line was
//! sold by, so the owner reads the sushi business and the bar business as
//! two numbers instead of one average dragged down by 79-lek coffees.
//!
//! THE RULE, in the order it is asked, and the first answer wins:
//!   1. A DISH THE CATALOGUE KNOWS takes its CATEGORY's side. A category's
//!      side is the flag the owner set on it (`side: "kitchen" | "drinks"`,
//!      the categories sheet), else `category_default`: a name carrying a
//!      drinks word in any of the venue's languages is drinks, EVERYTHING
//!      ELSE IS KITCHEN. An old record without the flag reads by this
//!      default; the catalogue is never rewritten.
//!   2. A TILL LINE NO DISH ANSWERS TO takes the till's own item category
//!      (`ebills_category`, measured 2026-11-04: ebills carries one on every
//!      sale record -- "Sushi", "alkool", "kafeteria", "freskuese",
//!      "Cocktail", "birra", "Lengje Frutash", "sweet", "Shisha"), read by
//!      the same category rule.
//!   3. A line with neither (imported before the till category was kept, or
//!      a dish since deleted) is read by its NAME: a drinks word is drinks, a
//!      kitchen word is kitchen, and a name with NEITHER is UNCLASSIFIED --
//!      shown with its money, never put on a side by a guess.
//!
//! Words are matched on `ebills::state::norm`'s spelling (case, diacritics
//! and punctuation folded), whole tokens or the stems below, so "Kokteje",
//! "Freskuese" and "напої" are drinks without a list of every inflection.
//! "sake" is NOT a drinks word: on a sushi menu it is the salmon.

use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Kitchen,
    Drinks,
}

impl Side {
    pub fn key(self) -> &'static str {
        match self {
            Side::Kitchen => "kitchen",
            Side::Drinks => "drinks",
        }
    }
    pub fn parse(s: &str) -> Option<Side> {
        match s {
            "kitchen" => Some(Side::Kitchen),
            "drinks" => Some(Side::Drinks),
            _ => None,
        }
    }
}

/// Whole tokens that say "drinks": sq, en, uk, ru, and the drinks a till
/// rings up by their own name (the 38 unmatched names of 2026-11-04 among them).
const DRINK_WORDS: &[&str] = &[
    // sq
    "pije", "pijet", "kafe", "kafene", "kafeteria", "caj", "cajra", "birra", "birre", "vere", "verera", "koktej", "kokteje", "nargjile",
    "leng", "lengje", "uje", "ujera", "freskuese", "alkool", "alkoolike", "limonade", "analkolik", "analcolic",
    // en
    "drink", "drinks", "bar", "coffee", "coffees", "tea", "teas", "beer", "beers", "wine", "wines", "cocktail", "cocktails", "shisha",
    "hookah", "juice", "juices", "water", "soda", "sodas", "spirits", "alcohol", "alcoholic", "lemonade", "smoothie", "smoothies",
    "milkshake", "milkshakes", "beverage", "beverages", "milk",
    // uk, ru
    "напої", "напій", "напитки", "напиток", "бар", "кава", "кофе", "чай", "пиво", "вино", "коктейль", "коктейлі", "коктейли",
    "кальян", "сік", "соки", "сок", "вода", "лимонад", "алкоголь", "безалкогольні", "безалкогольные",
    // what a till names a drink by
    "espresso", "makiato", "macchiato", "cappuccino", "capucino", "latte", "americano", "mocha", "frappe", "salep", "glintwine",
    "gluhwein", "punsh", "punch", "mojito", "martini", "negroni", "spritz", "aperol", "prosecco", "pinacolada", "margarita",
    "daiquiri", "whisky", "whiskey", "vodka", "gin", "rum", "tequila", "raki", "rakia", "cola", "pepsi", "fanta", "sprite",
    "schweppes", "tonic", "redbull", "lajthiza", "peroni", "paulaner", "heineken", "korca", "stella", "corona", "amstel",
    "erdinger", "tuborg",
];
/// Stems a drinks token may start with (inflections: "kokteje", "freskuese", "напоїв").
const DRINK_STEMS: &[&str] = &["kokt", "cocktail", "fresk", "alkoo", "alcoh", "birr", "leng", "coffe", "напо", "напит", "коктей", "безалког", "nargj", "shish", "hook"];

/// Whole tokens that say "kitchen", asked only of a NAME (rule 3).
const KITCHEN_WORDS: &[&str] = &[
    "sushi", "roll", "rolls", "role", "maki", "uramaki", "futomaki", "nigiri", "sashimi", "temaki", "gunkan", "kimbap", "bowl", "bowls",
    "poke", "salad", "sallate", "sallat", "салат", "soup", "supe", "supa", "суп", "ramen", "udon", "noodle", "noodles", "wok", "tempura",
    "gyoza", "dumpling", "dumplings", "burger", "burgers", "pizza", "pasta", "rice", "oriz", "рис", "chips", "patate", "fries", "kartof",
    "картопля", "картофель", "shrimp", "ebi", "tuna", "maguro", "salmon", "crab", "kani", "avocado", "croissant", "dessert", "embelsire",
    "десерт", "cake", "torte", "торт", "cheesecake", "tiramisu", "mochi", "sous", "sauce", "salce", "соус", "wasabi", "edamame", "miso",
    "yum", "philadelphia", "california", "volcano", "dragon", "crown", "set", "sete", "snack", "snacks", "kuzhine", "kitchen", "кухня",
];
const KITCHEN_STEMS: &[&str] = &["burger", "sush", "nigir", "sashim", "tempur", "gyoz", "sallat", "салат", "суп", "ushqim", "food", "їж", "еда"];

fn has(name: &str, words: &[&str], stems: &[&str]) -> bool {
    crate::ebills::state::norm(name)
        .split(' ')
        .filter(|w| !w.is_empty())
        .any(|w| words.contains(&w) || stems.iter().any(|s| w.starts_with(s)))
}

/// A CATEGORY's side from its name alone: drinks words, else kitchen.
pub fn category_default(name: &str) -> Side {
    if has(name, DRINK_WORDS, DRINK_STEMS) {
        Side::Drinks
    } else {
        Side::Kitchen
    }
}

/// A NAME's side (rule 3): drinks words, kitchen words, or nothing.
pub fn of_words(name: &str) -> Option<Side> {
    if has(name, DRINK_WORDS, DRINK_STEMS) {
        Some(Side::Drinks)
    } else if has(name, KITCHEN_WORDS, KITCHEN_STEMS) {
        Some(Side::Kitchen)
    } else {
        None
    }
}

/// A stored category record's side: the owner's flag, else the default. An
/// old record (no `side`) reads by its name -- the decoder supplies it.
pub fn of_category(rec: &Value) -> Side {
    rec.get("side")
        .and_then(Value::as_str)
        .and_then(Side::parse)
        .unwrap_or_else(|| category_default(rec.get("name").and_then(Value::as_str).unwrap_or("")))
}

/// Whether the owner set the flag (the sheet shows a default differently).
pub fn flag_set(rec: &Value) -> bool {
    rec.get("side").and_then(Value::as_str).and_then(Side::parse).is_some()
}

/// The venue's answer for every dish it sells: product id -> side, built
/// once per read from the catalogue's categories and products.
#[derive(Debug, Default, Clone)]
pub struct Sides {
    by_product: BTreeMap<String, Side>,
}

impl Sides {
    pub fn of(cat: &dowiz_hub::catalog::Catalog) -> Sides {
        let cats: BTreeMap<String, Side> = cat
            .categories()
            .into_iter()
            .map(|(id, j)| (id, of_category(&serde_json::from_str::<Value>(&j).unwrap_or(Value::Null))))
            .collect();
        let mut by_product = BTreeMap::new();
        for (id, j) in cat.products() {
            let p: Value = serde_json::from_str(&j).unwrap_or(Value::Null);
            // `categoryId` is what the editor and the importer write; `category_id` what older fixtures did.
            let cid = p.get("categoryId").or_else(|| p.get("category_id")).and_then(Value::as_str).unwrap_or("");
            let side = cats.get(cid).copied().unwrap_or_else(|| category_default(cid));
            by_product.insert(id, side);
        }
        Sides { by_product }
    }

    /// The three rules, in order (module doc).
    pub fn of_line(&self, product_id: Option<&str>, name: &str, till_category: Option<&str>) -> Option<Side> {
        if let Some(s) = product_id.and_then(|p| self.by_product.get(p)) {
            return Some(*s);
        }
        if let Some(c) = till_category.filter(|c| !c.trim().is_empty()) {
            return Some(category_default(c));
        }
        of_words(name)
    }

    /// One order line as the log holds it (`map::line`, `import::decide`).
    pub fn of_item(&self, it: &Value) -> Option<Side> {
        let s = |k: &str| it.get(k).and_then(Value::as_str);
        self.of_line(s("product_id").filter(|p| !p.is_empty()), s("name").unwrap_or(""), s("ebills_category"))
    }

    #[cfg(test)]
    pub fn with(pairs: &[(&str, Side)]) -> Sides {
        Sides { by_product: pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect() }
    }
}

/// THE ORDER'S GROUP: a check with any kitchen line is a kitchen check; one
/// whose every line is a drink is a drinks-only check; a check with an
/// unclassified line and no kitchen line (or no line) is unclassified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Kitchen,
    DrinksOnly,
    Unclassified,
}

pub fn group(sides: impl IntoIterator<Item = Option<Side>>) -> Group {
    let (mut any, mut kitchen, mut unknown) = (false, false, false);
    for s in sides {
        any = true;
        match s {
            Some(Side::Kitchen) => kitchen = true,
            Some(Side::Drinks) => {}
            None => unknown = true,
        }
    }
    match (any, kitchen, unknown) {
        (_, true, _) => Group::Kitchen,
        (true, false, false) => Group::DrinksOnly,
        _ => Group::Unclassified,
    }
}

#[cfg(test)]
#[path = "side/tests.rs"]
mod tests;
