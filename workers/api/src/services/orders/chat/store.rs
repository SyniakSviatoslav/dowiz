//! THE MESSAGES, in the venue's own `threads` image beside the venue's
//! conversations (`social.rs`), under their own kind with the ORDER as the
//! subject: one append per message, the Worker's clock as the time, and the
//! three rules that bound a thread -- length, rate, and one message per
//! client id. PURE where it can be; the two image reads are at the bottom.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

/// The kind under which a courier chat message is appended. Declared in the
/// personal-data registry (`privacy/registry/venue.rs`, image `threads`).
pub const K_CC: &str = "cc";
/// The venue's own thread kind beside it (`social::K_MSG`), erased together.
pub const K_VENUE_THREAD: &str = "m";
/// A message is at most this long: a sentence to a courier, not a letter.
pub const MAX_CHARS: usize = 500;
/// Per order, per side, per rolling hour.
pub const RATE_PER_HOUR: usize = 30;
pub const HOUR_MS: i64 = 60 * 60 * 1000;
/// A closed chat stays readable this long, then goes with the order's
/// personal data (operator 2026-10-02).
pub const KEEP_MS: i64 = 30 * 24 * HOUR_MS;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    pub from: String,
    pub text: String,
    pub at_ms: i64,
}

/// The text as it will be stored, or why it will not be.
pub fn clean(text: &str) -> Result<String, &'static str> {
    let t = text.trim();
    if t.is_empty() {
        return Err("a message needs some text");
    }
    if t.chars().count() > MAX_CHARS {
        return Err("a message is at most 500 characters");
    }
    Ok(t.to_string())
}

/// One id per (order, client id): a resend is the same message.
pub fn message_id(order_id: &str, client_id: &str) -> String {
    format!("cm_{}", &crate::auth::sha256_hex(&format!("{order_id}:{client_id}"))[..16])
}

/// How many `from` sent in the hour before `now_ms`.
pub fn sent_in_last_hour(rows: &[Row], from: &str, now_ms: i64) -> usize {
    rows.iter().filter(|r| r.from == from && r.at_ms > now_ms - HOUR_MS).count()
}

/// Oldest first, as a reader shows them. STABLE: two lines at the same
/// instant keep the order they were appended in, which is the only truth
/// there is about two same-millisecond messages.
pub fn sorted(mut rows: Vec<Row>) -> Vec<Row> {
    rows.sort_by_key(|r| r.at_ms);
    rows
}

/// The rows of one order's chat, oldest first (`about` answers newest first).
pub async fn load(place: &crate::hubstore::Place, order_id: &str) -> Result<Vec<Row>> {
    let log = crate::hubstore::load_log(place, crate::social::IMAGE_THREADS).await?.log;
    Ok(sorted(
        log.about(K_CC, Some(order_id), usize::MAX).into_iter().rev().filter_map(|e| serde_json::from_str::<Row>(&e.json).ok()).collect(),
    ))
}

/// Append one message under its order.
pub async fn append(place: &crate::hubstore::Place, order_id: &str, row: &Row) -> Result<()> {
    let (subject, json) = (order_id.to_string(), serde_json::to_string(row).unwrap_or_default());
    crate::hubstore::with_log(place, crate::social::IMAGE_THREADS, move |log| {
        log.append(K_CC, &subject, &json).map_err(|e| Error::RustError(format!("chat: {e:?}")))
    })
    .await
}

/// The image rebuilt, or nothing when nothing went.
pub type Rebuilt = (Option<dowiz_hub::logimage::LogImage>, usize);

/// THE PRUNE IS A REBUILD (an append-only image forgets nothing in place):
/// the image written back holds every entry `keep` says yes to, oldest first,
/// and nothing else. `None` when every entry stays, so nothing is rewritten.
pub fn rebuilt(log: &dowiz_hub::logimage::LogImage, keep: impl Fn(&dowiz_hub::logimage::Entry) -> bool)
    -> std::result::Result<Rebuilt, String>
{
    let all = log.entries(); // newest first
    let kept: Vec<_> = all.iter().filter(|e| keep(e)).collect();
    let dropped = all.len() - kept.len();
    if dropped == 0 {
        return Ok((None, 0));
    }
    let mut out = dowiz_hub::logimage::LogImage::create_sized(log.to_bytes().len()).map_err(|e| format!("{e:?}"))?;
    for e in kept.into_iter().rev() {
        out.append(&e.kind, &e.subject, &e.json).map_err(|e| format!("{e:?}"))?;
    }
    Ok((Some(out), dropped))
}

/// Everything said about these orders -- the courier chat AND the venue's own
/// thread -- gone with the person (`hubdo/forget.rs`).
pub fn without_orders(log: &dowiz_hub::logimage::LogImage, orders: &BTreeSet<String>) -> std::result::Result<Rebuilt, String> {
    rebuilt(log, |e| !((e.kind == K_CC || e.kind == K_VENUE_THREAD) && orders.contains(&e.subject)))
}

