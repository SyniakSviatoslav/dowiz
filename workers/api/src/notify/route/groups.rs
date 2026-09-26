//! THE GROUPS MODEL (W-TG T2): which Telegram chats the venue's bot writes to,
//! and what each one wants. ONE settings key, a JSON array; no new image.
//!
//! A LEGACY VENUE KEEPS WORKING WITH NO WRITE: with no `notify.tg.groups`, the
//! two old keys are READ as groups (kitchen: today's events and full ticket;
//! bar). The owner's first change on the Telegram screen writes them.
//!
//! PURE: a group list in, a group list out. The settings image is the caller's.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::events;

/// The groups, as JSON (`Vec<Group>`).
pub const KEY_GROUPS: &str = "notify.tg.groups";
/// The one pending link code (`Pending`).
pub const KEY_LINK: &str = "notify.tg.link";
/// The webhook's `secret_token`; the key ends in `secret`, so it is redacted.
pub const KEY_SECRET: &str = "notify.tg.secret";
/// The bot's @username and when its webhook was set (`Bot`).
pub const KEY_BOT: &str = "notify.tg.bot";
pub const LEGACY_CHAT: &str = "notify.telegram.chat";
pub const LEGACY_BAR: &str = "notify.telegram.chat.bar";
/// The summary's default time, venue-local minutes (09:00).
pub const DIGEST_AT: i64 = 9 * 60;
/// A venue with more groups than this is a configuration error, not a venue.
pub const MAX_GROUPS: usize = 12;

/// How much of the CUSTOMER a group's messages carry. The default is none.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Pii {
    /// Dishes, quantities, kind of order and table. No name, phone, address, note.
    #[default]
    None,
    /// + the name initial, phone, address and note: what a courier needs.
    Fulfil,
    /// Everything today's ticket carries, and a customer's message text.
    Full,
}

/// What a group gets for one event.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Now,
    Digest,
    Off,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum State {
    #[default]
    Active,
    /// The bot was removed (403) or left: nothing is sent until it is re-linked.
    Left,
    /// The owner paused it.
    Muted,
}

/// Venue-local minutes of the day, `from` inclusive, `to` exclusive; may wrap midnight.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    pub from: i64,
    pub to: i64,
}

/// Who linked the group. `tg_user` is a Telegram user id: personal data of staff.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Linked {
    pub at_ms: i64,
    pub by: String,
    pub tg_user: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub id: String,
    pub chat: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread: Option<i64>,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default = "default_lang")]
    pub lang: String,
    #[serde(default)]
    pub pii: Pii,
    #[serde(default)]
    pub subs: BTreeMap<String, Mode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quiet: Option<Window>,
    #[serde(default = "default_digest_at")]
    pub digest_at: i64,
    /// `kitchen` / `bar`: a split order sends it only its lines (`bell_route`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub station: Option<String>,
    #[serde(default)]
    pub state: State,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked: Option<Linked>,
}

fn default_lang() -> String {
    "en".into()
}
fn default_digest_at() -> i64 {
    DIGEST_AT
}

impl Group {
    /// `chat` or `chat:thread`, the outbox entry's `to`.
    pub fn target(&self) -> String {
        crate::notify::tg::target_text(&self.chat, self.thread)
    }

    /// What this group gets for `ev`. A group that is not active gets nothing.
    pub fn mode(&self, ev: &str) -> Mode {
        if self.state != State::Active {
            return Mode::Off;
        }
        self.subs.get(ev).copied().unwrap_or(Mode::Off)
    }

    /// A freshly linked group: nothing of the customer, the likeliest events.
    pub fn fresh(id: String, chat: String, thread: Option<i64>, title: String, kind: String, lang: &str) -> Group {
        let mut subs = BTreeMap::new();
        for ev in ["order.placed", "order.amended", "stock.low", "system.alert"] {
            subs.insert(ev.to_string(), Mode::Now);
        }
        Group {
            id, chat, thread, title, kind, lang: lang.to_string(), pii: Pii::None, subs, quiet: None,
            digest_at: DIGEST_AT, station: None, state: State::Active, linked: None,
        }
    }
}

/// The groups a venue has, and whether they were read from the old keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Groups {
    pub list: Vec<Group>,
    /// True when `notify.tg.groups` was never written: `list` is derived.
    pub legacy: bool,
}

/// Read the groups from the three settings values. PURE.
///
/// `stored` is `notify.tg.groups`; an unreadable value is an ERROR, never
/// silently "no groups" -- that would stop every message without a word.
pub fn load(stored: Option<&str>, chat: &str, bar: &str, lang: &str) -> Result<Groups, String> {
    match stored.map(str::trim).filter(|s| !s.is_empty()) {
        Some(json) => serde_json::from_str::<Vec<Group>>(json)
            .map(|list| Groups { list, legacy: false })
            .map_err(|e| format!("{KEY_GROUPS} is unreadable: {e}")),
        None => Ok(Groups { list: legacy(chat, bar, lang), legacy: true }),
    }
}

