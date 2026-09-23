//! PURE. The one field of the record that prevents harm, used where it can:
//! at placement, against what each dish DECLARES.
//!
//! A REFUSAL, NOT A WARNING (§6 item 1: "refuses with a named reason, the way
//! `allergens.rs` refuses an undeclared dish"). The customer's card is the
//! venue's own paper card by the till; a kitchen that knows a regular is
//! allergic to fish does not sell them a salmon roll because a box was
//! clicked. The refusal names the dish and the allergen, so the person can
//! take that one line out -- or ask the venue to correct a card that is wrong.
//!
//! UNDECLARED IS NOT SAFE. `dowiz_hub::allergens` keeps "nobody has said" and
//! "none of the fourteen" apart; to a person with a recorded allergy the first
//! is not a claim the dish is free of it, so it is refused too.

use dowiz_hub::allergens::{read, Declaration};

/// `person` is the record's `allergens`; `dishes` is `(name, product JSON)`
/// for every line of the basket. `Ok` when nothing on the card is in the
/// basket; otherwise the reason, naming every dish that would hurt.
pub fn refuse(person: &[String], dishes: &[(String, String)]) -> Result<(), String> {
    if person.is_empty() {
        return Ok(());
    }
    let mut named: Vec<String> = Vec::new();
    for (name, json) in dishes {
        let why = match read(json) {
            Declaration::Undeclared => Some("has no allergen declaration".to_string()),
            Declaration::None => None,
            Declaration::Contains(codes) => {
                let hit: Vec<&str> =
                    codes.iter().filter(|c| person.contains(c)).map(String::as_str).collect();
                (!hit.is_empty()).then(|| format!("contains {}", hit.join(", ")))
            }
        };
        if let Some(w) = why {
            let line = format!("{name} {w}");
            if !named.contains(&line) {
                named.push(line);
            }
        }
    }
    if named.is_empty() {
        return Ok(());
    }
    Err(format!(
        "allergy on this customer's card ({}): {}",
        person.join(", "),
        named.join("; ")
    ))
}

/// The record's allergens, from its stored JSON. Absent or unreadable = none
/// recorded, which is the state every customer starts in.
pub fn of_record(record: Option<&str>) -> Vec<String> {
    record
        .and_then(|r| serde_json::from_str::<serde_json::Value>(r).ok())
        .and_then(|v| v.get("allergens").and_then(|a| a.as_array()).cloned())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}
