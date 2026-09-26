//! WHAT TELEGRAM TELLS THE HUB, AND WHAT IT CHANGES (W-TG T3). PURE.
//!
//! A GROUP IS LINKED BY A CODE, NEVER BY BEING ADDED. Anyone can add the
//! venue's bot to any group, so `my_chat_member` alone links nothing: a code
//! minted in the owner's console, sent as `/link CODE` (or `/start CODE` from
//! the `startgroup` deep link) in the group, is the only authority. The bot
//! keeps privacy mode ON: it hears commands addressed to it and service
//! messages, never the kitchen's chatter.
//!
//! `my_chat_member` and the migration service messages only KEEP state: a
//! kicked bot marks its groups left, an upgraded group moves to its new id.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::notify::route::groups::{self, Group, Linked, State};

/// A link code lives this long, and is good once.
pub const TTL_MS: i64 = 10 * 60 * 1000;
/// No 0/O, 1/I: the code is read off a phone and typed into another.
const ALPHABET: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// The one pending code (`notify.tg.link`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Pending {
    pub code: String,
    pub exp_ms: i64,
    /// The owner who minted it (their user id).
    pub by: String,
}

/// A code from eight random bytes.
pub fn mint(bytes: [u8; 8], by: &str, now_ms: i64) -> Pending {
    let code = bytes.iter().map(|b| ALPHABET[(*b as usize) % 32] as char).collect();
    Pending { code, exp_ms: now_ms + TTL_MS, by: by.to_string() }
}

/// Typed with a dash, in lower case, with spaces: still the code. Constant time.
pub fn matches(p: &Pending, typed: &str, now_ms: i64) -> bool {
    use subtle::ConstantTimeEq;
    let t: String = typed.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_uppercase()).collect();
    now_ms <= p.exp_ms && t.len() == p.code.len() && bool::from(t.as_bytes().ct_eq(p.code.as_bytes()))
}

/// One update, read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Update {
    Link { chat: String, thread: Option<i64>, title: String, kind: String, code: String, from: i64 },
    Migrated { from: String, to: String },
    Member { chat: String, status: String },
    Ignore,
}

fn id_of(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_i64).map(|n| n.to_string())
}

/// Read an update. `bot` is the bot's @username (without `@`), so a command
/// addressed to ANOTHER bot in the same group is not ours.
pub fn parse(u: &Value, bot: &str) -> Update {
    if let Some(m) = u.get("my_chat_member") {
        let chat = id_of(m.get("chat").and_then(|c| c.get("id")));
        let status = m.get("new_chat_member").and_then(|n| n.get("status")).and_then(Value::as_str);
        return match (chat, status) {
            (Some(chat), Some(s)) => Update::Member { chat, status: s.to_string() },
            _ => Update::Ignore,
        };
    }
    let Some(m) = u.get("message") else { return Update::Ignore };
    let Some(chat) = id_of(m.get("chat").and_then(|c| c.get("id"))) else { return Update::Ignore };
    if let Some(to) = id_of(m.get("migrate_to_chat_id")) {
        return Update::Migrated { from: chat, to };
    }
    if let Some(from) = id_of(m.get("migrate_from_chat_id")) {
        return Update::Migrated { from, to: chat };
    }
    let text = m.get("text").and_then(Value::as_str).unwrap_or("").trim();
    let mut words = text.split_whitespace();
    let (Some(cmd), Some(code)) = (words.next(), words.next()) else { return Update::Ignore };
    let (name, to_bot) = cmd.split_once('@').map_or((cmd, None), |(c, b)| (c, Some(b)));
    if !matches!(name, "/link" | "/start") {
        return Update::Ignore;
    }
    if let Some(b) = to_bot {
        if !b.eq_ignore_ascii_case(bot.trim_start_matches('@')) {
            return Update::Ignore;
        }
    }
    let c = m.get("chat").cloned().unwrap_or(Value::Null);
    let topic = m.get("is_topic_message").and_then(Value::as_bool).unwrap_or(false);
    let title = c
        .get("title")
        .or_else(|| c.get("first_name"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .chars()
        .take(64)
        .collect();
    Update::Link {
        chat,
        thread: if topic { m.get("message_thread_id").and_then(Value::as_i64) } else { None },
        title,
        kind: c.get("type").and_then(Value::as_str).unwrap_or("").to_string(),
        code: code.to_string(),
        from: m.get("from").and_then(|f| f.get("id")).and_then(Value::as_i64).unwrap_or(0),
    }
}

/// What an update did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Effect {
    pub changed: bool,
    /// The code was used and must be cleared.
    pub consumed: bool,
    /// One line back to the chat (`target`, text), for a link attempt.
    pub reply: Option<(String, String)>,
}

/// The reply words, per language (the console's languages; English otherwise).
fn said(lang: &str, ok: bool) -> &'static str {
    match (lang, ok) {
        ("sq", true) => "U lidh me",
        ("sq", false) => "Kodi nuk vlen ose ka skaduar. Krijoni një të ri në panel.",
        ("uk", true) => "Підключено до",
        ("uk", false) => "Код недійсний або прострочений. Створіть новий у консолі.",
        ("ru", true) => "Подключено к",
        ("ru", false) => "Код недействителен или истёк. Создайте новый в консоли.",
        (_, true) => "Linked to",
        (_, false) => "This code is not valid or has expired. Make a new one in the console.",
    }
}

/// Apply one update to the groups. `lang` is the venue's language (a new
/// group starts in it), `venue` its name for the reply.
pub fn apply(list: &mut Vec<Group>, pending: Option<&Pending>, u: &Update, lang: &str, venue: &str, now_ms: i64) -> Effect {
    match u {
        Update::Link { chat, thread, title, kind, code, from } => {
            let target = crate::notify::tg::target_text(chat, *thread);
            let ok = pending.is_some_and(|p| matches(p, code, now_ms));
            let known = list.iter().position(|g| &g.chat == chat && g.thread == *thread);
            if !ok || (known.is_none() && list.len() >= groups::MAX_GROUPS) {
                return Effect { reply: Some((target, format!("✗ {}", said(lang, false)))), ..Effect::default() };
            }
            let by = pending.map(|p| p.by.clone()).unwrap_or_default();
            let linked = Linked { at_ms: now_ms, by, tg_user: *from };
            match known {
                // RE-LINKING a group the bot was removed from revives it, as it was.
                Some(i) => {
                    let g = &mut list[i];
                    g.state = State::Active;
                    g.title = title.clone();
                    g.linked = Some(linked);
                }
                None => {
                    let mut g = Group::fresh(groups::slug(title, list), chat.clone(), *thread, title.clone(), kind.clone(), lang);
                    g.linked = Some(linked);
                    list.push(g);
                }
            }
            Effect { changed: true, consumed: true, reply: Some((target, format!("✅ {} {venue}", said(lang, true)))) }
        }
        Update::Migrated { from, to } => Effect { changed: groups::migrate(list, from, to), ..Effect::default() },
        Update::Member { chat, status } if status == "left" || status == "kicked" => {
            Effect { changed: !groups::left(list, chat).is_empty(), ..Effect::default() }
        }
        Update::Member { .. } | Update::Ignore => Effect::default(),
    }
}

#[cfg(test)]
mod tests;
