//! THE VENUE'S MONEY BY HAND, AND THE MONTH'S NET: taxes are expenses too, and an expense is
//! a plain line, a name and an amount ("Бензин 500", "Прикраси на Хеловін 3200").
//!
//! What this file decides, from arguments alone: what an expense is, and what a month is.
//! `profit` folds the month's plus and minus; `routes` is the owner's door; the image is
//! one `Table` per venue (`IMAGE`, kind `KIND`, key the entry's id), written from the Worker
//! under the guard like any other table image and folded in the venue's object for the read.
//!
//! TAXES ARE ENTERED BY HAND, like expenses (operator 2026-11-08): no rate, no calculation.
//! Money is whole minor units from the first byte (`Expense::amount`, `i64`); a tax is a
//! `kind`, not a separate store.
//!
//! A NEW ENTRY MAY CARRY THE FORM'S OWN ID (`client_id`): the entry is filed under it, so the
//! same form sent twice -- a second tap, a retry after a lost answer -- is one entry.

use serde::{Deserialize, Serialize};

pub mod profit;
pub mod routes;

/// The image's name in the venue's object.
pub const IMAGE: &str = "finance";
/// The one kind in it: one expense.
pub const KIND: &str = "exp";
pub const FINANCE_BYTES: usize = dowiz_hub::CEILING_BYTES;
/// A hundred billion lek: more is a typing mistake, not an expense.
pub const AMOUNT_MAX: i64 = 1_000_000_000_000;
pub const NAME_MAX: usize = 120;
/// Months of the short history, the selected month included.
pub const HISTORY: usize = 6;

/// A tax is an expense with a word on it (operator: entered by hand).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Tax,
    Other,
}

/// `POST /api/owner/finance/expenses` and its edit: what the owner typed.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpenseIn {
    pub date: String,
    pub name: String,
    /// Whole minor units: 12.5 is refused by the parser, not rounded.
    pub amount: i64,
    pub kind: Kind,
    /// The form's own id, for a new entry only (`client_key`).
    #[serde(default)]
    pub client_id: Option<String>,
}

/// One stored entry; the table key is its id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Expense {
    pub date: String,
    /// The same date as `yyyymmdd`, for the month it belongs to.
    pub day: i64,
    pub name: String,
    pub amount: i64,
    pub kind: Kind,
    /// When it was first entered; an edit keeps it.
    pub at: i64,
}

/// What the owner sent, as the entry to file, or the refusal by name. `today` is the
/// venue's day: a date after it is a typing mistake (`parse_day` already refuses one before 2000).
pub fn validate(b: ExpenseIn, today: i64) -> Result<Expense, String> {
    use dowiz_hub::stock::meta::{parse_day, show_day};
    let day = parse_day(&b.date).ok_or_else(|| format!("{:?} is not a date (yyyy-mm-dd)", b.date))?;
    if day > today {
        return Err(format!("{} is in the future", b.date));
    }
    let name = b.name.trim().to_string();
    if name.is_empty() {
        return Err("name: say what it was for".into());
    }
    if name.chars().count() > NAME_MAX {
        return Err(format!("name: at most {NAME_MAX} characters"));
    }
    if !(1..=AMOUNT_MAX).contains(&b.amount) {
        return Err(format!("amount must be a whole number from 1 to {AMOUNT_MAX} minor units"));
    }
    Ok(Expense { date: show_day(day), day, name, amount: b.amount, kind: b.kind, at: 0 })
}

/// `"2026-11"` -> 202611; anything else `None`.
pub fn parse_month(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() != 7 || b[4] != b'-' || !b[..4].iter().chain(&b[5..7]).all(u8::is_ascii_digit) {
        return None;
    }
    let (y, m) = (s[..4].parse::<i64>().ok()?, s[5..7].parse::<i64>().ok()?);
    ((1000..=9999).contains(&y) && (1..=12).contains(&m)).then_some(y * 100 + m)
}

/// 202611 -> `"2026-11"`.
pub fn show_month(m: i64) -> String {
    format!("{:04}-{:02}", m / 100, m % 100)
}

/// A new entry's id when the form named none: the instant, in hex (`campaign::new_id`'s
/// shape); the table write bumps it past a collision.
pub fn new_id(now_ms: i64) -> String {
    format!("e{now_ms:x}")
}

pub const CLIENT_ID_MIN: usize = 8;
pub const CLIENT_ID_MAX: usize = 40;

/// The table key of an entry filed under the form's id: 8 to 40 lowercase letters and digits,
/// behind a `c` no `new_id` starts with. Anything else is refused by name: the id goes into
/// a path (`/expenses/:id`), so it is never passed through as typed.
pub fn client_key(id: &str) -> Result<String, String> {
    let fits = (CLIENT_ID_MIN..=CLIENT_ID_MAX).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
    if !fits {
        return Err(format!("client_id must be {CLIENT_ID_MIN} to {CLIENT_ID_MAX} lowercase letters and digits"));
    }
    Ok(format!("c{id}"))
}

#[cfg(test)]
#[path = "finance/tests.rs"]
mod tests;
