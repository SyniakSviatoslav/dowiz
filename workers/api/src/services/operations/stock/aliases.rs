//! WHAT A SUPPLIER CALLS OUR SUPPLIES (W-OCR, research 2026-10-03 P10/P11), pure.
//!
//! An invoice line says "Salmon file Norvegjeze 2,5 kg"; the shelf calls it
//! `salmon`. Once the owner has matched the two on one delivery, the next
//! invoice from the same supplier arrives matched: the photo sheet and the
//! e-invoice import (`public/admin/receipt-photo*.js`, `einvoice-logic.js`) ask
//! the fold below before they ask the owner.
//!
//! Through the stock door that already exists, `POST /api/owner/stock/alias`
//! (no new route), body `{"card": {...}}` like the other notes:
//!
//!   alias  {supplier: <card id>, nipt?: "K12345678A", lines?: [{text, item}]}
//!          -- `text` is the line's words as the supplier prints them (the
//!          client drops the numbers); `item` a supply id, or "" to forget.
//!          `nipt` is the supplier's tax number, read off an e-invoice, so the
//!          next e-invoice finds its supplier card by itself.
//!
//! WRITE ONLY WHAT CHANGED: a line whose alias already says the same thing is
//! not written again, so confirming the tenth invoice of a supplier adds
//! nothing to the log. Each line is ONE note (`notes::BODY_MAX` is 2 KB).
//!
//! NO PERSONAL DATA. The text is a product's name as printed on a supplier's
//! invoice, the item a supply id, the NIPT the supplier BUSINESS's tax number
//! (published on every invoice it issues, and on the public business register).
//! The person on the card -- a contact name, a phone -- stays on the card, which
//! the privacy registry already lists (`registry/venue.rs`, image `stock`).

use std::collections::BTreeMap;

use dowiz_hub::stock::StockLog;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::suppliers::cards;
use super::turn::{StockTurnIn, Told};

/// The movement's word, and the note's kind.
pub const KIND: &str = "alias";
/// A line's words, normalised, are at most this long.
pub const TEXT_MAX: usize = 120;
/// Lines in one request (one invoice).
pub const LINES_MAX: usize = 100;
/// Aliases one supplier may hold: an invoice catalogue, not a dictionary.
pub const PER_SUPPLIER_MAX: usize = 500;

type Bad = (u16, String);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineIn {
    pub text: String,
    pub item: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AliasIn {
    pub supplier: String,
    #[serde(default)]
    pub nipt: Option<String>,
    #[serde(default)]
    pub lines: Vec<LineIn>,
}

/// What one supplier's invoices are known to say.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Known {
    pub nipt: String,
    /// normalised text -> supply id.
    pub lines: BTreeMap<String, String>,
}

/// The words of a line as an alias key: trimmed, lower case, one space between
/// words. The console's `aliasKey` (`receipt-photo-logic.js`) is the same rule.
pub fn key(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// A NIPT as stored: upper case, a letter, eight digits, a letter. `None`: not one.
pub fn nipt(raw: &str) -> Option<String> {
    let n = raw.trim().to_uppercase();
    let b = n.as_bytes();
    let ok = b.len() == 10 && b[0].is_ascii_uppercase() && b[9].is_ascii_uppercase() && b[1..9].iter().all(u8::is_ascii_digit);
    ok.then_some(n)
}

/// Every supplier's aliases, from the notes, oldest first: the newest word on a
/// text wins, an empty item forgets it.
pub fn fold(log: &StockLog) -> BTreeMap<String, Known> {
    let mut out: BTreeMap<String, Known> = BTreeMap::new();
    for rec in log.notes(KIND) {
        let Ok(v) = serde_json::from_str::<Value>(&rec) else { continue };
        let Some(sup) = v["supplier"].as_str() else { continue };
        let k = out.entry(sup.to_string()).or_default();
        if let Some(n) = v["nipt"].as_str() {
            k.nipt = n.to_string();
        }
        if let (Some(text), Some(item)) = (v["text"].as_str(), v["item"].as_str()) {
            if item.is_empty() {
                k.lines.remove(text);
            } else {
                k.lines.insert(text.to_string(), item.to_string());
            }
        }
    }
    out
}

/// The request as the notes it adds -- only what changes what the fold says.
/// PURE over what is known. A refusal is `(status, words)`.
pub fn plan(a: AliasIn, known: &BTreeMap<String, Known>, has_card: impl Fn(&str) -> bool, has_supply: impl Fn(&str) -> bool) -> Result<Vec<Value>, Bad> {
    let supplier = a.supplier.trim().to_string();
    if !has_card(&supplier) {
        return Err((404, format!("no supplier card {supplier}")));
    }
    if a.lines.len() > LINES_MAX {
        return Err((400, format!("at most {LINES_MAX} lines")));
    }
    let empty = Known::default();
    let now = known.get(&supplier).unwrap_or(&empty);
    let mut notes = Vec::new();
    if let Some(raw) = a.nipt.as_deref().filter(|s| !s.trim().is_empty()) {
        let n = nipt(raw).ok_or((400, "a NIPT is a letter, eight digits and a letter".to_string()))?;
        if n != now.nipt {
            notes.push(json!({ "supplier": supplier, "nipt": n }));
        }
    }
    let mut lines = now.lines.clone();
    for l in a.lines {
        let text = key(&l.text);
        if text.is_empty() || text.chars().count() > TEXT_MAX {
            return Err((400, format!("a line's words are 1 to {TEXT_MAX} characters")));
        }
        let item = l.item.trim().to_string();
        if !item.is_empty() && !has_supply(&item) {
            return Err((404, format!("not found: {item}")));
        }
        let same = lines.get(&text).map(String::as_str).unwrap_or("") == item;
        if same {
            continue;
        }
        if item.is_empty() {
            lines.remove(&text);
        } else {
            lines.insert(text.clone(), item.clone());
        }
        notes.push(json!({ "supplier": supplier, "text": text, "item": item }));
    }
    if lines.len() > PER_SUPPLIER_MAX {
        return Err((400, format!("a supplier keeps at most {PER_SUPPLIER_MAX} aliases")));
    }
    Ok(notes)
}

/// The aliases as the venue object's turn: checked, the changed ones appended,
/// answered with what the supplier's invoices are now known to say.
pub fn run(log: &mut StockLog, input: &StockTurnIn) -> Result<(Value, Told), Bad> {
    let raw = input.body.get("card").cloned().ok_or((400, "the body is {\"card\": {...}}".to_string()))?;
    let a = serde_json::from_value::<AliasIn>(raw).map_err(|e| (400, format!("bad alias: {e}")))?;
    let ids: Vec<String> = cards(log).into_iter().map(|c| c.id).collect();
    let known = fold(log);
    let supplier = a.supplier.trim().to_string();
    let notes = plan(a, &known, |s| ids.iter().any(|i| i == s), |i| input.supplies.contains_key(i))?;
    log.set_clock(input.now_ms);
    for n in &notes {
        let mut body = n.clone();
        body["by_"] = json!(input.by);
        log.append_note(KIND, &body.to_string()).map_err(|e| (500, e.to_string()))?;
    }
    let now = fold(log).remove(&supplier).unwrap_or_default();
    Ok((json!({ "ok": true, "kind": KIND, "written": notes.len(), "known": now }), Vec::new()))
}

#[cfg(test)]
#[path = "aliases/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "aliases/route_tests.rs"]
mod route_tests;
