//! PURE. An intent, checked against who is speaking, the menu and the room,
//! turned into one of three answers: do it now (it changes nothing a guest
//! would notice), propose it (a write -- the route signs it and a person
//! confirms), or refuse it in the speaker's language.
//!
//! A WAITER CAN NEVER DO BY VOICE WHAT THEIR CAPABILITIES REFUSE: the check is
//! here, before anything is proposed, and the route the confirmation reaches
//! (`room_admits`) asks again. Voice moves a hand, never a right.

use super::dish::{self, Dish, Miss};
use super::grammar::{Method, Said};
use super::words::MAX_SAID;
use super::{room, say, scope};
use dowiz_hub::caps::{Cap, Caps};
use serde_json::{json, Value};

/// The most lines one spoken round may send.
pub const DRAFT_MAX: usize = 30;

/// The round being built on the phone: its table and `(product_id, qty)`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Draft {
    pub table: String,
    pub items: Vec<(String, u32)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Out {
    /// Runs at once.
    Now(Value),
    /// A write. The route mints `voice:<verb>:<arg>`; `extra` rides along to
    /// the surface so it can show what the read-back is about.
    Propose { verb: &'static str, arg: String, readback: String, extra: Value },
    Refuse(String),
}

/// What a waiter's intent is checked against.
pub struct Room<'a> {
    pub lang: &'a str,
    pub caps: Caps,
    pub sittings: &'a [Value],
    pub menu: &'a [Dish],
    pub draft: Option<&'a Draft>,
}

fn refuse(key: &str, lang: &str) -> Out {
    Out::Refuse(say::line(key, lang).to_string())
}

/// The dish a phrase names, or the refusal that says why not.
fn one_dish<'a>(menu: &'a [Dish], phrase: &str, lang: &str) -> Result<&'a Dish, Out> {
    match dish::find(menu, phrase) {
        Ok(d) => Ok(d),
        Err(Miss::None) => Err(Out::Refuse(format!("{} «{phrase}»", say::line("no_dish", lang)))),
        Err(Miss::Many(names)) => Err(Out::Refuse(format!("{} {}", say::line("many_dishes", lang), names.join(", ")))),
    }
}

fn name(d: &Dish) -> &str {
    d.names.first().map(String::as_str).unwrap_or(&d.id)
}

/// A waiter's intent.
pub fn waiter(said: &Said, r: &Room) -> Out {
    let lang = r.lang;
    let orders = r.caps.allows(Cap::TakeOrders);
    match said {
        Said::Unclear(k) => refuse(k, lang),
        Said::Status | Said::Open { .. } | Said::Add { .. } | Said::Send { .. } if !orders => refuse("cap_orders", lang),
        Said::Status => {
            let (open, waiting) = room::glance(r.sittings);
            Out::Now(json!({ "action": "status", "open": open, "waiting": waiting }))
        }
        Said::Open { table, guests } => Out::Now(json!({ "action": "open", "table": table.to_string(), "guests": guests })),
        Said::Add { dish, qty, table } => {
            let table = match (table, r.draft) {
                (Some(n), _) => n.to_string(),
                (None, Some(d)) if !d.table.trim().is_empty() => d.table.trim().to_string(),
                _ => return refuse("which_table", lang),
            };
            let d = match one_dish(r.menu, dish, lang) {
                Ok(d) => d,
                Err(o) => return o,
            };
            if !d.available {
                return refuse("dish_off", lang);
            }
            match room::editable(r.sittings, &table) {
                Err(k) => refuse(k, lang),
                Ok(Some(rr)) => {
                    let (seq, q) = (rr.seq.to_string(), qty.to_string());
                    match scope::arg(&[&rr.id, &seq, &d.id, &q]) {
                        Some(arg) => Out::Propose {
                            verb: "add",
                            arg,
                            readback: say::add(lang, *qty, name(d), &table),
                            extra: json!({ "orderId": rr.id, "table": table }),
                        },
                        None => refuse("bad_draft", lang),
                    }
                }
                // No round the kitchen has not taken: the dish joins the round
                // being built on the phone. Nothing is written until it is sent.
                // A round being built for ANOTHER table is not silently
                // re-pointed: it is refused, and the screen says which.
                Ok(None) if r.draft.is_some_and(|d| !d.items.is_empty() && d.table.trim() != table) => refuse("other_table", lang),
                Ok(None) => Out::Now(json!({ "action": "draft_add", "table": table, "productId": d.id, "name": name(d), "quantity": qty })),
            }
        }
        Said::Send { table } => {
            let Some(draft) = r.draft.filter(|d| !d.items.is_empty()) else { return refuse("nothing_to_send", lang) };
            let at = draft.table.trim();
            if at.is_empty() {
                return refuse("which_table", lang);
            }
            if table.is_some_and(|n| n.to_string() != at) {
                return refuse("other_table", lang);
            }
            if draft.items.len() > DRAFT_MAX {
                return refuse("bad_draft", lang);
            }
            let mut named: Vec<(String, u32)> = Vec::new();
            for (pid, q) in &draft.items {
                let Some(d) = r.menu.iter().find(|d| &d.id == pid) else { return refuse("bad_draft", lang) };
                if !(1..=MAX_SAID).contains(q) {
                    return refuse("bad_draft", lang);
                }
                if !d.available {
                    return Out::Refuse(format!("{} «{}»", say::line("dish_off", lang), name(d)));
                }
                named.push((name(d).to_string(), *q));
            }
            match scope::place(at, &draft.items) {
                Some(arg) => Out::Propose {
                    verb: "place",
                    arg,
                    readback: say::send(lang, at, &named),
                    extra: json!({ "table": at }),
                },
                None => refuse("bad_draft", lang),
            }
        }
        Said::Paid { .. } if !r.caps.allows(Cap::TakePayment) => refuse("cap_payment", lang),
        Said::Paid { table, method } => {
            let table = table.to_string();
            match room::payable(r.sittings, &table) {
                Err(k) => refuse(k, lang),
                Ok((rr, owed)) => {
                    let (seq, amount) = (rr.seq.to_string(), owed.to_string());
                    match scope::arg(&[&rr.id, &seq, &amount, method.as_str()]) {
                        Some(arg) => Out::Propose {
                            verb: "pay",
                            arg,
                            readback: say::paid(lang, &table, *method == Method::Cash),
                            extra: json!({ "orderId": rr.id, "table": table, "amount": owed, "method": method.as_str() }),
                        },
                        None => refuse("bad_draft", lang),
                    }
                }
            }
        }
        Said::DishSale { .. } | Said::Venue { .. } => refuse("not_room", lang),
    }
}

/// The owner's extra intents: the stop-list and the venue's state.
pub fn owner(said: &Said, lang: &str, menu: &[Dish]) -> Out {
    match said {
        Said::DishSale { dish, on } => match one_dish(menu, dish, lang) {
            Err(o) => o,
            Ok(d) => match scope::arg(&[&d.id]) {
                Some(arg) => Out::Propose {
                    verb: if *on { "dish_on" } else { "dish_off" },
                    arg,
                    readback: say::dish_sale(lang, name(d), *on),
                    extra: json!({ "productId": d.id }),
                },
                None => refuse("no_dish", lang),
            },
        },
        Said::Venue { state } => Out::Propose {
            verb: "venue",
            arg: (*state).to_string(),
            readback: say::venue(lang, state),
            extra: json!({ "state": state }),
        },
        Said::Unclear(k) => refuse(k, lang),
        _ => refuse("not_room", lang),
    }
}

#[cfg(test)]
mod tests;
