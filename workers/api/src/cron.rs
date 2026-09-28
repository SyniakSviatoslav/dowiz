//! THE VENUE'S TIMED WORK, RUN INSIDE DURABLE OBJECTS -- WOKEN BY ALARMS.
//!
//! The Free plan gives a Worker invocation 10 ms of CPU, and a cron is an
//! invocation. The minute cron used to drain the outbox, poll e-bills and send
//! fiscal receipts for EVERY venue inside that one invocation, parsing each
//! venue's images in the Worker. At four venues it crossed 10 ms (measured
//! 2026-09-26: `exceededResources` at exactly 10.0 ms from 18:00 UTC, the hour
//! a fourth venue was created), was killed mid-flight, and the isolate it died
//! in answered ordinary requests with 1101 in 0.4 ms until it was recycled --
//! the storefront menu failed about half the time.
//!
//! So the work runs in the venue's RUNNER object (`cron~<venue>`, a separate
//! instance of the same class, so it can call the venue's own object without
//! calling itself), under a Durable Object's budget, which is seconds, not
//! milliseconds.
//!
//! AND NOTHING WAKES IT ON A CLOCK ANY MORE (DAG Phase 2, 2026-09-28). The
//! `* * * * *` cron is gone: the venue's own object sets an ALARM for the
//! instant its work is due (`timer`, `hubdo/timer.rs`), and its `alarm()` asks
//! the runner for one run. An idle venue sets no alarm and costs nothing. The
//! nightly keeps one job here: `nightly` asks every venue's object whether it
//! has work due and no alarm -- a lost alarm -- and re-arms it.

use worker::*;

/// The runner object's name for a venue. Never a venue id itself: `~` is not
/// in any location id, so a runner can never be mistaken for a venue's hub.
pub fn runner_name(venue: &str) -> String {
    format!("cron~{venue}")
}

/// The request path a runner answers, with the venue and the one clock read.
pub fn runner_path(venue: &str, now_ms: i64) -> String {
    Url::parse_with_params("https://cron/fold/cron", &[("venue", venue), ("now", &now_ms.to_string())])
        .map(|u| u.to_string())
        .unwrap_or_default()
}

/// `(venue, now_ms)` back out of a runner request's query, or `None` when
/// either is missing or the clock is not a number.
pub fn parse_runner_query(pairs: impl Iterator<Item = (String, String)>) -> Option<(String, i64)> {
    let (mut venue, mut now) = (None, None);
    for (k, v) in pairs {
        match k.as_str() {
            "venue" if !v.is_empty() => venue = Some(v),
            "now" => now = v.parse::<i64>().ok(),
            _ => {}
        }
    }
    Some((venue?, now?))
}

/// THE NIGHTLY SAFETY NET, the Worker's half: one request per venue to its
/// own object (`/fold/timer`), which re-arms an alarm that was lost while work
/// is due. One line counts the night; each lost alarm is loud in its venue's
/// log, because a lost alarm is a message that waited up to a day.
pub async fn nightly(env: &Env, now_ms: i64) {
    let registry = match crate::identity_store::registry(env).await {
        Ok(t) => t,
        Err(e) => return console_error!("timers: registry unreadable: {e}"),
    };
    let Ok(ns) = env.durable_object("HUB") else {
        return console_error!("timers: no HUB binding");
    };
    let mut tally = timer::Tally::default();
    for (venue, _) in registry.all(crate::identity_store::K_LOC) {
        let seen = async {
            let stub = ns.id_from_name(&venue)?.get_stub()?;
            let req = Request::new_with_init(&timer_path(&venue, now_ms), RequestInit::new().with_method(Method::Post))?;
            let mut res = stub.fetch_with_request(req).await?;
            match res.status_code() {
                200 => res.json::<timer::Seen>().await,
                s => Err(Error::RustError(format!("answered {s}"))),
            }
        }
        .await
        .map_err(|e| e.to_string());
        if seen == Ok(timer::Seen::Rearmed) {
            crate::loud!(&ns, Some(&venue), "timer.rearmed", "work was due and no alarm was set; re-armed by the nightly");
        }
        tally.add(&venue, seen);
    }
    console_log!("{}", tally.line());
}

/// The path a venue's own object answers the nightly on (`hubdo/timer.rs`).
/// The venue travels with it so the object can learn -- and check -- its name.
pub fn timer_path(venue: &str, now_ms: i64) -> String {
    Url::parse_with_params("https://hub/fold/timer", &[("venue", venue), ("now", &now_ms.to_string())])
        .map(|u| u.to_string())
        .unwrap_or_default()
}

/// The runner's half, inside the `cron~<venue>` object: the three jobs the
/// minute cron always did, for this one venue, in the same order. Called when
/// the venue's alarm fires (`hubdo/timer.rs`).
pub async fn run(env: &Env, venue: &str, now_ms: i64) {
    crate::outbox::drain_venue(env, venue, now_ms).await;
    crate::ebills::poll::tick_venue(env, venue, now_ms).await;
    // AFTER the poll, never beside it: one session per venue per minute.
    crate::fiscal::rail::venue_minute(env, venue, now_ms).await;
}

/// When the work is due, and what to do with the alarm (pure).
pub mod timer;

#[cfg(test)]
mod tests;
