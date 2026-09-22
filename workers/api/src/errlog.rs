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

/// WHERE A FAILURE'S RECORDS LIVE — decided ONCE, for the writer, the reader
/// and the prune.
///
/// THE DEFECT THAT PUT THIS HERE, and it made the whole instrument write-only.
/// `record` chose `platform_store::ERRORS` for a platform failure and
/// `hubstore::IMAGE_AUDIT` for a venue's. `recent` and `prune_at` read and
/// pruned `IMAGE_AUDIT` unconditionally. So EVERY 500 in the Worker -- which is
/// the one thing `lib.rs` instruments, and the reason this module exists -- was
/// appended to an image with no reader anywhere in the crate, and `worker.500`
/// records could not appear in any console. The predecessor of this module was
/// a D1 table that never received a row; its replacement received them and put
/// them where nobody could look.
///
/// Two callers agreeing by hand is how that happens. One function, called by
/// all three, is the only fix that cannot drift back.
pub fn image_of(venue: Option<&str>) -> &'static str {
    match venue {
        Some(_) => crate::hubstore::IMAGE_AUDIT,
        None => crate::platform_store::ERRORS,
    }
}

/// And which object holds it: the venue's own, or the platform's.
pub fn object_of(venue: Option<&str>) -> &str {
    venue.unwrap_or(crate::platform_store::PLATFORM)
}

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
    let object = object_of(venue);
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
    let image = image_of(venue);
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

/// What one read of the audit image says: the newest failures, and every
/// record in it this build cannot read.
///
/// THE TWO COME FROM ONE LOAD, on purpose. The image is already fetched for
/// the console, and a second fetch for the quarantine would put a request on
/// the bill for bytes that are already in hand. An instrument that costs extra
/// is an instrument that gets switched off.
pub struct Recent {
    pub errors: Vec<serde_json::Value>,
    pub quarantined: Vec<dowiz_hub::Quarantined>,
}

impl Default for Recent {
    /// WHAT AN UNREADABLE IMAGE ANSWERS. No errors and no quarantine -- which
    /// is the same shape as a healthy venue, and is why the caller logs the
    /// failure rather than letting this stand in for a measurement.
    fn default() -> Self {
        Recent { errors: Vec::new(), quarantined: Vec::new() }
    }
}

/// The newest failures, for the console and for an operator.
pub async fn recent(ns: &ObjectNamespace, venue: Option<&str>, limit: usize) -> Result<Recent> {
    let stub = ns.id_from_name(object_of(venue))?.get_stub()?;
    let loaded = crate::platform_store::load_log_at(&stub, image_of(venue)).await?;
    Ok(Recent {
        errors: loaded
            .log
            .about(KIND, None, limit)
            .into_iter()
            .filter_map(|e| serde_json::from_str::<serde_json::Value>(&e.json).ok())
            .collect(),
        quarantined: loaded.log.quarantined(),
    })
}

/// Drop what is older than `KEEP_MS`, and anything past `KEEP_MOST`.
///
/// Called by the nightly cron, per venue AND once for the platform, where the
/// object is already in hand. `venue` picks the image the same way the write
/// did -- see `image_of`, and why guessing it twice was the defect.
pub async fn prune_at(stub: &Stub, venue: Option<&str>, now_ms: i64) -> Result<usize> {
    let before = now_ms - KEEP_MS;
    crate::platform_store::with_log_at(stub, image_of(venue), move |log| {
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
    /// THE ONE THAT WOULD HAVE CAUGHT IT. `record` chose the image from the
    /// venue; `recent` and `prune_at` used `IMAGE_AUDIT` whatever they were
    /// given. A platform failure was therefore written to `errors` and read
    /// from `audit`, so the entire `worker.500` class -- every 500 this crate
    /// instruments -- went into an image with no reader.
    ///
    /// The assertion is not "the two constants differ". It is that a PLATFORM
    /// failure does not land in a VENUE's image, which is the sentence the old
    /// code got wrong.
    #[test]
    fn a_platform_failure_is_not_written_to_a_venues_image() {
        assert_ne!(
            super::image_of(None),
            crate::hubstore::IMAGE_AUDIT,
            "a platform failure must not be filed under a venue's audit image"
        );
        assert_eq!(super::image_of(None), crate::platform_store::ERRORS);
        assert_eq!(super::image_of(Some("sushi-durres")), crate::hubstore::IMAGE_AUDIT);
    }

    /// And the object follows the image. Reading the platform's errors out of
    /// a venue's object would answer an empty list from a healthy venue --
    /// which is the same shape as "no failures" and is why this is asserted
    /// rather than assumed.
    #[test]
    fn the_object_follows_the_venue_and_the_platform_has_its_own() {
        assert_eq!(super::object_of(None), crate::platform_store::PLATFORM);
        assert_eq!(super::object_of(Some("sushi-durres")), "sushi-durres");
        assert_ne!(super::object_of(None), super::object_of(Some("sushi-durres")));
    }

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
