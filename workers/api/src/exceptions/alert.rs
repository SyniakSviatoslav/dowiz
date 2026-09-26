//! THE EVENT ALERT: a count of exception EVENTS at a venue in a period crossed
//! the owner's threshold, and the owner's own chat is told which events and
//! who signed each (BLUEPRINT-OPERATIONAL-BLIND-SPOTS §2.5).
//!
//! A COUNT PER VENUE-PERIOD, NEVER PER PERSON. The alert counts one kind of
//! event at the venue — "3 voids after the kitchen in this till period" — and
//! lists the rows. It carries no per-person total, no rank, no tier: the line
//! OD-8 draws is a persisted or sorted number per participant, and this never
//! computes one.
//!
//! PURE. The object calls [`due`] after a turn that may have written an
//! exception and puts what it returns into the venue's `outbox`, where the
//! minute cron's existing drain delivers kind `telegram` to `to`. The recipient
//! is the venue's OWN chat (`notify.telegram.chat`) — the consent gate's
//! allowed `&e.to`, not a customer.
//!
//! ONCE PER CROSSING, NOT PER TURN. A kind alerts when the rows written in THIS
//! turn (their `at` is the turn's `now_ms`) carry its count across a multiple
//! of the threshold; the entry id names the period start, the kind and the
//! level, so a turn replayed by the idempotency layer overwrites its own entry.

use super::fold::Row;
use crate::outbox::Entry;

/// Owner settings. Read with `Settings::get`, so an unset key takes the
/// default here; see the hand-back for the `KNOWN` entries that let the
/// settings pane show them.
pub const THRESHOLD_KEY: &str = "alerts.exceptions.threshold";
pub const LATE_KEY: &str = "alerts.exceptions.late_min";
pub const DEFAULT_THRESHOLD: usize = 3;
pub const DEFAULT_LATE_MIN: i64 = 30;
/// The period when no till is open: the last day.
pub const WINDOW_MS: i64 = 24 * 60 * 60 * 1000;
/// How many rows one message lists.
const LISTED: usize = 10;

/// The threshold an owner set: a count, `0` = alerts off. Unreadable = default.
pub fn threshold(raw: Option<&str>) -> usize {
    raw.and_then(|s| s.trim().parse().ok()).unwrap_or(DEFAULT_THRESHOLD)
}

/// Minutes after placement past which an amendment is reported.
pub fn late_ms(raw: Option<&str>) -> i64 {
    raw.and_then(|s| s.trim().parse::<i64>().ok()).filter(|m| *m > 0).unwrap_or(DEFAULT_LATE_MIN) * 60_000
}

/// The alerts this turn owes. `rows` is every exception row; `since` is the
/// period's start (the open till's `opened_at`, else `now_ms - WINDOW_MS`).
pub fn due(rows: &[Row], since: i64, now_ms: i64, threshold: usize, venue: &Voice, chat: &str) -> Vec<Entry> {
    if threshold == 0 || chat.trim().is_empty() {
        return Vec::new();
    }
    let mut kinds: Vec<&'static str> = rows.iter().map(|r| r.kind).collect();
    kinds.sort_unstable();
    kinds.dedup();
    let mut out = Vec::new();
    for kind in kinds {
        let mine: Vec<&Row> = rows.iter().filter(|r| r.kind == kind && r.at >= since && r.at <= now_ms).collect();
        let after = mine.len();
        let before = mine.iter().filter(|r| r.at < now_ms).count();
        if after / threshold <= before / threshold {
            continue;
        }
        let level = (after / threshold) * threshold;
        // A legacy chat gets the venue's language as today; a route (`@...`,
        // once the venue has groups) carries one text per language.
        out.push(crate::notify::route::alert_entry(
            format!("exceptions/{since}/{kind}/{level}"),
            chat.trim(),
            &|l| text(&venue.speaking(l), kind, &mine, since),
            now_ms,
        ));
    }
    out
}

/// Who the message speaks for: the venue's name, its zone (the period start is
/// shown in venue-local time) and its language — the venue record's
/// `default_locale` when it is one of the console's three, else English.
pub struct Voice<'a> {
    pub venue: &'a str,
    pub zone: dowiz_hub::tz::Zone,
    pub lang: &'a str,
}

