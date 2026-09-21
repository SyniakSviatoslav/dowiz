//! Errors that survive log sampling.
//!
//! `[observability] head_sampling_rate = 0.1` cut the largest line on the bill
//! and, with it, nine out of ten traces -- including the one that carried the
//! failure. Worse, nothing on this box can read Workers Logs: no token here
//! holds the permission, so a failure that lives only in a trace is a failure
//! nobody will ever see. This module is the other half of that trade: the error
//! is ALSO a record, the venue's console can show it, and it costs one image
//! append on a path that has already gone wrong.
//!
//! NO SQL. The rows were `worker_errors` in D1; they are now an append-only
//! bebop image, per venue, in that venue's own object -- which is also what
//! makes the tenant boundary a property of where the bytes are rather than a
//! `WHERE venue = ?` somebody has to remember.

use worker::*;

/// How long a record is kept. The nightly cron prunes the rest.
pub const KEEP_MS: i64 = 7 * 24 * 60 * 60 * 1000;

/// And how many, which is the bound that actually binds.
///
/// AN APPEND LOG NEEDS A COUNT, NOT ONLY AN AGE. A burst of failures inside one
/// day is exactly when this instrument matters and exactly when an age-only
/// rule keeps everything: the old `DELETE ... WHERE at_ms < ?` would have held
/// a week of a crash loop. Five hundred short records is a few tens of KiB.
pub const KEEP_MOST: usize = 500;

/// Longer than this and it is a stack trace, not a message.
const MAX_MESSAGE: usize = 500;

/// The record kind inside the audit image.
pub const KIND: &str = "error";

/// Record one failure. `place` says WHERE in the code, in dotted form
/// (`notify.telegram`, `cloud.nightly`), so a console can group by it.
///
/// WHERE IT LANDS. A venue's failure goes in THAT VENUE'S audit image, and a
/// platform-level one goes in the platform object's. That is not tidiness: a
/// venue's console reads its own errors, and an error table shared by every
/// venue is one more place a tenant boundary has to be remembered. Here the
/// boundary is which object the bytes are in.
///
/// WRITTEN BEST-EFFORT AND NEVER FAILING THE CALLER. A handler that is already
/// reporting one failure must not become a second, different failure because
/// the log would not take the record. The write is awaited -- a Worker may be
/// cut off at the end of a response and an unawaited write is the thing that
/// gets dropped -- but its result is only logged.
pub async fn record(ns: &ObjectNamespace, venue: Option<&str>, place: &str, message: &str) {
    let short: String = message.chars().take(MAX_MESSAGE).collect();
    let object = venue.unwrap_or(crate::platform_store::PLATFORM);
    let rec = serde_json::json!({
        "atMs": Date::now().as_millis() as i64,
        "place": place,
        "message": short,
    })
    .to_string();
    let stub = match ns.id_from_name(object).and_then(|id| id.get_stub()) {
        Ok(s) => s,
        Err(e) => {
            console_log!("errlog: no object for {object}: {e}");
            return;
        }
    };
    let image = if venue.is_some() {
        crate::hubstore::IMAGE_AUDIT
    } else {
        crate::platform_store::ERRORS
    };
    let subject = place.to_string();
    if let Err(e) = crate::platform_store::with_log_at(&stub, image, move |log| {
        log.append(KIND, &subject, &rec)
            .map_err(|x| Error::RustError(format!("{x:?}")))?;
        // PRUNED ON WRITE, not only nightly. The nightly job is what keeps the
        // age rule; this is what keeps a crash loop from filling the image
        // between two nights.
        if log.len() > KEEP_MOST + KEEP_MOST / 4 {
            log.keep(KEEP_MOST).map_err(|x| Error::RustError(format!("{x:?}")))?;
        }
        Ok(())
    })
    .await
    {
        console_log!("errlog: could not record {place}: {e}");
    }
}

/// Log it AND record it, in the caller's own words.
///
/// `loud!(&place.ns, Some(&place.venue), "notify.telegram", "refused: {e}")`
/// prints exactly what `console_error!` printed before and adds the record.
/// Only usable in an async function, on purpose: a fire-and-forget write is the
/// one that gets dropped when the isolate goes away.
#[macro_export]
macro_rules! loud {
    ($ns:expr, $venue:expr, $place:expr, $($arg:tt)*) => {{
        let __msg = format!($($arg)*);
        worker::console_error!("{}: {}", $place, __msg);
        $crate::errlog::record($ns, $venue, $place, &__msg).await;
    }};
}

/// The newest failures, for the console and for an operator.
pub async fn recent(
    ns: &ObjectNamespace,
    venue: &str,
    limit: usize,
) -> Result<Vec<serde_json::Value>> {
    let stub = ns.id_from_name(venue)?.get_stub()?;
    let loaded = crate::platform_store::load_log_at(&stub, crate::hubstore::IMAGE_AUDIT).await?;
    Ok(loaded
        .log
        .about(KIND, None, limit)
        .into_iter()
        .filter_map(|e| serde_json::from_str::<serde_json::Value>(&e.json).ok())
        .collect())
}

/// Drop what is older than `KEEP_MS`, and anything past `KEEP_MOST`.
///
/// Called by the nightly cron, per venue, where the object is already in hand.
pub async fn prune_at(stub: &Stub, now_ms: i64) -> Result<usize> {
    let before = now_ms - KEEP_MS;
    crate::platform_store::with_log_at(stub, crate::hubstore::IMAGE_AUDIT, move |log| {
        let keep: usize = log
            .about(KIND, None, usize::MAX)
            .into_iter()
            .take_while(|e| {
                serde_json::from_str::<serde_json::Value>(&e.json)
                    .ok()
                    .and_then(|v| v.get("atMs").and_then(serde_json::Value::as_i64))
                    .map_or(true, |at| at >= before)
            })
            .count();
        // `about` is newest first, so `take_while` counts the run of records
        // that are still young enough -- and the moment one is too old, every
        // record after it is older still.
        log.keep(keep.min(KEEP_MOST)).map_err(|x| Error::RustError(format!("{x:?}")))
    })
    .await
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
