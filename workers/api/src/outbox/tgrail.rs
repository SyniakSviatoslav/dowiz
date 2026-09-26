//! THE DRAIN'S TELEGRAM SIDE (W-TG T1/T4): fan-out of routed events, the
//! classified refusals, per-chat pacing, the summaries, and the one list of
//! changes the drain writes back in ONE image write (`Ops`).
//!
//! The rules are pure and tested where they live (`notify::tg::outcome`,
//! `notify::route::fan_out`, `outbox::digest`); this file is the
//! bookkeeping between them, and [`Ops::apply`] is its only write.

use std::collections::BTreeMap;

use super::{digest, Entry, KIND};
use crate::notify::route::{self, health::Health, Group, DIGEST_KIND};
use crate::notify::tg::{self, Fail, Outcome};

/// Everything one drain changes in the outbox image.
#[derive(Default)]
pub struct Ops {
    /// Whole entries to write (retried, rescheduled, fanned out).
    pub puts: BTreeMap<String, Entry>,
    pub removes: Vec<String>,
    /// (record kind, key, value; `None` removes).
    pub records: Vec<(&'static str, String, Option<String>)>,
    /// Chats the bot is gone from: every Telegram entry to them is abandoned.
    pub gone: Vec<(String, String)>,
    /// Chats that moved (old, new): every entry to the old one is re-aimed.
    pub moved: Vec<(String, String)>,
}

impl Ops {
    pub fn put(&mut self, e: Entry) {
        self.removes.retain(|id| *id != e.id);
        self.puts.insert(e.id.clone(), e);
    }

    pub fn remove(&mut self, id: &str) {
        self.puts.remove(id);
        self.removes.push(id.to_string());
    }

    /// Apply to the image, and name every entry abandoned for a gone chat.
    pub fn apply(&self, t: &mut dowiz_hub::table::Table) -> Result<Vec<String>, String> {
        for id in &self.removes {
            t.remove(KIND, id);
        }
        let mut dropped = Vec::new();
        for (id, j) in t.all(KIND) {
            let Ok(mut e) = serde_json::from_str::<Entry>(&j) else { continue };
            if e.kind != "telegram" || self.puts.contains_key(&id) {
                continue;
            }
            let chat = tg::target_of(&e.to).chat;
            if self.gone.iter().any(|(c, _)| *c == chat) {
                t.remove(KIND, &id);
                dropped.push(id);
            } else if let Some((_, to)) = self.moved.iter().find(|(from, _)| *from == chat) {
                e.to = tg::target_text(to, tg::target_of(&e.to).thread);
                put(t, KIND, &id, &serde_json::to_string(&e).unwrap_or_default())?;
            }
        }
        for (id, e) in &self.puts {
            let mut e = e.clone();
            if e.kind == "telegram" {
                let at = tg::target_of(&e.to);
                if self.gone.iter().any(|(c, _)| *c == at.chat) {
                    t.remove(KIND, id);
                    dropped.push(id.clone());
                    continue;
                }
                if let Some((_, to)) = self.moved.iter().find(|(from, _)| *from == at.chat) {
                    e.to = tg::target_text(to, at.thread);
                }
            }
            put(t, KIND, id, &serde_json::to_string(&e).unwrap_or_default())?;
        }
        for (kind, key, v) in &self.records {
            match v {
                Some(v) => put(t, kind, key, v)?,
                None => {
                    t.remove(kind, key);
                }
            }
        }
        Ok(dropped)
    }
}

fn put(t: &mut dowiz_hub::table::Table, kind: &str, id: &str, rec: &str) -> Result<(), String> {
    t.put(kind, id, rec, &[], &[]).map_err(|e| format!("outbox: {e:?}"))
}

/// Replace every routed entry by its per-group entries and digest lines;
/// bring the summaries in line with the groups. Returns the working list.
pub fn route_all(entries: Vec<Entry>, groups: &[Group], zone: dowiz_hub::tz::Zone, now_ms: i64, ops: &mut Ops) -> Vec<Entry> {
    let have: Vec<&Entry> = entries.iter().filter(|e| e.kind == digest::KIND).collect();
    let (put, remove) = digest::reconcile(&have, &digest::wanted(groups, zone, now_ms));
    let mut work: Vec<Entry> = Vec::new();
    for e in &entries {
        if remove.contains(&e.id) {
            ops.remove(&e.id);
        } else if e.kind == route::ROUTE_KIND {
            let plan = route::fan_out(e, groups, zone, now_ms);
            ops.remove(&e.id);
            for (gid, line) in plan.digest {
                ops.records.push((DIGEST_KIND, format!("{gid}/{}", e.id), Some(digest::line_record(&line, now_ms))));
            }
            for x in plan.send {
                ops.put(x.clone());
                work.push(x);
            }
        } else if !put.iter().any(|p| p.id == e.id) {
            work.push(e.clone());
        }
    }
    for p in put {
        ops.put(p.clone());
        work.push(p);
    }
    work
}

/// Per-drain Telegram state: pacing, stopped chats, health.
#[derive(Default)]
pub struct Pace {
    sent: BTreeMap<String, usize>,
    stopped: Vec<String>,
    pub health: BTreeMap<String, Health>,
}

impl Pace {
    /// May this target be sent to now? Counts the attempt when it may.
    pub fn admit(&mut self, to: &str) -> bool {
        let chat = tg::target_of(to).chat;
        if self.stopped.contains(&chat) {
            return false;
        }
        let n = self.sent.entry(chat).or_default();
        *n += 1;
        *n <= route::PER_CHAT
    }