impl<'a> Voice<'a> {
    /// The same voice in `lang`; empty = the venue's own.
    pub fn speaking(&self, lang: &'a str) -> Voice<'a> {
        Voice { venue: self.venue, zone: self.zone, lang: if lang.is_empty() { self.lang } else { lang } }
    }
}

/// (header words, "since", "by", "and N more"), per language.
fn words(lang: &str) -> [&'static str; 4] {
    match lang {
        "sq" => ["përjashtime", "që nga", "nga", "të tjera te"],
        "uk" => ["винятків", "з", "від", "ще у"],
        "ru" => ["исключений", "с", "от", "ещё в"],
        _ => ["exceptions", "since", "by", "more in"],
    }
}

/// A kind's name in the message's language (the pane's own words).
pub fn kind_word(lang: &str, kind: &str) -> &'static str {
    let i = [super::fold::VOID_AFTER_KITCHEN, super::fold::COMP, super::fold::LATE_AMENDMENT, super::fold::REFUND,
        super::fold::CASH_OUTSIDE_TILL, super::fold::PAY_OUT_KIND, super::fold::OVER_SHORT,
        super::legs::LEG_MISSING, super::legs::LEG_REFUSED, super::legs::LEG_MISMATCHED, super::legs::LEG_ORPHAN]
        .iter()
        .position(|k| *k == kind)
        .unwrap_or(0);
    let names = match lang {
        "sq" => ["anulim pas kuzhinës", "dhuratë", "ndryshim i vonë", "rimbursim", "para jashtë arkës", "pagesë nga arka", "diferencë arke",
            "mungon hapi i portofolit", "hapi i portofolit i refuzuar", "hapi i portofolit nuk përputhet", "hap portofoli pa pagesë"],
        "uk" => ["скасування після кухні", "комплімент", "пізня зміна", "повернення", "готівка поза касою", "виплата з каси", "розбіжність каси",
            "немає списання з гаманця", "списання з гаманця відмовлено", "списання з гаманця не збігається", "списання з гаманця без оплати"],
        "ru" => ["отмена после кухни", "комплимент", "поздняя правка", "возврат", "наличные вне кассы", "выплата из кассы", "расхождение кассы",
            "нет списания с кошелька", "списание с кошелька отклонено", "списание с кошелька не совпадает", "списание с кошелька без оплаты"],
        _ => ["void after kitchen", "comp", "late amendment", "refund", "cash outside a till", "pay-out", "till over/short",
            "wallet leg missing", "wallet leg refused", "wallet leg mismatched", "wallet leg orphan"],
    };
    names[i]
}

/// `YYYY-MM-DD HH:MM` in the venue's zone (Hinnant's civil_from_days).
pub fn local_stamp(zone: dowiz_hub::tz::Zone, utc_ms: i64) -> String {
    let ms = dowiz_hub::tz::local_ms(zone, utc_ms);
    let (days, rem) = (ms.div_euclid(86_400_000), ms.rem_euclid(86_400_000) / 60_000);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}", rem / 60, rem % 60)
}

/// The message: the count, then one line per event with its signer.
pub fn text(v: &Voice, kind: &str, rows: &[&Row], since: i64) -> String {
    let [head, from, by, more] = words(v.lang);
    let mut t = format!("{}: {} {head} · {} · {from} {}\n", v.venue, rows.len(), kind_word(v.lang, kind), local_stamp(v.zone, since));
    for r in rows.iter().rev().take(LISTED) {
        let what = r.order_id.as_deref().or(r.till_id.as_deref()).or(r.tx_id.as_deref()).unwrap_or("-");
        let cur = r.currency.as_deref().unwrap_or("");
        let why = r.reason.as_deref().unwrap_or("-");
        t.push_str(&format!("- {what}: {} {cur}, {why}, {by} {}\n", r.amount, r.by));
    }
    if rows.len() > LISTED {
        t.push_str(&format!("... {} {more} /api/owner/exceptions\n", rows.len() - LISTED));
    }
    t
}

#[cfg(test)]
mod tests;
