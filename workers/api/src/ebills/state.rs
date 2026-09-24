//! THE VENUE'S `ebills` IMAGE: its configuration, the poller's state, the
//! owner's crosswalk and the codes the till has been seen to sell. A `Table`
//! (one record per key), held by the venue's own object -- so a second venue
//! is a second image, never a second row in something shared.
//!
//! WHY NOT THE SETTINGS IMAGE. Its key space is closed and declared
//! (`settings/known.rs`), it is read on every order a customer places, and a
//! session cookie that rotates would rewrite it under them. This image is
//! touched by the poller and the owner's crosswalk screen and nothing else.
//!
//! WHY PER-VENUE CREDENTIALS, NOT A WORKER SECRET (card item 4). The platform
//! is one Worker for every venue (a subdomain each); an `EBILLS_PASSWORD`
//! secret would bind the whole platform to one venue's till. The password is
//! kept here at rest the way the settings image keeps S3 and Meta secrets
//! (`dowiz_hub::settings`'s header says why that is the honest arrangement),
//! is written by the owner's route and never read back by any answer.

use super::client::Session;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) const IMAGE: &str = "ebills";
/// The live floor, its own small image: rewritten only when a table changes,
/// never appended to the order log (§3.2 (5)).
pub(crate) const FLOOR_IMAGE: &str = "floor";
/// A few hundred item codes and one state record fit many times over.
pub(crate) const CEILING: usize = 512 * 1024;

pub(crate) const K_CONFIG: &str = "config";
pub(crate) const K_STATE: &str = "state";
pub(crate) const K_MAP: &str = "map";
pub(crate) const K_SEEN: &str = "seen";
pub(crate) const ONE: &str = "venue";

/// How many refusals and unjoined bills are kept: the newest, for the owner.
pub(crate) const KEEP: usize = 30;
/// A bill that has found no courses for this long is given up on, loudly.
pub(crate) const PENDING_FOR_MS: i64 = 2 * 24 * 3_600_000;

pub(crate) const MINUTE: i64 = 60_000;
/// Cron jitter: a minute cron may fire a few seconds early.
const SLACK: i64 = 5_000;
pub(crate) const SALES_OPEN_MS: i64 = 5 * MINUTE;
pub(crate) const SALES_CLOSED_MS: i64 = 30 * MINUTE;
pub(crate) const REREAD_MS: i64 = 24 * 60 * MINUTE;
const BACKOFF_CAP_MS: i64 = 60 * MINUTE;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Config {
    pub(crate) enabled: bool,
    pub(crate) pos_id: i64,
    pub(crate) user: String,
    #[serde(default)]
    pub(crate) secret: String,
}

