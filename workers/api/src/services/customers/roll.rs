//! PURE. The fold that turns a venue's orders into the people who placed them.
//!
//! NO REGISTRY IS WRITTEN. This list exists for the length of one request and
//! is stored nowhere, so the venue holds exactly what it held before.

use serde_json::Value;

/// One person, as the console shows them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The stable, non-reversible handle. The audit entry must not carry the
    /// number it is about, and a URL holding a phone number puts it in every
    /// proxy log between here and the browser.
    pub key: String,
    pub name: String,
    pub phone: String,
    pub orders: i64,
    pub spent: i64,
    pub last_at: i64,
}

/// What the owner asked to be sorted by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    Spent,
    Orders,
    /// The default. The question at the end of a shift is who has just been
    /// in, not who is worth the most.
    Recent,
}

impl Sort {
    pub fn of(q: Option<&str>) -> Sort {
        match q {
            Some("spent") => Sort::Spent,
            Some("orders") => Sort::Orders,
            _ => Sort::Recent,
        }
    }
}

/// What ONE order contributed to the venue.
///
/// A REFUSED ORDER IS NOT MONEY TAKEN, and the tip went to the courier — the
/// venue never had it, so a list sorted by "spent" that counted tips would
/// rank a generous customer above a profitable one.
fn spent_on(o: &Value) -> i64 {
    crate::services::orders::status::venue_took(
        o.get("total").and_then(Value::as_i64).unwrap_or(0),
        o.get("tip").and_then(Value::as_i64).unwrap_or(0),
        o.get("status").and_then(Value::as_str).unwrap_or(""),
    )
}

/// DOES THIS PHONE NAME A PERSON? Only if it carries a digit once the `00`
/// access prefix is gone. A room round is placed with `contact.phone: ""`
/// (the table is not a customer), and `customer_key` of no digits is ONE
/// constant key per venue: every phone-less round would be one "—" customer
/// with every table's orders and spend. The customer paths (this fold, the
/// reveal, the allergy card) all ask this one question first.
pub fn names_a_person(phone: &str) -> bool {
    let digits: String = phone.chars().filter(char::is_ascii_digit).collect();
    !digits.strip_prefix("00").unwrap_or(&digits).is_empty()
}

/// Fold this venue's orders into one row per person.
///
/// `key_of` turns a phone number into the handle; it is passed in because it
/// needs the hub's signing secret, and a fold that needed a secret could not
/// be tested.
///
/// `resolve` takes a key and returns the canonical key it should be grouped by.
/// This allows aliases (multiple phone spellings mapped to one canonical form)
/// to be grouped into a single row.
///
/// AN ORDER WITHOUT A PHONE IS NOT A PERSON. The number is optional by
/// operator decision, so a basket placed without one is counted in the venue's
/// takings and simply has nobody to attribute it to.
pub fn roll(
    orders: &[Value],
    key_of: impl Fn(&str) -> String,
    resolve: impl Fn(&str) -> String,
    mask_name: impl Fn(&str) -> String,
    mask_phone: impl Fn(&str) -> String,
    sort: Sort,
) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::new();
    for o in orders {
        let contact = o.get("contact");
        let Some(phone) = contact.and_then(|c| c.get("phone")).and_then(Value::as_str).filter(|p| names_a_person(p))
        else {
            continue;
        };
        let at = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        let spent = spent_on(o);
        let key = key_of(phone);
        let canonical = resolve(&key);
        match rows.iter_mut().find(|r| r.key == canonical) {
            Some(r) => {
                // A REFUSED ORDER STILL COUNTS AS A VISIT. They came, and the
                // venue said no; hiding that from the count would hide the
                // venue's own behaviour from it.
                r.orders += 1;
                r.spent += spent;
                r.last_at = r.last_at.max(at);
            }
            None => {
                let name = contact
                    .and_then(|c| c.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                rows.push(Row {
                    key: canonical,
                    name: mask_name(name),
                    phone: mask_phone(phone),
                    orders: 1,
                    spent,
                    last_at: at,
                });
            }
        }
    }
    // TIES BREAK ON RECENCY, always. Two customers with three orders each are
    // listed with the one who was here last night first, and the order does
    // not wobble between requests.
    match sort {
        Sort::Spent => rows.sort_by(|a, b| b.spent.cmp(&a.spent).then(b.last_at.cmp(&a.last_at))),
        Sort::Orders => rows.sort_by(|a, b| b.orders.cmp(&a.orders).then(b.last_at.cmp(&a.last_at))),
        Sort::Recent => rows.sort_by(|a, b| b.last_at.cmp(&a.last_at)),
    }
    rows
}

#[cfg(test)]
mod tests;
