//! The kitchen's words, spoken or typed into the hub's assistant (operator Q9,
//! 2026-09-26): move a ticket, 86 a dish, receive or write off stock, "show me"
//! a screen. PURE but for `hear`, which reads only what the words need.
//!
//! THE SAME CONTRACT AS EVERY VOICE VERB (`voice.rs`): a write is a PROPOSAL
//! signed as `voice:<verb>:<arg>` that a person confirms with one tap, and the
//! confirmation's instruction goes through the SAME route the button calls,
//! which authorises again. Here, before anything is proposed, each verb is
//! asked for its capability: a ticket needs `advance`, a dish `catalog`, the
//! shelf `stock` (a write-off also `open_till`). Voice moves a hand, never a right.

use super::decide::Out;
use super::dish::{self, Dish, Miss};
use super::words::{has, norm, number, words};
use super::{grammar, say, scope};
use dowiz_hub::caps::{Cap, Caps};
use dowiz_hub::voice::{classify, Command, Speaker, Target};
use serde_json::{json, Value};

/// A member of staff who works the kitchen, not the room: their words are read
/// here rather than by the waiter's grammar.
pub fn is_kitchen(caps: &Caps) -> bool {
    !caps.allows(Cap::TakeOrders) && [Cap::Advance, Cap::Catalog, Cap::Stock].iter().any(|c| caps.allows(*c))
}