    fn stop(&mut self, to: &str) {
        self.stopped.push(tg::target_of(to).chat);
    }

    /// Record one attempt's result in the target's health.
    pub fn note(&mut self, before: Option<String>, to: &str, r: &Result<(), Fail>, now_ms: i64) {
        let h = self.health.remove(to).unwrap_or_else(|| Health::read(before.as_deref()));
        let err = r.as_ref().err().map(Fail::words).unwrap_or_default();
        self.health.insert(to.to_string(), h.after(r.is_ok(), &err, now_ms));
    }
}

/// Apply one Telegram attempt's outcome. Returns (sent, kept, abandoned line).
pub fn settle(e: &Entry, r: &Result<(), Fail>, now_ms: i64, ops: &mut Ops, pace: &mut Pace) -> (bool, bool, Option<String>) {
    match tg::outcome(e, r, now_ms) {
        Outcome::Sent => {
            ops.remove(&e.id);
            (true, false, None)
        }
        Outcome::Retry { tries, next_at_ms } => {
            if tries == e.tries {
                pace.stop(&e.to); // 429: this chat waits, the others do not
            }
            ops.put(Entry { tries, next_at_ms, ..e.clone() });
            (false, true, None)
        }
        Outcome::Moved { to } => {
            let old = tg::target_of(&e.to).chat;
            ops.moved.push((old, tg::target_of(&to).chat));
            pace.stop(&e.to);
            ops.put(Entry { to, next_at_ms: now_ms, ..e.clone() });
            (false, true, None)
        }
        Outcome::Gone { chat, why } => {
            pace.stop(&e.to);
            ops.remove(&e.id);
            let line = format!("{}: the bot is gone from chat {chat}: {why}", e.id);
            ops.gone.push((chat, why));
            (false, false, Some(line))
        }
        Outcome::Abandon { after } => {
            ops.remove(&e.id);
            (false, false, Some(format!("{} after {after} attempts", e.id)))
        }
    }
}

/// The owners' notice that a group is gone: routed as `system.alert`, in
/// every language, naming the group by its title.
pub fn gone_notice(chat: &str, groups: &[Group], now_ms: i64) -> Entry {
    let title = groups.iter().find(|g| g.chat == chat).map_or(chat.to_string(), |g| if g.title.is_empty() { g.id.clone() } else { g.title.clone() });
    let texts: serde_json::Map<String, serde_json::Value> = route::render::WORDS
        .iter()
        .map(|w| (w.lang.to_string(), serde_json::Value::from(format!("⚠ {} {title}", w.gone))))
        .collect();
    route::routed(format!("system/gone/{chat}/{now_ms}"), "system.alert", &serde_json::json!({ "texts": texts }), now_ms)
}

#[cfg(test)]
mod tests;
