//! PURE. One row of the owner's customer list: the FOLD's fields, and the
//! card joined on by key.
//!
//! THE FOLD STAYS A FOLD. `orders`, `spent`, `lastAt`, the masked name and
//! phone come from `roll` exactly as before; the card adds only what the
//! venue wrote down. A customer with no card is a row with no card fields --
//! not empty ones, which would read as "the owner cleared this".

use serde_json::{json, Value};

use super::roll::Row;

/// The card's stored field and the name the console reads it by.
const CARD: [(&str, &str); 6] = [
    ("note", "note"),
    ("tags", "tags"),
    ("allergens", "allergens"),
    ("lang", "lang"),
    ("usual_table", "usualTable"),
    ("birthday_md", "birthdayMd"),
];

/// `offers` is whether the consent fold holds a `Consented` for WhatsApp
/// marketing right now (§3.2) -- shown so the owner can see who said yes
/// before anything like a campaign exists.
///
/// `linked` are the keys shown under this row by an alias (§3.4), so the
/// console can unlink each; absent when there are none.
pub fn row_json(r: &Row, record: Option<&str>, offers: bool, linked: &[String]) -> Value {
    let mut v = json!({
        "key": r.key, "name": r.name, "phone": r.phone,
        "orders": r.orders, "spent": r.spent, "lastAt": r.last_at,
        "offersWhatsapp": offers,
    });
    if !linked.is_empty() {
        v["linked"] = json!(linked);
    }
    let card = record.and_then(|j| serde_json::from_str::<Value>(j).ok());
    if let Some(card) = card {
        for (stored, shown) in CARD {
            if let Some(x) = card.get(stored) {
                v[shown] = x.clone();
            }
        }
    }
    v
}
