//! P14, the kitchen's half of a spoken count: the supply found on the shelf,
//! the unit checked against the one it is counted in, and a signed PROPOSAL.
//! Nothing is written here; the confirmation's `{itemId, observed}` goes to
//! the count route the console's count sheet already calls.

use super::super::decide::Out;
use super::super::dish::{self, Dish, Miss};
use super::super::kitchen::Supply;
use super::super::{say, scope};
use dowiz_hub::caps::{Cap, Caps};
use serde_json::json;

fn refuse(key: &str, lang: &str) -> Out {
    Out::Refuse(say::line(key, lang).to_string())
}

/// A counted line: the kitchen's (`stock`), proposed as `count:<item>|<qty>`,
/// added by the confirmation to the open session (`POST /api/owner/stock/count`).
/// Two supplies that fit, or none, is a question with the candidates.
pub fn line(item: &str, qty: i64, unit: Option<&str>, caps: &Caps, lang: &str, shelf: &[Supply]) -> Out {
    if !caps.allows(Cap::Stock) {
        return refuse("cap_kitchen", lang);
    }
    let dishes: Vec<Dish> = shelf.iter().map(|s| s.dish.clone()).collect();
    let d = match dish::find(&dishes, item) {
        Ok(d) => d,
        Err(Miss::None) => return Out::Refuse(format!("{} «{item}»", say::line("no_supply", lang))),
        Err(Miss::Many(n)) => return Out::Refuse(format!("{} {}", say::line("many_supplies", lang), n.join(", "))),
    };
    let s = shelf.iter().find(|s| s.dish.id == d.id).map(|s| s.unit.as_str()).unwrap_or("g");
    if unit.is_some_and(|u| u != s) {
        return refuse("stock_unit", lang);
    }
    let name = d.names.first().cloned().unwrap_or_else(|| d.id.clone());
    match scope::arg(&[&d.id, &qty.to_string()]) {
        Some(arg) => Out::Propose { verb: "count", arg, readback: say::count(lang, qty, s, &name), extra: json!({ "itemId": d.id, "observed": qty }) },
        None => refuse("which_supply", lang),
    }
}
