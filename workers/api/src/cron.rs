//! THE MINUTE CRON, SPLIT BY VENUE AND RUN INSIDE DURABLE OBJECTS.
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
//! So the Worker now does only what is cheap: read the registry and send one
//! request per venue to that venue's RUNNER object (`cron~<venue>`, a separate
//! instance of the same class, so it can call the venue's own object without
//! calling itself). The runner does the parsing under a Durable Object's
//! budget, which is seconds, not milliseconds.

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

/// The Worker's half: one cheap request per venue.
pub async fn minute(env: &Env, now_ms: i64) {
    let registry = match crate::identity_store::registry(env).await {
        Ok(t) => t,
        Err(e) => return console_error!("cron: registry unreadable: {e}"),
    };
    let Ok(ns) = env.durable_object("HUB") else {
        return console_error!("cron: no HUB binding");
    };
    for (venue, _) in registry.all(crate::identity_store::K_LOC) {
        let sent = async {
            let stub = ns.id_from_name(&runner_name(&venue))?.get_stub()?;
            let req = Request::new_with_init(&runner_path(&venue, now_ms), RequestInit::new().with_method(Method::Post))?;
            stub.fetch_with_request(req).await
        }
        .await;
        match sent {
            Ok(r) if r.status_code() == 200 => {}
            Ok(r) => console_error!("cron {venue}: runner answered {}", r.status_code()),
            Err(e) => console_error!("cron {venue}: runner unreachable: {e}"),
        }
    }
}

/// The runner's half, inside the `cron~<venue>` object: the three jobs the
/// minute cron always did, for this one venue, in the same order.
pub async fn run(env: &Env, venue: &str, now_ms: i64) {
    crate::outbox::drain_venue(env, venue, now_ms).await;
    crate::ebills::poll::tick_venue(env, venue, now_ms).await;
    // AFTER the poll, never beside it: one session per venue per minute.
    crate::fiscal::rail::venue_minute(env, venue, now_ms).await;
}

#[cfg(test)]
mod tests;
