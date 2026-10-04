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
//! So the work runs inside the venue's own Durable Object, under an object's
//! budget, which is seconds, not milliseconds -- since W-LOOP (2026-10-03) in
//! the venue object's OWN alarm turn, with its reads answered in process
//! (`edge::Own`); the runner object `cron~<venue>` that used to run it, and read
//! the venue back ten times a firing, is no longer called.
//!
//! AND NOTHING WAKES IT ON A CLOCK ANY MORE (DAG Phase 2, 2026-09-28). The
//! `* * * * *` cron is gone: the venue's own object sets an ALARM for the
//! instant its work is due (`timer`, `hubdo/timer.rs`), and its `alarm()` runs
//! the jobs. An idle venue sets no alarm and costs nothing. The nightly asks the
//! platform object ONE question (`nightly`); the fan-out to the venues, their
//! lost-alarm check and their night run in object turns (`hubdo/timer/night.rs`).

use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

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

/// THE NIGHT, the Worker's half: ONE request, to the platform object, whatever
/// the number of venues (W-LOOP row 3). It fans the night out to the venues a
/// batch per alarm turn (`hubdo/timer/night.rs`); each venue re-arms a lost
/// alarm, then runs its own night (`cloud::night_venue`) in its own alarm.
pub async fn nightly(env: &Env, now_ms: i64) {
    let started = async {
        let stub = env.durable_object("HUB")?.id_from_name(crate::platform_store::PLATFORM)?.get_stub()?;
        let path = Url::parse_with_params(
            "https://hub/fold/timer",
            &[("venue", crate::platform_store::PLATFORM), ("now", &now_ms.to_string()), ("fan", "1")],
        )
        .map_err(|e| Error::RustError(e.to_string()))?;
        let mut res = stub.fetch_with_request(Request::new_with_init(path.as_str(), RequestInit::new().with_method(Method::Post))?).await?;
        match res.status_code() {
            200 => res.text().await,
            s => Err(Error::RustError(format!("the platform answered {s}"))),
        }
    }
    .await;
    match started {
        Ok(v) => log_line!("night started: {v}"),
        Err(e) => log_error!("night: the fan-out did not start: {e}"),
    }
}

/// The path a venue's object is told its night is due on, by the platform's
/// fan-out: the safety net's question plus `night=1`.
pub fn night_path(venue: &str, now_ms: i64) -> String {
    Url::parse_with_params("https://hub/fold/timer", &[("venue", venue), ("now", &now_ms.to_string()), ("night", "1")])
        .map(|u| u.to_string())
        .unwrap_or_default()
}

/// The venue's timed jobs: the three the minute cron always did, for this one
/// venue, in the same order. Called in the venue object's own alarm turn
/// (`hubdo/timer.rs`) with an `Env` that answers the venue in process.
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

/// The minute loop, counted over a whole venue day (W-LOOP row 1).
#[cfg(test)]
#[path = "cron/cadence/tests.rs"]
pub(crate) mod cadence_tests;

/// The night fan-out and the venue's own alarm turn, natively (W-COV C2, W-LOOP).
#[cfg(test)]
#[path = "cron/routes/tests.rs"]
mod route_tests;
