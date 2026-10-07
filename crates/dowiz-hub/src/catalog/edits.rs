//! THE CATALOGUE'S EDIT JOURNAL = point-in-time recovery for a KV image (W-PITR, D.1 #11).
//!
//! The catalogue is a KV image: it holds the menu as it is NOW and nothing about how it got
//! there. Every write that changes it also appends one record per changed key to an append
//! log (`LogImage`, kind [`KIND`]): when, who (a STAFF id, never a customer), the key, the
//! sha256 of the value it replaced, and the new value (`None` = removed). Replaying the log
//! from its first record rebuilds the catalogue at any point; [`rebuild`] turns that state
//! back into a catalogue whose COMPACTED bytes equal the compacted bytes of the catalogue the
//! venue had then. Compacted, not the stored image: since W-DELTA a stored image is a delta
//! chain whose bytes depend on its write history, so two equal menus can differ there; the
//! compacted form is canonical (entries sorted, one root), and `root()` agrees with it.
//!
//! THE OLD DIGEST IS THE CHECK. A record says what the key held before it; replay refuses
//! the first record whose `old` does not match the state rebuilt so far. So a record lost
//! from the middle, or one edited in place, stops the replay by name ([`ReplayError::Gap`])
//! instead of yielding a plausible wrong menu.
//!
//! THE WRITER DIFFS AGAINST THE JOURNAL ITSELF ([`journal`]). Before recording `before ->
//! after` it replays the journal and records any difference between that and `before` as
//! [`UNSEEN`] ("changed by a path that did not journal, or whose journal write failed").
//! The first journaled write therefore records the whole menu as its baseline, and a missed
//! write is caught at the next one -- with the later write's time, and said so.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use super::Catalog;
use crate::logimage::{Entry, LogImage};
use crate::HubError;

/// The record kind. Decoded by [`decode`] -- which exists before any writer does.
pub const KIND: &str = "catalog.edit";
/// The record's format version.
pub const VERSION: i64 = 1;
/// `by` of a change the journal had not seen when the next journaled write found it.
pub const UNSEEN: &str = "?";
/// `by` of the menu an empty journal first meets, and of a compaction's state at its cut.
pub const BASELINE: &str = "baseline";
/// Past this many records a write compacts the journal to a baseline plus the newest [`KEEP`].
pub const MAX_RECORDS: usize = 600;
pub const KEEP: usize = 300;

/// A catalogue as its keys: `product:<id>`, `category:<id>`, `supply:`, `promo:`, `location`.
pub type State = BTreeMap<String, String>;

/// One decoded record. `seq` is its position in the journal, oldest = 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub seq: u64,
    pub at_ms: i64,
    pub by: String,
    pub key: String,
    /// sha256 (hex) of what the key held before; `None` = it did not exist.
    pub old: Option<String>,
    /// What it holds after; `None` = removed.
    pub new: Option<String>,
    /// On the LAST record of a write: the catalogue IMAGE's generation after it, so the next
    /// write can tell in O(1) that nothing wrote the catalogue behind the journal's back.
    pub gen: Option<i64>,
}

/// Why a replay stopped. Either is a failing state, never an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    /// A record this build cannot read (a failed crc, a foreign kind, a bad field).
    Unreadable { seq: u64, reason: &'static str },
    /// A record whose `old` is not the digest of what replay holds for its key: a record
    /// before it is missing or was changed.
    Gap { seq: u64, key: String },
}

/// Where a replay stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cut {
    All,
    /// Records stamped at or before this time, in journal order, up to the first after it.
    Time(i64),
    /// Records before this position.
    Seq(u64),
}

