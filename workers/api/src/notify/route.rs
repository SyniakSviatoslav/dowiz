//! THE ROUTER (W-TG T4): one event, every group that wants it, in each
//! group's language and with each group's amount of the customer. PURE.
//!
//! THE FLOW. A producer writes ONE routed entry into the venue's outbox in the
//! same turn as its event (kind [`ROUTE_KIND`], `to` = the event key, `text` =
//! the payload as JSON) -- it never needs to know who the groups are. The
//! minute drain (`outbox/rails.rs`) reads the groups from the settings it
//! already loads, calls [`fan_out`], and puts one ordinary `telegram` entry per
//! group back into the same image in the same write (id `{id}/g/{group}`), so
//! a group whose chat fails retries alone, and the others are not sent twice.
//! Quiet hours are `next_at_ms`; a "daily summary" choice is a digest line.
//!
//! A LEGACY VENUE never sees a routed entry: `bell` answers the old tickets,
//! byte for byte, when `notify.tg.groups` was never written.

use serde_json::Value;

use crate::outbox::Entry;
use dowiz_hub::tz::Zone;

pub mod events;
pub mod groups;
pub mod health;
pub mod render;
pub use groups::{Group, Groups, Mode, State, Window};

/// The outbox entry kind of an event waiting to be fanned out.
pub const ROUTE_KIND: &str = "route";
/// Outbox table record kind: a line waiting for a group's next summary.
pub const DIGEST_KIND: &str = "dg";
/// Outbox table record kind: a target's last success and last error.
pub const HEALTH_KIND: &str = "h";
/// At most this many sends to one chat per drain (`outbox::tgrail::Pace`):
/// Telegram allows about 20 a minute in a group, and the drain runs once a minute.
pub const PER_CHAT: usize = 15;
const DAY: i64 = 86_400_000;

/// A routed event, ready for the outbox. `payload` is one of:
/// `{"ticket": text, "lines": [...], "amend": bool}` (an order),
/// `{"texts": {"en": ..}}` (already written per language),
/// `{"data": {..}}` (rendered by [`render::event`]).
pub fn routed(id: String, ev: &str, payload: &Value, now_ms: i64) -> Entry {
    Entry::new(id, ROUTE_KIND, ev.to_string(), payload.to_string(), now_ms)
}

/// The groups of a venue from its settings (legacy keys read as groups).
pub fn groups_of(s: &dowiz_hub::settings::Settings, lang: &str) -> Result<Groups, String> {
    groups::load(
        s.get(groups::KEY_GROUPS).as_deref(),
        &s.known(groups::LEGACY_CHAT),
        &s.known(groups::LEGACY_BAR),
        lang,
    )
}

/// THE BELL (hand-back for `hubdo::enqueue_bell`): today's tickets for a
/// venue that never wrote groups, else one routed entry.
pub fn bell(s: &dowiz_hub::settings::Settings, order_id: &str, text: &str, lines: &[Value], amend_seq: Option<u64>, now_ms: i64) -> Vec<Entry> {
    if s.get(groups::KEY_GROUPS).is_none() {
        return crate::bell_route::telegram_tickets(order_id, text, lines, amend_seq, &s.known(groups::LEGACY_CHAT), &s.known(groups::LEGACY_BAR))
            .into_iter()
            .map(|t| Entry::new(t.id, "telegram", t.to, t.text, now_ms))
            .collect();
    }
    let (id, ev) = match amend_seq {
        Some(seq) => (format!("{order_id}/amend/{seq}/route"), "order.amended"),
        None => (format!("{order_id}/route"), "order.placed"),
    };
    let payload = serde_json::json!({ "ticket": text, "lines": lines, "amend": amend_seq.is_some() });
    vec![routed(id, ev, &payload, now_ms)]
}

/// Where an exception alert goes: the legacy chat, or `@order.exception`
/// (a route) once groups exist. Empty = nobody to tell.
pub fn alert_target(s: &dowiz_hub::settings::Settings) -> String {
    if s.get(groups::KEY_GROUPS).is_some() {
        return format!("@{}", "order.exception");
    }
    s.known(groups::LEGACY_CHAT).trim().to_string()
}

/// An alert entry: to a legacy chat as today, or routed with one text per language.
pub fn alert_entry(id: String, to: &str, texts: &dyn Fn(&str) -> String, now_ms: i64) -> Entry {
    match to.strip_prefix('@') {
        Some(ev) => {
            let all: serde_json::Map<String, Value> = render::langs().into_iter().map(|l| (l.to_string(), Value::from(texts(l)))).collect();
            routed(id, ev, &serde_json::json!({ "texts": all }), now_ms)
        }
        None => Entry::new(id, "telegram", to.to_string(), texts(""), now_ms),
    }
}

/// What one routed entry becomes.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Plan {
    pub send: Vec<Entry>,
    /// (group id, line) for the group's next summary.
    pub digest: Vec<(String, String)>,
}