impl Config {
    pub(crate) fn usable(&self) -> bool {
        self.enabled && self.pos_id > 0 && !self.user.is_empty() && !self.secret.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Noted {
    pub(crate) at_ms: i64,
    pub(crate) sale_id: i64,
    pub(crate) why: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct PendingBill {
    pub(crate) sale_id: i64,
    pub(crate) paid: Value,
    pub(crate) since_ms: i64,
}

/// A course from BEFORE the first window (a "lead"): kept aside, placed only
/// if a bill in the window turns out to cover it (`import::decide`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct PendingLead {
    pub(crate) sale_id: i64,
    pub(crate) envelope: Value,
    pub(crate) since_ms: i64,
}

/// How many ids before the first window a venue's first run reads, at most:
/// a sitting's courses are rung up within a few dozen ids of its bill
/// (measured 2026-09-23: 55 ids a day, the widest sitting 9 ids apart).
pub(crate) const LEAD_IDS: i64 = 40;
/// Closed courses re-checked per firing by the daily pass (§6.6).
pub(crate) const RECHECK_BUDGET: u32 = 10;
/// Refused sales read again per firing while they may still fiscalise.
pub(crate) const RETRY_BUDGET: usize = 5;

/// A refusal that time can undo: a sale not yet fiscalised or not yet
/// closed (a till that lost the tax authority issues offline and sends
/// within 48 h). `why` is `MapError`'s `Debug`, as `poll.rs` records it.
pub(crate) fn transient(why: &str) -> bool {
    why.starts_with("NotFiscalised") || why.starts_with("NotFinished")
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct State {
    /// Every sale id at or below this has been handled (§4).
    pub(crate) watermark: i64,
    /// Ids above the watermark were left for lack of budget: poll again now.
    pub(crate) backlog: bool,
    pub(crate) last_sales_ms: i64,
    pub(crate) last_reread_ms: i64,
    pub(crate) last_ok_ms: i64,
    pub(crate) last_error: Option<Noted>,
    /// Consecutive failed firings; drives the backoff.
    pub(crate) failures: u32,
    /// MFA or several tenants: nothing unattended can fix it. Cleared by the
    /// owner saving the configuration again.
    pub(crate) halted: bool,
    pub(crate) placed: u64,
    pub(crate) noted: u64,
    pub(crate) paid: u64,
    pub(crate) refused: Vec<Noted>,
    pub(crate) pending: Vec<PendingBill>,
    /// Supplies a served dish drove below zero (§6.4), from the last import.
    pub(crate) short: Vec<(String, i64)>,
    pub(crate) session: Option<Session>,
    /// Ids below this were read as LEADS on the first run (0: not yet).
    pub(crate) lead_below: i64,
    pub(crate) leads: Vec<PendingLead>,
    /// The daily re-check of closed courses: the next id, and the last.
    pub(crate) recheck_from: i64,
    pub(crate) recheck_until: i64,
}

impl State {
    /// Keep the newest `KEEP` of a list: it is a window for the owner, not a log.
    pub(crate) fn trim(&mut self) {
        let cut = self.refused.len().saturating_sub(KEEP);
        self.refused.drain(0..cut);
        let cut = self.pending.len().saturating_sub(KEEP);
        self.pending.drain(0..cut);
        let cut = self.leads.len().saturating_sub(2 * LEAD_IDS as usize);
        self.leads.drain(0..cut);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Mapping {
    pub(crate) product_id: String,
    pub(crate) at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Seen {
    pub(crate) name: String,
    pub(crate) price: i64,
    pub(crate) at_ms: i64,
}

/// WHAT THIS FIRING SHOULD DO, decided by the object from its own state --
/// the Worker does not keep a clock of its own for any of this.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct Plan {
    pub(crate) enabled: bool,
    pub(crate) pos_id: i64,
    pub(crate) user: String,
    pub(crate) secret: String,
    pub(crate) session: Option<Session>,
    pub(crate) watermark: i64,
    pub(crate) floor: bool,
    pub(crate) sales: bool,
    pub(crate) reread: bool,
    /// The venue's `YYYY-MM-DD` for the list window: yesterday..today, and
    /// a week back for the daily re-read (§4, §6.6).
    pub(crate) today: String,
    pub(crate) yesterday: String,
    pub(crate) week_ago: String,
    /// See `State::lead_below`.
    pub(crate) lead_below: i64,
    /// A re-check pass is under way: read closed courses from `recheck_from`.
    pub(crate) recheck: bool,
    pub(crate) recheck_from: i64,
    pub(crate) recheck_until: i64,
    /// Sales refused for a reason time can undo, read again (`transient`).
    pub(crate) retry: Vec<i64>,
}

/// The backoff after `n` failed firings: 1, 2, 4 … minutes, capped at an hour.
pub(crate) fn backoff_ms(n: u32) -> i64 {
    if n == 0 {
        return 0;
    }
    (MINUTE << (n - 1).min(6)).min(BACKOFF_CAP_MS)
}

/// The cadence (§4): the floor every minute while open, the sales every five
/// minutes while open and every thirty while closed (the last bills of the
/// night land after closing), at once while a backlog remains, the week's
/// re-read once a day. Nothing while halted, disabled, or backing off.
/// `local_ms` is `now` on the venue's wall clock.
pub(crate) fn plan(cfg: Option<&Config>, st: &State, now_ms: i64, open: bool, local_ms: i64) -> Plan {
    let Some(cfg) = cfg.filter(|c| c.usable()) else { return Plan::default() };
    let day = |back: i64| super::time::day_of(local_ms - back * 24 * 60 * MINUTE);
    let mut p = Plan {
        enabled: true,
        pos_id: cfg.pos_id,
        user: cfg.user.clone(),
        secret: cfg.secret.clone(),
        session: st.session.clone(),
        watermark: st.watermark,
        today: day(0),
        yesterday: day(1),
        week_ago: day(7),
        lead_below: st.lead_below,
        recheck_from: st.recheck_from,
        recheck_until: st.recheck_until,
        ..Plan::default()
    };
    let waited = st.last_error.as_ref().map_or(i64::MAX, |e| now_ms - e.at_ms);
    if st.halted || (st.failures > 0 && waited + SLACK < backoff_ms(st.failures)) {
        return p;
    }
    let every = if open { SALES_OPEN_MS } else { SALES_CLOSED_MS };
    p.floor = open;
    p.sales = st.backlog || now_ms - st.last_sales_ms + SLACK >= every;
    p.reread = now_ms - st.last_reread_ms + SLACK >= REREAD_MS;
    p.recheck = st.recheck_from > 0 && st.recheck_from <= st.recheck_until;
    // THE SAME WINDOW A BILL WAITS: a sale still unfiscalised after two days
    // stays refused, by name, and is read no more.
    let fresh = |n: &&Noted| n.sale_id > 0 && transient(&n.why) && now_ms - n.at_ms <= PENDING_FOR_MS;
    p.retry = if p.sales { st.refused.iter().filter(fresh).map(|n| n.sale_id).take(RETRY_BUDGET).collect() } else { Vec::new() };
    p
}

/// A name as a comparison key: lower case, accents and punctuation gone,
/// runs of space collapsed. `"Sake Nigiri (2 copë)"` and `"sake nigiri 2 cope"`
/// are one key.
pub(crate) fn norm(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    // The catalogue stores `&` escaped (`J&amp;B`, measured on the live menu).
    for c in s.replace("&amp;", "&").to_lowercase().chars() {
        let c = match c {
            'ë' | 'è' | 'é' | 'ê' => 'e',
            'ç' | 'č' | 'ć' => 'c',
            'à' | 'á' | 'â' | 'ä' => 'a',
            'ò' | 'ó' | 'ô' | 'ö' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            c if c.is_alphanumeric() => c,
            _ => ' ',
        };
        if c != ' ' || !out.ends_with(' ') {
            out.push(c);
        }
    }
    out.trim().to_string()
}

/// A SUGGESTION, never a mapping (card item 3): the catalogue's products
/// whose normalised name equals the till's, with whether the price agrees.
/// `products` is `(id, name, price)`. Exact name + price first.
pub(crate) fn suggest(name: &str, price: i64, products: &[(String, String, i64)]) -> Vec<(String, bool)> {
    let key = norm(name);
    let mut out: Vec<(String, bool)> = products
        .iter()
        .filter(|(_, n, _)| !key.is_empty() && norm(n) == key)
        .map(|(id, _, p)| (id.clone(), *p == price))
        .collect();
    out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    out
}

#[cfg(test)]
mod tests;

/// One typed record of the image. A record that is THERE and does not parse
/// is an error, never `None`: a state read as "absent" would restart the
/// watermark at zero and re-import a week of sales.
pub(crate) fn get<T: serde::de::DeserializeOwned>(t: &dowiz_hub::table::Table, kind: &str, id: &str) -> Result<Option<T>, String> {
    match t.get(kind, id) {
        None => Ok(None),
        Some(j) => serde_json::from_str(&j).map(Some).map_err(|e| format!("ebills {kind}/{id} is unreadable: {e}")),
    }
}

pub(crate) fn put<T: Serialize>(t: &mut dowiz_hub::table::Table, kind: &str, id: &str, v: &T) -> Result<(), String> {
    let j = serde_json::to_string(v).map_err(|e| format!("ebills {kind}: {e}"))?;
    t.put(kind, id, &j, &[], &[]).map_err(|e| format!("ebills {kind}: {e}"))
}
