//! AN OPTION THAT DRAWS STOCK (research 2026-10-03 P8, row R13, W-LOST):
//! "extra salmon" takes 20 g of salmon off the shelf like the roll does.
//!
//! STORED ON THE DISH, KEYED BY OPTION: `"optionBom": {"<option id>":
//! [{"supply","qty"}]}`, a top-level key of the product record, beside its
//! `bom`, never inside `modifierGroups`. Two reasons, both load-bearing:
//!
//!   * the public menu passes `modifierGroups` through AS STORED
//!     (`fold/menu_venue.rs`), and a recipe is the venue's, not the guest's;
//!   * `stock::bom_of` takes the FIRST `"bom": [` of a record, at any depth.
//!     An option's `"bom"` written before the dish's own would have been read
//!     as the DISH's recipe. `"optionBom"` is not `"bom"` to that reader.
//!
//! THE OLD-IMAGE RULE: a dish with no `optionBom` reserves exactly what it
//! did; the key is removed, not left empty, when its last line goes, so a
//! dish edited back to no option recipe is byte-for-byte what it was.
//!
//! THE LINES JOIN THE RESERVATION as one more basket line,
//! `{"of":"<dish>","delta":<lek>,"bom":[...]}` (`option_line`): no `id`, so the
//! cost stamp (`command/place/cost.rs`) does not mistake it for a dish, and
//! `stock::draws_for` sums its supplies with the dish's -- one decision, all
//! or nothing, against the shelf.

use super::{groups_of, Group};
use crate::stock::{bom_of, BomLine};
use serde_json::{json, Map, Value};

/// The dish record's key.
pub const KEY: &str = "optionBom";
/// Lines per option: an extra is a handful of supplies, not a recipe book.
pub const LINES_MAX: usize = 8;

/// The recipe of option `option` on the dish, whole units.
pub fn option_bom(product: &Value, option: &str) -> Vec<BomLine> {
    match product.get(KEY).and_then(|m| m.get(option)) {
        Some(lines) if lines.is_array() => bom_of(&json!({ "bom": lines }).to_string()),
        _ => Vec::new(),
    }
}

/// The basket line one ordered dish's chosen options add, or `None` when
/// they draw nothing and cost nothing extra. `qty` is the portions ordered.
pub fn option_line(product_id: &str, product_json: &str, chosen: &[String], qty: i64) -> Option<(String, i64)> {
    if chosen.is_empty() || qty <= 0 {
        return None;
    }
    let product: Value = serde_json::from_str(product_json).ok()?;
    let groups = groups_of(product_json);
    let delta: i64 = groups
        .iter()
        .flat_map(|g| g.options.iter())
        .filter(|o| chosen.iter().any(|c| *c == o.id))
        .map(|o| o.price_delta)
        .fold(0i64, i64::saturating_add);
    let mut lines: Vec<BomLine> = Vec::new();
    for c in chosen {
        for l in option_bom(&product, c) {
            match lines.iter_mut().find(|x| x.supply == l.supply) {
                Some(x) => *x = BomLine::whole(x.supply.clone(), x.qty.saturating_add(l.qty)),
                None => lines.push(l),
            }
        }
    }
    if lines.is_empty() && delta == 0 {
        return None;
    }
    let bom: Vec<Value> = lines.iter().map(|l| json!({ "supply": l.supply, "qty": l.qty })).collect();
    Some((json!({ "of": product_id, "delta": delta, "bom": bom }).to_string(), qty))
}

/// Every option of the dish, `(group id, group name, option id, option name)`.
pub fn options_of(product_json: &str) -> Vec<(String, String, String, String)> {
    let groups: Vec<Group> = groups_of(product_json);
    groups
        .iter()
        .flat_map(|g| g.options.iter().map(move |o| (g.id.clone(), g.name.clone(), o.id.clone(), o.name.clone())))
        .collect()
}

/// Set option `option`'s recipe on the dish record. Refused, nothing
/// changed: an option the dish does not have, too many lines, a line that is
/// not a supply and a positive whole quantity, a supply named twice. Empty
/// `lines` clears it.
pub fn set_option_bom(product: &mut Value, option: &str, lines: &[BomLine], qty_max: i64) -> Result<(), String> {
    let pj = product.to_string();
    if !options_of(&pj).iter().any(|(_, _, id, _)| id == option) {
        return Err(format!("{option} is not an option on this dish"));
    }
    if lines.len() > LINES_MAX {
        return Err(format!("an option takes at most {LINES_MAX} lines"));
    }
    for (i, l) in lines.iter().enumerate() {
        if l.supply.trim().is_empty() || l.qty <= 0 || l.qty > qty_max {
            return Err("a recipe line is a supply and a positive quantity".into());
        }
        if lines[..i].iter().any(|o| o.supply == l.supply) {
            return Err(format!("{} is named twice", l.supply));
        }
    }
    let Some(obj) = product.as_object_mut() else { return Err("the dish record is not an object".into()) };
    let mut map: Map<String, Value> = obj.get(KEY).and_then(Value::as_object).cloned().unwrap_or_default();
    if lines.is_empty() {
        map.remove(option);
    } else {
        map.insert(option.to_string(), Value::Array(lines.iter().map(|l| json!({ "supply": l.supply, "qty": l.qty })).collect()));
    }
    if map.is_empty() {
        obj.remove(KEY);
    } else {
        obj.insert(KEY.to_string(), Value::Object(map));
    }
    Ok(())
}

#[cfg(test)]
#[path = "bom_tests.rs"]
mod tests;