/// The old keys as groups: today's events, today's full ticket.
pub fn legacy(chat: &str, bar: &str, lang: &str) -> Vec<Group> {
    let (chat, bar) = (chat.trim(), bar.trim());
    let mut out = Vec::new();
    let mk = |id: &str, chat: &str, evs: &[&str], station: Option<&str>| {
        let mut g = Group::fresh(id.into(), chat.into(), None, String::new(), String::new(), lang);
        g.subs = evs.iter().map(|e| (e.to_string(), Mode::Now)).collect();
        g.pii = Pii::Full;
        g.station = station.map(str::to_string);
        g
    };
    if !chat.is_empty() {
        let station = (!bar.is_empty()).then_some("kitchen");
        out.push(mk("main", chat, &["order.placed", "order.amended", "order.exception", "inbox.message", "system.alert"], station));
    }
    if !bar.is_empty() {
        out.push(mk("bar", bar, &["order.placed", "order.amended"], Some("bar")));
    }
    out
}

/// What an owner may change about a group from the console.
#[derive(Deserialize, Debug, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct Patch {
    pub lang: Option<String>,
    pub pii: Option<Pii>,
    pub subs: Option<BTreeMap<String, Mode>>,
    /// `Some(None)` clears the quiet hours.
    #[serde(default, deserialize_with = "present")]
    pub quiet: Option<Option<Window>>,
    pub digest_at: Option<i64>,
    pub muted: Option<bool>,
    pub title: Option<String>,
}

/// Apply one owner change. Refuses rather than guesses: an unknown event, a
/// language nobody renders, a minute outside the day.
pub fn apply(list: &mut [Group], id: &str, p: &Patch) -> Result<(), String> {
    let g = list.iter_mut().find(|g| g.id == id).ok_or_else(|| format!("no group {id:?}"))?;
    if let Some(l) = &p.lang {
        if !super::render::speaks(l) {
            return Err(format!("language {l:?}: the messages are written in {}", super::render::langs().join(", ")));
        }
        g.lang = l.clone();
    }
    if let Some(pii) = p.pii {
        g.pii = pii;
    }
    if let Some(subs) = &p.subs {
        for (ev, mode) in subs {
            let def = events::get(ev).ok_or_else(|| format!("no event {ev:?}"))?;
            if *mode == Mode::Digest && def.class == events::Class::Scheduled {
                return Err(format!("{ev} is itself a summary: now or off"));
            }
            g.subs.insert(ev.clone(), *mode);
        }
    }
    if let Some(q) = p.quiet {
        if let Some(w) = q {
            if !minute_ok(w.from) || !minute_ok(w.to) || w.from == w.to {
                return Err("quiet hours are two different times of the day".into());
            }
        }
        g.quiet = q;
    }
    if let Some(at) = p.digest_at {
        if !minute_ok(at) {
            return Err("the summary time is a time of the day".into());
        }
        g.digest_at = at;
    }
    if let Some(m) = p.muted {
        // Muting never revives a group the bot was removed from: that takes a new link.
        g.state = match (g.state, m) {
            (State::Left, _) => State::Left,
            (_, true) => State::Muted,
            (_, false) => State::Active,
        };
    }
    if let Some(t) = &p.title {
        g.title = t.trim().chars().take(64).collect();
    }
    Ok(())
}

/// A field that is present is `Some`, even when it is `null` (= clear it).
fn present<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Option<Window>>, D::Error> {
    Option::<Window>::deserialize(d).map(Some)
}

fn minute_ok(m: i64) -> bool {
    (0..1440).contains(&m)
}

/// A stable id for a new group: from its title, unique in the list.
pub fn slug(title: &str, list: &[Group]) -> String {
    let base: String = title
        .chars()
        .filter_map(|c| if c.is_ascii_alphanumeric() { Some(c.to_ascii_lowercase()) } else if c == ' ' || c == '-' { Some('-') } else { None })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(24)
        .collect();
    let base = if base.is_empty() { "group".to_string() } else { base };
    let mut id = base.clone();
    let mut n = 2;
    while list.iter().any(|g| g.id == id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

/// A group moved chats (a supergroup upgrade): every group on `from` now
/// lives on `to`. Answers whether anything changed.
pub fn migrate(list: &mut [Group], from: &str, to: &str) -> bool {
    let mut hit = false;
    for g in list.iter_mut().filter(|g| g.chat == from) {
        g.chat = to.to_string();
        hit = true;
    }
    hit
}

/// The bot is gone from `chat`: every group on it is marked left.
pub fn left(list: &mut [Group], chat: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for g in list.iter_mut().filter(|g| g.chat == chat && g.state != State::Left) {
        g.state = State::Left;
        ids.push(g.id.clone());
    }
    ids
}

#[cfg(test)]
mod tests;