pub fn digest(v: &str) -> String {
    Sha256::digest(v.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// Every key of a catalogue and its value.
pub fn state_of(cat: &Catalog) -> State {
    cat.entries_with_prefix("").into_iter().collect()
}

/// The changes that turn `before` into `after`, in key order: `(key, old digest, new)`.
pub fn diff(before: &State, after: &State) -> Vec<(String, Option<String>, Option<String>)> {
    let mut out = Vec::new();
    for (k, v) in after {
        if before.get(k) != Some(v) {
            out.push((k.clone(), before.get(k).map(|o| digest(o)), Some(v.clone())));
        }
    }
    for (k, o) in before {
        if !after.contains_key(k) {
            out.push((k.clone(), Some(digest(o)), None));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn record_json(at_ms: i64, by: &str, key: &str, old: &Option<String>, new: &Option<String>, gen: Option<i64>) -> String {
    let mut j = serde_json::json!({ "v": VERSION, "at": at_ms, "by": by, "key": key, "old": old, "new": new });
    if let Some(g) = gen {
        j["gen"] = serde_json::json!(g);
    }
    j.to_string()
}

/// The catalogue generation the newest record carries, if it carries one.
fn last_gen(log: &LogImage) -> Option<i64> {
    log.newest().and_then(|e| decode(&e).ok()).and_then(|e| e.gen)
}

/// One record, or the reason it is not one of ours.
pub fn decode(e: &Entry) -> Result<Edit, &'static str> {
    if e.kind != KIND {
        return Err("foreign-kind");
    }
    let v: serde_json::Value = serde_json::from_str(&e.json).map_err(|_| "json")?;
    if v.get("v").and_then(|x| x.as_i64()) != Some(VERSION) {
        return Err("version");
    }
    let at_ms = v.get("at").and_then(|x| x.as_i64()).ok_or("at")?;
    let by = v.get("by").and_then(|x| x.as_str()).ok_or("by")?.to_string();
    let key = v.get("key").and_then(|x| x.as_str()).ok_or("key")?.to_string();
    if key != e.subject {
        return Err("subject");
    }
    let opt = |f: &str| -> Result<Option<String>, &'static str> {
        match v.get(f) {
            Some(serde_json::Value::Null) => Ok(None),
            Some(serde_json::Value::String(s)) => Ok(Some(s.clone())),
            _ => Err("old-or-new"),
        }
    };
    let (old, new) = (opt("old")?, opt("new")?);
    if old.as_deref().is_some_and(|d| d.len() != 64) {
        return Err("old-digest");
    }
    let gen = match v.get("gen") {
        None => None,
        Some(g) => Some(g.as_i64().ok_or("gen")?),
    };
    Ok(Edit { seq: e.seq, at_ms, by, key, old, new, gen })
}

/// Every record, OLDEST first, decoded; the first one that is not readable stops it.
pub fn edits(log: &LogImage) -> Result<Vec<Edit>, ReplayError> {
    if let Some(q) = log.quarantined().first() {
        let seq = (log.len() - 1 - q.at) as u64;
        return Err(ReplayError::Unreadable { seq, reason: q.reason });
    }
    let mut all = log.entries();
    all.reverse();
    all.iter().map(|e| decode(e).map_err(|reason| ReplayError::Unreadable { seq: e.seq, reason })).collect()
}

/// Apply records in order, checking each one's `old` against the state so far.
pub fn apply(state: &mut State, list: &[Edit], cut: Cut) -> Result<(), ReplayError> {
    for e in list {
        match cut {
            Cut::Time(t) if e.at_ms > t => break,
            Cut::Seq(n) if e.seq >= n => break,
            _ => {}
        }
        if state.get(&e.key).map(|v| digest(v)) != e.old {
            return Err(ReplayError::Gap { seq: e.seq, key: e.key.clone() });
        }
        match &e.new {
            Some(v) => state.insert(e.key.clone(), v.clone()),
            None => state.remove(&e.key),
        };
    }
    Ok(())
}

/// The catalogue's keys as the journal says they stood at `cut`.
pub fn replay(log: &LogImage, cut: Cut) -> Result<State, ReplayError> {
    let mut state = State::new();
    apply(&mut state, &edits(log)?, cut)?;
    Ok(state)
}

/// Set one key of a catalogue by its full name (`None` removes it).
pub fn set_key(cat: &mut Catalog, key: &str, value: Option<&str>) {
    match value {
        Some(v) => cat.put(key.to_string(), v),
        None => {
            cat.del(key.to_string());
        }
    }
}

/// A fresh catalogue holding exactly `state`. Its `compact()` bytes are the canonical form.
pub fn rebuild(state: &State) -> Result<Catalog, HubError> {
    let mut cat = Catalog::create()?;
    for (k, v) in state {
        set_key(&mut cat, k, Some(v));
    }
    Ok(cat)
}

/// What [`journal`] wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Journaled {
    /// Records for the write itself.
    pub edits: usize,
    /// Records for changes the journal had not seen (`by` = [`UNSEEN`]).
    pub unseen: usize,
    /// Records of the menu an empty journal first met (`by` = [`BASELINE`]).
    pub baseline: usize,
    /// The journal was compacted to a baseline.
    pub compacted: bool,
}

/// Record the write `before -> after` by `by` at `at_ms`. See the module for the order.
///
/// `gens` = the catalogue image's generation before and after this write.
///
/// THE COMMON PATH IS O(write), NOT O(journal): when the newest record's `gen` is the
/// generation `before` was read at, nothing wrote the catalogue behind the journal and it is
/// not replayed -- no hashing of the whole menu on every edit (the Free plan's 10 ms). Otherwise it is
/// replayed in full and the difference is written first, as [`UNSEEN`]. A journal that does
/// not replay is NOT written to: the caller says so out loud.
pub fn journal(log: &mut LogImage, before: &State, after: &State, at_ms: i64, by: &str, gens: (i64, i64)) -> Result<Journaled, JournalError> {
    let mut out = Journaled::default();
    let mut rows: Vec<(&str, String, Option<String>, Option<String>)> = vec![];
    let in_step = if log.is_empty() { before.is_empty() } else { last_gen(log) == Some(gens.0) };
    if !in_step {
        // An EMPTY journal meeting a menu is where the history starts (the first write after
        // this shipped, or a venue restored without its journal): a baseline, not drift.
        let who = if log.is_empty() { BASELINE } else { UNSEEN };
        let known = replay(log, Cut::All).map_err(JournalError::Replay)?;
        rows.extend(diff(&known, before).into_iter().map(|(k, o, n)| (who, k, o, n)));
        if who == UNSEEN { out.unseen = rows.len() } else { out.baseline = rows.len() }
    }
    rows.extend(diff(before, after).into_iter().map(|(k, o, n)| (by, k, o, n)));
    out.edits = rows.len() - out.unseen - out.baseline;
    let last = rows.len().checked_sub(1);
    for (i, (who, key, old, new)) in rows.iter().enumerate() {
        let g = (Some(i) == last).then_some(gens.1);
        log.append(KIND, key, &record_json(at_ms, who, key, old, new, g)).map_err(JournalError::Hub)?;
    }
    if log.len() > MAX_RECORDS {
        *log = compacted(log, KEEP)?;
        out.compacted = true;
    }
    Ok(out)
}

#[derive(Debug)]
pub enum JournalError {
    Replay(ReplayError),
    Hub(HubError),
}

/// Compaction, the console's newest records, and what a key held before an edit.
mod history;
pub use history::{before, compacted, recent};

#[cfg(test)]
mod tests;