/// THE ROUTER. `e` is a [`ROUTE_KIND`] entry.
pub fn fan_out(e: &Entry, groups: &[Group], zone: Zone, now_ms: i64) -> Plan {
    let ev = e.to.as_str();
    let payload: Value = serde_json::from_str(&e.text).unwrap_or(Value::Null);
    let lines: Vec<Value> = payload.get("lines").and_then(Value::as_array).cloned().unwrap_or_default();
    let has_bar = lines.iter().any(|l| crate::bell_route::Station::of_line(l) == crate::bell_route::Station::Bar);
    let split = has_bar && groups.iter().any(|g| g.station.as_deref() == Some("bar") && g.mode(ev) == Mode::Now);
    let mut plan = Plan::default();
    for g in groups {
        let mode = g.mode(ev);
        if mode == Mode::Off {
            continue;
        }
        let Some(text) = text_for(g, ev, &payload, &lines, split) else { continue };
        if mode == Mode::Digest {
            plan.digest.push((g.id.clone(), text.lines().take(3).collect::<Vec<_>>().join(" · ")));
            continue;
        }
        let mut out = Entry::new(format!("{}/g/{}", e.id, g.id), "telegram", g.target(), text, e.queued_at_ms);
        out.next_at_ms = now_ms;
        if let (Some(w), true) = (g.quiet, events::holds_in_quiet(ev)) {
            if let Some(end) = quiet_until(w, zone, now_ms) {
                out.next_at_ms = end;
            }
        }
        plan.send.push(out);
    }
    plan
}

/// One group's text, or `None` when it has nothing to say to that group.
fn text_for(g: &Group, ev: &str, p: &Value, lines: &[Value], split: bool) -> Option<String> {
    use crate::bell_route::{header_of, ticket_text, Station};
    let t = if let Some(ticket) = p.get("ticket").and_then(Value::as_str) {
        let t = render::ticket(ticket, &g.lang, g.pii);
        let amend = p.get("amend").and_then(Value::as_bool).unwrap_or(false);
        let station = match g.station.as_deref() {
            Some("bar") => Some(Station::Bar),
            Some(_) => Some(Station::Kitchen),
            None => None,
        };
        match (split, station) {
            (true, Some(st)) => {
                let mine: Vec<Value> = lines.iter().filter(|l| Station::of_line(l) == st).cloned().collect();
                if mine.is_empty() {
                    return None;
                }
                ticket_text(header_of(&t), Some(st), &mine)
            }
            (false, Some(Station::Bar)) => return None,
            _ if amend => ticket_text(header_of(&t), None, lines),
            _ => t,
        }
    } else if let Some(texts) = p.get("texts") {
        texts.get(&g.lang).or_else(|| texts.get("en")).and_then(Value::as_str).unwrap_or("").to_string()
    } else if ev == "inbox.message" {
        render::inbox(p.get("data").unwrap_or(&Value::Null), &g.lang, g.pii)
    } else {
        render::event(ev, p.get("data").unwrap_or(&Value::Null), &g.lang)
    };
    (!t.trim().is_empty()).then_some(t)
}

/// When a quiet window that holds `now_ms` ends, or `None` outside it.
pub fn quiet_until(w: Window, zone: Zone, now_ms: i64) -> Option<i64> {
    let local = dowiz_hub::tz::local_ms(zone, now_ms);
    let day0 = local.div_euclid(DAY) * DAY;
    let m = (local - day0) / 60_000;
    let inside = if w.from < w.to { m >= w.from && m < w.to } else { m >= w.from || m < w.to };
    if !inside {
        return None;
    }
    let mut end = day0 + w.to * 60_000;
    if end <= local {
        end += DAY;
    }
    Some(end - (local - now_ms))
}

/// The next instant, after `now_ms`, of venue-local minute `at`; `weekly`
/// = the next Monday at that minute.
pub fn next_at(at: i64, weekly: bool, zone: Zone, now_ms: i64) -> i64 {
    let local = dowiz_hub::tz::local_ms(zone, now_ms);
    let day0 = local.div_euclid(DAY) * DAY;
    let mut t = day0 + at * 60_000;
    if t <= local {
        t += DAY;
    }
    // 1970-01-01 was a Thursday: (days + 3) % 7 == 0 is a Monday.
    while weekly && (t.div_euclid(DAY) + 3).rem_euclid(7) != 0 {
        t += DAY;
    }
    t - (local - now_ms)
}

/// The summaries each group is owed: `(key, next_ms)`, key `{group}/daily`
/// or `{group}/weekly`. A group that put anything "in the summary" gets the
/// daily one, or its lines would wait for ever.
pub fn dues(groups: &[Group], zone: Zone, now_ms: i64) -> Vec<(String, i64)> {
    let mut out = Vec::new();
    for g in groups.iter().filter(|g| g.state == State::Active) {
        let any_digest = g.subs.values().any(|m| *m == Mode::Digest);
        if g.mode("digest.daily") == Mode::Now || any_digest {
            out.push((format!("{}/daily", g.id), next_at(g.digest_at, false, zone, now_ms)));
        }
        if g.mode("digest.weekly") == Mode::Now {
            out.push((format!("{}/weekly", g.id), next_at(g.digest_at, true, zone, now_ms)));
        }
    }
    out
}

#[cfg(test)]
mod tests;