/// Courier chat older than `KEEP_MS` goes; everything else in the image stays.
pub fn pruned(log: &dowiz_hub::logimage::LogImage, now_ms: i64) -> std::result::Result<Rebuilt, String> {
    let before = now_ms - KEEP_MS;
    rebuilt(log, |e| {
        e.kind != K_CC
            || serde_json::from_str::<Value>(&e.json).ok().and_then(|v| v.get("at_ms").and_then(Value::as_i64)).map_or(true, |at| at >= before)
    })
}

/// The nightly's call (cloud.rs, per venue): how many messages went.
pub async fn prune_at(stub: &Stub, now_ms: i64) -> Result<usize> {
    crate::platform_store::with_log_at(stub, crate::social::IMAGE_THREADS, move |log| {
        let (next, dropped) = pruned(log, now_ms).map_err(Error::RustError)?;
        if let Some(next) = next {
            *log = next;
        }
        Ok(dropped)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use dowiz_hub::logimage::LogImage;

    fn image() -> LogImage {
        let mut log = LogImage::create_sized(64 * 1024).unwrap();
        let row = |id: &str, from: &str, at: i64| serde_json::to_string(&Row { id: id.into(), from: from.into(), text: "hi".into(), at_ms: at }).unwrap();
        log.append(K_CC, "o1", &row("a", "CUSTOMER", 1_000)).unwrap();
        log.append(K_CC, "o1", &row("b", "COURIER", 2_000)).unwrap();
        log.append(K_CC, "o2", &row("c", "CUSTOMER", 3_000)).unwrap();
        log.append(K_VENUE_THREAD, "o1", r#"{"id":"m1"}"#).unwrap();
        log.append("other", "o1", r#"{"id":"x"}"#).unwrap();
        log
    }

    #[test]
    fn the_text_is_trimmed_bounded_and_never_empty() {
        assert_eq!(clean("  at the gate "), Ok("at the gate".to_string()));
        assert!(clean("   ").is_err());
        assert!(clean(&"x".repeat(MAX_CHARS)).is_ok());
        assert!(clean(&"x".repeat(MAX_CHARS + 1)).is_err());
        assert!(clean(&"é".repeat(MAX_CHARS)).is_ok(), "characters, not bytes");
    }

    #[test]
    fn one_id_per_client_id_and_the_rate_counts_one_side_in_one_hour() {
        assert_eq!(message_id("o1", "c1"), message_id("o1", "c1"));
        assert_ne!(message_id("o1", "c1"), message_id("o2", "c1"));
        let rows: Vec<Row> = (0..40).map(|i| Row { id: format!("{i}"), from: if i % 2 == 0 { "CUSTOMER" } else { "COURIER" }.into(), text: "x".into(), at_ms: 10_000 + i * 60_000 }).collect();
        let now = 10_000 + 40 * 60_000;
        assert_eq!(sent_in_last_hour(&rows, "CUSTOMER", now), 20, "every customer line is inside the hour");
        assert_eq!(sent_in_last_hour(&rows, "CUSTOMER", now + HOUR_MS), 0, "an hour later none is");
        assert_eq!(sent_in_last_hour(&rows, "COURIER", now + 30 * 60_000), 15, "half an hour on, the first ten minutes have left the window");
    }

    #[test]
    fn a_forgotten_orders_lines_go_and_everything_else_stays() {
        let log = image();
        let (next, dropped) = without_orders(&log, &["o1".to_string()].into_iter().collect()).unwrap();
        assert_eq!(dropped, 3, "two chat lines and the venue's thread about o1");
        let next = next.expect("rebuilt");
        assert_eq!(next.about(K_CC, Some("o1"), 9).len(), 0);
        assert_eq!(next.about(K_CC, Some("o2"), 9).len(), 1, "another order's chat stays");
        assert_eq!(next.about("other", None, 9).len(), 1, "an unrelated kind stays");
        assert_eq!(next.about(K_VENUE_THREAD, Some("o1"), 9).len(), 0);
        let (none, n) = without_orders(&log, &["o9".to_string()].into_iter().collect()).unwrap();
        assert!(none.is_none() && n == 0, "nothing to drop writes nothing");
    }

    #[test]
    fn chat_older_than_thirty_days_is_pruned_and_the_rest_is_kept_oldest_first() {
        let log = image();
        let (next, dropped) = pruned(&log, 2_000 + KEEP_MS).unwrap();
        assert_eq!(dropped, 1, "only the line at 1000 is past thirty days");
        let next = next.expect("rebuilt");
        let o1 = sorted(next.about(K_CC, Some("o1"), 9).into_iter().rev().filter_map(|e| serde_json::from_str::<Row>(&e.json).ok()).collect());
        assert_eq!(o1.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), vec!["b"]);
        assert_eq!(next.about(K_VENUE_THREAD, None, 9).len(), 1, "the venue's thread is not this prune's");
        assert!(matches!(pruned(&log, 3_000 + KEEP_MS - 1).unwrap(), (Some(_), 2)));
        assert!(matches!(pruned(&log, 500).unwrap(), (None, 0)));
    }
}
