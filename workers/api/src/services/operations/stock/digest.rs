//! THE WEEKLY LOSS DIGEST (W-STOCK P4), pure: `stock.digest` to the groups
//! that chose it, in each group's language -- the biggest unexplained losses
//! the count just closed, valued at the average cost of that count.
//!
//!   stock.digest {items: [{name, qty, unit, value?}], currency, more}
//!
//! WHEN: with a count session that closes at least one tracked window, and at
//! most once in [`EVERY_MS`]. The log is the memory: the digest leaves a
//! `digest` note (`dowiz_hub::stock::notes`) and the next one is due when the
//! newest such note is older than that. A count is when the numbers exist, so
//! a venue that counts on Mondays hears it on Mondays, and one that never
//! counts is never told "no losses" about a shelf nobody looked at.
//!
//! WHAT: the windows closed by THIS session (`kitchen::report::avt`), of
//! supplies a recipe names -- an untracked supply is never a loss -- whose
//! unexplained quantity is above zero, money first, at most `avt::TOP`.

use dowiz_hub::stock::journal::Journal;
use dowiz_hub::stock::notes::at_of;
use dowiz_hub::stock::StockLog;
use serde_json::{json, Value};

use crate::services::analytics::kitchen::report::avt;

/// The event's key on the routing (`notify::route::events`).
pub const EVENT: &str = "stock.digest";
/// The note that remembers a digest was owed.
pub const NOTE: &str = "digest";
/// At most one digest in this long: a week, less half a day of slack so a
/// count at 18:00 next Monday is not refused for one made at 18:30 today.
pub const EVERY_MS: i64 = 7 * 86_400_000 - 43_200_000;

/// Is a digest due at `now`, by the notes the log holds?
pub fn due(notes: &[String], now: i64) -> bool {
    notes.iter().filter_map(|n| at_of(n)).max().is_none_or(|last| now - last >= EVERY_MS)
}

/// The digest's data for the session written at `now`, or `None` when it
/// closed no tracked window. `named` answers (name, unit) and whether a recipe
/// names the supply.
pub fn data(after: &Journal, now: i64, named: &dyn Fn(&str) -> Option<(String, String, bool)>, currency: &str) -> Option<Value> {
    let mut rows: Vec<(i64, i64, String, String)> = Vec::new();
    let mut closed = 0;
    for (id, ws) in avt::windows(&after.entries) {
        let Some(w) = ws.last().filter(|w| w.closed_at == Some(now)) else { continue };
        let Some((name, unit, true)) = named(&id) else { continue };
        closed += 1;
        let u = w.unexplained();
        if u > 0 {
            rows.push((avt::value_of(w, &id, &|_, _| None).unwrap_or(i64::MIN), u, name, unit));
        }
    }
    if closed == 0 {
        return None;
    }
    rows.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)).then(a.2.cmp(&b.2)));
    let more = rows.len().saturating_sub(avt::TOP);
    let items: Vec<Value> = rows
        .into_iter()
        .take(avt::TOP)
        .map(|(v, q, name, unit)| {
            let mut i = json!({ "name": name, "qty": q, "unit": unit });
            if v != i64::MIN {
                i["value"] = json!(v);
            }
            i
        })
        .collect();
    Some(json!({ "items": items, "currency": currency, "more": more, "checked": closed }))
}

/// Owe the digest for a count session: the data when due, with its note
/// written to `log` in the same turn. `None`: nothing to tell.
pub fn owe(log: &mut StockLog, after: &Journal, now: i64, named: &dyn Fn(&str) -> Option<(String, String, bool)>, currency: &str) -> Option<Value> {
    if !due(&log.notes(NOTE), now) {
        return None;
    }
    let d = data(after, now, named, currency)?;
    log.set_clock(now);
    log.append_note(NOTE, "{}").ok()?;
    Some(d)
}

/// The words, per language: head, nothing lost, and the pointer to the records.
const WORDS: [(&str, &str, &str, &str); 4] = [
    ("sq", "Kontrolli i javës: humbje të pashpjeguara që nga numërimi i fundit", "Asgjë e pashpjeguar që nga numërimi i fundit.", "Regjistrimet: Përbërësit > Humbjet."),
    ("en", "Weekly stock check: unexplained since the last count", "Nothing unexplained since the last count.", "The records: Ingredients > Losses."),
    ("uk", "Тижнева перевірка складу: непояснені втрати від останньої інвентаризації", "Від останньої інвентаризації нічого непоясненого.", "Записи: Інгредієнти > Втрати."),
    ("ru", "Еженедельная проверка склада: необъяснённые потери с последней инвентаризации", "С последней инвентаризации ничего необъяснённого.", "Записи: Ингредиенты > Потери."),
];

/// The digest in `lang` (English when the group speaks another).
pub fn text(d: &Value, lang: &str) -> String {
    let w = WORDS.iter().find(|w| w.0 == lang).unwrap_or(&WORDS[1]);
    let items = d.get("items").and_then(Value::as_array).cloned().unwrap_or_default();
    let cur = d.get("currency").and_then(Value::as_str).unwrap_or("");
    let mut t = format!("🧾 {}\n", w.1);
    if items.is_empty() {
        t.push_str(w.2);
        return t;
    }
    for (n, i) in items.iter().enumerate() {
        let s = |k: &str| i.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        let mut line = format!("{}. {}: {} {}", n + 1, s("name"), i.get("qty").and_then(Value::as_i64).unwrap_or(0), s("unit"));
        if let (Some(v), false) = (i.get("value").and_then(Value::as_i64), cur.is_empty()) {
            line.push_str(&format!(" · {}", crate::notify::money_text(v, cur)));
        }
        t.push_str(line.trim_end());
        t.push('\n');
    }
    let more = d.get("more").and_then(Value::as_i64).unwrap_or(0);
    if more > 0 {
        t.push_str(&format!("+{more}\n"));
    }
    t.push_str(w.3);
    t
}

#[cfg(test)]
#[path = "digest/tests.rs"]
mod tests;
