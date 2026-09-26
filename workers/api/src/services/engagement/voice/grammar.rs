//! PURE. What a waiter said, and the owner's two extra verbs (a dish on or
//! off sale, the venue's state), as a closed set of intents.
//!
//! THE SAME RULES AS `dowiz_hub::voice`: deterministic, three languages, two
//! commands in one breath refused rather than resolved by precedence, and a
//! missing piece (which table? cash or card?) asked for rather than guessed.
//! Nothing here knows the menu or the room; `dish` and `room` resolve the
//! words against them, and nothing here acts.

use super::words::{has, norm, number, words};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Cash,
    Card,
}

impl Method {
    /// The wire word the pay route takes (`room/logic.js` METHODS).
    pub fn as_str(self) -> &'static str {
        match self {
            Method::Cash => "cash",
            Method::Card => "card",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Said {
    /// "open table 5 for 4": the phone starts a round for that table.
    Open { table: u32, guests: Option<u32> },
    /// "add 2 margherita to table 5". No table = the round being built.
    Add { dish: String, qty: u32, table: Option<u32> },
    /// "send the round": the round being built goes to the kitchen.
    Send { table: Option<u32> },
    /// "table 5 paid cash".
    Paid { table: u32, method: Method },
    Status,
    /// The owner's stop-list, by voice.
    DishSale { dish: String, on: bool },
    /// The owner's header chip: open | busy | closed.
    Venue { state: &'static str },
    /// Heard, not understood: a key for `say::line`.
    Unclear(&'static str),
}

const TABLE: &[&str] = &[
    "table", "стіл", "столу", "стола", "столик", "столика", "столі", "tavolina", "tavolinën",
    "tavolinen", "tavolinës", "tavolines", "tavolinë", "tavoline",
];
const OPEN: &[&str] = &["open", "seat", "відкрий", "відкрити", "посади", "посадити", "hap", "hape"];
const ADD: &[&str] = &["add", "plus", "додай", "додати", "плюс", "shto", "shtoj", "shtoni"];
const SEND: &[&str] = &["send", "fire", "відправ", "відправити", "надішли", "надіслати", "dërgo", "dergo", "dërgoje", "dergoje"];
const PAID: &[&str] = &[
    "paid", "pay", "pays", "settled", "оплатив", "оплатила", "оплатили", "оплата", "оплачено",
    "розрахувався", "розрахувалась", "розрахувались", "paguar", "pagoi", "paguan", "pagesë", "pagese",
];
const CASH: &[&str] = &["cash", "готівка", "готівкою", "готівку", "кеш", "кешем", "kesh", "para"];
const CARD: &[&str] = &["card", "карта", "картка", "карткою", "картою", "карту", "картку", "kartë", "karte", "kartën", "karten", "kartelë", "kartele"];
const STATUS: &[&str] = &["status", "statusi", "статус", "скільки", "many", "sa"];
/// Words that carry no meaning of their own between the ones that do.
const FILLER: &[&str] = &[
    "to", "at", "on", "for", "the", "a", "an", "please", "of", "times", "portion", "portions", "more",
    "round", "order", "and", "guests", "people", "pax",
    "до", "на", "у", "в", "за", "для", "будь", "ласка", "порції", "порцію", "порцій", "штуки",
    "штук", "шт", "рази", "раз", "ще", "раунд", "замовлення", "гостей", "гості", "осіб", "людей",
    "në", "ne", "te", "tek", "për", "per", "ju", "lutem", "copë", "cope", "porcion", "porcione",
    "herë", "here", "edhe", "porosinë", "porosine", "porosia", "persona", "veta", "vetë",
];

/// The number said right after a table word, and the word's position.
/// `Some(None)` = a table word with no number behind it.
fn table_of(ws: &[&str]) -> Option<(usize, Option<u32>)> {
    let i = ws.iter().position(|w| TABLE.contains(w))?;
    Some((i, ws.get(i + 1).and_then(|w| number(w))))
}

/// A waiter's utterance.
pub fn waiter(transcript: &str) -> Said {
    let t = norm(transcript);
    let ws = words(&t);
    if ws.is_empty() {
        return Said::Unclear("nothing");
    }
    let (open, add, send, paid) = (has(&ws, OPEN), has(&ws, ADD), has(&ws, SEND), has(&ws, PAID));
    if [open, add, send, paid].iter().filter(|x| **x).count() > 1 {
        return Said::Unclear("more_than_one");
    }
    let table = table_of(&ws);
    if matches!(table, Some((_, None))) {
        return Said::Unclear("which_table");
    }
    let n = table.and_then(|(_, n)| n);
    if paid {
        let (cash, card) = (has(&ws, CASH), has(&ws, CARD));
        return match (n, cash, card) {
            (None, ..) => Said::Unclear("which_table"),
            (Some(table), true, false) => Said::Paid { table, method: Method::Cash },
            (Some(table), false, true) => Said::Paid { table, method: Method::Card },
            _ => Said::Unclear("cash_or_card"),
        };
    }
    if send {
        return Said::Send { table: n };
    }
    if open {
        let Some((at, Some(table))) = table else { return Said::Unclear("which_table") };
        // The guests are the first number AFTER the table's own.
        let guests = ws.iter().skip(at + 2).find_map(|w| number(w));
        return Said::Open { table, guests };
    }
    if add {
        let skip = table.map(|(at, _)| [at, at + 1]);
        let mut qty: Vec<u32> = Vec::new();
        let mut dish: Vec<&str> = Vec::new();
        for (i, w) in ws.iter().enumerate() {
            if skip.is_some_and(|s| s.contains(&i)) || ADD.contains(w) || FILLER.contains(w) || *w == "x" || *w == "х" {
                continue;
            }
            match number(w) {
                Some(q) => qty.push(q),
                None => dish.push(w),
            }
        }
        if qty.len() > 1 {
            return Said::Unclear("two_numbers");
        }
        if dish.is_empty() {
            return Said::Unclear("which_dish");
        }
        return Said::Add { dish: dish.join(" "), qty: qty.first().copied().unwrap_or(1), table: n };
    }
    if has(&ws, STATUS) {
        return Said::Status;
    }
    Said::Unclear("not_room")
}

const VENUE: &[&str] = &["venue", "restaurant", "заклад", "закладу", "ресторан", "ресторану", "lokali", "lokalin", "lokal", "restoranti", "restorantin"];
const V_OPEN: &[&str] = &["open", "reopen", "відкрий", "відкрити", "відкритий", "відкрито", "hap", "hape", "hapur"];
const V_BUSY: &[&str] = &["busy", "зайнятий", "зайнято", "завантажений", "завантажено", "zënë", "zene", "ngarkuar"];
const V_CLOSED: &[&str] = &["close", "closed", "закрий", "закрити", "закритий", "закрито", "mbyll", "mbylle", "mbyllur"];
const OFF: &[&str] = &[
    "off", "86", "unavailable", "stop", "стоп", "зніми", "зняти", "закінчилась", "закінчився",
    "закінчилось", "закінчились", "скінчилась", "скінчився", "скінчилось", "hiq", "hiqe", "mbaroi", "mbaruan", "ndalo",
];
const ON: &[&str] = &["back", "available", "restore", "поверни", "повернути", "увімкни", "увімкнути", "віднови", "відновити", "rikthe", "ktheje", "aktivizo"];
const SALE_FILLER: &[&str] = &[
    "sale", "menu", "is", "are", "put", "take", "from", "sold", "out", "it", "the", "on", "please", "again",
    "з", "із", "в", "у", "продажу", "продаж", "меню", "будь", "ласка", "вже", "знову",
    "nga", "në", "ne", "shitja", "shitje", "shitjes", "menuja", "menusë", "ju", "lutem", "është", "eshte", "u", "përsëri", "perseri",
];

/// The owner's extra verbs, or `None` when the utterance is not one of them
/// (the hub's grammar then reads it: orders, status, a question).
pub fn owner(transcript: &str) -> Option<Said> {
    let t = norm(transcript);
    let ws = words(&t);
    if has(&ws, VENUE) {
        let states: Vec<&'static str> = [(V_OPEN, "open"), (V_BUSY, "busy"), (V_CLOSED, "closed")]
            .iter()
            .filter(|(w, _)| has(&ws, w))
            .map(|(_, s)| *s)
            .collect();
        return match states.as_slice() {
            [] => None,
            [s] => Some(Said::Venue { state: s }),
            _ => Some(Said::Unclear("more_than_one")),
        };
    }
    let off = has(&ws, OFF) || (has(&ws, &["sold"]) && has(&ws, &["out"]));
    let on = has(&ws, ON);
    if !off && !on {
        return None;
    }
    if off && on {
        return Some(Said::Unclear("more_than_one"));
    }
    let dish: Vec<&str> = ws
        .iter()
        .filter(|w| !OFF.contains(w) && !ON.contains(w) && !SALE_FILLER.contains(w))
        .copied()
        .collect();
    if dish.is_empty() {
        return Some(Said::Unclear("which_dish"));
    }
    Some(Said::DishSale { dish: dish.join(" "), on })
}

#[cfg(test)]
mod tests;
