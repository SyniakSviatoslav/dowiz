//! PURE. AN OFFLINE SALE PAST ITS 48 HOURS IS SAID ONCE.
//!
//! The paper receipt promised fiscalisation within 48 hours of the sale; with
//! sending OFF (`fiscal::SEND_ENABLED`) nothing registers it, so the owner
//! must be told when the promise lapses. THE EXISTING ALERT PATH carries it:
//! the venue's alert chat (`notify::route::alert_target`), one outbox entry
//! per run, its marker written beside it in the SAME outbox image write, as
//! the wallet legs' first-sight alert does (`exceptions::legs::first`).
//!
//! ONCE MEANS ONCE, WITH OR WITHOUT A CHAT. A venue with no alert chat still
//! gets the markers: the console shows the overdue sale in red, and an
//! unmarked deadline in the past would otherwise wake the venue's alarm every
//! minute for ever (`hubdo/timer.rs`).
//!
//! NOTHING HERE TOUCHES THE NETWORK: it returns outbox entries; the drain is
//! the venue's ordinary outbox, which is how every other alert leaves.

use super::PREFIX;
use crate::exceptions::alert::Voice;
use crate::fiscal::queue::DEADLINE_MS;
use crate::outbox::Entry;

/// The marker kind in the outbox table (the drain reads only `outbox::KIND`).
pub const MARK_KIND: &str = "offline.overdue.alerted";
/// How many sales one message names before "and N more".
const NAMED: usize = 5;

/// The fiscal entries that belong to offline sales.
pub fn offline(entries: &[Entry]) -> Vec<&Entry> {
    entries.iter().filter(|e| e.kind == crate::fiscal::queue::KIND && e.to.starts_with(PREFIX)).collect()
}

fn deadline(e: &Entry) -> i64 {
    e.queued_at_ms.saturating_add(DEADLINE_MS)
}

/// When the next alert is owed: the earliest deadline not yet marked.
pub fn next_alert(entries: &[Entry], marked: &dyn Fn(&str) -> bool) -> Option<i64> {
    offline(entries).into_iter().filter(|e| !marked(&e.id)).map(deadline).min()
}

fn short(order_id: &str) -> String {
    order_id.strip_prefix(PREFIX).unwrap_or(order_id).chars().take(8).collect()
}

/// The message in one language; `""` is the venue's own.
pub fn text(lang: &str, ids: &[String]) -> String {
    let n = ids.len();
    let mut named: Vec<String> = ids.iter().take(NAMED).map(|i| short(i)).collect();
    if n > NAMED {
        named.push(format!("+{}", n - NAMED));
    }
    let list = named.join(", ");
    match lang {
        "sq" => format!("Shitje offline pa NIVF: {n} kanë kaluar afatin 48 orë për fiskalizim ({list}). Fiskalizojini tani."),
        "uk" => format!("Офлайн-продажі без NIVF: {n} пропустили 48-годинний строк фіскалізації ({list}). Фіскалізуйте їх зараз."),
        "ru" => format!("Офлайн-продажи без NIVF: {n} пропустили 48-часовой срок фискализации ({list}). Фискализируйте их сейчас."),
        _ => format!("Offline sales without NIVF: {n} passed the 48 h fiscal deadline ({list}). Fiscalise them now."),
    }
}

/// The alert owed at `now_ms` and the markers to write with it. A deadline AT
/// `now` has passed (as `fiscal::queue::health` reads it).
pub fn due(entries: &[Entry], marked: &dyn Fn(&str) -> bool, now_ms: i64, venue: &Voice, chat: &str) -> (Vec<Entry>, Vec<String>) {
    let mut late: Vec<&Entry> = offline(entries).into_iter().filter(|e| !marked(&e.id) && deadline(e) <= now_ms).collect();
    if late.is_empty() {
        return (Vec::new(), Vec::new());
    }
    late.sort_by_key(|e| (e.queued_at_ms, e.id.clone()));
    let marks: Vec<String> = late.iter().map(|e| e.id.clone()).collect();
    let orders: Vec<String> = late.iter().map(|e| e.to.clone()).collect();
    let chat = chat.trim();
    if chat.is_empty() {
        return (Vec::new(), marks);
    }
    let id = format!("offline.overdue:{}", marks[0]);
    let entry = crate::notify::route::alert_entry(id, chat, &|l| text(venue.speaking(l).lang, &orders), now_ms);
    (vec![entry], marks)
}
