//! THE SUMMARIES (W-TG T4: `digest.daily`, `digest.weekly`) AS RECURRING
//! OUTBOX ENTRIES.
//!
//! A summary is an ordinary entry of kind [`KIND`] whose `next_at_ms` is the
//! group's next summary time: the minute sweep already reads every venue's
//! outbox, so a due summary costs no extra read to notice. When it is due the
//! drain folds the orders (`services::analytics::fold`, the pane's own
//! numbers), adds the lines the group chose to receive "in the summary"
//! (record kind `route::DIGEST_KIND`), sends it, and moves the entry on to
//! the next day -- it is never removed by being sent.
//!
//! RECONCILED, NOT WRITTEN BY THE CONSOLE. The owner's save writes the
//! settings only (one image per handler); the drain, which already holds both
//! the settings and the outbox, puts in what is missing and takes out what the
//! owner switched off. The cost of that choice, written down: a summary
//! switched on at a venue where nothing at all happens is noticed at the next
//! event, not at the next minute.

use serde_json::Value;

use super::Entry;
use crate::notify::route::{self, Group};
use dowiz_hub::tz::Zone;

/// The entry kind of a summary.
pub const KIND: &str = "digest";

/// The summary entries these groups should have: id `digest/{group}/{daily|weekly}`,
/// `to` the group, `text` = `{which}@{minute}` so a changed time is a changed entry.
pub fn wanted(groups: &[Group], zone: Zone, now_ms: i64) -> Vec<Entry> {
    route::dues(groups, zone, now_ms)
        .into_iter()
        .filter_map(|(key, next)| {
            let (gid, which) = key.rsplit_once('/')?;
            let at = groups.iter().find(|g| g.id == gid)?.digest_at;
            let mut e = Entry::new(format!("digest/{key}"), KIND, gid.to_string(), format!("{which}@{at}"), now_ms);
            e.next_at_ms = next;
            Some(e)
        })
        .collect()
}

/// What to put and what to remove so the outbox holds exactly `want`. An
/// entry already there with the same text keeps its schedule.
pub fn reconcile(have: &[&Entry], want: &[Entry]) -> (Vec<Entry>, Vec<String>) {
    let put = want.iter().filter(|w| !have.iter().any(|h| h.id == w.id && h.text == w.text)).cloned().collect();
    let remove = have.iter().filter(|h| !want.iter().any(|w| w.id == h.id)).map(|h| h.id.clone()).collect();
    (put, remove)
}

/// Is this summary the weekly one?
pub fn weekly(e: &Entry) -> bool {
    e.text.starts_with("weekly")
}

/// The digest lines waiting for `gid`, oldest first, and their record ids.
pub fn lines_for(records: &[(String, String)], gid: &str) -> (Vec<String>, Vec<String>) {
    let prefix = format!("{gid}/");
    let mut mine: Vec<(i64, String, String)> = records
        .iter()
        .filter(|(k, _)| k.starts_with(&prefix))
        .filter_map(|(k, v)| {
            let v: Value = serde_json::from_str(v).ok()?;
            Some((v.get("at").and_then(Value::as_i64).unwrap_or(0), v.get("line").and_then(Value::as_str)?.to_string(), k.clone()))
        })
        .collect();
    mine.sort();
    (mine.iter().map(|m| m.1.clone()).collect(), mine.into_iter().map(|m| m.2).collect())
}

/// One digest line as it is stored.
pub fn line_record(line: &str, at_ms: i64) -> String {
    serde_json::json!({ "at": at_ms, "line": line }).to_string()
}

/// The numbers of the last day (or week) up to `now_ms`, from the folded
/// orders; `name` turns a product id into its name.
pub fn summary(venue: &str, orders: &[Value], zone: Zone, now_ms: i64, weekly: bool, currency: &str, name: &dyn Fn(&str) -> String) -> route::render::Summary {
    let span = if weekly { 7 } else { 1 } * 86_400_000;
    let r = crate::services::analytics::fold::fold(orders, zone, &[now_ms - span], now_ms);
    route::render::Summary {
        venue: venue.to_string(),
        orders: r.orders,
        revenue: crate::notify::money_text(r.revenue, currency),
        top: r.top_products.iter().take(5).map(|d| (name(&d.id), d.quantity)).collect(),
    }
}

#[cfg(test)]
mod tests;
