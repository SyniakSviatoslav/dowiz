//! Errors that survive log sampling.
//!
//! `[observability] head_sampling_rate = 0.1` cut the largest line on the bill
//! and, with it, nine out of ten traces -- including the one that carried the
//! failure. Worse, nothing on this box can read Workers Logs: no token here
//! holds the permission, so a failure that lives only in a trace is a failure
//! nobody will ever see. This module is the other half of that trade: the error
//! is ALSO a row, the venue's console can show it, and it costs one D1 write on
//! a path that has already gone wrong.
//!
//! WRITTEN BEST-EFFORT AND NEVER FAILING THE CALLER. A handler that is already
//! reporting one failure must not turn into a second, different failure because
//! the error table would not take the row. The write is awaited -- a Worker may
//! be cut off at the end of a response and an unawaited insert would be the
//! thing that gets dropped -- but its result is only logged.

use worker::*;

/// How long a row is kept. The nightly cron deletes the rest.
pub const KEEP_MS: i64 = 7 * 24 * 60 * 60 * 1000;

/// Longer than this and it is a stack trace, not a message.
const MAX_MESSAGE: usize = 500;

/// Record one failure. `place` says WHERE in the code, in dotted form
/// (`notify.telegram`, `cloud.nightly`), so a console can group by it.
pub async fn record(db: &D1Database, venue: Option<&str>, place: &str, message: &str) {
    let short: String = message.chars().take(MAX_MESSAGE).collect();
    let venue_js = match venue {
        Some(v) => worker::wasm_bindgen::JsValue::from_str(v),
        None => worker::wasm_bindgen::JsValue::NULL,
    };
    let stmt = db
        .prepare("INSERT INTO worker_errors (at_ms, venue, place, message) VALUES (?1,?2,?3,?4)")
        .bind(&[
            worker::wasm_bindgen::JsValue::from_f64(Date::now().as_millis() as f64),
            venue_js,
            place.into(),
            short.into(),
        ]);
    match stmt {
        // The table may not exist yet on a deployment whose migration has not
        // run. That is not worth a second error; the console_error! the caller
        // already emitted stands.
        Ok(s) => {
            if let Err(e) = s.run().await {
                console_log!("errlog: could not record {place}: {e}");
            }
        }
        Err(e) => console_log!("errlog: bad statement for {place}: {e}"),
    }
}

/// Log it AND record it, in the caller's own words.
///
/// `loud!(db, Some(&venue), "notify.telegram", "refused: {e}")` prints exactly
/// what `console_error!` printed before and adds the row. Only usable in an
/// async function, on purpose: a fire-and-forget write is the one that gets
/// dropped when the isolate goes away.
#[macro_export]
macro_rules! loud {
    ($db:expr, $venue:expr, $place:expr, $($arg:tt)*) => {{
        let __msg = format!($($arg)*);
        worker::console_error!("{}: {}", $place, __msg);
        $crate::errlog::record($db, $venue, $place, &__msg).await;
    }};
}

/// The newest failures, for the console and for an operator. Venue-scoped when
/// a venue is given; a NULL-venue row is platform-wide and shown to nobody in
/// particular.
pub async fn recent(db: &D1Database, venue: &str, limit: u32) -> Result<Vec<serde_json::Value>> {
    #[derive(serde::Deserialize)]
    struct Row {
        at_ms: i64,
        place: String,
        message: String,
    }
    let rows: Vec<Row> = db
        .prepare(
            "SELECT at_ms, place, message FROM worker_errors \
             WHERE venue = ?1 ORDER BY at_ms DESC LIMIT ?2",
        )
        .bind(&[venue.into(), worker::wasm_bindgen::JsValue::from_f64(limit as f64)])?
        .all()
        .await?
        .results()?;
    Ok(rows
        .into_iter()
        .map(|r| serde_json::json!({ "atMs": r.at_ms, "place": r.place, "message": r.message }))
        .collect())
}

/// Delete what is older than `KEEP_MS`. Called by the nightly cron beside the
/// GPS prune.
pub async fn prune(db: &D1Database, now_ms: i64) {
    let before = now_ms - KEEP_MS;
    let stmt = db
        .prepare("DELETE FROM worker_errors WHERE at_ms < ?1")
        .bind(&[worker::wasm_bindgen::JsValue::from_f64(before as f64)]);
    match stmt {
        Ok(s) => match s.run().await {
            Ok(r) => console_log!(
                "nightly prune: worker errors older than 7 d removed ({:?})",
                r.meta().ok().flatten().and_then(|m| m.changes)
            ),
            Err(e) => console_error!("nightly prune: worker_errors refused: {e}"),
        },
        Err(e) => console_error!("nightly prune: bad worker_errors statement: {e}"),
    }
}

#[cfg(test)]
mod tests {
    /// The truncation is the only pure thing here and it is the one that can
    /// silently cost money: an unbounded message is a D1 row as large as
    /// whatever the failure printed.
    #[test]
    fn a_long_message_is_cut_to_five_hundred_characters() {
        let long = "e".repeat(5000);
        let short: String = long.chars().take(super::MAX_MESSAGE).collect();
        assert_eq!(short.len(), 500);
    }

    /// And the cut is by CHARACTER, not by byte: an Albanian or Ukrainian
    /// message cut mid-codepoint would not be valid UTF-8 at all.
    #[test]
    fn a_non_ascii_message_is_cut_without_breaking_a_character() {
        let long = "ї".repeat(5000);
        let short: String = long.chars().take(super::MAX_MESSAGE).collect();
        assert_eq!(short.chars().count(), 500);
        assert!(short.ends_with('ї'));
    }
}