/// The one order a spoken target names, among `pool`. AMBIGUITY IS REFUSED,
/// not guessed: two orders ending in the same digits is exactly when a guess
/// moves the wrong one.
pub fn resolve(pool: &[Value], t: &Target) -> Result<String, &'static str> {
    let id = |o: &Value| o.get("id").and_then(Value::as_str).unwrap_or("").to_string();
    let at = |o: &Value| o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
    match t {
        Target::Newest => pool.iter().max_by_key(|o| at(o)).map(id).ok_or("зараз немає замовлень"),
        Target::Oldest => pool.iter().min_by_key(|o| at(o)).map(id).ok_or("зараз немає замовлень"),
        Target::Digits(d) => {
            let hits: Vec<String> = pool.iter().filter(|o| id(o).ends_with(d.as_str())).map(id).collect();
            match hits.len() {
                1 => Ok(hits[0].clone()),
                0 => Err("такого номера серед відкритих немає"),
                _ => Err("під цей номер підходить кілька — скажіть більше цифр"),
            }
        }
        Target::Unsaid => match pool.len() {
            1 => Ok(id(&pool[0])),
            0 => Err("зараз немає замовлень"),
            _ => Err("яке саме?"),
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Said {
    /// "received 4 kg salmon": a delivery onto the shelf.
    Receive { item: String, qty: i64, unit: Option<&'static str> },
    /// "write off 300 g salmon spoiled".
    Waste { item: String, qty: i64, unit: Option<&'static str>, reason: Option<&'static str> },
    /// "show me the stock": navigate, write nothing.
    Show(&'static str),
    Unclear(&'static str),
}

const RECEIVE: &[&str] = &["received", "receive", "arrived", "delivery", "прийшло", "прийшов", "прийшла", "прихід", "надійшло", "erdhi", "erdhën", "mbërriti", "mberriti", "pranova", "pranim"];
const WASTE: &[&str] = &["waste", "wasted", "binned", "write", "списати", "спиши", "списання", "викинути", "викинули", "hidh", "hodhëm", "hodhem", "shkruaj", "humbje"];
const SHOW: &[&str] = &["show", "go", "open", "покажи", "показати", "відкрий", "перейди", "trego", "hap", "shko"];
/// Screens a "show me" reaches, the words for each, and the capabilities that
/// open each (any one). Ingredients and stock are ONE screen (operator,
/// 2026-09-26), so both words land on `stock`.
const SCREENS: &[(&str, &[&str], &[Cap])] = &[
    ("kitchen", &["kitchen", "board", "tickets", "кухня", "кухню", "чеки", "kuzhina", "kuzhinën", "kuzhinen"], &[Cap::Advance]),
    ("menu", &["menu", "dishes", "меню", "страви", "menuja", "menunë", "menune", "pjatat"], &[Cap::Catalog]),
    ("stock", &["stock", "shelf", "ingredients", "склад", "інгредієнти", "магазин", "magazina", "magazinën", "magazinen", "përbërësit", "përbërës", "ingredient", "інгредієнтів"], &[Cap::Stock, Cap::Catalog]),
];
/// The five waste reasons (`dowiz_hub::stock::WasteReason`), by the words for each.
const REASONS: &[(&str, &[&str])] = &[
    ("spoiled", &["spoiled", "spoilt", "зіпсувалось", "зіпсувався", "зіпсувалася", "зіпсоване", "prishur", "prishet"]),
    ("dropped", &["dropped", "fell", "впало", "впав", "упало", "уронили", "rrëzua", "rrezua", "ra"]),
    ("unsold", &["unsold", "leftover", "непродане", "залишок", "pashitur", "mbeti"]),
    ("returned", &["returned", "return", "повернули", "повернення", "kthyer", "ktheu"]),
    ("staff_meal", &["staff", "персонал", "персоналу", "stafi", "stafit"]),
];
/// Unit words: `(word, base unit, factor to the base)`.
const UNITS: &[(&str, &str, i64)] = &[
    ("g", "g", 1), ("gr", "g", 1), ("gram", "g", 1), ("grams", "g", 1), ("г", "g", 1), ("грам", "g", 1), ("грамів", "g", 1), ("gramë", "g", 1),
    ("kg", "g", 1000), ("kilo", "g", 1000), ("кг", "g", 1000), ("кілограм", "g", 1000), ("кілограмів", "g", 1000),
    ("ml", "ml", 1), ("мл", "ml", 1), ("l", "ml", 1000), ("litre", "ml", 1000), ("liter", "ml", 1000), ("л", "ml", 1000), ("літр", "ml", 1000), ("litër", "ml", 1000),
    ("pcs", "unit", 1), ("pieces", "unit", 1), ("шт", "unit", 1), ("штук", "unit", 1), ("copë", "unit", 1), ("cope", "unit", 1),
];
const FILLER: &[&str] = &["off", "me", "the", "a", "of", "please", "мені", "будь", "ласка", "мене", "на", "të", "te", "nga", "ju", "lutem", "из", "з", "із"];
/// The most a spoken movement may carry: a large delivery, not a typo.
pub const QTY_MAX: i64 = 1_000_000;

fn qty_of(w: &str) -> Option<i64> {
    if !w.is_empty() && w.chars().all(|c| c.is_ascii_digit()) {
        return w.parse::<i64>().ok().filter(|n| (1..=QTY_MAX).contains(n));
    }
    number(w).map(i64::from)
}

/// The kitchen's shelf and navigation words, or `None` when the utterance is
/// not one of them (the order and stop-list grammars then read it).
pub fn stock_said(transcript: &str) -> Option<Said> {
    let t = norm(transcript);
    let ws = words(&t);
    let (rec, waste, show) = (has(&ws, RECEIVE), has(&ws, WASTE), has(&ws, SHOW));
    if show && !rec && !waste {
        let hits: Vec<&'static str> = SCREENS.iter().filter(|(_, w, _)| has(&ws, w)).map(|(s, ..)| *s).collect();
        return match hits.as_slice() {
            [] => None,
            [s] => Some(Said::Show(s)),
            _ => Some(Said::Unclear("more_than_one")),
        };
    }
    if !rec && !waste {
        return None;
    }
    if rec && waste {
        return Some(Said::Unclear("more_than_one"));
    }
    let reason_words: Vec<&str> = REASONS.iter().flat_map(|(_, w)| w.iter().copied()).collect();
    let (mut qty, mut unit, mut item) = (Vec::new(), None, Vec::new());
    for w in &ws {
        if RECEIVE.contains(w) || WASTE.contains(w) || FILLER.contains(w) || (waste && reason_words.contains(w)) {
            continue;
        }
        if let Some((_, base, f)) = UNITS.iter().find(|(u, ..)| u == w) {
            unit = Some((*base, *f));
        } else if let Some(n) = qty_of(w) {
            qty.push(n);
        } else {
            item.push(*w);
        }
    }
    let q = match qty.as_slice() {
        [] => return Some(Said::Unclear("how_much")),
        [n] => n.saturating_mul(unit.map_or(1, |u| u.1)),
        _ => return Some(Said::Unclear("two_numbers")),
    };
    if item.is_empty() || q > QTY_MAX {
        return Some(Said::Unclear(if item.is_empty() { "which_supply" } else { "how_much" }));
    }
    let (item, unit) = (item.join(" "), unit.map(|u| u.0));
    if rec {
        return Some(Said::Receive { item, qty: q, unit });
    }
    let reason = REASONS.iter().find(|(_, w)| has(&ws, w)).map(|(r, _)| *r);
    Some(Said::Waste { item, qty: q, unit, reason })
}

fn refuse(key: &str, lang: &str) -> Out {
    Out::Refuse(say::line(key, lang).to_string())
}

/// A supply as the matcher sees it, with its unit.
pub struct Supply {
    pub dish: Dish,
    pub unit: String,
}

/// The shelf and navigation intents, checked against the speaker's caps.
pub fn decide(said: &Said, caps: &Caps, lang: &str, shelf: &[Supply]) -> Out {
    let (item, qty, unit, reason) = match said {
        Said::Unclear(k) => return refuse(k, lang),
        Said::Show(screen) => {
            let open = SCREENS.iter().find(|(s, ..)| s == screen).is_some_and(|(.., c)| c.iter().any(|c| caps.allows(*c)));
            return if open { Out::Now(json!({ "action": "show", "screen": screen })) } else { refuse("cap_kitchen", lang) };
        }
        Said::Receive { item, qty, unit } => (item, *qty, *unit, None),
        Said::Waste { item, qty, unit, reason } => (item, *qty, *unit, Some(*reason)),
    };
    let binning = reason.is_some();
    if !(caps.allows(Cap::Stock) || (binning && caps.allows(Cap::OpenTill))) {
        return refuse("cap_kitchen", lang);
    }
    let dishes: Vec<Dish> = shelf.iter().map(|s| s.dish.clone()).collect();
    let d = match dish::find(&dishes, item) {
        Ok(d) => d,
        Err(Miss::None) => return Out::Refuse(format!("{} «{item}»", say::line("no_supply", lang))),
        Err(Miss::Many(n)) => return Out::Refuse(format!("{} {}", say::line("many_dishes", lang), n.join(", "))),
    };
    let s = shelf.iter().find(|s| s.dish.id == d.id).map(|s| s.unit.as_str()).unwrap_or("g");
    if unit.is_some_and(|u| u != s) {
        return refuse("stock_unit", lang);
    }
    let name = d.names.first().cloned().unwrap_or_else(|| d.id.clone());
    let q = qty.to_string();
    match reason {
        None => match scope::arg(&[&d.id, &q]) {
            Some(arg) => Out::Propose { verb: "receive", arg, readback: say::receive(lang, qty, s, &name), extra: json!({ "itemId": d.id }) },
            None => refuse("which_supply", lang),
        },
        Some(None) => refuse("waste_reason", lang),
        Some(Some(r)) => match scope::arg(&[&d.id, &q, r]) {
            Some(arg) => Out::Propose { verb: "waste", arg, readback: say::waste(lang, qty, s, &name, r), extra: json!({ "itemId": d.id }) },
            None => refuse("which_supply", lang),
        },
    }
}

/// An order verb from the hub's grammar, for the kitchen: `advance` or refused.
pub fn order(cmd: &Command, caps: &Caps, pool: &[Value], lang: &str) -> Out {
    match cmd {
        Command::Order { verb, target } if caps.allows(Cap::Advance) => match resolve(pool, target) {
            Err(why) => Out::Refuse(why.to_string()),
            Ok(id) => Out::Propose { verb: *verb, arg: id.clone(), readback: cmd.readback(lang), extra: json!({ "orderId": id }) },
        },
        Command::Order { .. } => refuse("cap_kitchen", lang),
        Command::Status => {
            let waiting = pool.iter().filter(|o| o.get("status").and_then(Value::as_str) == Some("PENDING")).count();
            Out::Now(json!({ "action": "status", "open": pool.len(), "waiting": waiting }))
        }
        Command::Ask(q) => Out::Now(json!({ "action": "ask", "question": q })),
        Command::Unclear(why) => Out::Refuse((*why).to_string()),
        _ => refuse("cap_kitchen", lang),
    }
}

/// A kitchen principal's utterance, read in order: shelf and screens, the stop
/// list, then the hub's order verbs. Reads the catalogue only for a dish or a
/// supply, the orders only for a ticket.
pub async fn hear(place: &crate::hubstore::Place, loc: &str, transcript: &str, lang: &str, caps: Caps) -> worker::Result<Out> {
    if let Some(s) = stock_said(transcript) {
        let shelf = match s {
            Said::Receive { .. } | Said::Waste { .. } => supplies(&crate::hubstore::load_catalog(place).await?.catalog.supplies()),
            _ => Vec::new(),
        };
        return Ok(decide(&s, &caps, lang, &shelf));
    }
    if let Some(g) = grammar::owner(transcript) {
        return Ok(match g {
            grammar::Said::DishSale { .. } if caps.allows(Cap::Catalog) => super::decide::owner(&g, lang, &super::menu::load(place, lang).await?),
            grammar::Said::Unclear(k) => refuse(k, lang),
            _ => refuse("cap_kitchen", lang),
        });
    }
    let cmd = classify(transcript, 1.0, true, Speaker::Owner);
    let pool: Vec<Value> = if matches!(cmd, Command::Order { .. } | Command::Status) {
        let listed: Vec<(String, String)> =
            crate::hubstore::orders(place).await?.into_iter().map(|o| (o.order_id, o.order_json)).collect();
        crate::services::orders::kitchen_ack::board::board(&listed, loc)
    } else {
        Vec::new()
    };
    Ok(order(&cmd, &caps, &pool, lang))
}

/// The catalogue's supplies as the matcher sees them.
pub fn supplies(rows: &[(String, String)]) -> Vec<Supply> {
    rows.iter()
        .filter_map(|(id, j)| {
            let v: Value = serde_json::from_str(j).ok()?;
            let name = v.get("name").and_then(Value::as_str).unwrap_or(id).to_string();
            let unit = v.get("unit").and_then(Value::as_str).unwrap_or("g").to_string();
            Some(Supply { dish: Dish { id: id.clone(), names: vec![name, id.clone()], available: true }, unit })
        })
        .collect()
}

#[cfg(test)]
mod tests;
