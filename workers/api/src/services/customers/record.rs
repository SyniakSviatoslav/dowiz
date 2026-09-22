//! The customer RECORD: what a fold over the orders cannot know.
//!
//! §3.1 of `BLUEPRINT-CRM-CONSENT-LOYALTY-2026-09-22`. The record holds only
//! what the venue would otherwise write on a paper card by the till — a note,
//! tags from a closed list, EU-14 allergen codes, the usual table, a language
//! and a birthday as MM-DD with no year. Name, phone, order count, spend and
//! last visit are FOLDS (`roll.rs`); a stored copy is a second number that can
//! disagree, so they are not fields here and `Card` refuses them at the door.
//!
//! PURE: `merge` takes the stored JSON and the request's clock and returns the
//! new JSON, so every rule is proved without a Durable Object.

use serde::Deserialize;
use serde_json::{Map, Value};

/// The owner's edit. A CLOSED shape: a body carrying `spent` or `tier` is a
/// 400, not a silently ignored field. `None` = untouched; empty = cleared.
#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Card {
    pub note: Option<String>,
    pub tags: Option<Vec<String>>,
    pub allergens: Option<Vec<String>>,
    pub usual_table: Option<String>,
    pub lang: Option<String>,
    pub birthday_md: Option<String>,
}

/// The paper card's length, in CHARACTERS (Albanian `ë` is two bytes).
pub const NOTE_MAX_CHARS: usize = 280;
/// A table is a label ("7", "terrace 2"), not a second note.
pub const TABLE_MAX_CHARS: usize = 16;

/// The closed tag list. A TAG IS NOT A TIER: nothing here ranks a person, and
/// a word that would (`vip`, `difficult`) is refused because it is not listed.
pub const TAGS: [&str; 8] = [
    "regular",
    "office_lunch",
    "family",
    "group",
    "takeaway",
    "delivery",
    "tourist",
    "event",
];

/// The fields a record may hold. Everything else in the stored JSON is dropped
/// on write except the three the placement path owns.
const OWNED_BY_PLACEMENT: [&str; 3] = ["id", "phone_hash", "created_at_ms"];

fn closed_list(what: &str, got: &[String], allowed: &[&str]) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for g in got {
        if !allowed.contains(&g.as_str()) {
            return Err(format!("{what} {g:?} is not in the closed list"));
        }
        if !out.contains(g) {
            out.push(g.clone());
        }
    }
    Ok(out)
}

/// MM-DD, a day that exists in SOME year (so 02-29 is allowed), no year.
fn birthday_ok(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 5 || b[2] != b'-' || !b.iter().enumerate().all(|(i, c)| i == 2 || c.is_ascii_digit()) {
        return false;
    }
    let m = (b[0] - b'0') as u32 * 10 + (b[1] - b'0') as u32;
    let d = (b[3] - b'0') as u32 * 10 + (b[4] - b'0') as u32;
    let max = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => 29,
        _ => return false,
    };
    (1..=max).contains(&d)
}

fn set_str(out: &mut Map<String, Value>, k: &str, v: &str) {
    if v.trim().is_empty() {
        out.remove(k);
    } else {
        out.insert(k.into(), Value::String(v.trim().to_string()));
    }
}

fn set_list(out: &mut Map<String, Value>, k: &str, v: Vec<String>) {
    if v.is_empty() {
        out.remove(k);
    } else {
        out.insert(k.into(), Value::Array(v.into_iter().map(Value::String).collect()));
    }
}

/// Apply `card` to the stored record `existing`, stamped with `now_ms`.
///
/// Every refusal is an `Err` with the reason; the handler answers it 400 and
/// nothing is written.
pub fn merge(existing: &str, card: &Card, now_ms: i64) -> Result<String, String> {
    let old: Map<String, Value> = serde_json::from_str::<Value>(existing)
        .ok()
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    let mut out = Map::new();
    for k in OWNED_BY_PLACEMENT.iter().chain(["note", "tags", "allergens", "usual_table", "lang", "birthday_md"].iter()) {
        if let Some(v) = old.get(*k) {
            out.insert((*k).into(), v.clone());
        }
    }

    if let Some(n) = &card.note {
        if n.chars().count() > NOTE_MAX_CHARS {
            return Err(format!("a note is at most {NOTE_MAX_CHARS} characters"));
        }
        set_str(&mut out, "note", n);
    }
    if let Some(t) = &card.tags {
        set_list(&mut out, "tags", closed_list("tag", t, &TAGS)?);
    }
    if let Some(a) = &card.allergens {
        set_list(&mut out, "allergens", closed_list("allergen", a, &dowiz_hub::allergens::EU14)?);
    }
    if let Some(t) = &card.usual_table {
        if t.trim().chars().count() > TABLE_MAX_CHARS {
            return Err(format!("a table is a label of at most {TABLE_MAX_CHARS} characters"));
        }
        set_str(&mut out, "usual_table", t);
    }
    if let Some(l) = &card.lang {
        if !l.is_empty() && !dowiz_hub::consent::LANGS.contains(&l.as_str()) {
            return Err(format!("language {l:?} is not one the storefront speaks"));
        }
        set_str(&mut out, "lang", l);
    }
    if let Some(b) = &card.birthday_md {
        if !b.is_empty() && !birthday_ok(b) {
            return Err(format!("birthday {b:?} is not MM-DD"));
        }
        set_str(&mut out, "birthday_md", b);
    }
    out.insert("updated_at_ms".into(), Value::from(now_ms));
    Ok(Value::Object(out).to_string())
}
